package cli

import (
	"encoding/json"
	"errors"
	"fmt"
	"io"
	"os"
	"path/filepath"
	"text/tabwriter"

	"github.com/falsisdev/vessel/core/internal/plugin"
)

var builtinPlugins = []struct {
	ID   string
	Name string
}{
	{ID: "com.vessel.cinema.cinemasis", Name: "Cinemasis (TMDB Cinema)"},
	{ID: "com.vessel.reading.mangile", Name: "Mangile (Sanity Manga/Novel)"},
}

// PluginList lists all installed and built-in plugins.
func PluginList(pluginsDir string, out io.Writer) error {
	cfg, err := LoadConfig()
	if err != nil {
		return err
	}

	if pluginsDir == "" {
		pluginsDir = cfg.PluginsDir
	}

	w := tabwriter.NewWriter(out, 0, 0, 3, ' ', 0)
	fmt.Fprintln(w, "ID\tNAME\tTYPE\tSTATUS\tPATH")
	fmt.Fprintln(w, "--\t----\t----\t------\t----")

	// Builtin plugins
	for _, b := range builtinPlugins {
		status := "ENABLED"
		if cfg.DisabledPlugins[b.ID] {
			status = "DISABLED"
		}
		fmt.Fprintf(w, "%s\t%s\tBUILTIN\t%s\t(embedded runtime)\n", b.ID, b.Name, status)
	}

	// External plugins from pluginsDir
	if pluginsDir != "" {
		descriptors, _ := plugin.DiscoverPlugins(pluginsDir)
		for _, d := range descriptors {
			status := "ENABLED"
			if !d.Enabled || cfg.DisabledPlugins[d.ID] {
				status = "DISABLED"
			}
			name := d.Name
			if name == "" {
				name = d.ID
			}
			fmt.Fprintf(w, "%s\t%s\tEXTERNAL\t%s\t%s\n", d.ID, name, status, d.ExecutablePath)
		}
	}

	return w.Flush()
}

// PluginInstall installs a plugin package from a directory, archive, or URL.
func PluginInstall(source, pluginsDir string, out io.Writer) error {
	if source == "" {
		return errors.New("source path or URL is required")
	}

	cfg, err := LoadConfig()
	if err != nil {
		return err
	}

	if pluginsDir == "" {
		pluginsDir = cfg.PluginsDir
	}

	if err := os.MkdirAll(pluginsDir, 0755); err != nil {
		return fmt.Errorf("failed to create plugins directory %s: %w", pluginsDir, err)
	}

	// Extract to temporary directory for inspection
	stagingDir, err := os.MkdirTemp("", "vessel-plugin-install-*")
	if err != nil {
		return fmt.Errorf("failed to create temp staging directory: %w", err)
	}
	defer os.RemoveAll(stagingDir)

	if err := ExtractSource(source, stagingDir); err != nil {
		return fmt.Errorf("failed to extract plugin source: %w", err)
	}

	// Locate plugin.json
	manifestPath := filepath.Join(stagingDir, "plugin.json")
	if _, err := os.Stat(manifestPath); err != nil {
		// Check subdirectories if archive was wrapped in an outer folder
		entries, _ := os.ReadDir(stagingDir)
		found := false
		for _, e := range entries {
			if e.IsDir() {
				sub := filepath.Join(stagingDir, e.Name(), "plugin.json")
				if _, err := os.Stat(sub); err == nil {
					manifestPath = sub
					stagingDir = filepath.Join(stagingDir, e.Name())
					found = true
					break
				}
			}
		}
		if !found {
			return errors.New("invalid plugin package: plugin.json manifest not found")
		}
	}

	desc, err := plugin.LoadDescriptor(manifestPath)
	if err != nil {
		return fmt.Errorf("failed to parse plugin.json: %w", err)
	}

	if desc.ID == "" {
		return errors.New("invalid plugin: id is required in plugin.json")
	}

	// Verify executable exists
	binPath := desc.ExecutablePath
	if !filepath.IsAbs(binPath) {
		binPath = filepath.Join(stagingDir, binPath)
	}
	binInfo, err := os.Stat(binPath)
	if err != nil {
		return fmt.Errorf("plugin executable not found at %s: %w", desc.ExecutablePath, err)
	}

	// Make binary executable
	_ = os.Chmod(binPath, binInfo.Mode()|0111)

	// Destination target directory: pluginsDir/<id>
	targetDir := filepath.Join(pluginsDir, desc.ID)
	_ = os.RemoveAll(targetDir)

	if err := copyDir(stagingDir, targetDir); err != nil {
		return fmt.Errorf("failed to copy plugin to %s: %w", targetDir, err)
	}

	// Re-verify target binary permissions
	targetBin := filepath.Join(targetDir, filepath.Base(desc.ExecutablePath))
	if _, err := os.Stat(targetBin); err == nil {
		_ = os.Chmod(targetBin, 0755)
	}

	// Enable plugin in config
	delete(cfg.DisabledPlugins, desc.ID)
	_ = SaveConfig(cfg)

	fmt.Fprintf(out, "✓ Successfully installed plugin '%s' (%s) into %s\n", desc.Name, desc.ID, targetDir)
	return nil
}

// PluginRemove uninstalls a non-builtin plugin.
func PluginRemove(id, pluginsDir string, out io.Writer) error {
	if id == "" {
		return errors.New("plugin ID is required")
	}

	for _, b := range builtinPlugins {
		if b.ID == id {
			return fmt.Errorf("cannot remove built-in plugin '%s'", id)
		}
	}

	cfg, err := LoadConfig()
	if err != nil {
		return err
	}
	if pluginsDir == "" {
		pluginsDir = cfg.PluginsDir
	}

	targetDir := filepath.Join(pluginsDir, id)
	if _, err := os.Stat(targetDir); err != nil {
		return fmt.Errorf("plugin '%s' not found in %s", id, pluginsDir)
	}

	if err := os.RemoveAll(targetDir); err != nil {
		return fmt.Errorf("failed to delete plugin directory %s: %w", targetDir, err)
	}

	delete(cfg.DisabledPlugins, id)
	_ = SaveConfig(cfg)

	fmt.Fprintf(out, "✓ Successfully removed plugin '%s'\n", id)
	return nil
}

// PluginEnable enables a plugin.
func PluginEnable(id, pluginsDir string, out io.Writer) error {
	if id == "" {
		return errors.New("plugin ID is required")
	}

	cfg, err := LoadConfig()
	if err != nil {
		return err
	}

	delete(cfg.DisabledPlugins, id)
	if err := SaveConfig(cfg); err != nil {
		return err
	}

	// Also update plugin.json if it exists in pluginsDir
	if pluginsDir == "" {
		pluginsDir = cfg.PluginsDir
	}
	jsonPath := filepath.Join(pluginsDir, id, "plugin.json")
	if data, err := os.ReadFile(jsonPath); err == nil {
		var desc plugin.PluginDescriptor
		if json.Unmarshal(data, &desc) == nil {
			desc.Enabled = true
			if updated, err := json.MarshalIndent(desc, "", "  "); err == nil {
				_ = os.WriteFile(jsonPath, updated, 0644)
			}
		}
	}

	fmt.Fprintf(out, "✓ Plugin '%s' enabled\n", id)
	return nil
}

// PluginDisable disables a plugin.
func PluginDisable(id, pluginsDir string, out io.Writer) error {
	if id == "" {
		return errors.New("plugin ID is required")
	}

	cfg, err := LoadConfig()
	if err != nil {
		return err
	}

	cfg.DisabledPlugins[id] = true
	if err := SaveConfig(cfg); err != nil {
		return err
	}

	// Also update plugin.json if it exists in pluginsDir
	if pluginsDir == "" {
		pluginsDir = cfg.PluginsDir
	}
	jsonPath := filepath.Join(pluginsDir, id, "plugin.json")
	if data, err := os.ReadFile(jsonPath); err == nil {
		var desc plugin.PluginDescriptor
		if json.Unmarshal(data, &desc) == nil {
			desc.Enabled = false
			if updated, err := json.MarshalIndent(desc, "", "  "); err == nil {
				_ = os.WriteFile(jsonPath, updated, 0644)
			}
		}
	}

	fmt.Fprintf(out, "✓ Plugin '%s' disabled\n", id)
	return nil
}
