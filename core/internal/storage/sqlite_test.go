package storage_test

import (
	"context"
	"testing"
	"time"

	"github.com/falsisdev/vessel/core/internal/domain/library"
	"github.com/falsisdev/vessel/core/internal/storage"
	pluginv1 "github.com/falsisdev/vessel/proto/gen/go/plugin/v1"
)

func TestSQLiteStorage_LibraryItemCRUD(t *testing.T) {
	s, err := storage.NewSQLiteStorage(":memory:")
	if err != nil {
		t.Fatalf("failed to create memory sqlite: %v", err)
	}
	defer s.Close()

	ctx := context.Background()

	item := &library.Item{
		ProviderID: "com.vessel.cinema.cinemasis",
		MediaID:    "movie:155",
		Domain:     pluginv1.Domain_DOMAIN_CINEMA,
		Title:      "The Dark Knight",
		Type:       pluginv1.MediaType_MEDIA_TYPE_MOVIE,
		PosterURL:  "https://image.tmdb.org/t/p/w500/darkknight.jpg",
		Status:     library.StatusWatching,
		UserRating: 9.5,
	}

	if err := s.SaveLibraryItem(ctx, item); err != nil {
		t.Fatalf("SaveLibraryItem failed: %v", err)
	}

	fetched, err := s.GetLibraryItem(ctx, "com.vessel.cinema.cinemasis", "movie:155")
	if err != nil {
		t.Fatalf("GetLibraryItem failed: %v", err)
	}
	if fetched.Title != "The Dark Knight" || fetched.Status != library.StatusWatching {
		t.Errorf("unexpected fetched item: %+v", fetched)
	}

	// Update item
	item.Status = library.StatusCompleted
	item.UserRating = 10.0
	if err := s.SaveLibraryItem(ctx, item); err != nil {
		t.Fatalf("Update SaveLibraryItem failed: %v", err)
	}

	updated, err := s.GetLibraryItem(ctx, "com.vessel.cinema.cinemasis", "movie:155")
	if err != nil {
		t.Fatalf("GetLibraryItem after update failed: %v", err)
	}
	if updated.Status != library.StatusCompleted || updated.UserRating != 10.0 {
		t.Errorf("unexpected updated item: %+v", updated)
	}

	// List items
	list, total, err := s.ListLibraryItems(ctx, library.Filter{
		Domain: pluginv1.Domain_DOMAIN_CINEMA,
		Status: library.StatusCompleted,
	})
	if err != nil {
		t.Fatalf("ListLibraryItems failed: %v", err)
	}
	if total != 1 || len(list) != 1 {
		t.Fatalf("expected 1 item in list, got %d (total: %d)", len(list), total)
	}

	// Delete item
	if err := s.DeleteLibraryItem(ctx, "com.vessel.cinema.cinemasis", "movie:155"); err != nil {
		t.Fatalf("DeleteLibraryItem failed: %v", err)
	}

	_, err = s.GetLibraryItem(ctx, "com.vessel.cinema.cinemasis", "movie:155")
	if err != storage.ErrNotFound {
		t.Errorf("expected ErrNotFound, got %v", err)
	}
}

func TestSQLiteStorage_PlaybackAndReadingProgress(t *testing.T) {
	s, err := storage.NewSQLiteStorage(":memory:")
	if err != nil {
		t.Fatalf("failed to create memory sqlite: %v", err)
	}
	defer s.Close()

	ctx := context.Background()

	// Playback Progress
	playProg := &library.PlaybackProgress{
		ProviderID:             "com.vessel.cinema.cinemasis",
		MediaID:                "series:1399",
		Domain:                 pluginv1.Domain_DOMAIN_CINEMA,
		SeasonNumber:           1,
		EpisodeNumber:          1,
		CurrentPositionSeconds: 1200.5,
		TotalDurationSeconds:   3600.0,
		ProgressPercent:        33.3,
		IsCompleted:            false,
		UpdatedAt:              time.Now(),
	}

	if err := s.SavePlaybackProgress(ctx, playProg); err != nil {
		t.Fatalf("SavePlaybackProgress failed: %v", err)
	}

	fetchedPlay, err := s.GetPlaybackProgress(ctx, "com.vessel.cinema.cinemasis", "series:1399", 1, 1)
	if err != nil {
		t.Fatalf("GetPlaybackProgress failed: %v", err)
	}
	if fetchedPlay.CurrentPositionSeconds != 1200.5 {
		t.Errorf("unexpected playback position: %f", fetchedPlay.CurrentPositionSeconds)
	}

	recentPlay, err := s.ListRecentPlaybackProgress(ctx, 10)
	if err != nil || len(recentPlay) != 1 {
		t.Fatalf("ListRecentPlaybackProgress error: %v, count: %d", err, len(recentPlay))
	}

	// Reading Progress
	readProg := &library.ReadingProgress{
		ProviderID:      "com.vessel.reading.mangile",
		MediaID:         "manga-solo",
		Domain:          pluginv1.Domain_DOMAIN_MANGA,
		ChapterID:       "ch-solo-1",
		ChapterNumber:   1.0,
		CurrentPage:     15,
		TotalPages:      30,
		TextScrollRatio: 0.0,
		IsCompleted:     false,
	}

	if err := s.SaveReadingProgress(ctx, readProg); err != nil {
		t.Fatalf("SaveReadingProgress failed: %v", err)
	}

	fetchedRead, err := s.GetReadingProgress(ctx, "com.vessel.reading.mangile", "manga-solo", "ch-solo-1")
	if err != nil {
		t.Fatalf("GetReadingProgress failed: %v", err)
	}
	if fetchedRead.CurrentPage != 15 || fetchedRead.TotalPages != 30 {
		t.Errorf("unexpected reading progress: %+v", fetchedRead)
	}

	recentRead, err := s.ListRecentReadingProgress(ctx, 10)
	if err != nil || len(recentRead) != 1 {
		t.Fatalf("ListRecentReadingProgress error: %v, count: %d", err, len(recentRead))
	}
}

func TestSQLiteStorage_Settings(t *testing.T) {
	s, err := storage.NewSQLiteStorage(":memory:")
	if err != nil {
		t.Fatalf("failed to create memory sqlite: %v", err)
	}
	defer s.Close()

	ctx := context.Background()

	// Setting non-existent
	_, err = s.GetSetting(ctx, "debrid:realdebrid:token")
	if err != storage.ErrNotFound {
		t.Fatalf("expected ErrNotFound, got %v", err)
	}

	// Set and get
	if err := s.SetSetting(ctx, "debrid:realdebrid:token", "secret123"); err != nil {
		t.Fatalf("SetSetting failed: %v", err)
	}

	val, err := s.GetSetting(ctx, "debrid:realdebrid:token")
	if err != nil || val != "secret123" {
		t.Fatalf("unexpected setting value: %s, err: %v", val, err)
	}

	// Update
	if err := s.SetSetting(ctx, "debrid:realdebrid:token", "updated456"); err != nil {
		t.Fatalf("SetSetting update failed: %v", err)
	}
	val, err = s.GetSetting(ctx, "debrid:realdebrid:token")
	if err != nil || val != "updated456" {
		t.Fatalf("unexpected updated value: %s, err: %v", val, err)
	}

	// Delete
	if err := s.DeleteSetting(ctx, "debrid:realdebrid:token"); err != nil {
		t.Fatalf("DeleteSetting failed: %v", err)
	}
	_, err = s.GetSetting(ctx, "debrid:realdebrid:token")
	if err != storage.ErrNotFound {
		t.Fatalf("expected ErrNotFound after delete, got %v", err)
	}
}

