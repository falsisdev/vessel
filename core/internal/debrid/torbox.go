package debrid

import (
	"context"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"net/url"
	"strings"
	"sync"
	"time"

	"github.com/falsisdev/vessel/core/internal/streaming"
)

type TorBoxProvider struct {
	apiKey  string
	baseURL string
	client  *http.Client
	mu      sync.RWMutex
}

func NewTorBoxProvider(apiKey string) *TorBoxProvider {
	return &TorBoxProvider{
		apiKey:  apiKey,
		baseURL: "https://api.torbox.app/v1/api",
		client: &http.Client{
			Timeout: 20 * time.Second,
		},
	}
}

func (p *TorBoxProvider) SetBaseURL(url string) {
	p.mu.Lock()
	defer p.mu.Unlock()
	p.baseURL = url
}

func (p *TorBoxProvider) Name() string {
	return "torbox"
}

func (p *TorBoxProvider) IsConfigured() bool {
	p.mu.RLock()
	defer p.mu.RUnlock()
	return strings.TrimSpace(p.apiKey) != ""
}

func (p *TorBoxProvider) SetAPIKey(apiKey string) {
	p.mu.Lock()
	defer p.mu.Unlock()
	p.apiKey = strings.TrimSpace(apiKey)
}

func (p *TorBoxProvider) GetAccountStatus(ctx context.Context) (*AccountStatus, error) {
	p.mu.RLock()
	key := p.apiKey
	base := p.baseURL
	p.mu.RUnlock()

	if key == "" {
		return nil, ErrNotConfigured
	}

	req, err := http.NewRequestWithContext(ctx, http.MethodGet, fmt.Sprintf("%s/user/me", base), nil)
	if err != nil {
		return nil, err
	}
	req.Header.Set("Authorization", fmt.Sprintf("Bearer %s", key))

	resp, err := p.client.Do(req)
	if err != nil {
		return nil, fmt.Errorf("failed to reach torbox: %w", err)
	}
	defer resp.Body.Close()

	if resp.StatusCode == http.StatusUnauthorized || resp.StatusCode == http.StatusForbidden {
		return nil, ErrInvalidAPIKey
	}
	if resp.StatusCode != http.StatusOK {
		return nil, fmt.Errorf("%w: status %d", ErrProviderFailure, resp.StatusCode)
	}

	var tbResp struct {
		Success bool `json:"success"`
		Data    struct {
			ID        any    `json:"id"`
			Email     string `json:"email"`
			Plan      int    `json:"plan"`
			ExpiresAt string `json:"expires_at"`
		} `json:"data"`
	}

	if err := json.NewDecoder(resp.Body).Decode(&tbResp); err != nil {
		return nil, fmt.Errorf("failed to decode torbox user: %w", err)
	}

	var expTimestamp int64
	if tbResp.Data.ExpiresAt != "" {
		t, err := time.Parse(time.RFC3339, tbResp.Data.ExpiresAt)
		if err == nil {
			expTimestamp = t.Unix()
		}
	}

	isPremium := tbResp.Data.Plan > 0
	status := "free"
	if isPremium {
		status = fmt.Sprintf("plan-%d", tbResp.Data.Plan)
	}

	return &AccountStatus{
		Provider:            p.Name(),
		Username:            tbResp.Data.Email,
		Email:               tbResp.Data.Email,
		ExpirationTimestamp: expTimestamp,
		IsPremium:           isPremium,
		Points:              0,
		Status:              status,
	}, nil
}

func (p *TorBoxProvider) CheckAvailability(ctx context.Context, hashes []string) (map[string]bool, error) {
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
		req, err := http.NewRequestWithContext(ctx, http.MethodGet, fmt.Sprintf("%s/torrents/checkcached?hash=%s&format=list", base, hashLower), nil)
		if err != nil {
			continue
		}
		req.Header.Set("Authorization", fmt.Sprintf("Bearer %s", key))

		resp, err := p.client.Do(req)
		if err != nil {
			continue
		}

		if resp.StatusCode == http.StatusOK {
			var checkResp struct {
				Success bool     `json:"success"`
				Data    []string `json:"data"`
			}
			if err := json.NewDecoder(resp.Body).Decode(&checkResp); err == nil && checkResp.Success {
				for _, h := range checkResp.Data {
					if strings.EqualFold(h, hashLower) {
						result[hashLower] = true
					}
				}
			}
		}
		resp.Body.Close()
	}

	return result, nil
}

func (p *TorBoxProvider) ResolveMagnet(ctx context.Context, magnetURI string, season, episode int) (*StreamResult, error) {
	p.mu.RLock()
	key := p.apiKey
	base := p.baseURL
	p.mu.RUnlock()

	if key == "" {
		return nil, ErrNotConfigured
	}

	_, err := streaming.ParseMagnet(magnetURI)
	if err != nil {
		return nil, fmt.Errorf("invalid magnet: %w", err)
	}

	// 1. Create Torrent on TorBox
	formData := url.Values{}
	formData.Set("magnet", magnetURI)

	createReq, err := http.NewRequestWithContext(ctx, http.MethodPost, fmt.Sprintf("%s/torrents/createtorrent", base), strings.NewReader(formData.Encode()))
	if err != nil {
		return nil, err
	}
	createReq.Header.Set("Authorization", fmt.Sprintf("Bearer %s", key))
	createReq.Header.Set("Content-Type", "application/x-www-form-urlencoded")

	createResp, err := p.client.Do(createReq)
	if err != nil {
		return nil, fmt.Errorf("failed to create torrent on torbox: %w", err)
	}
	defer createResp.Body.Close()

	if createResp.StatusCode != http.StatusOK && createResp.StatusCode != http.StatusCreated {
		body, _ := io.ReadAll(createResp.Body)
		return nil, fmt.Errorf("%w: status %d: %s", ErrProviderFailure, createResp.StatusCode, string(body))
	}

	var createResult struct {
		Success bool `json:"success"`
		Data    struct {
			TorrentID any `json:"torrent_id"`
		} `json:"data"`
	}
	if err := json.NewDecoder(createResp.Body).Decode(&createResult); err != nil {
		return nil, fmt.Errorf("failed to decode createtorrent response: %w", err)
	}

	var torrentID string
	switch v := createResult.Data.TorrentID.(type) {
	case float64:
		torrentID = fmt.Sprintf("%.0f", v)
	case string:
		torrentID = v
	default:
		torrentID = fmt.Sprintf("%v", v)
	}

	// 2. Fetch Torrent file list
	listReq, err := http.NewRequestWithContext(ctx, http.MethodGet, fmt.Sprintf("%s/torrents/mylist?id=%s", base, torrentID), nil)
	if err != nil {
		return nil, err
	}
	listReq.Header.Set("Authorization", fmt.Sprintf("Bearer %s", key))

	listResp, err := p.client.Do(listReq)
	if err != nil {
		return nil, fmt.Errorf("failed to fetch torbox torrent info: %w", err)
	}
	defer listResp.Body.Close()

	var listResult struct {
		Success bool `json:"success"`
		Data    struct {
			ID    any    `json:"id"`
			Name  string `json:"name"`
			Files []struct {
				ID   any    `json:"id"`
				Name string `json:"name"`
				Size int64  `json:"size"`
			} `json:"files"`
		} `json:"data"`
	}
	if err := json.NewDecoder(listResp.Body).Decode(&listResult); err != nil {
		return nil, fmt.Errorf("failed to decode mylist response: %w", err)
	}

	if len(listResult.Data.Files) == 0 {
		return nil, ErrNoMatchingFile
	}

	// 3. Select best file
	selectedFileID, selectedFileName, selectedFileSize := p.selectBestFile(listResult.Data.Files, season, episode)

	// 4. Request Download Link
	dlReq, err := http.NewRequestWithContext(ctx, http.MethodGet, fmt.Sprintf("%s/torrents/requestdl?token=%s&torrent_id=%s&file_id=%s&zip=false", base, url.QueryEscape(key), torrentID, selectedFileID), nil)
	if err != nil {
		return nil, err
	}
	dlReq.Header.Set("Authorization", fmt.Sprintf("Bearer %s", key))

	dlResp, err := p.client.Do(dlReq)
	if err != nil {
		return nil, fmt.Errorf("failed to request dl from torbox: %w", err)
	}
	defer dlResp.Body.Close()

	var dlResult struct {
		Success bool   `json:"success"`
		Data    string `json:"data"`
	}
	if err := json.NewDecoder(dlResp.Body).Decode(&dlResult); err != nil {
		return nil, fmt.Errorf("failed to decode requestdl response: %w", err)
	}

	if !dlResult.Success || dlResult.Data == "" {
		return nil, fmt.Errorf("%w: torbox returned no download url", ErrProviderFailure)
	}

	return &StreamResult{
		OriginalURL: magnetURI,
		PlaybackURL: dlResult.Data,
		StreamType:  "debrid_cached",
		Provider:    p.Name(),
		Filename:    selectedFileName,
		FileSize:    selectedFileSize,
		Quality:     DetectQuality(selectedFileName),
		IsCached:    true,
	}, nil
}

func (p *TorBoxProvider) selectBestFile(files []struct {
	ID   any    `json:"id"`
	Name string `json:"name"`
	Size int64  `json:"size"`
}, season, episode int) (fileIDStr, fileName string, size int64) {
	videoExtensions := map[string]bool{
		".mkv": true, ".mp4": true, ".avi": true, ".ts": true, ".webm": true, ".mov": true,
	}

	var candidates []struct {
		ID   string
		Name string
		Size int64
	}

	for _, f := range files {
		idStr := fmt.Sprintf("%v", f.ID)
		lower := strings.ToLower(f.Name)
		for ext := range videoExtensions {
			if strings.HasSuffix(lower, ext) {
				candidates = append(candidates, struct {
					ID   string
					Name string
					Size int64
				}{ID: idStr, Name: f.Name, Size: f.Size})
				break
			}
		}
	}

	if len(candidates) == 0 {
		first := files[0]
		return fmt.Sprintf("%v", first.ID), first.Name, first.Size
	}

	if season > 0 && episode > 0 {
		patterns := []string{
			fmt.Sprintf("s%02de%02d", season, episode),
			fmt.Sprintf("s%de%d", season, episode),
			fmt.Sprintf("%dx%02d", season, episode),
			fmt.Sprintf("%dx%d", season, episode),
		}

		for _, c := range candidates {
			lower := strings.ToLower(c.Name)
			for _, pat := range patterns {
				if strings.Contains(lower, pat) {
					return c.ID, c.Name, c.Size
				}
			}
		}
	}

	// Largest file
	var best struct {
		ID   string
		Name string
		Size int64
	}
	for _, c := range candidates {
		if c.Size > best.Size {
			best = c
		}
	}

	return best.ID, best.Name, best.Size
}
