package cli_test

import (
	"archive/zip"
	"bytes"
	"encoding/json"
	"os"
	"path/filepath"
	"strings"
	"testing"

	"github.com/falsisdev/vessel/core/internal/cli"
	"github.com/falsisdev/vessel/core/internal/plugin"
	"github.com/falsisdev/vessel/core/internal/theme"
)

func TestPluginList(t *testing.T) {
	tempPluginsDir := t.TempDir()
	var out bytes.Buffer

	err := cli.PluginList(tempPluginsDir, &out)
	if err != nil {
		t.Fatalf("PluginList failed: %v", err)
	}

	output := out.String()
	if !strings.Contains(output, "com.vessel.cinema.cinemasis") {
		t.Errorf("expected builtin cinemasis plugin in list: %s", output)
	}
	if !strings.Contains(output, "com.vessel.reading.mangile") {
		t.Errorf("expected builtin mangile plugin in list: %s", output)
	}
}

func TestPluginInstallRemoveAndToggle(t *testing.T) {
	tempPluginsDir := t.TempDir()
	sourceDir := t.TempDir()

	// Create dummy plugin source
	dummyBin := filepath.Join(sourceDir, "runner.sh")
	if err := os.WriteFile(dummyBin, []byte("#!/bin/sh\necho test\n"), 0755); err != nil {
		t.Fatalf("failed to write dummy bin: %v", err)
	}

	desc := plugin.PluginDescriptor{
		ID:             "com.test.anime",
		Name:           "Test Anime Provider",
		ExecutablePath: "runner.sh",
		Enabled:        true,
	}
	data, _ := json.Marshal(desc)
	if err := os.WriteFile(filepath.Join(sourceDir, "plugin.json"), data, 0644); err != nil {
		t.Fatalf("failed to write plugin.json: %v", err)
	}

	var installOut bytes.Buffer
	if err := cli.PluginInstall(sourceDir, tempPluginsDir, &installOut); err != nil {
		t.Fatalf("PluginInstall failed: %v", err)
	}
	if !strings.Contains(installOut.String(), "Successfully installed plugin") {
		t.Errorf("unexpected install output: %s", installOut.String())
	}

	// Verify plugin list contains newly installed plugin
	var listOut bytes.Buffer
	if err := cli.PluginList(tempPluginsDir, &listOut); err != nil {
		t.Fatalf("PluginList failed: %v", err)
	}
	if !strings.Contains(listOut.String(), "com.test.anime") {
		t.Fatalf("expected com.test.anime in list: %s", listOut.String())
	}

	// Test Disable
	var disableOut bytes.Buffer
	if err := cli.PluginDisable("com.test.anime", tempPluginsDir, &disableOut); err != nil {
		t.Fatalf("PluginDisable failed: %v", err)
	}
	listOut.Reset()
	_ = cli.PluginList(tempPluginsDir, &listOut)
	if !strings.Contains(listOut.String(), "DISABLED") {
		t.Errorf("expected DISABLED status after disabling: %s", listOut.String())
	}

	// Test Enable
	var enableOut bytes.Buffer
	if err := cli.PluginEnable("com.test.anime", tempPluginsDir, &enableOut); err != nil {
		t.Fatalf("PluginEnable failed: %v", err)
	}

	// Test Remove
	var removeOut bytes.Buffer
	if err := cli.PluginRemove("com.test.anime", tempPluginsDir, &removeOut); err != nil {
		t.Fatalf("PluginRemove failed: %v", err)
	}
	if !strings.Contains(removeOut.String(), "Successfully removed plugin") {
		t.Errorf("unexpected remove output: %s", removeOut.String())
	}

	// Cannot remove builtin
	if err := cli.PluginRemove("com.vessel.cinema.cinemasis", tempPluginsDir, &removeOut); err == nil {
		t.Fatal("expected error when removing builtin plugin, got nil")
	}
}

func TestPluginZipInstall(t *testing.T) {
	tempPluginsDir := t.TempDir()
	zipFile := filepath.Join(t.TempDir(), "plugin.zip")

	// Create a zip archive
	buf := new(bytes.Buffer)
	zw := zip.NewWriter(buf)

	binWriter, _ := zw.Create("script.sh")
	_, _ = binWriter.Write([]byte("#!/bin/sh\necho hi\n"))

	jsonWriter, _ := zw.Create("plugin.json")
	desc := plugin.PluginDescriptor{
		ID:             "com.test.zip-plugin",
		Name:           "Zip Plugin",
		ExecutablePath: "script.sh",
		Enabled:        true,
	}
	descData, _ := json.Marshal(desc)
	_, _ = jsonWriter.Write(descData)
	_ = zw.Close()

	if err := os.WriteFile(zipFile, buf.Bytes(), 0644); err != nil {
		t.Fatalf("failed to write zip file: %v", err)
	}

	var installOut bytes.Buffer
	if err := cli.PluginInstall(zipFile, tempPluginsDir, &installOut); err != nil {
		t.Fatalf("PluginInstall from zip failed: %v", err)
	}

	var listOut bytes.Buffer
	_ = cli.PluginList(tempPluginsDir, &listOut)
	if !strings.Contains(listOut.String(), "com.test.zip-plugin") {
		t.Errorf("expected zip plugin in list: %s", listOut.String())
	}
}

func TestThemeList(t *testing.T) {
	tempThemesDir := t.TempDir()
	var out bytes.Buffer

	err := cli.ThemeList(tempThemesDir, &out)
	if err != nil {
		t.Fatalf("ThemeList failed: %v", err)
	}

	output := out.String()
	if !strings.Contains(output, "mangile") {
		t.Errorf("expected builtin mangile theme in list: %s", output)
	}
	if !strings.Contains(output, "Mangile Duman") {
		t.Errorf("expected Mangile Duman in list: %s", output)
	}
	if !strings.Contains(output, "Mangile Leylak") {
		t.Errorf("expected Mangile Leylak in list: %s", output)
	}
	if !strings.Contains(output, "vessel-dark") {
		t.Errorf("expected vessel-dark in list: %s", output)
	}
}

func TestThemeInstallApplyAndRemove(t *testing.T) {
	tempThemesDir := t.TempDir()
	sourceDir := t.TempDir()

	themeManifest := theme.ThemeManifest{
		ID:             "synth-neon",
		Name:           "Synth Neon",
		Version:        "1.0.0",
		Author:         "RetroDev",
		DefaultVariant: "glow",
		Variants: []theme.Variant{
			{
				ID:     "glow",
				Name:   "Glow Variant",
				IsDark: true,
				Tokens: map[string]string{
					"bg-base":        "#10002b",
					"accent-primary": "#e0aaff",
				},
			},
		},
	}
	data, _ := json.Marshal(themeManifest)
	if err := os.WriteFile(filepath.Join(sourceDir, "theme.json"), data, 0644); err != nil {
		t.Fatalf("failed to write theme.json: %v", err)
	}
	if err := os.WriteFile(filepath.Join(sourceDir, "user.css"), []byte("body { filter: blur(0); }"), 0644); err != nil {
		t.Fatalf("failed to write user.css: %v", err)
	}

	var installOut bytes.Buffer
	if err := cli.ThemeInstall(sourceDir, tempThemesDir, &installOut); err != nil {
		t.Fatalf("ThemeInstall failed: %v", err)
	}
	if !strings.Contains(installOut.String(), "Successfully installed theme") {
		t.Errorf("unexpected theme install output: %s", installOut.String())
	}

	// Test Apply
	var applyOut bytes.Buffer
	if err := cli.ThemeApply("synth-neon", "glow", tempThemesDir, &applyOut); err != nil {
		t.Fatalf("ThemeApply failed: %v", err)
	}
	if !strings.Contains(applyOut.String(), "Applied theme 'Synth Neon'") {
		t.Errorf("unexpected apply output: %s", applyOut.String())
	}

	// Apply Mangile Duman theme
	var applyMangile bytes.Buffer
	if err := cli.ThemeApply("mangile", "dark", tempThemesDir, &applyMangile); err != nil {
		t.Fatalf("ThemeApply mangile failed: %v", err)
	}

	// Test Remove
	var removeOut bytes.Buffer
	if err := cli.ThemeRemove("synth-neon", tempThemesDir, &removeOut); err != nil {
		t.Fatalf("ThemeRemove failed: %v", err)
	}
	if !strings.Contains(removeOut.String(), "Successfully removed theme") {
		t.Errorf("unexpected theme remove output: %s", removeOut.String())
	}

	// Cannot remove builtin
	if err := cli.ThemeRemove("vessel-dark", tempThemesDir, &removeOut); err == nil {
		t.Fatal("expected error when removing builtin theme, got nil")
	}
}

func TestRootExecute(t *testing.T) {
	var stdout, stderr bytes.Buffer

	// Help command
	err := cli.ExecuteWithOutput([]string{"help"}, &stdout, &stderr)
	if err != nil {
		t.Fatalf("Execute help failed: %v", err)
	}
	if !strings.Contains(stdout.String(), "USAGE:") {
		t.Errorf("expected USAGE in help output: %s", stdout.String())
	}

	// Version command
	stdout.Reset()
	err = cli.ExecuteWithOutput([]string{"version"}, &stdout, &stderr)
	if err != nil {
		t.Fatalf("Execute version failed: %v", err)
	}
	if !strings.Contains(stdout.String(), "Vessel Core v1.0.0") {
		t.Errorf("expected version output: %s", stdout.String())
	}

	// Plugin list via root
	stdout.Reset()
	err = cli.ExecuteWithOutput([]string{"plugin", "list"}, &stdout, &stderr)
	if err != nil {
		t.Fatalf("Execute plugin list failed: %v", err)
	}
	if !strings.Contains(stdout.String(), "Cinemasis") {
		t.Errorf("expected Cinemasis in plugin list: %s", stdout.String())
	}

	// Theme list via root
	stdout.Reset()
	err = cli.ExecuteWithOutput([]string{"theme", "list"}, &stdout, &stderr)
	if err != nil {
		t.Fatalf("Execute theme list failed: %v", err)
	}
	if !strings.Contains(stdout.String(), "Mangile Duman") {
		t.Errorf("expected Mangile Duman in theme list: %s", stdout.String())
	}
}
