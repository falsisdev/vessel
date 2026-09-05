package integration_test

import (
	"context"
	"net"
	"testing"
	"time"

	coreclient "github.com/falsisdev/vessel/core/internal/client"
	"github.com/falsisdev/vessel/core/internal/plugin"
	coreserver "github.com/falsisdev/vessel/core/internal/server"
	"github.com/falsisdev/vessel/core/internal/service"
	"github.com/falsisdev/vessel/core/internal/storage"
	"github.com/falsisdev/vessel/core/internal/theme"
	corev1 "github.com/falsisdev/vessel/proto/gen/go/core/v1"
	pluginv1 "github.com/falsisdev/vessel/proto/gen/go/plugin/v1"
)

func TestLibraryAndProgressOverIPC(t *testing.T) {
	// 1. In-memory SQLite storage
	sqliteStorage, err := storage.NewSQLiteStorage(":memory:")
	if err != nil {
		t.Fatalf("failed to initialize sqlite: %v", err)
	}
	defer sqliteStorage.Close()

	mgr := plugin.NewManager()
	cinemaSvc := service.NewCinemaService(mgr, 3*time.Second)
	readingSvc := service.NewReadingService(mgr, 3*time.Second)
	librarySvc := service.NewLibraryService(sqliteStorage)
	themeMgr := theme.NewManager()

	// 2. Start Core IPC Server
	l, err := net.Listen("tcp", "127.0.0.1:0")
	if err != nil {
		t.Fatalf("failed to listen on port: %v", err)
	}
	tcpAddr := l.Addr().String()
	_ = l.Close()

	srv := coreserver.NewServer(coreserver.ServerConfig{
		ListenAddr: tcpAddr,
		Version:    "1.0.0-library-test",
	}, cinemaSvc, readingSvc, librarySvc, mgr, themeMgr)

	if err := srv.Start(); err != nil {
		t.Fatalf("failed to start server: %v", err)
	}
	defer srv.Stop()

	ctx, cancel := context.WithTimeout(context.Background(), 10*time.Second)
	defer cancel()

	// 3. Connect Client
	client, err := coreclient.Dial(ctx, tcpAddr)
	if err != nil {
		t.Fatalf("failed to dial core: %v", err)
	}
	defer func() { _ = client.Close() }()

	// --- A. Library Items ---
	// 1. Save Movie Item (PLAN_TO_WATCH)
	movieItem := &corev1.LibraryItem{
		ProviderId: "com.vessel.cinema.cinemasis",
		MediaId:    "movie:155",
		Domain:     pluginv1.Domain_DOMAIN_CINEMA,
		Title:      "The Dark Knight",
		Type:       pluginv1.MediaType_MEDIA_TYPE_MOVIE,
		PosterUrl:  "https://image.tmdb.org/darkknight.jpg",
		Status:     corev1.LibraryStatus_LIBRARY_STATUS_PLAN_TO_WATCH,
		UserRating: 9.0,
	}

	saveResp, err := client.SaveLibraryItem(ctx, movieItem)
	if err != nil {
		t.Fatalf("SaveLibraryItem movie failed: %v", err)
	}
	if saveResp.Item.Title != "The Dark Knight" {
		t.Errorf("expected The Dark Knight, got %s", saveResp.Item.Title)
	}

	// 2. Save Manga Item (FAVORITE)
	mangaItem := &corev1.LibraryItem{
		ProviderId: "com.vessel.reading.mangile",
		MediaId:    "manga-solo",
		Domain:     pluginv1.Domain_DOMAIN_MANGA,
		Title:      "Solo Leveling",
		Type:       pluginv1.MediaType_MEDIA_TYPE_WEBTOON,
		PosterUrl:  "https://cdn.sanity.io/solo.jpg",
		Status:     corev1.LibraryStatus_LIBRARY_STATUS_FAVORITE,
		UserRating: 9.8,
	}

	if _, err := client.SaveLibraryItem(ctx, mangaItem); err != nil {
		t.Fatalf("SaveLibraryItem manga failed: %v", err)
	}

	// 3. GetLibraryItem
	getItemResp, err := client.GetLibraryItem(ctx, "com.vessel.cinema.cinemasis", "movie:155")
	if err != nil {
		t.Fatalf("GetLibraryItem failed: %v", err)
	}
	if getItemResp.Item.Status != corev1.LibraryStatus_LIBRARY_STATUS_PLAN_TO_WATCH {
		t.Errorf("expected PLAN_TO_WATCH, got %v", getItemResp.Item.Status)
	}

	// 4. ListLibraryItems by Domain
	listResp, err := client.ListLibraryItems(ctx, pluginv1.Domain_DOMAIN_MANGA, corev1.LibraryStatus_LIBRARY_STATUS_UNSPECIFIED, 10, 0)
	if err != nil {
		t.Fatalf("ListLibraryItems failed: %v", err)
	}
	if listResp.TotalCount != 1 || len(listResp.Items) != 1 || listResp.Items[0].Title != "Solo Leveling" {
		t.Errorf("unexpected list response: %+v", listResp)
	}

	// --- B. Playback Progress & Resume ---
	// 5. Save Playback Progress (50% watched)
	playProg := &corev1.PlaybackProgress{
		ProviderId:             "com.vessel.cinema.cinemasis",
		MediaId:                "movie:155",
		Domain:                 pluginv1.Domain_DOMAIN_CINEMA,
		CurrentPositionSeconds: 3600.0,
		TotalDurationSeconds:   7200.0,
	}

	savePlayResp, err := client.SavePlaybackProgress(ctx, playProg)
	if err != nil {
		t.Fatalf("SavePlaybackProgress failed: %v", err)
	}
	if savePlayResp.Progress.ProgressPercent != 50.0 || savePlayResp.Progress.IsCompleted {
		t.Errorf("unexpected saved progress: %+v", savePlayResp.Progress)
	}

	// Verify library item automatically transitioned from PLAN_TO_WATCH to WATCHING
	itemAfterPlay, err := client.GetLibraryItem(ctx, "com.vessel.cinema.cinemasis", "movie:155")
	if err != nil {
		t.Fatalf("GetLibraryItem after play failed: %v", err)
	}
	if itemAfterPlay.Item.Status != corev1.LibraryStatus_LIBRARY_STATUS_WATCHING {
		t.Errorf("expected status to transition to WATCHING, got %v", itemAfterPlay.Item.Status)
	}

	// 6. Complete movie (95% watched)
	playProg.CurrentPositionSeconds = 6900.0
	savePlayResp2, err := client.SavePlaybackProgress(ctx, playProg)
	if err != nil {
		t.Fatalf("SavePlaybackProgress 95%% failed: %v", err)
	}
	if !savePlayResp2.Progress.IsCompleted {
		t.Error("expected progress to be completed at >90%")
	}

	// 7. GetPlaybackProgress & ListRecent
	recentPlay, err := client.ListRecentPlaybackProgress(ctx, 5)
	if err != nil || len(recentPlay.Items) != 1 {
		t.Fatalf("ListRecentPlaybackProgress failed: %v, count: %d", err, len(recentPlay.Items))
	}

	// --- C. Reading Progress & Resume ---
	// 8. Save Reading Progress for Webook (96% scrolled -> auto complete)
	readProg := &corev1.ReadingProgress{
		ProviderId:      "com.vessel.reading.mangile",
		MediaId:         "novel-tbate",
		Domain:          pluginv1.Domain_DOMAIN_WEBOOK,
		ChapterId:       "ch-tbate-1",
		ChapterNumber:   1.0,
		TextScrollRatio: 0.98,
	}

	saveReadResp, err := client.SaveReadingProgress(ctx, readProg)
	if err != nil {
		t.Fatalf("SaveReadingProgress failed: %v", err)
	}
	if !saveReadResp.Progress.IsCompleted {
		t.Error("expected reading progress to be completed at 0.98 scroll ratio")
	}

	// 9. GetReadingProgress & ListRecentReading
	recentRead, err := client.ListRecentReadingProgress(ctx, 5)
	if err != nil || len(recentRead.Items) != 1 {
		t.Fatalf("ListRecentReadingProgress failed: %v, count: %d", err, len(recentRead.Items))
	}

	// --- D. Delete Library Item ---
	delResp, err := client.DeleteLibraryItem(ctx, "com.vessel.cinema.cinemasis", "movie:155")
	if err != nil || !delResp.Success {
		t.Fatalf("DeleteLibraryItem failed: %v", err)
	}

	_, err = client.GetLibraryItem(ctx, "com.vessel.cinema.cinemasis", "movie:155")
	if err == nil {
		t.Error("expected error getting deleted library item, got nil")
	}
}
