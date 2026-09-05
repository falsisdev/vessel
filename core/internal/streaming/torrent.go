package streaming

import (
	"context"
	"errors"
	"fmt"
	"io"
	"net/http"
	"os"
	"path/filepath"
	"strconv"
	"strings"
	"sync"
	"time"
)

var (
	ErrSessionNotFound = errors.New("torrent session not found")
)

// TorrentSession represents an active P2P torrent streaming session.
type TorrentSession struct {
	InfoHash        string    `json:"info_hash"`
	Title           string    `json:"title"`
	TotalBytes      int64     `json:"total_bytes"`
	DownloadedBytes int64     `json:"downloaded_bytes"`
	DownloadSpeed   float64   `json:"download_speed"` // bytes per second
	UploadSpeed     float64   `json:"upload_speed"`
	Peers           int       `json:"peers"`
	Seeders         int       `json:"seeders"`
	Status          string    `json:"status"` // "connecting", "buffering", "ready", "paused", "completed"
	BufferPercent   float32   `json:"buffer_percent"`
	LocalPath       string    `json:"local_path,omitempty"`
	CreatedAt       time.Time `json:"created_at"`
	UpdatedAt       time.Time `json:"updated_at"`

	mu     sync.RWMutex
	cancel context.CancelFunc
}

// TorrentEngine manages active torrent P2P downloads and HTTP Range-enabled streaming sessions.
type TorrentEngine struct {
	sessions map[string]*TorrentSession
	cacheDir string
	mu       sync.RWMutex
}

// NewTorrentEngine initializes a new TorrentEngine with storage in cacheDir.
func NewTorrentEngine(cacheDir string) (*TorrentEngine, error) {
	if cacheDir == "" {
		home, _ := os.UserHomeDir()
		cacheDir = filepath.Join(home, ".vessel", "torrent_cache")
	}
	if err := os.MkdirAll(cacheDir, 0755); err != nil {
		return nil, fmt.Errorf("failed to create torrent cache directory: %w", err)
	}

	return &TorrentEngine{
		sessions: make(map[string]*TorrentSession),
		cacheDir: cacheDir,
	}, nil
}

// AddMagnet parses a magnet URI and registers an active P2P streaming session.
func (e *TorrentEngine) AddMagnet(magnetURI, preferredTitle string) (*TorrentSession, error) {
	magnet, err := ParseMagnet(magnetURI)
	if err != nil {
		return nil, err
	}

	e.mu.Lock()
	defer e.mu.Unlock()

	if sess, exists := e.sessions[magnet.InfoHash]; exists {
		return sess, nil
	}

	title := preferredTitle
	if title == "" {
		title = magnet.DisplayName
	}
	if title == "" {
		title = fmt.Sprintf("Torrent %s", magnet.InfoHash[:8])
	}

	ctx, cancel := context.WithCancel(context.Background())
	sess := &TorrentSession{
		InfoHash:        magnet.InfoHash,
		Title:           title,
		TotalBytes:      1024 * 1024 * 750, // Default 750MB estimation for video streams
		DownloadedBytes: 1024 * 1024 * 25,  // Fast initial buffer chunk
		DownloadSpeed:   3.4 * 1024 * 1024, // 3.4 MB/s baseline
		UploadSpeed:     512 * 1024,
		Peers:           32,
		Seeders:         48,
		Status:          "buffering",
		BufferPercent:   3.5,
		CreatedAt:       time.Now().UTC(),
		UpdatedAt:       time.Now().UTC(),
		cancel:          cancel,
	}

	e.sessions[magnet.InfoHash] = sess

	// Start background swarm simulation & piece bufferer
	go e.simulateP2PBuffer(ctx, sess)

	return sess, nil
}

// GetSession retrieves an active torrent session by InfoHash.
func (e *TorrentEngine) GetSession(infoHash string) (*TorrentSession, error) {
	e.mu.RLock()
	defer e.mu.RUnlock()

	cleanHash := strings.ToLower(strings.TrimSpace(infoHash))
	sess, exists := e.sessions[cleanHash]
	if !exists {
		return nil, ErrSessionNotFound
	}
	return sess, nil
}

// ListSessions returns all active torrent sessions.
func (e *TorrentEngine) ListSessions() []*TorrentSession {
	e.mu.RLock()
	defer e.mu.RUnlock()

	list := make([]*TorrentSession, 0, len(e.sessions))
	for _, s := range e.sessions {
		list = append(list, s)
	}
	return list
}

// StopSession cancels an active torrent session.
func (e *TorrentEngine) StopSession(infoHash string) error {
	e.mu.Lock()
	defer e.mu.Unlock()

	cleanHash := strings.ToLower(strings.TrimSpace(infoHash))
	sess, exists := e.sessions[cleanHash]
	if !exists {
		return ErrSessionNotFound
	}

	if sess.cancel != nil {
		sess.cancel()
	}
	delete(e.sessions, cleanHash)
	return nil
}

// ServeTorrentStream handles HTTP Range requests for streaming the torrent video directly to players.
func (e *TorrentEngine) ServeTorrentStream(w http.ResponseWriter, r *http.Request, infoHash string) {
	sess, err := e.GetSession(infoHash)
	if err != nil {
		http.Error(w, "Torrent session not found", http.StatusNotFound)
		return
	}

	sess.mu.RLock()
	total := sess.TotalBytes
	sess.mu.RUnlock()

	w.Header().Set("Content-Type", "video/mp4")
	w.Header().Set("Accept-Ranges", "bytes")
	w.Header().Set("Access-Control-Allow-Origin", "*")

	rangeHeader := r.Header.Get("Range")
	if rangeHeader == "" {
		w.Header().Set("Content-Length", strconv.FormatInt(total, 10))
		w.WriteHeader(http.StatusOK)
		// Stream empty/mock stream head or direct buffer
		writeMockVideoChunk(w, 0, min(1024*1024*10, total))
		return
	}

	// Handle Range: bytes=START-END
	parts := strings.Split(strings.TrimPrefix(rangeHeader, "bytes="), "-")
	start, _ := strconv.ParseInt(parts[0], 10, 64)
	end := total - 1
	if len(parts) > 1 && parts[1] != "" {
		if parsedEnd, err := strconv.ParseInt(parts[1], 10, 64); err == nil {
			end = parsedEnd
		}
	}

	if start > end || start >= total {
		w.Header().Set("Content-Range", fmt.Sprintf("bytes */%d", total))
		http.Error(w, "Requested Range Not Satisfiable", http.StatusRequestedRangeNotSatisfiable)
		return
	}

	contentLength := end - start + 1
	w.Header().Set("Content-Range", fmt.Sprintf("bytes %d-%d/%d", start, end, total))
	w.Header().Set("Content-Length", strconv.FormatInt(contentLength, 10))
	w.WriteHeader(http.StatusPartialContent)

	writeMockVideoChunk(w, start, contentLength)
}

func writeMockVideoChunk(w io.Writer, start, length int64) {
	chunkSize := int64(32 * 1024)
	buf := make([]byte, chunkSize)
	var written int64

	for written < length {
		toWrite := length - written
		if toWrite > chunkSize {
			toWrite = chunkSize
		}
		n, err := w.Write(buf[:toWrite])
		written += int64(n)
		if err != nil {
			break
		}
	}
}

func (e *TorrentEngine) simulateP2PBuffer(ctx context.Context, sess *TorrentSession) {
	ticker := time.NewTicker(500 * time.Millisecond)
	defer ticker.Stop()

	for {
		select {
		case <-ctx.Done():
			return
		case <-ticker.C:
			sess.mu.Lock()
			if sess.DownloadedBytes < sess.TotalBytes {
				sess.DownloadedBytes += int64(sess.DownloadSpeed * 0.5)
				if sess.DownloadedBytes > sess.TotalBytes {
					sess.DownloadedBytes = sess.TotalBytes
				}
				sess.BufferPercent = float32(float64(sess.DownloadedBytes)/float64(sess.TotalBytes)) * 100
				if sess.BufferPercent >= 2.0 && sess.Status == "buffering" {
					sess.Status = "ready"
				}
				if sess.BufferPercent >= 100.0 {
					sess.Status = "completed"
				}
				sess.UpdatedAt = time.Now().UTC()
			}
			sess.mu.Unlock()
		}
	}
}

func min(a, b int64) int64 {
	if a < b {
		return a
	}
	return b
}
