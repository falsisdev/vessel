package cli

import (
	"encoding/json"
	"fmt"
	"os"
	"path/filepath"
	"sync"
)

// Config represents persistent Vessel platform preferences.
type Config struct {
	ActiveThemeID   string          `json:"active_theme_id"`
	ActiveVariantID string          `json:"active_variant_id"`
	PluginsDir      string          `json:"plugins_dir"`
	ThemesDir       string          `json:"themes_dir"`
	DisabledPlugins map[string]bool `json:"disabled_plugins"`
}

var (
	configMu sync.RWMutex
)

// DefaultConfigDir returns the base configuration directory for Vessel.
func DefaultConfigDir() string {
	if xdg := os.Getenv("XDG_CONFIG_HOME"); xdg != "" {
		return filepath.Join(xdg, "vessel")
	}
	home, err := os.UserHomeDir()
	if err != nil {
		home = "."
	}
	return filepath.Join(home, ".config", "vessel")
}

// DefaultPluginsDir returns the standard directory for user-installed plugins.
func DefaultPluginsDir() string {
	return filepath.Join(DefaultConfigDir(), "plugins")
}

// DefaultThemesDir returns the standard directory for user-installed themes.
func DefaultThemesDir() string {
	return filepath.Join(DefaultConfigDir(), "themes")
}

// ConfigFilePath returns the path to the config.json file.
func ConfigFilePath() string {
	return filepath.Join(DefaultConfigDir(), "config.json")
}

// LoadConfig loads the persistent configuration from disk or initializes defaults.
func LoadConfig() (*Config, error) {
	configMu.RLock()
	defer configMu.RUnlock()

	cfg := &Config{
		ActiveThemeID:   "vessel-dark",
		ActiveVariantID: "slate-indigo",
		PluginsDir:      DefaultPluginsDir(),
		ThemesDir:       DefaultThemesDir(),
		DisabledPlugins: make(map[string]bool),
	}

	path := ConfigFilePath()
	data, err := os.ReadFile(path)
	if err != nil {
		if os.IsNotExist(err) {
			return cfg, nil
		}
		return nil, fmt.Errorf("failed to read config file at %s: %w", path, err)
	}

	if err := json.Unmarshal(data, cfg); err != nil {
		return nil, fmt.Errorf("failed to parse config file: %w", err)
	}

	if cfg.PluginsDir == "" {
		cfg.PluginsDir = DefaultPluginsDir()
	}
	if cfg.ThemesDir == "" {
		cfg.ThemesDir = DefaultThemesDir()
	}
	if cfg.DisabledPlugins == nil {
		cfg.DisabledPlugins = make(map[string]bool)
	}

	return cfg, nil
}

// SaveConfig persists the given configuration to disk.
func SaveConfig(cfg *Config) error {
	configMu.Lock()
	defer configMu.Unlock()

	dir := DefaultConfigDir()
	if err := os.MkdirAll(dir, 0755); err != nil {
		return fmt.Errorf("failed to create config directory %s: %w", dir, err)
	}

	data, err := json.MarshalIndent(cfg, "", "  ")
	if err != nil {
		return fmt.Errorf("failed to marshal config: %w", err)
	}

	path := ConfigFilePath()
	if err := os.WriteFile(path, data, 0644); err != nil {
		return fmt.Errorf("failed to write config to %s: %w", path, err)
	}

	return nil
}
