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

func (c *Client) SaveLibraryItem(ctx context.Context, item *corev1.LibraryItem) (*corev1.SaveLibraryItemResponse, error) {
	return c.service.SaveLibraryItem(ctx, &corev1.SaveLibraryItemRequest{Item: item})
}

func (c *Client) GetLibraryItem(ctx context.Context, providerID, mediaID string) (*corev1.GetLibraryItemResponse, error) {
	return c.service.GetLibraryItem(ctx, &corev1.GetLibraryItemRequest{
		ProviderId: providerID,
		MediaId:    mediaID,
	})
}

func (c *Client) ListLibraryItems(ctx context.Context, domain pluginv1.Domain, status corev1.LibraryStatus, limit, offset int32) (*corev1.ListLibraryItemsResponse, error) {
	return c.service.ListLibraryItems(ctx, &corev1.ListLibraryItemsRequest{
		Domain: domain,
		Status: status,
		Limit:  limit,
		Offset: offset,
	})
}

func (c *Client) DeleteLibraryItem(ctx context.Context, providerID, mediaID string) (*corev1.DeleteLibraryItemResponse, error) {
	return c.service.DeleteLibraryItem(ctx, &corev1.DeleteLibraryItemRequest{
		ProviderId: providerID,
		MediaId:    mediaID,
	})
}

func (c *Client) SavePlaybackProgress(ctx context.Context, p *corev1.PlaybackProgress) (*corev1.SavePlaybackProgressResponse, error) {
	return c.service.SavePlaybackProgress(ctx, &corev1.SavePlaybackProgressRequest{Progress: p})
}

func (c *Client) GetPlaybackProgress(ctx context.Context, providerID, mediaID string, season, episode int32) (*corev1.GetPlaybackProgressResponse, error) {
	return c.service.GetPlaybackProgress(ctx, &corev1.GetPlaybackProgressRequest{
		ProviderId:    providerID,
		MediaId:       mediaID,
		SeasonNumber:  season,
		EpisodeNumber: episode,
	})
}

func (c *Client) ListRecentPlaybackProgress(ctx context.Context, limit int32) (*corev1.ListRecentPlaybackProgressResponse, error) {
	return c.service.ListRecentPlaybackProgress(ctx, &corev1.ListRecentPlaybackProgressRequest{Limit: limit})
}

func (c *Client) SaveReadingProgress(ctx context.Context, p *corev1.ReadingProgress) (*corev1.SaveReadingProgressResponse, error) {
	return c.service.SaveReadingProgress(ctx, &corev1.SaveReadingProgressRequest{Progress: p})
}

func (c *Client) GetReadingProgress(ctx context.Context, providerID, mediaID, chapterID string) (*corev1.GetReadingProgressResponse, error) {
	return c.service.GetReadingProgress(ctx, &corev1.GetReadingProgressRequest{
		ProviderId: providerID,
		MediaId:    mediaID,
		ChapterId:  chapterID,
	})
}

func (c *Client) ListRecentReadingProgress(ctx context.Context, limit int32) (*corev1.ListRecentReadingProgressResponse, error) {
	return c.service.ListRecentReadingProgress(ctx, &corev1.ListRecentReadingProgressRequest{Limit: limit})
}

func (c *Client) ResolveStream(ctx context.Context, streamURL, title string, season, episode int32, preferredProvider string) (*corev1.ResolveStreamResponse, error) {
	return c.service.ResolveStream(ctx, &corev1.ResolveStreamRequest{
		StreamUrl:         streamURL,
		Title:             title,
		SeasonNumber:      season,
		EpisodeNumber:     episode,
		PreferredProvider: preferredProvider,
	})
}

func (c *Client) GetDebridStatus(ctx context.Context, provider string) (*corev1.GetDebridStatusResponse, error) {
	return c.service.GetDebridStatus(ctx, &corev1.GetDebridStatusRequest{Provider: provider})
}

func (c *Client) ConfigureDebrid(ctx context.Context, provider, apiKey string, enabled bool) (*corev1.ConfigureDebridResponse, error) {
	return c.service.ConfigureDebrid(ctx, &corev1.ConfigureDebridRequest{
		Provider: provider,
		ApiKey:   apiKey,
		Enabled:  enabled,
	})
}

func (c *Client) Close() error {
	if c.conn != nil {
		return c.conn.Close()
	}
	return nil
}
