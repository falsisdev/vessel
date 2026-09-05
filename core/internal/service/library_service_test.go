package service_test

import (
	"context"
	"testing"

	"github.com/falsisdev/vessel/core/internal/domain/library"
	"github.com/falsisdev/vessel/core/internal/service"
	"github.com/falsisdev/vessel/core/internal/storage"
	pluginv1 "github.com/falsisdev/vessel/proto/gen/go/plugin/v1"
)

func TestLibraryService_PlaybackAndReadingLogic(t *testing.T) {
	st, err := storage.NewSQLiteStorage(":memory:")
	if err != nil {
		t.Fatalf("failed to create sqlite: %v", err)
	}
	defer st.Close()

	svc := service.NewLibraryService(st)
	ctx := context.Background()

	// 1. Save an item with PLAN_TO_WATCH
	item := &library.Item{
		ProviderID: "com.vessel.cinema.cinemasis",
		MediaID:    "movie:155",
		Domain:     pluginv1.Domain_DOMAIN_CINEMA,
		Title:      "The Dark Knight",
		Type:       pluginv1.MediaType_MEDIA_TYPE_MOVIE,
		Status:     library.StatusPlanToWatch,
	}
	if err := svc.SaveItem(ctx, item); err != nil {
		t.Fatalf("SaveItem failed: %v", err)
	}

	// 2. Record Playback Progress (at 92% -> should auto mark is_completed)
	play := &library.PlaybackProgress{
		ProviderID:             "com.vessel.cinema.cinemasis",
		MediaID:                "movie:155",
		Domain:                 pluginv1.Domain_DOMAIN_CINEMA,
		CurrentPositionSeconds: 920.0,
		TotalDurationSeconds:   1000.0,
	}
	if err := svc.RecordPlayback(ctx, play); err != nil {
		t.Fatalf("RecordPlayback failed: %v", err)
	}

	savedPlay, err := svc.GetPlayback(ctx, "com.vessel.cinema.cinemasis", "movie:155", 0, 0)
	if err != nil {
		t.Fatalf("GetPlayback failed: %v", err)
	}
	if savedPlay.ProgressPercent != 92.0 {
		t.Errorf("expected 92%%, got %f", savedPlay.ProgressPercent)
	}
	if !savedPlay.IsCompleted {
		t.Error("expected playback to be marked completed at 92%")
	}

	// Verify linked item status updated to WATCHING
	linkedItem, err := svc.GetItem(ctx, "com.vessel.cinema.cinemasis", "movie:155")
	if err != nil {
		t.Fatalf("GetItem failed: %v", err)
	}
	if linkedItem.Status != library.StatusWatching {
		t.Errorf("expected status WATCHING, got %s", linkedItem.Status)
	}

	// 3. Record Reading Progress (novel text scroll ratio >= 0.95 -> should auto complete)
	read := &library.ReadingProgress{
		ProviderID:      "com.vessel.reading.mangile",
		MediaID:         "novel:mt",
		Domain:          pluginv1.Domain_DOMAIN_WEBOOK,
		ChapterID:       "ch-1",
		TextScrollRatio: 0.96,
	}
	if err := svc.RecordReading(ctx, read); err != nil {
		t.Fatalf("RecordReading failed: %v", err)
	}

	savedRead, err := svc.GetReading(ctx, "com.vessel.reading.mangile", "novel:mt", "ch-1")
	if err != nil {
		t.Fatalf("GetReading failed: %v", err)
	}
	if !savedRead.IsCompleted {
		t.Error("expected reading to be marked completed at text scroll ratio 0.96")
	}
}
