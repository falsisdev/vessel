package service

import (
	"context"
	"strings"

	"github.com/falsisdev/vessel/core/internal/debrid"
	"github.com/falsisdev/vessel/core/internal/streaming"
	corev1 "github.com/falsisdev/vessel/proto/gen/go/core/v1"
)

type StreamService struct {
	debridMgr *debrid.Manager
	proxy     *streaming.Proxy
}

func NewStreamService(debridMgr *debrid.Manager, proxy *streaming.Proxy) *StreamService {
	return &StreamService{
		debridMgr: debridMgr,
		proxy:     proxy,
	}
}

func (s *StreamService) ResolveStream(ctx context.Context, streamURL, title string, season, episode int, preferredProvider string) (*corev1.ResolvedStream, error) {
	trimmedURL := strings.TrimSpace(streamURL)

	// Case 1: Magnet URI
	if streaming.IsMagnetURI(trimmedURL) {
		magnet, _ := streaming.ParseMagnet(trimmedURL)
		displayTitle := title
		if displayTitle == "" && magnet != nil {
			displayTitle = magnet.DisplayName
		}

		// Attempt Debrid resolution
		if s.debridMgr != nil {
			res, err := s.debridMgr.ResolveMagnet(ctx, trimmedURL, season, episode, preferredProvider)
			if err == nil && res != nil {
				playbackURL := res.PlaybackURL
				if s.proxy != nil {
					playbackURL = s.proxy.BuildProxyURL(res.PlaybackURL)
				}

				return &corev1.ResolvedStream{
					OriginalUrl: trimmedURL,
					PlaybackUrl: playbackURL,
					StreamType:  "debrid_cached",
					Provider:    res.Provider,
					Quality:     res.Quality,
					FileSize:    res.FileSize,
					Filename:    res.Filename,
					IsCached:    true,
				}, nil
			}
		}

		// Fallback for uncached magnet
		return &corev1.ResolvedStream{
			OriginalUrl: trimmedURL,
			PlaybackUrl: trimmedURL,
			StreamType:  "magnet",
			Provider:    "direct",
			Quality:     debrid.DetectQuality(displayTitle),
			Filename:    displayTitle,
			IsCached:    false,
		}, nil
	}

	// Case 2: Direct HTTP / HLS Stream
	playbackURL := trimmedURL
	streamType := "direct"
	if strings.Contains(trimmedURL, ".m3u8") || strings.Contains(trimmedURL, "hls") {
		streamType = "hls"
	} else if s.proxy != nil && (strings.HasPrefix(trimmedURL, "http://") || strings.HasPrefix(trimmedURL, "https://")) {
		playbackURL = s.proxy.BuildProxyURL(trimmedURL)
	}

	return &corev1.ResolvedStream{
		OriginalUrl: trimmedURL,
		PlaybackUrl: playbackURL,
		StreamType:  streamType,
		Provider:    "direct",
		Quality:     debrid.DetectQuality(trimmedURL),
		IsCached:    true,
	}, nil
}

func (s *StreamService) GetDebridStatus(ctx context.Context, provider string) ([]*corev1.DebridAccountStatus, error) {
	if s.debridMgr == nil {
		return nil, nil
	}

	statuses, err := s.debridMgr.GetStatuses(ctx, provider)
	if err != nil {
		return nil, err
	}

	var results []*corev1.DebridAccountStatus
	for _, st := range statuses {
		results = append(results, &corev1.DebridAccountStatus{
			Provider:            st.Provider,
			Username:            st.Username,
			Email:               st.Email,
			ExpirationTimestamp: st.ExpirationTimestamp,
			IsPremium:           st.IsPremium,
			Points:              st.Points,
			Status:              st.Status,
		})
	}

	return results, nil
}

func (s *StreamService) ConfigureDebrid(ctx context.Context, provider, apiKey string, enabled bool) (*corev1.DebridAccountStatus, error) {
	if s.debridMgr == nil {
		return nil, debrid.ErrNotConfigured
	}

	st, err := s.debridMgr.Configure(ctx, provider, apiKey, enabled)
	if err != nil {
		return nil, err
	}

	return &corev1.DebridAccountStatus{
		Provider:            st.Provider,
		Username:            st.Username,
		Email:               st.Email,
		ExpirationTimestamp: st.ExpirationTimestamp,
		IsPremium:           st.IsPremium,
		Points:              st.Points,
		Status:              st.Status,
	}, nil
}
