package plugin

import (
	"context"
	"encoding/json"
	"net/http"

	pluginv1 "github.com/falsisdev/vessel/proto/gen/go/plugin/v1"
)

type NuvioAdapter struct{}

type nuvioManifest struct {
	Name        string `json:"name"`
	Version     string `json:"version"`
	Author      string `json:"author"`
	Description string `json:"description"`
	Scrapers    []struct {
		ID          string   `json:"id"`
		Name        string   `json:"name"`
		Description string   `json:"description"`
		SupportedTypes []string `json:"supportedTypes"`
		Filename    string   `json:"filename"`
	} `json:"scrapers"`
}

func (a *NuvioAdapter) GetManifest(ctx context.Context, url string) (*pluginv1.PluginManifest, error) {
	resp, err := http.Get(url)
	if err != nil {
		return nil, err
	}
	defer resp.Body.Close()

	var m nuvioManifest
	if err := json.NewDecoder(resp.Body).Decode(&m); err != nil {
		return nil, err
	}

	return &pluginv1.PluginManifest{
		Id:          "com.vessel.bridge.nuvio",
		Name:        m.Name,
		Description: m.Description,
		Author:      m.Author,
		Domain:      pluginv1.Domain_DOMAIN_UNSPECIFIED,
		Capabilities: []pluginv1.Capability{
			pluginv1.Capability_CAPABILITY_SEARCH,
			pluginv1.Capability_CAPABILITY_METADATA,
			pluginv1.Capability_CAPABILITY_STREAMS,
		},
		IsBuiltin: true,
		ProtocolVersion: "1.0.0",
	}, nil
}

func (a *NuvioAdapter) Search(ctx context.Context, url string, query string, page int32) (*pluginv1.SearchResponse, error) {
	return &pluginv1.SearchResponse{
		Items: []*pluginv1.MediaItem{},
	}, nil
}

func (a *NuvioAdapter) GetMetadata(ctx context.Context, url string, mediaID string) (*pluginv1.GetMetadataResponse, error) {
	return &pluginv1.GetMetadataResponse{
		Details: &pluginv1.MediaDetails{
			Id:    mediaID,
			Title: "Nuvio Media",
		},
	}, nil
}

func (a *NuvioAdapter) GetStreams(ctx context.Context, url string, mediaID string, season, episode int32) (*pluginv1.GetStreamsResponse, error) {
	return &pluginv1.GetStreamsResponse{
		Streams: []*pluginv1.StreamSource{},
		Subtitles: []*pluginv1.Subtitle{},
	}, nil
}
