package plugin

import (
	"context"
	"fmt"
	"time"

	pluginv1 "github.com/falsisdev/vessel/proto/gen/go/plugin/v1"
	"google.golang.org/grpc"
	"google.golang.org/grpc/credentials/insecure"
)

type Client interface {
	Manifest() *pluginv1.PluginManifest
	Search(ctx context.Context, query string, page int32) (*pluginv1.SearchResponse, error)
	GetMetadata(ctx context.Context, mediaID string) (*pluginv1.GetMetadataResponse, error)
	GetStreams(ctx context.Context, mediaID string, season, episode int32) (*pluginv1.GetStreamsResponse, error)
	Close() error
}

type GRPCClient struct {
	conn     *grpc.ClientConn
	service  pluginv1.PluginServiceClient
	manifest *pluginv1.PluginManifest
}

func Dial(ctx context.Context, target string, dialOpts ...grpc.DialOption) (*GRPCClient, error) {
	opts := []grpc.DialOption{
		grpc.WithTransportCredentials(insecure.NewCredentials()),
	}
	opts = append(opts, dialOpts...)

	conn, err := grpc.NewClient(target, opts...)
	if err != nil {
		return nil, fmt.Errorf("failed to dial plugin at %s: %w", target, err)
	}

	service := pluginv1.NewPluginServiceClient(conn)

	handshakeCtx, cancel := context.WithTimeout(ctx, 3*time.Second)
	defer cancel()

	resp, err := service.GetManifest(handshakeCtx, &pluginv1.GetManifestRequest{})
	if err != nil {
		_ = conn.Close()
		return nil, fmt.Errorf("failed to fetch manifest from plugin at %s: %w", target, err)
	}

	return &GRPCClient{
		conn:     conn,
		service:  service,
		manifest: resp.Manifest,
	}, nil
}

func (c *GRPCClient) Manifest() *pluginv1.PluginManifest {
	return c.manifest
}

func (c *GRPCClient) Search(ctx context.Context, query string, page int32) (*pluginv1.SearchResponse, error) {
	return c.service.Search(ctx, &pluginv1.SearchRequest{
		Query: query,
		Page:  page,
	})
}

func (c *GRPCClient) GetMetadata(ctx context.Context, mediaID string) (*pluginv1.GetMetadataResponse, error) {
	return c.service.GetMetadata(ctx, &pluginv1.GetMetadataRequest{
		MediaId: mediaID,
	})
}

func (c *GRPCClient) GetStreams(ctx context.Context, mediaID string, season, episode int32) (*pluginv1.GetStreamsResponse, error) {
	return c.service.GetStreams(ctx, &pluginv1.GetStreamsRequest{
		MediaId:       mediaID,
		SeasonNumber:  season,
		EpisodeNumber: episode,
	})
}

func (c *GRPCClient) Close() error {
	if c.conn != nil {
		return c.conn.Close()
	}
	return nil
}
