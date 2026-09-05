package integration_test

import (
	"context"
	"encoding/json"
	"io"
	"net"
	"net/http"
	"net/http/httptest"
	"strings"
	"testing"
	"time"

	coreclient "github.com/falsisdev/vessel/core/internal/client"
	"github.com/falsisdev/vessel/core/internal/debrid"
	"github.com/falsisdev/vessel/core/internal/plugin"
	coreserver "github.com/falsisdev/vessel/core/internal/server"
	"github.com/falsisdev/vessel/core/internal/service"
	"github.com/falsisdev/vessel/core/internal/storage"
	"github.com/falsisdev/vessel/core/internal/streaming"
	"github.com/falsisdev/vessel/core/internal/theme"
)

func TestStreamAndDebridOverIPC(t *testing.T) {
	ctx, cancel := context.WithTimeout(context.Background(), 15*time.Second)
	defer cancel()

	// 1. Mock Video CDN Server
	videoPayload := "STREAMING_TEST_VIDEO_DATA_0123456789"
	cdnServer := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Content-Type", "video/mp4")
		w.Header().Set("Accept-Ranges", "bytes")
		w.WriteHeader(http.StatusOK)
		_, _ = w.Write([]byte(videoPayload))
	}))
	defer cdnServer.Close()

	// 2. Mock Real-Debrid API Server
	rdServer := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		switch r.URL.Path {
		case "/user":
			if r.Header.Get("Authorization") != "Bearer test_rd_token" {
				http.Error(w, "Unauthorized", http.StatusUnauthorized)
				return
			}
			w.Header().Set("Content-Type", "application/json")
			_ = json.NewEncoder(w).Encode(map[string]any{
				"id":         1,
				"username":   "debrid_guru",
				"email":      "guru@debrid.test",
				"points":     888,
				"type":       "premium",
				"expiration": "2027-01-01T00:00:00Z",
			})

		case "/torrents/addMagnet":
			w.Header().Set("Content-Type", "application/json")
			w.WriteHeader(http.StatusCreated)
			_ = json.NewEncoder(w).Encode(map[string]any{
				"id":  "TORRENT_99",
				"uri": "https://api.real-debrid.com/rest/1.0/torrents/info/TORRENT_99",
			})

		case "/torrents/info/TORRENT_99":
			w.Header().Set("Content-Type", "application/json")
			_ = json.NewEncoder(w).Encode(map[string]any{
				"id":       "TORRENT_99",
				"filename": "Batman.2022.1080p.mkv",
				"status":   "downloaded",
				"files": []map[string]any{
					{"id": 1, "path": "/Batman.2022.1080p.mkv", "bytes": 8000000000},
				},
				"links": []string{"https://real-debrid.com/d/LINK99"},
			})

		case "/torrents/selectFiles/TORRENT_99":
			w.WriteHeader(http.StatusNoContent)

		case "/unrestrict/link":
			w.Header().Set("Content-Type", "application/json")
			_ = json.NewEncoder(w).Encode(map[string]any{
				"id":         "UNRESTRICT_99",
				"filename":   "Batman.2022.1080p.mkv",
				"filesize":   8000000000,
				"download":   cdnServer.URL + "/Batman.2022.1080p.mkv",
				"streamable": 1,
			})

		default:
			http.NotFound(w, r)
		}
	}))
	defer rdServer.Close()

	// 3. Setup SQLite storage and Debrid Manager
	store, err := storage.NewSQLiteStorage(":memory:")
	if err != nil {
		t.Fatalf("failed to init sqlite: %v", err)
	}
	defer store.Close()

	debridMgr := debrid.NewManager(store)
	rdProvider := debrid.NewRealDebridProvider("")
	rdProvider.SetBaseURL(rdServer.URL)
	debridMgr.RegisterProvider(rdProvider, true)

	proxy, err := streaming.NewProxy()
	if err != nil {
		t.Fatalf("failed to start streaming proxy: %v", err)
	}
	defer proxy.Close()

	streamSvc := service.NewStreamService(debridMgr, proxy)
	pluginMgr := plugin.NewManager()
	themeMgr := theme.NewManager()

	// 4. Start Core IPC Server
	l, err := net.Listen("tcp", "127.0.0.1:0")
	if err != nil {
		t.Fatalf("failed to listen on tcp: %v", err)
	}
	tcpAddr := l.Addr().String()
	_ = l.Close()

	srv := coreserver.NewServer(coreserver.ServerConfig{
		ListenAddr: tcpAddr,
		Version:    "1.0.0-stream-test",
	}, nil, nil, nil, streamSvc, pluginMgr, themeMgr)

	if err := srv.Start(); err != nil {
		t.Fatalf("failed to start core server: %v", err)
	}
	defer srv.Stop()

	// 5. Connect Client via Core IPC
	client, err := coreclient.Dial(ctx, tcpAddr)
	if err != nil {
		t.Fatalf("failed to dial core server: %v", err)
	}
	defer client.Close()

	// 6. Test ConfigureDebrid RPC
	cfgResp, err := client.ConfigureDebrid(ctx, "realdebrid", "test_rd_token", true)
	if err != nil {
		t.Fatalf("ConfigureDebrid failed: %v", err)
	}
	if !cfgResp.Success || !cfgResp.Status.IsPremium || cfgResp.Status.Username != "debrid_guru" {
		t.Errorf("unexpected ConfigureDebrid response: %+v", cfgResp)
	}

	// 7. Test GetDebridStatus RPC
	statusResp, err := client.GetDebridStatus(ctx, "realdebrid")
	if err != nil {
		t.Fatalf("GetDebridStatus failed: %v", err)
	}
	if len(statusResp.Accounts) != 1 || !statusResp.Accounts[0].IsPremium {
		t.Fatalf("unexpected GetDebridStatus response: %+v", statusResp)
	}

	// 8. Test ResolveStream with Magnet URI via Real-Debrid
	magnet := "magnet:?xt=urn:btih:c12fe1c06bba254a9dc9f519b335380dc742230b&dn=Batman.2022.1080p"
	res, err := client.ResolveStream(ctx, magnet, "The Batman", 0, 0, "realdebrid")
	if err != nil {
		t.Fatalf("ResolveStream failed: %v", err)
	}

	if !res.Stream.IsCached {
		t.Errorf("expected stream to be cached")
	}
	if res.Stream.StreamType != "debrid_cached" {
		t.Errorf("expected stream_type 'debrid_cached', got %s", res.Stream.StreamType)
	}
	if res.Stream.Provider != "realdebrid" {
		t.Errorf("expected provider 'realdebrid', got %s", res.Stream.Provider)
	}
	if res.Stream.Quality != "1080p" {
		t.Errorf("expected quality '1080p', got %s", res.Stream.Quality)
	}
	if !strings.Contains(res.Stream.PlaybackUrl, "/stream?url=") {
		t.Errorf("expected playback URL to be local proxy URL, got %s", res.Stream.PlaybackUrl)
	}

	// 9. Verify playback from local proxy
	resp, err := http.Get(res.Stream.PlaybackUrl)
	if err != nil {
		t.Fatalf("failed to fetch from proxy url: %v", err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusOK {
		t.Errorf("expected proxy HTTP 200, got %d", resp.StatusCode)
	}
	body, _ := io.ReadAll(resp.Body)
	if string(body) != videoPayload {
		t.Errorf("expected payload '%s', got '%s'", videoPayload, string(body))
	}

	// 10. Test Direct HTTP Stream Resolution
	directRes, err := client.ResolveStream(ctx, "https://mock.test/video.mp4", "Direct Movie", 0, 0, "")
	if err != nil {
		t.Fatalf("direct ResolveStream failed: %v", err)
	}
	if directRes.Stream.StreamType != "direct" || !directRes.Stream.IsCached {
		t.Errorf("unexpected direct stream result: %+v", directRes)
	}
}
