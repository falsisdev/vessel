package config_test

import (
	"os"
	"path/filepath"
	"testing"

	"github.com/falsisdev/vessel/pkg/config"
)

func TestLoadEnv(t *testing.T) {
	tempDir := t.TempDir()
	envPath := filepath.Join(tempDir, ".env")

	content := `
# Comment line
TEST_KEY_1=value1
TEST_KEY_2="quoted value with space"
TEST_KEY_3='single quoted'
`
	if err := os.WriteFile(envPath, []byte(content), 0644); err != nil {
		t.Fatalf("failed to write .env: %v", err)
	}

	_ = os.Setenv("TEST_KEY_1", "pre-existing")

	if err := config.LoadEnv(envPath); err != nil {
		t.Fatalf("LoadEnv failed: %v", err)
	}

	// Pre-existing should not be overwritten
	if val := os.Getenv("TEST_KEY_1"); val != "pre-existing" {
		t.Errorf("expected pre-existing, got %s", val)
	}

	if val := os.Getenv("TEST_KEY_2"); val != "quoted value with space" {
		t.Errorf("expected 'quoted value with space', got %s", val)
	}

	if val := os.Getenv("TEST_KEY_3"); val != "single quoted" {
		t.Errorf("expected 'single quoted', got %s", val)
	}
}
