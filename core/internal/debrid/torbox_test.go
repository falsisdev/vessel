package debrid_test

import (
	"context"
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"strings"
	"testing"

	"github.com/falsisdev/vessel/core/internal/debrid"
)

func TestTorBox_GetAccountStatus(t *testing.T) {
	ts := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if r.URL.Path != "/user/me" {
			http.NotFound(w, r)
			return
		}
		if r.Header.Get("Authorization") != "Bearer tb_valid_key" {
			http.Error(w, "Unauthorized", http.StatusUnauthorized)
			return
		}

		w.Header().Set("Content-Type", "application/json")
		_ = json.NewEncoder(w).Encode(map[string]any{
			"success": true,
			"data": map[string]any{
				"id":         101,
				"email":      "user@torbox.app",
				"plan":       2,
				"expires_at": "2026-12-31T23:59:59Z",
			},
		})
	}))
	defer ts.Close()

	p := debrid.NewTorBoxProvider("tb_valid_key")
	p.SetBaseURL(ts.URL)

	ctx := context.Background()
	status, err := p.GetAccountStatus(ctx)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}

	if status.Email != "user@torbox.app" || !status.IsPremium || status.Status != "plan-2" {
		t.Errorf("unexpected status: %+v", status)
	}
}

func TestTorBox_ResolveMagnet(t *testing.T) {
	ts := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		switch r.URL.Path {
		case "/torrents/createtorrent":
			w.Header().Set("Content-Type", "application/json")
			_ = json.NewEncoder(w).Encode(map[string]any{
				"success": true,
				"data": map[string]any{
					"torrent_id": 555,
				},
			})

		case "/torrents/mylist":
			w.Header().Set("Content-Type", "application/json")
			_ = json.NewEncoder(w).Encode(map[string]any{
				"success": true,
				"data": map[string]any{
					"id":   555,
					"name": "The.Dark.Knight.2008.2160p",
					"files": []map[string]any{
						{"id": 10, "name": "The.Dark.Knight.2008.2160p.mkv", "size": 18000000000},
					},
				},
			})

		case "/torrents/requestdl":
			w.Header().Set("Content-Type", "application/json")
			_ = json.NewEncoder(w).Encode(map[string]any{
				"success": true,
				"data":    "https://cdn.torbox.app/download/darkknight_2160p.mkv",
			})

		default:
			http.NotFound(w, r)
		}
	}))
	defer ts.Close()

	p := debrid.NewTorBoxProvider("tb_valid_key")
	p.SetBaseURL(ts.URL)

	magnet := "magnet:?xt=urn:btih:4XN3MVKJ23SFEQW6W3P3K24YDFL2B6H5&dn=The.Dark.Knight.2008.2160p"
	res, err := p.ResolveMagnet(context.Background(), magnet, 0, 0)
	if err != nil {
		t.Fatalf("TorBox ResolveMagnet failed: %v", err)
	}

	if !strings.HasPrefix(res.PlaybackURL, "https://cdn.torbox.app") {
		t.Errorf("unexpected playback url: %s", res.PlaybackURL)
	}
	if res.Quality != "4K" {
		t.Errorf("expected quality '4K', got %s", res.Quality)
	}
	if res.FileSize != 18000000000 {
		t.Errorf("expected filesize 18000000000, got %d", res.FileSize)
	}
}
