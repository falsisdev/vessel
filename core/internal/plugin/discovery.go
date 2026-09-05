package plugin

import (
	"encoding/json"
	"fmt"
	"os"
	"os/exec"
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
		candidate := filepath.Clean(filepath.Join(dir, desc.ExecutablePath))
		if fi, err := os.Stat(candidate); err == nil && !fi.IsDir() {
			desc.ExecutablePath = candidate
		} else {
			baseName := filepath.Base(desc.ExecutablePath)
			execPath, _ := os.Executable()
			execDir := filepath.Dir(execPath)

			altCandidates := []string{
				filepath.Join("bin", baseName),
				filepath.Join("..", "bin", baseName),
				filepath.Join(dir, "..", "..", "bin", baseName),
				filepath.Join(execDir, baseName),
				filepath.Join(execDir, "..", "bin", baseName),
			}

			found := false
			for _, alt := range altCandidates {
				if fi, err := os.Stat(alt); err == nil && !fi.IsDir() {
					desc.ExecutablePath, _ = filepath.Abs(alt)
					found = true
					break
				}
			}
			if !found {
				if _, err := os.Stat(filepath.Join(dir, "main.go")); err == nil {
					targetBin := filepath.Join(dir, baseName)
					cmd := exec.Command("go", "build", "-o", targetBin, dir)
					if err := cmd.Run(); err == nil {
						desc.ExecutablePath, _ = filepath.Abs(targetBin)
						found = true
					}
				}
			}
			if !found {
				if look, err := exec.LookPath(baseName); err == nil {
					desc.ExecutablePath = look
				} else {
					desc.ExecutablePath = candidate
				}
			}
		}
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
