package plugin_test

import (
	"context"
	"testing"

	"github.com/falsisdev/vessel/core/internal/plugin"
	pluginv1 "github.com/falsisdev/vessel/proto/gen/go/plugin/v1"
)

type dummyClient struct {
	manifest *pluginv1.PluginManifest
	closed   bool
}

func (d *dummyClient) Manifest() *pluginv1.PluginManifest {
	return d.manifest
}

func (d *dummyClient) Search(ctx context.Context, query string, page int32) (*pluginv1.SearchResponse, error) {
	return &pluginv1.SearchResponse{}, nil
}

func (d *dummyClient) GetMetadata(ctx context.Context, mediaID string) (*pluginv1.GetMetadataResponse, error) {
	return &pluginv1.GetMetadataResponse{}, nil
}

func (d *dummyClient) GetStreams(ctx context.Context, mediaID string, season, episode int32) (*pluginv1.GetStreamsResponse, error) {
	return &pluginv1.GetStreamsResponse{}, nil
}

func (d *dummyClient) Close() error {
	d.closed = true
	return nil
}

func TestManagerLifecycle(t *testing.T) {
	mgr := plugin.NewManager()

	c1 := &dummyClient{
		manifest: &pluginv1.PluginManifest{
			Id:           "p1",
			Domain:       pluginv1.Domain_DOMAIN_CINEMA,
			Capabilities: []pluginv1.Capability{pluginv1.Capability_CAPABILITY_SEARCH},
		},
	}

	c2 := &dummyClient{
		manifest: &pluginv1.PluginManifest{
			Id:           "p2",
			Domain:       pluginv1.Domain_DOMAIN_MANGA,
			Capabilities: []pluginv1.Capability{pluginv1.Capability_CAPABILITY_SEARCH},
		},
	}

	if err := mgr.Register(c1); err != nil {
		t.Fatalf("unexpected register error: %v", err)
	}
	if err := mgr.Register(c2); err != nil {
		t.Fatalf("unexpected register error: %v", err)
	}

	// Test duplicate registration error
	if err := mgr.Register(c1); err == nil {
		t.Fatal("expected error on duplicate registration")
	}

	// Test retrieval
	got, err := mgr.Get("p1")
	if err != nil || got != c1 {
		t.Fatalf("failed to retrieve registered client: %v", err)
	}

	// Test domain and capability filter
	cinemaSearchers := mgr.ListByDomainAndCapability(pluginv1.Domain_DOMAIN_CINEMA, pluginv1.Capability_CAPABILITY_SEARCH)
	if len(cinemaSearchers) != 1 || cinemaSearchers[0] != c1 {
		t.Fatalf("expected 1 cinema searcher, got %d", len(cinemaSearchers))
	}

	// Test close all
	if err := mgr.Close(); err != nil {
		t.Fatalf("unexpected close error: %v", err)
	}

	if !c1.closed || !c2.closed {
		t.Fatal("expected all clients to be closed")
	}

	if len(mgr.ListAll()) != 0 {
		t.Fatal("expected manager to be empty after close")
	}
}
