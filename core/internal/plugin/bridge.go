package plugin

import (
	"context"
	"fmt"

	pluginv1 "github.com/falsisdev/vessel/proto/gen/go/plugin/v1"
)

// BridgeType identifies the external ecosystem the bridge connects to.
type BridgeType string

const (
	BridgeTypeNuvio       BridgeType = "nuvio"
	BridgeTypeStremio     BridgeType = "stremio"
	BridgeTypeCloudstream BridgeType = "cloudstream"
)

// BridgeAdapter defines the interface for translating an external ecosystem's API to Vessel's Proto models.
type BridgeAdapter interface {
	GetManifest(ctx context.Context, url string) (*pluginv1.PluginManifest, error)
	Search(ctx context.Context, url string, query string, page int32) (*pluginv1.SearchResponse, error)
	GetMetadata(ctx context.Context, url string, mediaID string) (*pluginv1.GetMetadataResponse, error)
	GetStreams(ctx context.Context, url string, mediaID string, season, episode int32) (*pluginv1.GetStreamsResponse, error)
}

// BridgeClient implements the plugin.Client interface by proxying requests to an external system via an adapter.
type BridgeClient struct {
	bType      BridgeType
	adapter    BridgeAdapter
	manifestURL string
	manifest    *pluginv1.PluginManifest
}

// NewBridgeClient creates a new bridge plugin based on the specified type and manifest URL.
func NewBridgeClient(bType BridgeType, manifestURL string) (*BridgeClient, error) {
	var adapter BridgeAdapter

	switch bType {
	case BridgeTypeNuvio:
		adapter = &NuvioAdapter{}
	case BridgeTypeStremio:
		adapter = &StremioAdapter{}
	case BridgeTypeCloudstream:
		adapter = &CloudstreamAdapter{}
	default:
		return nil, fmt.Errorf("unsupported bridge type: %s", bType)
	}

	client := &BridgeClient{
		bType:       bType,
		adapter:     adapter,
		manifestURL: manifestURL,
	}

	ctx := context.Background()
	m, err := adapter.GetManifest(ctx, manifestURL)
	if err != nil {
		return nil, fmt.Errorf("failed to load bridge manifest from %s: %w", manifestURL, err)
	}
	client.manifest = m

	return client, nil
}

// Type returns the bridge type (nuvio/stremio/cloudstream).
func (c *BridgeClient) Type() BridgeType {
	return c.bType
}

// ManifestURL returns the URL used to load the bridge manifest.
func (c *BridgeClient) ManifestURL() string {
	return c.manifestURL
}

func (c *BridgeClient) Manifest() *pluginv1.PluginManifest {
	return c.manifest
}

func (c *BridgeClient) Search(ctx context.Context, query string, page int32) (*pluginv1.SearchResponse, error) {
	return c.adapter.Search(ctx, c.manifestURL, query, page)
}

func (c *BridgeClient) GetMetadata(ctx context.Context, mediaID string) (*pluginv1.GetMetadataResponse, error) {
	return c.adapter.GetMetadata(ctx, c.manifestURL, mediaID)
}

func (c *BridgeClient) GetStreams(ctx context.Context, mediaID string, season, episode int32) (*pluginv1.GetStreamsResponse, error) {
	return c.adapter.GetStreams(ctx, c.manifestURL, mediaID, season, episode)
}

func (c *BridgeClient) GetChapterContent(ctx context.Context, mediaID, chapterID string, chapterNumber float32) (*pluginv1.GetChapterContentResponse, error) {
	return nil, fmt.Errorf("chapter content not supported by bridge plugins")
}

func (c *BridgeClient) Close() error {
	return nil
}
