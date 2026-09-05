package cli

import (
	"encoding/json"
	"errors"
	"fmt"
	"io"
	"os"
	"path/filepath"
	"strings"
	"text/tabwriter"

	"github.com/falsisdev/vessel/core/internal/theme"
)

// ThemeList lists all installed and built-in themes.
func ThemeList(themesDir string, out io.Writer) error {
	cfg, err := LoadConfig()
	if err != nil {
		return err
	}

	if themesDir == "" {
		themesDir = cfg.ThemesDir
	}

	mgr := theme.NewManager(themesDir)
	themes := mgr.List()

	activeThemeID := cfg.ActiveThemeID
	if activeThemeID == "" {
		activeThemeID = "catppuccin"
	}
	activeVariantID := cfg.ActiveVariantID
	if activeVariantID == "" {
		activeVariantID = "mocha"
	}

	w := tabwriter.NewWriter(out, 0, 0, 3, ' ', 0)
	fmt.Fprintln(w, "ID\tNAME\tAUTHOR\tTYPE\tVARIANTS\tSTATUS")
	fmt.Fprintln(w, "--\t----\t------\t----\t--------\t------")

	for _, th := range themes {
		m := th.Manifest
		themeType := "COMMUNITY"
		if th.IsBuiltin {
			themeType = "BUILTIN"
		}

		var variantNames []string
		for _, v := range m.Variants {
			name := v.ID
			if th.Manifest.ID == activeThemeID && v.ID == activeVariantID {
				name += "*"
			}
			variantNames = append(variantNames, name)
		}
		variantStr := strings.Join(variantNames, ", ")

		status := ""
		if th.Manifest.ID == activeThemeID {
			status = fmt.Sprintf("[ACTIVE: %s]", activeVariantID)
		}

		fmt.Fprintf(w, "%s\t%s\t%s\t%s\t%s\t%s\n",
			m.ID, m.Name, m.Author, themeType, variantStr, status)
	}

	return w.Flush()
}

// ThemeInstall installs a community theme package from a directory, archive, or URL.
func ThemeInstall(source, themesDir string, out io.Writer) error {
	if source == "" {
		return errors.New("source path or URL is required")
	}

	cfg, err := LoadConfig()
	if err != nil {
		return err
	}

	if themesDir == "" {
		themesDir = cfg.ThemesDir
	}

	if err := os.MkdirAll(themesDir, 0755); err != nil {
		return fmt.Errorf("failed to create themes directory %s: %w", themesDir, err)
	}

	stagingDir, err := os.MkdirTemp("", "vessel-theme-install-*")
	if err != nil {
		return fmt.Errorf("failed to create temp staging directory: %w", err)
	}
	defer os.RemoveAll(stagingDir)

	if err := ExtractSource(source, stagingDir); err != nil {
		return fmt.Errorf("failed to extract theme source: %w", err)
	}

	manifestPath := filepath.Join(stagingDir, "theme.json")
	if _, err := os.Stat(manifestPath); err != nil {
		// Check subdirectories
		entries, _ := os.ReadDir(stagingDir)
		found := false
		for _, e := range entries {
			if e.IsDir() {
				sub := filepath.Join(stagingDir, e.Name(), "theme.json")
				if _, err := os.Stat(sub); err == nil {
					manifestPath = sub
					stagingDir = filepath.Join(stagingDir, e.Name())
					found = true
					break
				}
			}
		}
		if !found {
			return errors.New("invalid theme package: theme.json manifest not found")
		}
	}

	data, err := os.ReadFile(manifestPath)
	if err != nil {
		return fmt.Errorf("failed to read theme manifest: %w", err)
	}

	var manifest theme.ThemeManifest
	if err := json.Unmarshal(data, &manifest); err != nil {
		return fmt.Errorf("failed to parse theme.json: %w", err)
	}

	if manifest.ID == "" {
		return errors.New("theme manifest missing 'id'")
	}
	if len(manifest.Variants) == 0 {
		return errors.New("theme must define at least one variant")
	}

	targetDir := filepath.Join(themesDir, manifest.ID)
	_ = os.RemoveAll(targetDir)

	if err := copyDir(stagingDir, targetDir); err != nil {
		return fmt.Errorf("failed to copy theme to %s: %w", targetDir, err)
	}

	fmt.Fprintf(out, "✓ Successfully installed theme '%s' (%s) into %s\n", manifest.Name, manifest.ID, targetDir)
	return nil
}

// ThemeApply sets the active theme and variant.
func ThemeApply(themeID, variantID, themesDir string, out io.Writer) error {
	if themeID == "" {
		return errors.New("theme ID is required")
	}

	cfg, err := LoadConfig()
	if err != nil {
		return err
	}

	if themesDir == "" {
		themesDir = cfg.ThemesDir
	}

	mgr := theme.NewManager(themesDir)
	th, err := mgr.Get(themeID)
	if err != nil {
		return fmt.Errorf("theme '%s' not found: %w", themeID, err)
	}

	if variantID == "" {
		variantID = th.Manifest.DefaultVariant
		if variantID == "" && len(th.Manifest.Variants) > 0 {
			variantID = th.Manifest.Variants[0].ID
		}
	}

	variant, err := th.GetVariant(variantID)
	if err != nil {
		return fmt.Errorf("variant '%s' not found in theme '%s': %w", variantID, themeID, err)
	}

	// Verify CSS compilation succeeds
	if _, err := mgr.CompileCSS(themeID, variant.ID); err != nil {
		return fmt.Errorf("failed to compile theme CSS: %w", err)
	}

	cfg.ActiveThemeID = themeID
	cfg.ActiveVariantID = variant.ID
	if err := SaveConfig(cfg); err != nil {
		return fmt.Errorf("failed to persist active theme configuration: %w", err)
	}

	fmt.Fprintf(out, "✓ Applied theme '%s' (variant: %s)\n", th.Manifest.Name, variant.Name)
	return nil
}

// ThemeRemove uninstalls a community theme.
func ThemeRemove(themeID, themesDir string, out io.Writer) error {
	if themeID == "" {
		return errors.New("theme ID is required")
	}

	cfg, err := LoadConfig()
	if err != nil {
		return err
	}

	if themesDir == "" {
		themesDir = cfg.ThemesDir
	}

	mgr := theme.NewManager(themesDir)
	th, err := mgr.Get(themeID)
	if err != nil {
		return fmt.Errorf("theme '%s' not found", themeID)
	}

	if th.IsBuiltin {
		return fmt.Errorf("cannot remove built-in theme '%s'", themeID)
	}

	targetDir := filepath.Join(themesDir, themeID)
	if _, err := os.Stat(targetDir); err != nil {
		return fmt.Errorf("theme directory not found: %s", targetDir)
	}

	if err := os.RemoveAll(targetDir); err != nil {
		return fmt.Errorf("failed to delete theme directory %s: %w", targetDir, err)
	}

	if cfg.ActiveThemeID == themeID {
		cfg.ActiveThemeID = "catppuccin"
		cfg.ActiveVariantID = "mocha"
		_ = SaveConfig(cfg)
		fmt.Fprintf(out, "Active theme reset to default (catppuccin / mocha)\n")
	}

	fmt.Fprintf(out, "✓ Successfully removed theme '%s'\n", themeID)
	return nil
}
