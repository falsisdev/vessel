package client

import (
	"context"
	"fmt"
	"strings"
	"time"

	corev1 "github.com/falsisdev/vessel/proto/gen/go/core/v1"
	pluginv1 "github.com/falsisdev/vessel/proto/gen/go/plugin/v1"
	"google.golang.org/grpc"
	"google.golang.org/grpc/credentials/insecure"
)

type Client struct {
	conn    *grpc.ClientConn
	service corev1.CoreServiceClient
}

func Dial(ctx context.Context, target string, dialOpts ...grpc.DialOption) (*Client, error) {
	opts := []grpc.DialOption{
		grpc.WithTransportCredentials(insecure.NewCredentials()),
	}
	opts = append(opts, dialOpts...)

	cleanTarget := target
	if strings.HasPrefix(target, "unix://") {
		cleanTarget = target
	} else if strings.Contains(target, "/") && !strings.Contains(target, ":") {
		cleanTarget = "unix://" + target
	}

	conn, err := grpc.NewClient(cleanTarget, opts...)
	if err != nil {
		return nil, fmt.Errorf("failed to dial core at %s: %w", target, err)
	}

	service := corev1.NewCoreServiceClient(conn)

	pingCtx, cancel := context.WithTimeout(ctx, 3*time.Second)
	defer cancel()

	if _, err := service.Ping(pingCtx, &corev1.PingRequest{}); err != nil {
		_ = conn.Close()
		return nil, fmt.Errorf("failed to ping core at %s: %w", target, err)
	}

	return &Client{
		conn:    conn,
		service: service,
	}, nil
}

func (c *Client) Ping(ctx context.Context) (*corev1.PingResponse, error) {
	return c.service.Ping(ctx, &corev1.PingRequest{})
}

func (c *Client) SearchMedia(ctx context.Context, domain pluginv1.Domain, query string, page int32) (*corev1.SearchMediaResponse, error) {
	return c.service.SearchMedia(ctx, &corev1.SearchMediaRequest{
		Domain: domain,
		Query:  query,
		Page:   page,
	})
}

func (c *Client) GetMediaDetails(ctx context.Context, domain pluginv1.Domain, providerID, mediaID string) (*corev1.GetMediaDetailsResponse, error) {
	return c.service.GetMediaDetails(ctx, &corev1.GetMediaDetailsRequest{
		Domain:     domain,
		ProviderId: providerID,
		MediaId:    mediaID,
	})
}

func (c *Client) GetStreams(ctx context.Context, providerID, mediaID string, season, episode int32) (*corev1.GetStreamsResponse, error) {
	return c.service.GetStreams(ctx, &corev1.GetStreamsRequest{
		ProviderId:    providerID,
		MediaId:       mediaID,
		SeasonNumber:  season,
		EpisodeNumber: episode,
	})
}

func (c *Client) GetChapterContent(ctx context.Context, providerID, mediaID, chapterID string, chapterNumber float32) (*corev1.GetChapterContentResponse, error) {
	return c.service.GetChapterContent(ctx, &corev1.GetChapterContentRequest{
		ProviderId:    providerID,
		MediaId:       mediaID,
		ChapterId:     chapterID,
		ChapterNumber: chapterNumber,
	})
}

func (c *Client) ListPlugins(ctx context.Context) (*corev1.ListPluginsResponse, error) {
	return c.service.ListPlugins(ctx, &corev1.ListPluginsRequest{})
}

func (c *Client) ListThemes(ctx context.Context) (*corev1.ListThemesResponse, error) {
	return c.service.ListThemes(ctx, &corev1.ListThemesRequest{})
}

func (c *Client) GetActiveTheme(ctx context.Context) (*corev1.GetActiveThemeResponse, error) {
	return c.service.GetActiveTheme(ctx, &corev1.GetActiveThemeRequest{})
}

func (c *Client) SetActiveTheme(ctx context.Context, themeID, variantID string) (*corev1.SetActiveThemeResponse, error) {
	return c.service.SetActiveTheme(ctx, &corev1.SetActiveThemeRequest{
		ThemeId:   themeID,
		VariantId: variantID,
	})
}

func (c *Client) GetSupportedLocales(ctx context.Context) (*corev1.GetSupportedLocalesResponse, error) {
	return c.service.GetSupportedLocales(ctx, &corev1.GetSupportedLocalesRequest{})
}

func (c *Client) Close() error {
	if c.conn != nil {
		return c.conn.Close()
	}
	return nil
}
