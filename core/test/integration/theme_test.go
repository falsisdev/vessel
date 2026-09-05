package integration_test

import (
	"context"
	"net"
	"os"
	"path/filepath"
	"strings"
	"testing"
	"time"

	coreclient "github.com/falsisdev/vessel/core/internal/client"
	"github.com/falsisdev/vessel/core/internal/plugin"
	coreserver "github.com/falsisdev/vessel/core/internal/server"
	"github.com/falsisdev/vessel/core/internal/service"
	"github.com/falsisdev/vessel/core/internal/theme"
)

func TestThemeAndLocaleOverIPC(t *testing.T) {
	// Create temporary custom theme directory
	tempDir := t.TempDir()
	customThemeDir := filepath.Join(tempDir, "synthwave")
	if err := os.MkdirAll(customThemeDir, 0755); err != nil {
		t.Fatalf("failed to create custom theme dir: %v", err)
	}

	synthwaveJSON := `{
		"id": "synthwave-84",
		"name": "Synthwave '84",
		"version": "1.0.0",
		"author": "RobbOwen",
		"default_variant": "rad",
		"variants": [
			{
				"id": "rad",
				"name": "Radical Neon",
				"is_dark": true,
				"tokens": {
					"bg_base": "#262335",
					"accent_primary": "#ff7edb"
				}
			}
		]
	}`
	if err := os.WriteFile(filepath.Join(customThemeDir, "theme.json"), []byte(synthwaveJSON), 0644); err != nil {
		t.Fatalf("failed to write theme.json: %v", err)
	}

	userCSS := `
.neon-glow {
	text-shadow: 0 0 8px #ff7edb;
}
`
	if err := os.WriteFile(filepath.Join(customThemeDir, "user.css"), []byte(userCSS), 0644); err != nil {
		t.Fatalf("failed to write user.css: %v", err)
	}

	mgr := plugin.NewManager()
	cinemaSvc := service.NewCinemaService(mgr, 3*time.Second)
	readingSvc := service.NewReadingService(mgr, 3*time.Second)
	themeMgr := theme.NewManager(tempDir)

	// Allocate free TCP port
	l, err := net.Listen("tcp", "127.0.0.1:0")
	if err != nil {
		t.Fatalf("failed to listen on ephemeral port: %v", err)
	}
	tcpAddr := l.Addr().String()
	_ = l.Close()

	srv := coreserver.NewServer(coreserver.ServerConfig{
		ListenAddr: tcpAddr,
		Version:    "1.0.0-theme-test",
	}, cinemaSvc, readingSvc, nil, nil, mgr, themeMgr)

	if err := srv.Start(); err != nil {
		t.Fatalf("failed to start core IPC server: %v", err)
	}
	defer srv.Stop()

	ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
	defer cancel()

	client, err := coreclient.Dial(ctx, tcpAddr)
	if err != nil {
		t.Fatalf("failed to dial core IPC: %v", err)
	}
	defer func() { _ = client.Close() }()

	// 1. Verify Locales
	localesResp, err := client.GetSupportedLocales(ctx)
	if err != nil {
		t.Fatalf("GetSupportedLocales error: %v", err)
	}
	if len(localesResp.Locales) != 12 {
		t.Fatalf("expected 12 locales, got %d", len(localesResp.Locales))
	}
	if localesResp.DefaultLocale != "en" {
		t.Errorf("expected default locale 'en', got %s", localesResp.DefaultLocale)
	}

	localeMap := make(map[string]bool)
	rtlCount := 0
	for _, l := range localesResp.Locales {
		localeMap[l.Code] = true
		if l.IsRtl {
			rtlCount++
		}
	}

	expectedCodes := []string{"system", "en", "tr", "de", "fr", "es", "pt", "ru", "ja", "zh", "ar", "fa"}
	for _, code := range expectedCodes {
		if !localeMap[code] {
			t.Errorf("expected locale %s to be present", code)
		}
	}
	if rtlCount != 2 {
		t.Errorf("expected 2 RTL languages (ar, fa), got %d", rtlCount)
	}

	// 2. ListThemes
	listResp, err := client.ListThemes(ctx)
	if err != nil {
		t.Fatalf("ListThemes error: %v", err)
	}
	if len(listResp.Themes) < 6 {
		t.Fatalf("expected at least 6 themes, got %d", len(listResp.Themes))
	}
	if listResp.ActiveThemeId != "vessel-dark" {
		t.Errorf("expected active theme vessel-dark, got %s", listResp.ActiveThemeId)
	}

	// Verify custom theme was discovered
	var hasCustom bool
	for _, th := range listResp.Themes {
		if th.Id == "synthwave-84" {
			hasCustom = true
			if th.IsBuiltin {
				t.Error("expected synthwave-84 to not be builtin")
			}
			break
		}
	}
	if !hasCustom {
		t.Error("expected custom theme synthwave-84 to be discovered")
	}

	// 3. GetActiveTheme
	activeResp, err := client.GetActiveTheme(ctx)
	if err != nil {
		t.Fatalf("GetActiveTheme error: %v", err)
	}
	if activeResp.Theme.Id != "vessel-dark" {
		t.Errorf("expected vessel-dark, got %s", activeResp.Theme.Id)
	}
	if !strings.Contains(activeResp.Css, "--v-bg-base: #0b0f17;") {
		t.Errorf("expected compiled CSS with --v-bg-base, got %s", activeResp.Css)
	}

	// 4. SetActiveTheme to Custom Theme
	setResp, err := client.SetActiveTheme(ctx, "synthwave-84", "rad")
	if err != nil {
		t.Fatalf("SetActiveTheme error: %v", err)
	}
	if !setResp.Success {
		t.Error("expected SetActiveTheme to succeed")
	}
	if setResp.ActiveTheme.Theme.Id != "synthwave-84" {
		t.Errorf("expected synthwave-84, got %s", setResp.ActiveTheme.Theme.Id)
	}
	if !strings.Contains(setResp.ActiveTheme.Css, "--v-bg-base: #262335;") {
		t.Errorf("expected custom token in CSS: %s", setResp.ActiveTheme.Css)
	}
	if !strings.Contains(setResp.ActiveTheme.Css, ".neon-glow") {
		t.Errorf("expected user.css content in compiled CSS: %s", setResp.ActiveTheme.Css)
	}
}
