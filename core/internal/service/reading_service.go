package service

import (
	"context"
	"fmt"
	"sync"
	"time"

	"github.com/falsisdev/vessel/core/internal/domain/reading"
	"github.com/falsisdev/vessel/core/internal/plugin"
	pluginv1 "github.com/falsisdev/vessel/proto/gen/go/plugin/v1"
)

type ReadingService struct {
	manager       *plugin.Manager
	searchTimeout time.Duration
}

func NewReadingService(manager *plugin.Manager, searchTimeout time.Duration) *ReadingService {
	if searchTimeout <= 0 {
		searchTimeout = 5 * time.Second
	}
	return &ReadingService{
		manager:       manager,
		searchTimeout: searchTimeout,
	}
}

func (s *ReadingService) Search(ctx context.Context, domain pluginv1.Domain, query string) ([]reading.ReadingItem, error) {
	if domain == pluginv1.Domain_DOMAIN_UNSPECIFIED {
		domain = pluginv1.Domain_DOMAIN_MANGA
	}

	plugins := s.manager.ListByDomainAndCapability(domain, pluginv1.Capability_CAPABILITY_SEARCH)
	if len(plugins) == 0 {
		return []reading.ReadingItem{}, nil
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

	var aggregated []reading.ReadingItem
	for res := range resultCh {
		if res.err != nil {
			continue
		}
		for _, item := range res.items {
			aggregated = append(aggregated, reading.ReadingItem{
				ID:          item.Id,
				ProviderID:  res.providerID,
				Title:       item.Title,
				Type:        mapReadingType(item.Type),
				Year:        item.Year,
				PosterURL:   item.PosterUrl,
				Overview:    item.Overview,
				ExternalIDs: mapExternalIDs(item.ExternalIds),
			})
		}
	}

	return aggregated, nil
}

func (s *ReadingService) GetMetadata(ctx context.Context, providerID, mediaID string) (*reading.ReadingDetails, error) {
	client, err := s.manager.Get(providerID)
	if err != nil {
		return nil, err
	}

	resp, err := client.GetMetadata(ctx, mediaID)
	if err != nil {
		return nil, fmt.Errorf("failed to get reading metadata from %s: %w", providerID, err)
	}

	raw := resp.Details
	if raw == nil {
		return nil, fmt.Errorf("empty details returned from reading provider %s", providerID)
	}

	details := &reading.ReadingDetails{
		ID:          raw.Id,
		ProviderID:  providerID,
		Title:       raw.Title,
		Type:        mapReadingType(raw.Type),
		Year:        raw.Year,
		PosterURL:   raw.PosterUrl,
		Overview:    raw.Overview,
		Genres:      raw.Genres,
		ExternalIDs: mapExternalIDs(raw.ExternalIds),
	}

	for _, s := range raw.Seasons {
		for _, ep := range s.Episodes {
			details.Chapters = append(details.Chapters, reading.Chapter{
				ID:            fmt.Sprintf("%d", ep.EpisodeNumber),
				ChapterNumber: float64(ep.EpisodeNumber),
				VolumeNumber:  float64(s.SeasonNumber),
				Title:         ep.Title,
			})
		}
	}

	return details, nil
}

func (s *ReadingService) GetChapterContent(ctx context.Context, providerID, mediaID, chapterID string, chapterNumber float32) (*reading.ChapterContent, error) {
	client, err := s.manager.Get(providerID)
	if err != nil {
		return nil, err
	}

	resp, err := client.GetChapterContent(ctx, mediaID, chapterID, chapterNumber)
	if err != nil {
		return nil, fmt.Errorf("failed to get chapter content from %s: %w", providerID, err)
	}

	var pages []reading.Page
	for _, p := range resp.Pages {
		pages = append(pages, reading.Page{
			PageNumber: p.PageNumber,
			URL:        p.Url,
			Headers:    p.Headers,
		})
	}

	return &reading.ChapterContent{
		ChapterID:     resp.ChapterId,
		Title:         resp.Title,
		ChapterNumber: float64(resp.ChapterNumber),
		Pages:         pages,
		TextContent:   resp.TextContent,
	}, nil
}

func mapReadingType(t pluginv1.MediaType) reading.ReadingType {
	switch t {
	case pluginv1.MediaType_MEDIA_TYPE_MANGA:
		return reading.ReadingTypeManga
	case pluginv1.MediaType_MEDIA_TYPE_WEBTOON:
		return reading.ReadingTypeWebtoon
	case pluginv1.MediaType_MEDIA_TYPE_WEBOOK:
		return reading.ReadingTypeWebook
	case pluginv1.MediaType_MEDIA_TYPE_BOOK:
		return reading.ReadingTypeBook
	default:
		return reading.ReadingTypeUnspecified
	}
}
