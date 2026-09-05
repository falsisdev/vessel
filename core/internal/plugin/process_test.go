package plugin_test

import (
	"context"
	"os/exec"
	"path/filepath"
	"sync"
	"testing"
	"time"

	"github.com/falsisdev/vessel/core/internal/plugin"
)

func buildMockBinary(t *testing.T) string {
	t.Helper()
	binPath := filepath.Join(t.TempDir(), "cinema-mock")
	cmd := exec.Command("go", "build", "-o", binPath, "../../../plugins/examples/cinema-mock")
	out, err := cmd.CombinedOutput()
	if err != nil {
		t.Fatalf("failed to build mock cinema plugin: %v, output: %s", err, string(out))
	}
	return binPath
}

func TestProcessLifecycle(t *testing.T) {
	binPath := buildMockBinary(t)

	cfg := plugin.ProcessConfig{
		ID:             "mock-cinema",
		ExecutablePath: binPath,
	}

	var crashCalled bool
	var mu sync.Mutex
	proc := plugin.NewProcess(cfg, func(id string, err error) {
		mu.Lock()
		crashCalled = true
		mu.Unlock()
	})

	if proc.State() != plugin.StateStopped {
		t.Fatalf("expected state stopped, got %s", proc.State())
	}

	ctx, cancel := context.WithTimeout(context.Background(), 10*time.Second)
	defer cancel()

	if err := proc.Start(ctx); err != nil {
		t.Fatalf("failed to start process: %v", err)
	}

	if proc.Port() <= 0 {
		t.Fatalf("expected allocated port > 0, got %d", proc.Port())
	}

	if proc.State() != plugin.StateRunning {
		t.Fatalf("expected state running, got %s", proc.State())
	}

	// Verify cannot start again
	if err := proc.Start(ctx); err == nil {
		t.Fatal("expected error starting already running process")
	}

	// Stop cleanly
	if err := proc.Stop(2 * time.Second); err != nil {
		t.Fatalf("failed to stop process: %v", err)
	}

	if proc.State() != plugin.StateStopped {
		t.Fatalf("expected state stopped after clean exit, got %s", proc.State())
	}

	mu.Lock()
	if crashCalled {
		t.Fatal("crash handler should not be called on graceful stop")
	}
	mu.Unlock()
}
