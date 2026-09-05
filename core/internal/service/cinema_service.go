package service

import (
	"context"
	"fmt"
	"sync"
	"time"

	"github.com/falsisdev/vessel/core/internal/domain/cinema"
	"github.com/falsisdev/vessel/core/internal/plugin"
	pluginv1 "github.com/falsisdev/vessel/proto/gen/go/plugin/v1"
)

type CinemaService struct {
	manager       *plugin.Manager
	searchTimeout time.Duration
}

func NewCinemaService(manager *plugin.Manager, searchTimeout time.Duration) *CinemaService {
	if searchTimeout <= 0 {
		searchTimeout = 5 * time.Second
	}
	return &CinemaService{
		manager:       manager,
		searchTimeout: searchTimeout,
	}
}

func (s *CinemaService) Search(ctx context.Context, query string) ([]cinema.MediaItem, error) {
	plugins := s.manager.ListByDomainAndCapability(pluginv1.Domain_DOMAIN_CINEMA, pluginv1.Capability_CAPABILITY_SEARCH)
	if len(plugins) == 0 {
		return []cinema.MediaItem{}, nil
	}

	searchCtx, cancel := context.WithTimeout(ctx, s.searchTimeout)
	defer cancel()

	type searchResult struct {
		providerID string
		items      []*pluginv1.MediaItem
		err        error
	}

	resultCh := make(chan searchResult, len(plugins))
	var wg sync.WaitGroup

	for _, p := range plugins {
		wg.Add(1)
		go func(client plugin.Client) {
			defer wg.Done()
			manifest := client.Manifest()
			resp, err := client.Search(searchCtx, query, 1)
			if err != nil {
				resultCh <- searchResult{providerID: manifest.Id, err: err}
				return
			}
			resultCh <- searchResult{providerID: manifest.Id, items: resp.Items}
		}(p)
	}

	wg.Wait()
	close(resultCh)

	var aggregated []cinema.MediaItem
	for res := range resultCh {
		if res.err != nil {
			continue
		}
		for _, item := range res.items {
			aggregated = append(aggregated, cinema.MediaItem{
				ID:          item.Id,
				ProviderID:  res.providerID,
				Title:       item.Title,
				Type:        mapMediaType(item.Type),
				Year:        item.Year,
				PosterURL:   item.PosterUrl,
				Overview:    item.Overview,
				ExternalIDs: mapExternalIDs(item.ExternalIds),
			})
		}
	}

	return aggregated, nil
}

func (s *CinemaService) GetMetadata(ctx context.Context, providerID, mediaID string) (*cinema.MediaDetails, error) {
	client, err := s.manager.Get(providerID)
	if err != nil {
		return nil, err
	}

	resp, err := client.GetMetadata(ctx, mediaID)
	if err != nil {
		return nil, fmt.Errorf("failed to get metadata from %s: %w", providerID, err)
	}

	raw := resp.Details
	if raw == nil {
		return nil, fmt.Errorf("empty details returned from provider %s", providerID)
	}

	details := &cinema.MediaDetails{
		ID:          raw.Id,
		ProviderID:  providerID,
		Title:       raw.Title,
		Type:        mapMediaType(raw.Type),
		Year:        raw.Year,
		PosterURL:   raw.PosterUrl,
		Overview:    raw.Overview,
		Genres:      raw.Genres,
		ExternalIDs: mapExternalIDs(raw.ExternalIds),
	}

	for _, s := range raw.Seasons {
		season := cinema.Season{
			SeasonNumber: s.SeasonNumber,
			Title:        s.Title,
		}
		for _, ep := range s.Episodes {
			season.Episodes = append(season.Episodes, cinema.Episode{
				EpisodeNumber:   ep.EpisodeNumber,
				Title:           ep.Title,
				Overview:        ep.Overview,
				DurationSeconds: ep.DurationSeconds,
			})
		}
		details.Seasons = append(details.Seasons, season)
	}

	return details, nil
}

func (s *CinemaService) GetStreams(ctx context.Context, providerID, mediaID string, season, episode int32) ([]cinema.StreamSource, []cinema.Subtitle, error) {
	client, err := s.manager.Get(providerID)
	if err != nil {
		return nil, nil, err
	}

	resp, err := client.GetStreams(ctx, mediaID, season, episode)
	if err != nil {
		return nil, nil, fmt.Errorf("failed to get streams from %s: %w", providerID, err)
	}

	var streams []cinema.StreamSource
	for _, stream := range resp.Streams {
		streams = append(streams, cinema.StreamSource{
			ID:         stream.Id,
			ProviderID: providerID,
			Title:      stream.Title,
			URL:        stream.Url,
			Format:     mapStreamFormat(stream.Format),
			Quality:    stream.Quality,
			Headers:    stream.Headers,
		})
	}

	var subtitles []cinema.Subtitle
	for _, sub := range resp.Subtitles {
		subtitles = append(subtitles, cinema.Subtitle{
			Language:  sub.Language,
			Label:     sub.Label,
			URL:       sub.Url,
			Format:    mapSubtitleFormat(sub.Format),
			IsDefault: sub.IsDefault,
		})
	}

	return streams, subtitles, nil
}

func mapMediaType(t pluginv1.MediaType) cinema.MediaType {
	switch t {
	case pluginv1.MediaType_MEDIA_TYPE_MOVIE:
		return cinema.MediaTypeMovie
	case pluginv1.MediaType_MEDIA_TYPE_SERIES:
		return cinema.MediaTypeSeries
	case pluginv1.MediaType_MEDIA_TYPE_ANIME:
		return cinema.MediaTypeAnime
	default:
		return cinema.MediaTypeUnspecified
	}
}

func mapStreamFormat(f pluginv1.StreamFormat) cinema.StreamFormat {
	switch f {
	case pluginv1.StreamFormat_STREAM_FORMAT_HLS:
		return cinema.StreamFormatHLS
	case pluginv1.StreamFormat_STREAM_FORMAT_DASH:
		return cinema.StreamFormatDASH
	case pluginv1.StreamFormat_STREAM_FORMAT_MP4:
		return cinema.StreamFormatMP4
	case pluginv1.StreamFormat_STREAM_FORMAT_MKV:
		return cinema.StreamFormatMKV
	default:
		return cinema.StreamFormatUnspecified
	}
}

func mapSubtitleFormat(f pluginv1.SubtitleFormat) cinema.SubtitleFormat {
	switch f {
	case pluginv1.SubtitleFormat_SUBTITLE_FORMAT_VTT:
		return cinema.SubtitleFormatVTT
	case pluginv1.SubtitleFormat_SUBTITLE_FORMAT_SRT:
		return cinema.SubtitleFormatSRT
	default:
		return cinema.SubtitleFormatUnspecified
	}
}

func mapExternalIDs(raw *pluginv1.ExternalIDs) cinema.ExternalIDs {
	if raw == nil {
		return cinema.ExternalIDs{}
	}
	return cinema.ExternalIDs{
		IMDbID:    raw.ImdbId,
		TMDBID:    raw.TmdbId,
		SIMKLID:   raw.SimklId,
		MALID:     raw.MalId,
		AniListID: raw.AnilistId,
		KitsuID:   raw.KitsuId,
		SanityID:  raw.SanityId,
		Extra:     raw.Extra,
	}
}
