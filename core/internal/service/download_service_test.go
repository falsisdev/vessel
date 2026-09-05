package service

import (
	"os"
	"path/filepath"
	"testing"
)

func TestDownloadService_Lifecycle(t *testing.T) {
	tempDir, err := os.MkdirTemp("", "vessel-download-test-*")
	if err != nil {
		t.Fatalf("failed to create temp dir: %v", err)
	}
	defer os.RemoveAll(tempDir)

	svc, err := NewDownloadService(nil, tempDir)
	if err != nil {
		t.Fatalf("NewDownloadService failed: %v", err)
	}

	// Manually inject a completed downloaded chapter
	chDir := filepath.Join(tempDir, "manga", "com.test", "media-1", "ch-1")
	_ = os.MkdirAll(chDir, 0755)
	_ = os.WriteFile(filepath.Join(chDir, "content.txt"), []byte("Hello offline chapter content!"), 0644)
	_ = os.WriteFile(filepath.Join(chDir, "page_001.jpg"), []byte("fake-jpeg-data"), 0644)

	item := &DownloadedChapter{
		ProviderID:    "com.test",
		MediaID:       "media-1",
		ChapterID:     "ch-1",
		ChapterNumber: 1.0,
		Title:         "Chapter 1",
		TotalPages:    1,
		Downloaded:    1,
		Status:        DownloadStatusCompleted,
		LocalPath:     chDir,
	}
	svc.active[svc.makeKey("com.test", "media-1", "ch-1")] = item

	// 1. List downloads
	downloads := svc.ListDownloads("media-1")
	if len(downloads) != 1 {
		t.Fatalf("expected 1 download, got %d", len(downloads))
	}
	if downloads[0].ChapterID != "ch-1" {
		t.Errorf("expected ch-1, got %s", downloads[0].ChapterID)
	}

	// 2. Retrieve offline chapter content
	content, err := svc.GetChapterOffline("com.test", "media-1", "ch-1")
	if err != nil {
		t.Fatalf("GetChapterOffline failed: %v", err)
	}
	if content.TextContent != "Hello offline chapter content!" {
		t.Errorf("unexpected content: %s", content.TextContent)
	}
	if len(content.Pages) != 1 {
		t.Errorf("expected 1 page, got %d", len(content.Pages))
	}

	// 3. Delete download
	if err := svc.DeleteDownload("com.test", "media-1", "ch-1"); err != nil {
		t.Fatalf("DeleteDownload failed: %v", err)
	}
	if len(svc.ListDownloads("media-1")) != 0 {
		t.Errorf("expected 0 downloads after deletion")
	}
}
