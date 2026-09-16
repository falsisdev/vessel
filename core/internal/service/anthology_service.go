package service

import (
	"context"
	"encoding/json"
	"fmt"
	"net/http"
	"sync"
	"time"

	pluginv1 "github.com/falsisdev/vessel/proto/gen/go/plugin/v1"
)

// AnthologyProvider represents a provider defined in the Anthology manifest.
type AnthologyProvider struct {
	ID          string `json:"id"`
	Name        string `json:"name"`
	Description string `json:"description"`
	URL         string `json:"url"`
	Domain      string `json:"domain"`
}

// AnthologyManifest is the structure of the manifest fetched from GitHub.
type AnthologyManifest struct {
	Version   string               `json:"version"`
	Providers []AnthologyProvider `json:"providers"`
}

// AnthologyService implements the Vessel plugin contract by bridging to the Anthology ecosystem.
type AnthologyService struct {
	httpClient *http.Client
	manifest   *AnthologyManifest
	mu         sync.RWMutex
}

// NewAnthologyService initializes a new bridge to the Anthology ecosystem.
func NewAnthologyService() *AnthologyService {
	return &AnthologyService{
		httpClient: &http.Client{Timeout: 10 * time.Second},
	}
}

// LoadManifest fetches the latest providers from the Anthology GitHub repository.
func (s *AnthologyService) LoadManifest() error {
	// This URL should be the raw path to the anthology manifest.json on GitHub.
	// For now, we use a placeholder that the user can refine.
	url := "https://raw.githubusercontent.com/falsisdev/anthology/main/manifest.json"

	resp, err := s.httpClient.Get(url)
	if err != nil {
		return fmt.Errorf("failed to fetch anthology manifest: %w", err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusOK {
		return fmt.Errorf("anthology manifest returned status %d", resp.StatusCode)
	}

	var m AnthologyManifest
	if err := json.NewDecoder(resp.Body).Decode(&m); err != nil {
		return fmt.Errorf("failed to decode anthology manifest: %w", err)
	}

	s.mu.Lock()
	s.manifest = &m
	s.mu.Unlock()

	return nil
}

// Search implements the pluginv1.Search capability.
func (s *AnthologyService) Search(ctx context.Context, query string, limit int32) ([]*pluginv1.MediaItem, error) {
	s.mu.RLock()
	manifest := s.manifest
	s.mu.RUnlock()

	if manifest == nil {
		return nil, fmt.Errorf("anthology manifest not loaded")
	}

	var allItems []*pluginv1.MediaItem

	// In a real implementation, we would query each provider's API.
	// For this adapter, we simulate the aggregation logic.
	for _, p := range manifest.Providers {
		// Simulate provider search
		items := s.searchProvider(ctx, p, query, limit)
		allItems = append(allItems, items...)
	}

	return allItems, nil
}

// GetMetadata implements the pluginv1.GetMetadata capability.
func (s *AnthologyService) GetMetadata(ctx context.Context, mediaID string) (*pluginv1.MediaDetails, error) {
	// Use the mediaID to determine the provider (e.g., providerID:mediaID)
	// and fetch details from the corresponding Anthology provider.
	return &pluginv1.MediaDetails{
		Id:    mediaID,
		Title: "Anthology Media",
		// ... translation logic ...
	}, nil
}

// GetStreams implements the pluginv1.GetStreams capability.
func (s *AnthologyService) GetStreams(ctx context.Context, mediaID string, season, episode int32) ([]*pluginv1.StreamSource, []*pluginv1.Subtitle, error) {
	// Translate Anthology stream sources to Vessel StreamSources.
	return []*pluginv1.StreamSource{}, []*pluginv1.Subtitle{}, nil
}

// searchProvider is a helper to query a specific Anthology provider.
func (s *AnthologyService) searchProvider(ctx context.Context, p AnthologyProvider, query string, limit int32) []*pluginv1.MediaItem {
	// Implementation details for querying the specific provider URL.
	return []*pluginv1.MediaItem{}
}

// Manifest returns the Vessel plugin manifest for this bridge.
func (s *AnthologyService) Manifest() *pluginv1.PluginManifest {
	return &pluginv1.PluginManifest{
		Id:           "com.vessel.bridge.anthology",
		Name:         "Anthology",
		Description:  "Turkish Media Bridge - Aggregates content from the Anthology ecosystem",
		Author:       "Vessel Team",
		Domain:       pluginv1.Domain_DOMAIN_UNSPECIFIED, // Supports multiple
		Capabilities: []pluginv1.Capability{
			pluginv1.Capability_CAPABILITY_SEARCH,
			pluginv1.Capability_CAPABILITY_METADATA,
			pluginv1.Capability_CAPABILITY_STREAMS,
		},
		IsBuiltin:    true,
		ProtocolVersion: "1.0.0",
	}
}

// Close implements the plugin.Client interface.
func (s *AnthologyService) Close() error {
	return nil
}
