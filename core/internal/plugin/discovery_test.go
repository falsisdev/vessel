package plugin_test

import (
	"os"
	"path/filepath"
	"testing"

	"github.com/falsisdev/vessel/core/internal/plugin"
)

func TestLoadDescriptorAndDiscover(t *testing.T) {
	tempDir := t.TempDir()

	pluginDir := filepath.Join(tempDir, "mock-plugin")
	if err := os.MkdirAll(pluginDir, 0755); err != nil {
		t.Fatalf("failed to create temp plugin dir: %v", err)
	}

	manifestPath := filepath.Join(pluginDir, "plugin.json")
	jsonContent := `{
		"id": "com.vessel.test",
		"name": "Test Plugin",
		"executable_path": "./bin/test-exec",
		"args": ["-foo", "bar"],
		"is_builtin": false,
		"enabled": true
	}`

	if err := os.WriteFile(manifestPath, []byte(jsonContent), 0644); err != nil {
		t.Fatalf("failed to write plugin.json: %v", err)
	}

	desc, err := plugin.LoadDescriptor(manifestPath)
	if err != nil {
		t.Fatalf("failed to load descriptor: %v", err)
	}

	if desc.ID != "com.vessel.test" || desc.Name != "Test Plugin" {
		t.Fatalf("unexpected descriptor fields: %+v", desc)
	}

	expectedExecPath := filepath.Join(pluginDir, "bin/test-exec")
	if desc.ExecutablePath != expectedExecPath {
		t.Fatalf("expected resolved path %s, got %s", expectedExecPath, desc.ExecutablePath)
	}

	// Test Discovery
	discovered, err := plugin.DiscoverPlugins(tempDir)
	if err != nil {
		t.Fatalf("discovery error: %v", err)
	}

	if len(discovered) != 1 {
		t.Fatalf("expected 1 discovered plugin, got %d", len(discovered))
	}

	if discovered[0].ID != "com.vessel.test" {
		t.Fatalf("expected ID com.vessel.test, got %s", discovered[0].ID)
	}

	// Test direct plugin directory discovery
	directDiscovered, err := plugin.DiscoverPlugins(pluginDir)
	if err != nil {
		t.Fatalf("direct discovery error: %v", err)
	}
	if len(directDiscovered) != 1 || directDiscovered[0].ID != "com.vessel.test" {
		t.Fatalf("expected 1 directly discovered plugin, got %+v", directDiscovered)
	}
}
