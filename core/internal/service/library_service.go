package service

import (
	"context"
	"errors"
	"fmt"
	"time"

	"github.com/falsisdev/vessel/core/internal/domain/library"
	"github.com/falsisdev/vessel/core/internal/storage"
)

var (
	ErrInvalidItem = errors.New("provider_id, media_id and title are required")
)

type LibraryService struct {
	storage *storage.SQLiteStorage
}

func NewLibraryService(storage *storage.SQLiteStorage) *LibraryService {
	return &LibraryService{storage: storage}
}

func (s *LibraryService) SaveItem(ctx context.Context, item *library.Item) error {
	if item.ProviderID == "" || item.MediaID == "" || item.Title == "" {
		return ErrInvalidItem
	}
	return s.storage.SaveLibraryItem(ctx, item)
}

func (s *LibraryService) GetItem(ctx context.Context, providerID, mediaID string) (*library.Item, error) {
	return s.storage.GetLibraryItem(ctx, providerID, mediaID)
}

func (s *LibraryService) ListItems(ctx context.Context, filter library.Filter) ([]*library.Item, int, error) {
	return s.storage.ListLibraryItems(ctx, filter)
}

func (s *LibraryService) DeleteItem(ctx context.Context, providerID, mediaID string) error {
	return s.storage.DeleteLibraryItem(ctx, providerID, mediaID)
}

func (s *LibraryService) RecordPlayback(ctx context.Context, p *library.PlaybackProgress) error {
	if p.ProviderID == "" || p.MediaID == "" {
		return errors.New("provider_id and media_id are required")
	}

	if p.TotalDurationSeconds > 0 {
		p.ProgressPercent = float32((p.CurrentPositionSeconds / p.TotalDurationSeconds) * 100.0)
		if p.ProgressPercent > 100.0 {
			p.ProgressPercent = 100.0
		}
		if p.ProgressPercent >= 90.0 {
			p.IsCompleted = true
		}
	}

	p.UpdatedAt = time.Now().UTC()

	if err := s.storage.SavePlaybackProgress(ctx, p); err != nil {
		return fmt.Errorf("failed to save playback progress: %w", err)
	}

	// Auto touch or create library item
	item, err := s.storage.GetLibraryItem(ctx, p.ProviderID, p.MediaID)
	if err == nil && item != nil {
		item.LastInteractedAt = p.UpdatedAt
		if item.Status == library.StatusPlanToWatch || item.Status == library.StatusUnspecified {
			item.Status = library.StatusWatching
		}
		_ = s.storage.SaveLibraryItem(ctx, item)
	}

	return nil
}

func (s *LibraryService) GetPlayback(ctx context.Context, providerID, mediaID string, season, episode int32) (*library.PlaybackProgress, error) {
	return s.storage.GetPlaybackProgress(ctx, providerID, mediaID, season, episode)
}

func (s *LibraryService) ListRecentPlayback(ctx context.Context, limit int) ([]*library.PlaybackProgress, error) {
	return s.storage.ListRecentPlaybackProgress(ctx, limit)
}

func (s *LibraryService) RecordReading(ctx context.Context, p *library.ReadingProgress) error {
	if p.ChapterID == "" && p.ChapterNumber >= 0 {
		p.ChapterID = fmt.Sprintf("ch-%v", p.ChapterNumber)
	}
	if p.ProviderID == "" || p.MediaID == "" || p.ChapterID == "" {
		return errors.New("provider_id, media_id and chapter_id are required")
	}

	if p.TotalPages > 0 && p.CurrentPage >= p.TotalPages {
		p.IsCompleted = true
	} else if p.TextScrollRatio >= 0.95 {
		p.IsCompleted = true
	}

	p.UpdatedAt = time.Now().UTC()

	if err := s.storage.SaveReadingProgress(ctx, p); err != nil {
		return fmt.Errorf("failed to save reading progress: %w", err)
	}

	// Auto touch or create library item
	item, err := s.storage.GetLibraryItem(ctx, p.ProviderID, p.MediaID)
	if err == nil && item != nil {
		item.LastInteractedAt = p.UpdatedAt
		if item.Status == library.StatusPlanToWatch || item.Status == library.StatusUnspecified {
			item.Status = library.StatusWatching
		}
		_ = s.storage.SaveLibraryItem(ctx, item)
	}

	return nil
}

func (s *LibraryService) GetReading(ctx context.Context, providerID, mediaID, chapterID string) (*library.ReadingProgress, error) {
	return s.storage.GetReadingProgress(ctx, providerID, mediaID, chapterID)
}

func (s *LibraryService) ListRecentReading(ctx context.Context, limit int) ([]*library.ReadingProgress, error) {
	return s.storage.ListRecentReadingProgress(ctx, limit)
}

func (s *LibraryService) DeletePlayback(ctx context.Context, providerID, mediaID string, season, episode int32) error {
	return s.storage.DeletePlaybackProgress(ctx, providerID, mediaID, season, episode)
}

func (s *LibraryService) DeleteReading(ctx context.Context, providerID, mediaID, chapterID string) error {
	return s.storage.DeleteReadingProgress(ctx, providerID, mediaID, chapterID)
}
