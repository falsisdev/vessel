package plugin

import (
	"encoding/json"
	"fmt"
	"os"
	"path/filepath"
)

type PluginDescriptor struct {
	ID             string   `json:"id"`
	Name           string   `json:"name"`
	ExecutablePath string   `json:"executable_path"`
	Args           []string `json:"args,omitempty"`
	Env            []string `json:"env,omitempty"`
	IsBuiltin      bool     `json:"is_builtin"`
	Enabled        bool     `json:"enabled"`
}

func LoadDescriptor(path string) (*PluginDescriptor, error) {
	data, err := os.ReadFile(path)
	if err != nil {
		return nil, fmt.Errorf("failed to read plugin descriptor at %s: %w", path, err)
	}

	var desc PluginDescriptor
	if err := json.Unmarshal(data, &desc); err != nil {
		return nil, fmt.Errorf("failed to parse plugin descriptor at %s: %w", path, err)
	}

	if !filepath.IsAbs(desc.ExecutablePath) {
		dir := filepath.Dir(path)
		desc.ExecutablePath = filepath.Clean(filepath.Join(dir, desc.ExecutablePath))
	}

	return &desc, nil
}

func DiscoverPlugins(rootDir string) ([]*PluginDescriptor, error) {
	rootPluginJSON := filepath.Join(rootDir, "plugin.json")
	if _, err := os.Stat(rootPluginJSON); err == nil {
		desc, err := LoadDescriptor(rootPluginJSON)
		if err == nil {
			return []*PluginDescriptor{desc}, nil
		}
	}

	var descriptors []*PluginDescriptor

	entries, err := os.ReadDir(rootDir)
	if err != nil {
		if os.IsNotExist(err) {
			return nil, nil
		}
		return nil, fmt.Errorf("failed to read plugin root directory %s: %w", rootDir, err)
	}

	for _, entry := range entries {
		if !entry.IsDir() {
			continue
		}

		pluginJSON := filepath.Join(rootDir, entry.Name(), "plugin.json")
		if _, err := os.Stat(pluginJSON); err == nil {
			desc, err := LoadDescriptor(pluginJSON)
			if err != nil {
				continue
			}
			descriptors = append(descriptors, desc)
		}
	}

	return descriptors, nil
}
