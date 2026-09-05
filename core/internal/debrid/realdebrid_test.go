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

func TestRealDebrid_GetAccountStatus(t *testing.T) {
	ts := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if r.URL.Path != "/user" {
			http.NotFound(w, r)
			return
		}
		if r.Header.Get("Authorization") != "Bearer valid_key" {
			http.Error(w, "Unauthorized", http.StatusUnauthorized)
			return
		}

		w.Header().Set("Content-Type", "application/json")
		_ = json.NewEncoder(w).Encode(map[string]any{
			"id":         999,
			"username":   "testuser",
			"email":      "test@example.com",
			"points":     300,
			"type":       "premium",
			"expiration": "2026-12-31T23:59:59.000Z",
		})
	}))
	defer ts.Close()

	p := debrid.NewRealDebridProvider("valid_key")
	p.SetBaseURL(ts.URL)

	ctx := context.Background()
	status, err := p.GetAccountStatus(ctx)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}

	if status.Username != "testuser" || !status.IsPremium || status.Points != 300 {
		t.Errorf("unexpected status: %+v", status)
	}

	// Test unauthorized
	p.SetAPIKey("invalid_key")
	_, err = p.GetAccountStatus(ctx)
	if err != debrid.ErrInvalidAPIKey {
		t.Errorf("expected ErrInvalidAPIKey, got %v", err)
	}
}

func TestRealDebrid_ResolveMagnet(t *testing.T) {
	ts := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		switch r.URL.Path {
		case "/torrents/addMagnet":
			w.Header().Set("Content-Type", "application/json")
			w.WriteHeader(http.StatusCreated)
			_ = json.NewEncoder(w).Encode(map[string]any{
				"id":  "T123",
				"uri": "https://api.real-debrid.com/rest/1.0/torrents/info/T123",
			})

		case "/torrents/info/T123":
			w.Header().Set("Content-Type", "application/json")
			_ = json.NewEncoder(w).Encode(map[string]any{
				"id":       "T123",
				"filename": "Batman.Begins.2005.1080p.BluRay.x264",
				"status":   "downloaded",
				"files": []map[string]any{
					{"id": 1, "path": "/sample.mp4", "bytes": 10000000},
					{"id": 2, "path": "/Batman.Begins.2005.1080p.mkv", "bytes": 4500000000},
				},
				"links": []string{"https://real-debrid.com/d/LINK1"},
			})

		case "/torrents/selectFiles/T123":
			w.WriteHeader(http.StatusNoContent)

		case "/unrestrict/link":
			w.Header().Set("Content-Type", "application/json")
			_ = json.NewEncoder(w).Encode(map[string]any{
				"id":         "U456",
				"filename":   "Batman.Begins.2005.1080p.mkv",
				"filesize":   4500000000,
				"download":   "https://12.download.real-debrid.com/d/LINK1/Batman.Begins.2005.1080p.mkv",
				"streamable": 1,
			})

		default:
			http.NotFound(w, r)
		}
	}))
	defer ts.Close()

	p := debrid.NewRealDebridProvider("valid_key")
	p.SetBaseURL(ts.URL)

	magnet := "magnet:?xt=urn:btih:c12fe1c06bba254a9dc9f519b335380dc742230b&dn=Batman.Begins.2005.1080p"
	res, err := p.ResolveMagnet(context.Background(), magnet, 0, 0)
	if err != nil {
		t.Fatalf("ResolveMagnet failed: %v", err)
	}

	if !strings.HasPrefix(res.PlaybackURL, "https://12.download.real-debrid.com") {
		t.Errorf("unexpected playback url: %s", res.PlaybackURL)
	}
	if res.Quality != "1080p" {
		t.Errorf("expected quality '1080p', got %s", res.Quality)
	}
	if res.FileSize != 4500000000 {
		t.Errorf("expected filesize 4500000000, got %d", res.FileSize)
	}
	if !res.IsCached {
		t.Errorf("expected IsCached to be true")
	}
}
