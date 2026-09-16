package plugin

import (
	"context"
	"encoding/json"
	"fmt"
	"net/http"

	pluginv1 "github.com/falsisdev/vessel/proto/gen/go/plugin/v1"
)

type CloudstreamAdapter struct{}

type cloudstreamManifest struct {
	Name        string `json:"name"`
	Version     string `json:"version"`
	Description string `json:"description"`
	Author      string `json:"author"`
}

func (a *CloudstreamAdapter) GetManifest(ctx context.Context, url string) (*pluginv1.PluginManifest, error) {
	resp, err := http.Get(url)
	if err != nil {
		return nil, err
	}
	defer resp.Body.Close()

	var m cloudstreamManifest
	if err := json.NewDecoder(resp.Body).Decode(&m); err != nil {
		return nil, err
	}

	return &pluginv1.PluginManifest{
		Id:          "com.vessel.bridge.cloudstream",
		Name:        m.Name,
		Description: m.Description,
		Author:      m.Author,
		Capabilities: []pluginv1.Capability{
			pluginv1.Capability_CAPABILITY_SEARCH,
			pluginv1.Capability_CAPABILITY_METADATA,
			pluginv1.Capability_CAPABILITY_STREAMS,
		},
		ProtocolVersion: "1.0.0",
	}, nil
}

func (a *CloudstreamAdapter) Search(ctx context.Context, url string, query string, page int32) (*pluginv1.SearchResponse, error) {
	// Cloudstream search is provider-specific.
	// In a bridge implementation, we would call the provided API's search endpoint.
	return &pluginv1.SearchResponse{
		Items: []*pluginv1.MediaItem{},
	}, nil
}

func (a *CloudstreamAdapter) GetMetadata(ctx context.Context, url string, mediaID string) (*pluginv1.GetMetadataResponse, error) {
	// Placeholder for Cloudstream metadata translation
	return &pluginv1.GetMetadataResponse{
		Details: &pluginv1.MediaDetails{
			Id:    mediaID,
			Title: "Cloudstream Media",
		},
	}, nil
}

func (a *CloudstreamAdapter) GetStreams(ctx context.Context, url string, mediaID string, season, episode int32) (*pluginv1.GetStreamsResponse, error) {
	// Placeholder for Cloudstream stream translation
	return &pluginv1.GetStreamsResponse{
		Streams: []*pluginv1.StreamSource{},
	}, nil
}
