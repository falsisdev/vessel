package theme_test

import (
	"os"
	"path/filepath"
	"strings"
	"testing"

	"github.com/falsisdev/vessel/core/internal/theme"
)

func TestBuiltinThemes(t *testing.T) {
	mgr := theme.NewManager()
	themes := mgr.List()

	if len(themes) < 5 {
		t.Fatalf("expected at least 5 builtin themes, got %d", len(themes))
	}

	active, err := mgr.GetActive()
	if err != nil {
		t.Fatalf("GetActive error: %v", err)
	}

	if active.Theme.Manifest.ID != "catppuccin" {
		t.Errorf("expected default theme catppuccin, got %s", active.Theme.Manifest.ID)
	}
	if active.Variant.ID != "mocha" {
		t.Errorf("expected default variant mocha, got %s", active.Variant.ID)
	}
	if !strings.Contains(active.CompiledCSS, "--v-bg-base: #1e1e2e;") {
		t.Errorf("compiled CSS missing --v-bg-base variable: %s", active.CompiledCSS)
	}
}

func TestThemeSwitching(t *testing.T) {
	mgr := theme.NewManager()

	// Switch to Catppuccin Latte (Light variant)
	active, err := mgr.SetActive("catppuccin", "latte")
	if err != nil {
		t.Fatalf("SetActive error: %v", err)
	}

	if active.Theme.Manifest.ID != "catppuccin" {
		t.Errorf("expected catppuccin, got %s", active.Theme.Manifest.ID)
	}
	if active.Variant.ID != "latte" {
		t.Errorf("expected variant latte, got %s", active.Variant.ID)
	}
	if active.Variant.IsDark {
		t.Error("expected Latte variant to be light (is_dark=false)")
	}
	if !strings.Contains(active.CompiledCSS, "--v-bg-base: #eff1f5;") {
		t.Errorf("compiled CSS missing Latte bg-base: %s", active.CompiledCSS)
	}
}

func TestCustomThemeDiscovery(t *testing.T) {
	tempDir := t.TempDir()
	customThemeDir := filepath.Join(tempDir, "cyberpunk")
	if err := os.MkdirAll(customThemeDir, 0755); err != nil {
		t.Fatalf("failed to create dir: %v", err)
	}

	themeJSON := `{
		"id": "cyberpunk-2077",
		"name": "Cyberpunk 2077",
		"version": "1.0.0",
		"author": "NightCity",
		"default_variant": "neon",
		"variants": [
			{
				"id": "neon",
				"name": "Neon Yellow",
				"is_dark": true,
				"tokens": {
					"bg_base": "#fee801",
					"accent_primary": "#00f0ff"
				}
			}
		]
	}`
	if err := os.WriteFile(filepath.Join(customThemeDir, "theme.json"), []byte(themeJSON), 0644); err != nil {
		t.Fatalf("failed to write theme.json: %v", err)
	}

	userCSS := `
.cyber-glow {
	box-shadow: 0 0 10px #00f0ff;
}
`
	if err := os.WriteFile(filepath.Join(customThemeDir, "user.css"), []byte(userCSS), 0644); err != nil {
		t.Fatalf("failed to write user.css: %v", err)
	}

	mgr := theme.NewManager(tempDir)
	themes := mgr.List()

	var found *theme.Theme
	for _, th := range themes {
		if th.Manifest.ID == "cyberpunk-2077" {
			found = th
			break
		}
	}

	if found == nil {
		t.Fatal("cyberpunk-2077 custom theme was not discovered")
	}
	if found.IsBuiltin {
		t.Error("custom theme should have IsBuiltin = false")
	}
	if !strings.Contains(found.CustomCSS, ".cyber-glow") {
		t.Errorf("custom CSS not loaded properly: %s", found.CustomCSS)
	}

	// Switch to cyberpunk
	active, err := mgr.SetActive("cyberpunk-2077", "neon")
	if err != nil {
		t.Fatalf("failed to activate custom theme: %v", err)
	}
	if !strings.Contains(active.CompiledCSS, "--v-bg-base: #fee801;") {
		t.Errorf("compiled CSS missing custom token: %s", active.CompiledCSS)
	}
	if !strings.Contains(active.CompiledCSS, ".cyber-glow") {
		t.Errorf("compiled CSS missing user.css rules: %s", active.CompiledCSS)
	}
}
