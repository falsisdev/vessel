package debrid

import (
	"context"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"net/url"
	"regexp"
	"strconv"
	"strings"
	"sync"
	"time"

	"github.com/falsisdev/vessel/core/internal/streaming"
)

type RealDebridProvider struct {
	apiKey  string
	baseURL string
	client  *http.Client
	mu      sync.RWMutex
}

func NewRealDebridProvider(apiKey string) *RealDebridProvider {
	return &RealDebridProvider{
		apiKey:  apiKey,
		baseURL: "https://api.real-debrid.com/rest/1.0",
		client: &http.Client{
			Timeout: 20 * time.Second,
		},
	}
}

func (p *RealDebridProvider) SetBaseURL(url string) {
	p.mu.Lock()
	defer p.mu.Unlock()
	p.baseURL = url
}

func (p *RealDebridProvider) Name() string {
	return "realdebrid"
}

func (p *RealDebridProvider) IsConfigured() bool {
	p.mu.RLock()
	defer p.mu.RUnlock()
	return strings.TrimSpace(p.apiKey) != ""
}

func (p *RealDebridProvider) SetAPIKey(apiKey string) {
	p.mu.Lock()
	defer p.mu.Unlock()
	p.apiKey = strings.TrimSpace(apiKey)
}

func (p *RealDebridProvider) GetAccountStatus(ctx context.Context) (*AccountStatus, error) {
	p.mu.RLock()
	key := p.apiKey
	base := p.baseURL
	p.mu.RUnlock()

	if key == "" {
		return nil, ErrNotConfigured
	}

	req, err := http.NewRequestWithContext(ctx, http.MethodGet, fmt.Sprintf("%s/user", base), nil)
	if err != nil {
		return nil, err
	}
	req.Header.Set("Authorization", fmt.Sprintf("Bearer %s", key))

	resp, err := p.client.Do(req)
	if err != nil {
		return nil, fmt.Errorf("failed to reach real-debrid: %w", err)
	}
	defer resp.Body.Close()

	if resp.StatusCode == http.StatusUnauthorized || resp.StatusCode == http.StatusForbidden {
		return nil, ErrInvalidAPIKey
	}
	if resp.StatusCode != http.StatusOK {
		return nil, fmt.Errorf("%w: status %d", ErrProviderFailure, resp.StatusCode)
	}

	var rdUser struct {
		ID         int64  `json:"id"`
		Username   string `json:"username"`
		Email      string `json:"email"`
		Points     int32  `json:"points"`
		Type       string `json:"type"`
		Expiration string `json:"expiration"`
	}

	if err := json.NewDecoder(resp.Body).Decode(&rdUser); err != nil {
		return nil, fmt.Errorf("failed to decode real-debrid user: %w", err)
	}

	isPremium := rdUser.Type == "premium"
	var expTimestamp int64
	if rdUser.Expiration != "" {
		t, err := time.Parse(time.RFC3339, rdUser.Expiration)
		if err == nil {
			expTimestamp = t.Unix()
		}
	}

	return &AccountStatus{
		Provider:            p.Name(),
		Username:            rdUser.Username,
		Email:               rdUser.Email,
		ExpirationTimestamp: expTimestamp,
		IsPremium:           isPremium,
		Points:              rdUser.Points,
		Status:              rdUser.Type,
	}, nil
}

func (p *RealDebridProvider) CheckAvailability(ctx context.Context, hashes []string) (map[string]bool, error) {
	p.mu.RLock()
	key := p.apiKey
	base := p.baseURL
	p.mu.RUnlock()

	result := make(map[string]bool)
	if key == "" || len(hashes) == 0 {
		return result, nil
	}

	for _, hash := range hashes {
		hashLower := strings.ToLower(hash)
		req, err := http.NewRequestWithContext(ctx, http.MethodGet, fmt.Sprintf("%s/torrents/instantAvailability/%s", base, hashLower), nil)
		if err != nil {
			continue
		}
		req.Header.Set("Authorization", fmt.Sprintf("Bearer %s", key))

		resp, err := p.client.Do(req)
		if err != nil {
			continue
		}

		if resp.StatusCode == http.StatusOK {
			var availResp map[string]struct {
				RD []map[string]any `json:"rd"`
			}
			if err := json.NewDecoder(resp.Body).Decode(&availResp); err == nil {
				if item, ok := availResp[hashLower]; ok && len(item.RD) > 0 {
					result[hashLower] = true
				}
			}
		}
		resp.Body.Close()
	}

	return result, nil
}

func (p *RealDebridProvider) ResolveMagnet(ctx context.Context, magnetURI string, season, episode int) (*StreamResult, error) {
	p.mu.RLock()
	key := p.apiKey
	base := p.baseURL
	p.mu.RUnlock()

	if key == "" {
		return nil, ErrNotConfigured
	}

	magnet, err := streaming.ParseMagnet(magnetURI)
	if err != nil {
		return nil, fmt.Errorf("invalid magnet: %w", err)
	}
	_ = magnet

	// 1. Add Magnet to Real-Debrid
	formData := url.Values{}
	formData.Set("magnet", magnetURI)

	addReq, err := http.NewRequestWithContext(ctx, http.MethodPost, fmt.Sprintf("%s/torrents/addMagnet", base), strings.NewReader(formData.Encode()))
	if err != nil {
		return nil, err
	}
	addReq.Header.Set("Authorization", fmt.Sprintf("Bearer %s", key))
	addReq.Header.Set("Content-Type", "application/x-www-form-urlencoded")

	addResp, err := p.client.Do(addReq)
	if err != nil {
		return nil, fmt.Errorf("failed to add magnet to real-debrid: %w", err)
	}
	defer addResp.Body.Close()

	if addResp.StatusCode != http.StatusCreated && addResp.StatusCode != http.StatusOK {
		body, _ := io.ReadAll(addResp.Body)
		return nil, fmt.Errorf("%w: status %d: %s", ErrProviderFailure, addResp.StatusCode, string(body))
	}

	var addResult struct {
		ID  string `json:"id"`
		URI string `json:"uri"`
	}
	if err := json.NewDecoder(addResp.Body).Decode(&addResult); err != nil {
		return nil, fmt.Errorf("failed to decode addMagnet response: %w", err)
	}

	// 2. Fetch Torrent Info
	info, err := p.getTorrentInfo(ctx, base, key, addResult.ID)
	if err != nil {
		return nil, err
	}

	// 3. Select matching or best video file
	selectedFileID := p.selectBestVideoFile(info.Files, season, episode)
	if selectedFileID <= 0 {
		// Fallback: select all
		selectedFileID = 0
	}

	selectData := url.Values{}
	if selectedFileID > 0 {
		selectData.Set("files", strconv.Itoa(selectedFileID))
	} else {
		selectData.Set("files", "all")
	}

	selReq, err := http.NewRequestWithContext(ctx, http.MethodPost, fmt.Sprintf("%s/torrents/selectFiles/%s", base, addResult.ID), strings.NewReader(selectData.Encode()))
	if err != nil {
		return nil, err
	}
	selReq.Header.Set("Authorization", fmt.Sprintf("Bearer %s", key))
	selReq.Header.Set("Content-Type", "application/x-www-form-urlencoded")

	selResp, err := p.client.Do(selReq)
	if err != nil {
		return nil, fmt.Errorf("failed to select files on real-debrid: %w", err)
	}
	selResp.Body.Close()

	// 4. Fetch updated Torrent Info to get generated download links
	info, err = p.getTorrentInfo(ctx, base, key, addResult.ID)
	if err != nil {
		return nil, err
	}

	if len(info.Links) == 0 {
		return nil, ErrNotCached
	}

	// 5. Unrestrict the first/best download link
	downloadLink := info.Links[0]
	unrestrictData := url.Values{}
	unrestrictData.Set("link", downloadLink)

	unReq, err := http.NewRequestWithContext(ctx, http.MethodPost, fmt.Sprintf("%s/unrestrict/link", base), strings.NewReader(unrestrictData.Encode()))
	if err != nil {
		return nil, err
	}
	unReq.Header.Set("Authorization", fmt.Sprintf("Bearer %s", key))
	unReq.Header.Set("Content-Type", "application/x-www-form-urlencoded")

	unResp, err := p.client.Do(unReq)
	if err != nil {
		return nil, fmt.Errorf("failed to unrestrict link on real-debrid: %w", err)
	}
	defer unResp.Body.Close()

	if unResp.StatusCode != http.StatusOK {
		return nil, fmt.Errorf("failed to unrestrict link, status: %d", unResp.StatusCode)
	}

	var unrestrictResult struct {
		ID       string `json:"id"`
		Filename string `json:"filename"`
		Filesize int64  `json:"filesize"`
		Download string `json:"download"`
	}
	if err := json.NewDecoder(unResp.Body).Decode(&unrestrictResult); err != nil {
		return nil, fmt.Errorf("failed to decode unrestrict response: %w", err)
	}

	return &StreamResult{
		OriginalURL: magnetURI,
		PlaybackURL: unrestrictResult.Download,
		StreamType:  "debrid_cached",
		Provider:    p.Name(),
		Filename:    unrestrictResult.Filename,
		FileSize:    unrestrictResult.Filesize,
		Quality:     DetectQuality(unrestrictResult.Filename),
		IsCached:    true,
	}, nil
}

type rdTorrentFile struct {
	ID    int    `json:"id"`
	Path  string `json:"path"`
	Bytes int64  `json:"bytes"`
}

type rdTorrentInfo struct {
	ID       string          `json:"id"`
	Filename string          `json:"filename"`
	Status   string          `json:"status"`
	Files    []rdTorrentFile `json:"files"`
	Links    []string        `json:"links"`
}

func (p *RealDebridProvider) getTorrentInfo(ctx context.Context, base, key, torrentID string) (*rdTorrentInfo, error) {
	req, err := http.NewRequestWithContext(ctx, http.MethodGet, fmt.Sprintf("%s/torrents/info/%s", base, torrentID), nil)
	if err != nil {
		return nil, err
	}
	req.Header.Set("Authorization", fmt.Sprintf("Bearer %s", key))

	resp, err := p.client.Do(req)
	if err != nil {
		return nil, err
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusOK {
		return nil, fmt.Errorf("getTorrentInfo status %d", resp.StatusCode)
	}

	var info rdTorrentInfo
	if err := json.NewDecoder(resp.Body).Decode(&info); err != nil {
		return nil, err
	}
	return &info, nil
}

func (p *RealDebridProvider) selectBestVideoFile(files []rdTorrentFile, season, episode int) int {
	videoExtensions := map[string]bool{
		".mkv": true, ".mp4": true, ".avi": true, ".ts": true, ".webm": true, ".mov": true,
	}

	var candidateFiles []rdTorrentFile

	for _, f := range files {
		lower := strings.ToLower(f.Path)
		for ext := range videoExtensions {
			if strings.HasSuffix(lower, ext) {
				candidateFiles = append(candidateFiles, f)
				break
			}
		}
	}

	if len(candidateFiles) == 0 {
		return 0
	}

	// If TV episode target is specified (e.g. S01E02 or 1x02)
	if season > 0 && episode > 0 {
		patterns := []string{
			fmt.Sprintf("s%02de%02d", season, episode),
			fmt.Sprintf("s%de%d", season, episode),
			fmt.Sprintf("%dx%02d", season, episode),
			fmt.Sprintf("%dx%d", season, episode),
		}

		for _, f := range candidateFiles {
			lower := strings.ToLower(f.Path)
			for _, pat := range patterns {
				if strings.Contains(lower, pat) {
					return f.ID
				}
			}
		}
	}

	// Movie or fallback: choose the largest file
	var bestID int
	var maxBytes int64
	for _, f := range candidateFiles {
		if f.Bytes > maxBytes {
			maxBytes = f.Bytes
			bestID = f.ID
		}
	}

	return bestID
}

// DetectQuality parses resolution hints from a filename or stream title.
func DetectQuality(name string) string {
	lower := strings.ToLower(name)
	switch {
	case strings.Contains(lower, "2160p") || strings.Contains(lower, "4k") || strings.Contains(lower, "uhd"):
		return "4K"
	case strings.Contains(lower, "1080p") || strings.Contains(lower, "fhd"):
		return "1080p"
	case strings.Contains(lower, "720p") || strings.Contains(lower, "hd"):
		return "720p"
	case strings.Contains(lower, "480p") || strings.Contains(lower, "sd"):
		return "480p"
	default:
		// Check regex for resolution like 1920x1080
		re := regexp.MustCompile(`\b(1080|720|480|2160)\b`)
		if match := re.FindString(lower); match != "" {
			return match + "p"
		}
		return "HD"
	}
}
