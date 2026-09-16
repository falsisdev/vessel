package plugin

import (
	"context"
	"encoding/json"
	"fmt"
	"net/http"

	pluginv1 "github.com/falsisdev/vessel/proto/gen/go/plugin/v1"
)

type StremioAdapter struct{}

type stremioManifest struct {
	Manifest struct {
		Id          string `json:"id"`
		Name        string `json:"name"`
		Description string `json:"description"`
		Resources   []struct {
			Name string `json:"name"`
			Types []string `json:"types"`
		} `json:"resources"`
	} `json:"manifest"`
}

func (a *StremioAdapter) GetManifest(ctx context.Context, manifestURL string) (*pluginv1.PluginManifest, error) {
	resp, err := http.Get(manifestURL)
	if err != nil {
		return nil, err
	}
	defer resp.Body.Close()

	var m stremioManifest
	if err := json.NewDecoder(resp.Body).Decode(&m); err != nil {
		return nil, err
	}

	return &pluginv1.PluginManifest{
		Id:          m.Manifest.Id,
		Name:        m.Manifest.Name,
		Description: m.Manifest.Description,
		Capabilities: []pluginv1.Capability{
			pluginv1.Capability_CAPABILITY_SEARCH,
			pluginv1.Capability_CAPABILITY_METADATA,
			pluginv1.Capability_CAPABILITY_STREAMS,
		},
		ProtocolVersion: "1.0.0",
	}, nil
}

func (a *StremioAdapter) Search(ctx context.Context, baseURL string, query string, page int32) (*pluginv1.SearchResponse, error) {
	// Stremio addons don't have a universal 'search' endpoint in the manifest.
	// Search is usually handled by querying specific catalogs.
	// For the bridge, we'll attempt a generic search if the addon supports it or return empty.
	return &pluginv1.SearchResponse{
		Items: []*pluginv1.MediaItem{},
	}, nil
}

func (a *StremioAdapter) GetMetadata(ctx context.Context, baseURL string, mediaID string) (*pluginv1.GetMetadataResponse, error) {
	// mediaID in Stremio is usually like 'tt1234567'
	// We need to determine the type (movie/series) from the ID or a convention.
	mediaType := "movie"
	if len(mediaID) > 0 && (mediaID[0] == 's' || mediaID[0] == 'e') {
		mediaType = "series"
	}

	endpoint := fmt.Sprintf("%s/meta/%s/%s.json", baseURL, mediaType, mediaID)
	resp, err := http.Get(endpoint)
	if err != nil {
		return nil, err
	}
	defer resp.Body.Close()

	var meta struct {
		Meta struct {
			Name        string `json:"name"`
			Description string `json:"description"`
			Type        string `json:"type"`
			Poster      string `json:"poster"`
			Background  string `json:"background"`
			Release     string `json:"release"`
		} `json:"meta"`
	}

	if err := json.NewDecoder(resp.Body).Decode(&meta); err != nil {
		return nil, err
	}

	return &pluginv1.GetMetadataResponse{
		Details: &pluginv1.MediaDetails{
			Id:    mediaID,
			Title: meta.Meta.Name,
			Overview: meta.Meta.Description,
			PosterUrl: meta.Meta.Poster,
		},
	}, nil
}

func (a *StremioAdapter) GetStreams(ctx context.Context, baseURL string, mediaID string, season, episode int32) (*pluginv1.GetStreamsResponse, error) {
	// Stremio stream endpoint: /stream/<type>/<id>/<extra>.json
	mediaType := "movie"
	extra := ""
	if season > 0 {
		mediaType = "series"
		extra = fmt.Sprintf("%d", season) // Simplification: Stremio uses internal IDs for episodes usually
	}

	endpoint := fmt.Sprintf("%s/stream/%s/%s/%s.json", baseURL, mediaType, mediaID, extra)
	resp, err := http.Get(endpoint)
	if err != nil {
		return nil, err
	}
	defer resp.Body.Close()

	var streamResp struct {
		Streams []struct {
			Title string `json:"title"`
			URL   string `json:"url"`
		} `json:"streams"`
	}

	if err := json.NewDecoder(resp.Body).Decode(&streamResp); err != nil {
		return nil, err
	}

	var sources []*pluginv1.StreamSource
	for _, s := range streamResp.Streams {
		sources = append(sources, &pluginv1.StreamSource{
			Title: s.Title,
			Url:   s.URL,
		})
	}

	return &pluginv1.GetStreamsResponse{
		Streams: sources,
	}, nil
}
