package service

import (
	"context"
	"encoding/json"
	"errors"
	"fmt"
	"io"
	"net/http"
	"os"
	"path/filepath"
	"sync"
	"time"

	"github.com/falsisdev/vessel/core/internal/domain/reading"
)

var (
	ErrDownloadNotFound = errors.New("download not found")
)

type DownloadStatus string

const (
	DownloadStatusQueued     DownloadStatus = "queued"
	DownloadStatusInProgress DownloadStatus = "downloading"
	DownloadStatusCompleted  DownloadStatus = "completed"
	DownloadStatusFailed     DownloadStatus = "failed"
)

type DownloadedChapter struct {
	ProviderID    string         `json:"provider_id"`
	MediaID       string         `json:"media_id"`
	MediaTitle    string         `json:"media_title,omitempty"`
	PosterURL     string         `json:"poster_url,omitempty"`
	ChapterID     string         `json:"chapter_id"`
	ChapterNumber float64        `json:"chapter_number"`
	Title         string         `json:"title"`
	TotalPages    int            `json:"total_pages"`
	Downloaded    int            `json:"downloaded_pages"`
	TotalBytes    int64          `json:"total_bytes"`
	Status        DownloadStatus `json:"status"`
	LocalPath     string         `json:"local_path"`
	CreatedAt     time.Time      `json:"created_at"`
	CompletedAt   *time.Time     `json:"completed_at,omitempty"`
}

type DownloadService struct {
	readingSvc *ReadingService
	storageDir string
	httpClient *http.Client
	active     map[string]*DownloadedChapter
	mu         sync.RWMutex
}

func NewDownloadService(readingSvc *ReadingService, storageDir string) (*DownloadService, error) {
	if storageDir == "" {
		home, _ := os.UserHomeDir()
		storageDir = filepath.Join(home, ".vessel", "downloads")
	}
	if err := os.MkdirAll(storageDir, 0755); err != nil {
		return nil, fmt.Errorf("failed to create downloads directory: %w", err)
	}

	svc := &DownloadService{
		readingSvc: readingSvc,
		storageDir: storageDir,
		httpClient: &http.Client{Timeout: 30 * time.Second},
		active:     make(map[string]*DownloadedChapter),
	}
	svc.loadExistingDownloads()
	return svc, nil
}

func (s *DownloadService) makeKey(providerID, mediaID, chapterID string) string {
	return fmt.Sprintf("%s:%s:%s", providerID, mediaID, chapterID)
}

func (s *DownloadService) loadExistingDownloads() {
	s.mu.Lock()
	defer s.mu.Unlock()

	entries, err := os.ReadDir(filepath.Join(s.storageDir, "manga"))
	if err != nil {
		return
	}

	for _, provEntry := range entries {
		if !provEntry.IsDir() {
			continue
		}
		provID := provEntry.Name()
		mediaEntries, _ := os.ReadDir(filepath.Join(s.storageDir, "manga", provID))
		for _, mEntry := range mediaEntries {
			if !mEntry.IsDir() {
				continue
			}
			mediaID := mEntry.Name()
			chEntries, _ := os.ReadDir(filepath.Join(s.storageDir, "manga", provID, mediaID))
			for _, chEntry := range chEntries {
				if !chEntry.IsDir() {
					continue
				}
				metaFile := filepath.Join(s.storageDir, "manga", provID, mediaID, chEntry.Name(), "chapter.json")
				if data, err := os.ReadFile(metaFile); err == nil {
					var item DownloadedChapter
					if json.Unmarshal(data, &item) == nil {
						k := s.makeKey(item.ProviderID, item.MediaID, item.ChapterID)
						s.active[k] = &item
					}
				}
			}
		}
	}
}

// StartDownload initiates an asynchronous download of a chapter's pages or text content.
func (s *DownloadService) StartDownload(ctx context.Context, providerID, mediaID, chapterID string, chapterNum float64, mediaTitle, posterURL, chapterTitle string) (*DownloadedChapter, error) {
	k := s.makeKey(providerID, mediaID, chapterID)

	s.mu.Lock()
	if existing, exists := s.active[k]; exists && existing.Status == DownloadStatusCompleted {
		s.mu.Unlock()
		return existing, nil
	}

	targetDir := filepath.Join(s.storageDir, "manga", providerID, mediaID, chapterID)
	_ = os.MkdirAll(targetDir, 0755)

	title := chapterTitle
	if title == "" {
		title = fmt.Sprintf("Chapter %.1f", chapterNum)
	}

	item := &DownloadedChapter{
		ProviderID:    providerID,
		MediaID:       mediaID,
		MediaTitle:    mediaTitle,
		PosterURL:     posterURL,
		ChapterID:     chapterID,
		ChapterNumber: chapterNum,
		Title:         title,
		Status:        DownloadStatusInProgress,
		LocalPath:     targetDir,
		CreatedAt:     time.Now().UTC(),
	}
	s.active[k] = item
	s.mu.Unlock()

	go s.executeDownload(providerID, mediaID, chapterID, chapterNum, targetDir, item)

	return item, nil
}

func (s *DownloadService) executeDownload(providerID, mediaID, chapterID string, chapterNum float64, targetDir string, item *DownloadedChapter) {
	ctx, cancel := context.WithTimeout(context.Background(), 10*time.Minute)
	defer cancel()

	if s.readingSvc == nil {
		s.markFailed(item)
		return
	}

	content, err := s.readingSvc.GetChapterContent(ctx, providerID, mediaID, chapterID, float32(chapterNum))
	if err != nil || content == nil {
		s.markFailed(item)
		return
	}

	if content.Title != "" {
		item.Title = content.Title
	}
	item.TotalPages = len(content.Pages)

	var localPages []reading.Page
	var totalBytes int64

	// If novel/webook with TextContent
	if content.TextContent != "" {
		txtPath := filepath.Join(targetDir, "content.txt")
		_ = os.WriteFile(txtPath, []byte(content.TextContent), 0644)
		totalBytes += int64(len(content.TextContent))
	}

	// Download page images
	for idx, p := range content.Pages {
		filename := fmt.Sprintf("page_%03d.jpg", p.PageNumber)
		filePath := filepath.Join(targetDir, filename)

		req, err := http.NewRequestWithContext(ctx, http.MethodGet, p.URL, nil)
		if err == nil {
			for k, v := range p.Headers {
				req.Header.Set(k, v)
			}
			resp, err := s.httpClient.Do(req)
			if err == nil && resp.StatusCode == http.StatusOK {
				outFile, err := os.Create(filePath)
				if err == nil {
					n, _ := io.Copy(outFile, resp.Body)
					totalBytes += n
					outFile.Close()
				}
				resp.Body.Close()
			}
		}

		localPages = append(localPages, reading.Page{
			PageNumber: p.PageNumber,
			URL:        fmt.Sprintf("/api/reading/offline/page?provider=%s&media=%s&chapter=%s&page=%d", providerID, mediaID, chapterID, p.PageNumber),
		})

		s.mu.Lock()
		item.Downloaded = idx + 1
		item.TotalBytes = totalBytes
		s.mu.Unlock()
	}

	now := time.Now().UTC()
	s.mu.Lock()
	item.Status = DownloadStatusCompleted
	item.CompletedAt = &now
	item.TotalBytes = totalBytes
	s.mu.Unlock()

	// Persist chapter.json manifest
	metaData, _ := json.MarshalIndent(item, "", "  ")
	_ = os.WriteFile(filepath.Join(targetDir, "chapter.json"), metaData, 0644)
}

func (s *DownloadService) markFailed(item *DownloadedChapter) {
	s.mu.Lock()
	item.Status = DownloadStatusFailed
	s.mu.Unlock()
}

// GetChapterOffline returns cached chapter content if downloaded.
func (s *DownloadService) GetChapterOffline(providerID, mediaID, chapterID string) (*reading.ChapterContent, error) {
	k := s.makeKey(providerID, mediaID, chapterID)
	s.mu.RLock()
	item, exists := s.active[k]
	s.mu.RUnlock()

	if !exists || item.Status != DownloadStatusCompleted {
		return nil, ErrDownloadNotFound
	}

	targetDir := item.LocalPath
	var textContent string
	if txt, err := os.ReadFile(filepath.Join(targetDir, "content.txt")); err == nil {
		textContent = string(txt)
	}

	var pages []reading.Page
	for i := 1; i <= item.TotalPages; i++ {
		pages = append(pages, reading.Page{
			PageNumber: int32(i),
			URL:        fmt.Sprintf("/api/reading/offline/page?provider=%s&media=%s&chapter=%s&page=%d", providerID, mediaID, chapterID, i),
		})
	}

	return &reading.ChapterContent{
		ChapterID:     item.ChapterID,
		Title:         item.Title,
		ChapterNumber: item.ChapterNumber,
		Pages:         pages,
		TextContent:   textContent,
	}, nil
}

// ServeOfflinePage serves a downloaded manga page image directly from disk.
func (s *DownloadService) ServeOfflinePage(w http.ResponseWriter, r *http.Request, providerID, mediaID, chapterID string, pageNum int) {
	filePath := filepath.Join(s.storageDir, "manga", providerID, mediaID, chapterID, fmt.Sprintf("page_%03d.jpg", pageNum))
	if _, err := os.Stat(filePath); err != nil {
		http.Error(w, "Page not found in offline storage", http.StatusNotFound)
		return
	}
	w.Header().Set("Content-Type", "image/jpeg")
	w.Header().Set("Cache-Control", "public, max-age=86400")
	http.ServeFile(w, r, filePath)
}

// ListDownloads returns all downloaded or queued chapters for a media item.
func (s *DownloadService) ListDownloads(mediaID string) []*DownloadedChapter {
	s.mu.RLock()
	defer s.mu.RUnlock()

	var list []*DownloadedChapter
	for _, item := range s.active {
		if mediaID == "" || item.MediaID == mediaID {
			list = append(list, item)
		}
	}
	return list
}

// DeleteDownload removes a downloaded chapter from disk.
func (s *DownloadService) DeleteDownload(providerID, mediaID, chapterID string) error {
	k := s.makeKey(providerID, mediaID, chapterID)
	s.mu.Lock()
	item, exists := s.active[k]
	delete(s.active, k)
	s.mu.Unlock()

	if !exists {
		return ErrDownloadNotFound
	}

	return os.RemoveAll(item.LocalPath)
}
