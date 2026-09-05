package server_test

import (
	"bytes"
	"encoding/json"
	"io"
	"net/http"
	"net/http/httptest"
	"strings"
	"testing"

	"github.com/falsisdev/vessel/core/internal/debrid"
	"github.com/falsisdev/vessel/core/internal/server"
	"github.com/falsisdev/vessel/core/internal/service"
	"github.com/falsisdev/vessel/core/internal/storage"
	"github.com/falsisdev/vessel/core/internal/streaming"
	"github.com/falsisdev/vessel/core/internal/theme"
)

func TestGatewayServer_StaticAndAPI(t *testing.T) {
	memStore, err := storage.NewSQLiteStorage(":memory:")
	if err != nil {
		t.Fatalf("failed to create sqlite: %v", err)
	}
	defer memStore.Close()

	libSvc := service.NewLibraryService(memStore)
	themeMgr := theme.NewManager()
	proxy, _ := streaming.NewProxy()
	defer proxy.Close()
	debridMgr := debrid.NewManager(memStore)
	streamSvc := service.NewStreamService(debridMgr, proxy)

	gw := server.NewGatewayServer("127.0.0.1:0", nil, nil, libSvc, streamSvc, nil, themeMgr, proxy)
	if err := gw.Start(); err != nil {
		t.Fatalf("failed to start gateway: %v", err)
	}
	defer gw.Stop()

	baseURL := gw.URL()

	// 1. Test Static Index.html
	resp, err := http.Get(baseURL + "/")
	if err != nil {
		t.Fatalf("failed to GET /: %v", err)
	}
	if resp.StatusCode != http.StatusOK {
		t.Errorf("expected 200 for /, got %d", resp.StatusCode)
	}
	body, _ := io.ReadAll(resp.Body)
	resp.Body.Close()
	if !strings.Contains(string(body), "VESSEL") {
		t.Errorf("expected HTML to contain 'VESSEL'")
	}

	// 2. Test Styles.css
	resp, err = http.Get(baseURL + "/styles.css")
	if err != nil {
		t.Fatalf("failed to GET /styles.css: %v", err)
	}
	if resp.StatusCode != http.StatusOK {
		t.Errorf("expected 200 for /styles.css, got %d", resp.StatusCode)
	}
	cssBody, _ := io.ReadAll(resp.Body)
	resp.Body.Close()
	if !strings.Contains(string(cssBody), "--v-bg-base") {
		t.Errorf("expected styles.css to contain '--v-bg-base'")
	}

	// 3. Test App.js
	resp, err = http.Get(baseURL + "/app.js")
	if err != nil {
		t.Fatalf("failed to GET /app.js: %v", err)
	}
	if resp.StatusCode != http.StatusOK {
		t.Errorf("expected 200 for /app.js, got %d", resp.StatusCode)
	}
	resp.Body.Close()

	// 4. Test /api/ping
	resp, err = http.Get(baseURL + "/api/ping")
	if err != nil {
		t.Fatalf("failed to GET /api/ping: %v", err)
	}
	var pingData map[string]any
	_ = json.NewDecoder(resp.Body).Decode(&pingData)
	resp.Body.Close()
	if pingData["status"] != "ok" {
		t.Errorf("unexpected ping status: %+v", pingData)
	}

	// 5. Test /api/locales
	resp, err = http.Get(baseURL + "/api/locales")
	if err != nil {
		t.Fatalf("failed to GET /api/locales: %v", err)
	}
	var localesData struct {
		Locales []map[string]any `json:"locales"`
	}
	_ = json.NewDecoder(resp.Body).Decode(&localesData)
	resp.Body.Close()
	if len(localesData.Locales) != 12 {
		t.Errorf("expected 12 locales, got %d", len(localesData.Locales))
	}

	// 6. Test /api/themes and /api/theme/active
	resp, err = http.Get(baseURL + "/api/themes")
	if err != nil {
		t.Fatalf("failed to GET /api/themes: %v", err)
	}
	var themesData struct {
		Themes []map[string]any `json:"themes"`
	}
	_ = json.NewDecoder(resp.Body).Decode(&themesData)
	resp.Body.Close()
	if len(themesData.Themes) == 0 {
		t.Errorf("expected non-empty themes list")
	}

	// Switch theme via POST /api/theme/active
	switchPayload, _ := json.Marshal(map[string]string{
		"theme_id":   "dracula",
		"variant_id": "dracula",
	})
	resp, err = http.Post(baseURL+"/api/theme/active", "application/json", bytes.NewReader(switchPayload))
	if err != nil {
		t.Fatalf("failed to POST /api/theme/active: %v", err)
	}
	var activeData struct {
		Theme struct {
			ID string `json:"id"`
		} `json:"theme"`
		CompiledCSS string `json:"compiled_css"`
	}
	_ = json.NewDecoder(resp.Body).Decode(&activeData)
	resp.Body.Close()
	if activeData.Theme.ID != "dracula" {
		t.Errorf("expected active theme 'dracula', got %s", activeData.Theme.ID)
	}
	if !strings.Contains(activeData.CompiledCSS, "--v-bg-base") {
		t.Errorf("expected compiled CSS in response")
	}

	// 7. Test /api/library CRUD
	itemJSON, _ := json.Marshal(map[string]any{
		"provider_id": "cinemasis",
		"media_id":    "movie:123",
		"domain":      1,
		"title":       "Interstellar",
		"type":        1,
		"status":      "PLAN_TO_WATCH",
	})
	resp, err = http.Post(baseURL+"/api/library", "application/json", bytes.NewReader(itemJSON))
	if err != nil {
		t.Fatalf("failed to POST /api/library: %v", err)
	}
	if resp.StatusCode != http.StatusOK {
		t.Errorf("expected 200 on library post, got %d", resp.StatusCode)
	}
	resp.Body.Close()

	resp, err = http.Get(baseURL + "/api/library?status=PLAN_TO_WATCH")
	if err != nil {
		t.Fatalf("failed to GET /api/library: %v", err)
	}
	var libList struct {
		Items []any `json:"items"`
		Total int   `json:"total"`
	}
	_ = json.NewDecoder(resp.Body).Decode(&libList)
	resp.Body.Close()
	if libList.Total != 1 || len(libList.Items) != 1 {
		t.Errorf("expected 1 library item, got total: %d", libList.Total)
	}

	// 8. Test /api/stream/resolve
	resolveJSON, _ := json.Marshal(map[string]any{
		"stream_url": "https://sample.test/movie.mp4",
		"title":      "Test Movie",
	})
	resp, err = http.Post(baseURL+"/api/stream/resolve", "application/json", bytes.NewReader(resolveJSON))
	if err != nil {
		t.Fatalf("failed to POST /api/stream/resolve: %v", err)
	}
	var resolveData struct {
		Stream struct {
			PlaybackURL string `json:"playback_url"`
			StreamType  string `json:"stream_type"`
		} `json:"stream"`
	}
	_ = json.NewDecoder(resp.Body).Decode(&resolveData)
	resp.Body.Close()
	if resolveData.Stream.StreamType != "direct" {
		t.Errorf("expected direct stream type, got %s", resolveData.Stream.StreamType)
	}
}

func TestGatewayServer_StreamProxyRange(t *testing.T) {
	content := "0123456789ABCDEF"
	upstream := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Content-Type", "video/mp4")
		w.Header().Set("Accept-Ranges", "bytes")
		w.Header().Set("Content-Length", "16")
		w.WriteHeader(http.StatusOK)
		_, _ = w.Write([]byte(content))
	}))
	defer upstream.Close()

	gw := server.NewGatewayServer("127.0.0.1:0", nil, nil, nil, nil, nil, nil, nil)
	if err := gw.Start(); err != nil {
		t.Fatalf("failed to start gateway: %v", err)
	}
	defer gw.Stop()

	// Hit /stream?url=...
	proxyURL := gw.URL() + "/stream?url=" + upstream.URL
	resp, err := http.Get(proxyURL)
	if err != nil {
		t.Fatalf("failed to fetch from gateway proxy: %v", err)
	}
	defer resp.Body.Close()

	body, _ := io.ReadAll(resp.Body)
	if string(body) != content {
		t.Errorf("expected '%s', got '%s'", content, string(body))
	}
}
