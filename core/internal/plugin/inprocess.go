package plugin

import (
	"context"

	pluginv1 "github.com/falsisdev/vessel/proto/gen/go/plugin/v1"
)

// InProcessClient wraps a pluginv1.PluginServiceServer directly without gRPC/IPC overhead.
type InProcessClient struct {
	server   pluginv1.PluginServiceServer
	manifest *pluginv1.PluginManifest
}

// NewInProcessClient creates a new InProcessClient for an in-process plugin server.
func NewInProcessClient(server pluginv1.PluginServiceServer) (*InProcessClient, error) {
	resp, err := server.GetManifest(context.Background(), &pluginv1.GetManifestRequest{})
	if err != nil {
		return nil, err
	}
	return &InProcessClient{
		server:   server,
		manifest: resp.Manifest,
	}, nil
}

func (c *InProcessClient) Manifest() *pluginv1.PluginManifest {
	return c.manifest
}

func (c *InProcessClient) Search(ctx context.Context, query string, page int32) (*pluginv1.SearchResponse, error) {
	return c.server.Search(ctx, &pluginv1.SearchRequest{
		Query: query,
		Page:  page,
	})
}

func (c *InProcessClient) GetMetadata(ctx context.Context, mediaID string) (*pluginv1.GetMetadataResponse, error) {
	return c.server.GetMetadata(ctx, &pluginv1.GetMetadataRequest{
		MediaId: mediaID,
	})
}

func (c *InProcessClient) GetStreams(ctx context.Context, mediaID string, season, episode int32) (*pluginv1.GetStreamsResponse, error) {
	return c.server.GetStreams(ctx, &pluginv1.GetStreamsRequest{
		MediaId:       mediaID,
		SeasonNumber:  season,
		EpisodeNumber: episode,
	})
}

func (c *InProcessClient) GetChapterContent(ctx context.Context, mediaID, chapterID string, chapterNumber float32) (*pluginv1.GetChapterContentResponse, error) {
	return c.server.GetChapterContent(ctx, &pluginv1.GetChapterContentRequest{
		MediaId:       mediaID,
		ChapterId:     chapterID,
		ChapterNumber: chapterNumber,
	})
}

func (c *InProcessClient) Close() error {
	return nil
}
