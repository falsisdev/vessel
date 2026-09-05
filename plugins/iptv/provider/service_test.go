package provider_test

import (
	"context"
	"testing"

	"github.com/falsisdev/vessel/plugins/iptv/provider"
	pluginv1 "github.com/falsisdev/vessel/proto/gen/go/plugin/v1"
)

func TestIPTVService_GetManifest(t *testing.T) {
	svc := provider.NewIPTVService()
	resp, err := svc.GetManifest(context.Background(), &pluginv1.GetManifestRequest{})
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}

	if resp.Manifest.Id != provider.PluginID {
		t.Fatalf("expected id %s, got %s", provider.PluginID, resp.Manifest.Id)
	}
	if resp.Manifest.Domain != pluginv1.Domain_DOMAIN_IPTV {
		t.Fatalf("expected domain IPTV, got %v", resp.Manifest.Domain)
	}
}

func TestIPTVService_SearchAndStreams(t *testing.T) {
	svc := provider.NewIPTVService()

	// Test Search popular
	sResp, err := svc.Search(context.Background(), &pluginv1.SearchRequest{Query: "popular"})
	if err != nil {
		t.Fatalf("search error: %v", err)
	}
	if len(sResp.Items) == 0 {
		t.Fatal("expected channels in search results")
	}

	first := sResp.Items[0]
	// Test GetMetadata
	mResp, err := svc.GetMetadata(context.Background(), &pluginv1.GetMetadataRequest{MediaId: first.Id})
	if err != nil {
		t.Fatalf("metadata error: %v", err)
	}
	if mResp.Details.Title != first.Title {
		t.Fatalf("title mismatch: %s vs %s", mResp.Details.Title, first.Title)
	}

	// Test GetStreams
	streamsResp, err := svc.GetStreams(context.Background(), &pluginv1.GetStreamsRequest{MediaId: first.Id})
	if err != nil {
		t.Fatalf("streams error: %v", err)
	}
	if len(streamsResp.Streams) == 0 {
		t.Fatal("expected at least 1 stream source")
	}
	if streamsResp.Streams[0].Format != pluginv1.StreamFormat_STREAM_FORMAT_HLS {
		t.Fatalf("expected HLS format, got %v", streamsResp.Streams[0].Format)
	}
}
