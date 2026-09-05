package plugin_test

import (
	"context"
	"testing"
	"time"

	"github.com/falsisdev/vessel/core/internal/plugin"
	pluginv1 "github.com/falsisdev/vessel/proto/gen/go/plugin/v1"
)

func TestSupervisorLaunchAndLifecycle(t *testing.T) {
	binPath := buildMockBinary(t)

	mgr := plugin.NewManager()
	supervisor := plugin.NewSupervisor(mgr)

	ctx, cancel := context.WithTimeout(context.Background(), 15*time.Second)
	defer cancel()

	cfg := plugin.ProcessConfig{
		ID:             "com.vessel.cinema.mock",
		ExecutablePath: binPath,
	}

	client, err := supervisor.Launch(ctx, cfg)
	if err != nil {
		t.Fatalf("failed to launch plugin via supervisor: %v", err)
	}

	if client.Manifest().Id != "com.vessel.cinema.mock" {
		t.Fatalf("unexpected manifest id: %s", client.Manifest().Id)
	}

	// Verify manager has registered client
	if !mgr.Has("com.vessel.cinema.mock") {
		t.Fatal("expected manager to have plugin registered")
	}

	// Verify querying client works
	searchResp, err := client.Search(ctx, "Batman", 1)
	if err != nil {
		t.Fatalf("failed to search via client: %v", err)
	}
	if len(searchResp.Items) == 0 {
		t.Fatal("expected search items from mock plugin")
	}

	// Stop plugin via supervisor
	if err := supervisor.StopPlugin("com.vessel.cinema.mock", 2*time.Second); err != nil {
		t.Fatalf("failed to stop plugin: %v", err)
	}

	// Verify manager unregistered client
	if mgr.Has("com.vessel.cinema.mock") {
		t.Fatal("expected manager to unregister stopped plugin")
	}

	// Verify supervisor clean shutdown
	if err := supervisor.Shutdown(1 * time.Second); err != nil {
		t.Fatalf("supervisor shutdown error: %v", err)
	}
}

func TestSupervisorCrashIsolation(t *testing.T) {
	binPath := buildMockBinary(t)

	mgr := plugin.NewManager()
	supervisor := plugin.NewSupervisor(mgr)

	ctx, cancel := context.WithTimeout(context.Background(), 15*time.Second)
	defer cancel()

	cfg := plugin.ProcessConfig{
		ID:             "com.vessel.cinema.crash",
		ExecutablePath: binPath,
	}

	_, err := supervisor.Launch(ctx, cfg)
	if err != nil {
		t.Fatalf("failed to launch plugin: %v", err)
	}

	proc, exists := supervisor.GetProcess("com.vessel.cinema.crash")
	if !exists {
		t.Fatal("expected process to be tracked by supervisor")
	}

	// Simulate unexpected crash by killing the process
	if err := proc.Kill(); err != nil {
		t.Fatalf("failed to kill process for crash simulation: %v", err)
	}

	// Wait briefly for crash callback to unregister from manager
	time.Sleep(300 * time.Millisecond)

	if mgr.Has("com.vessel.cinema.crash") {
		t.Fatal("expected crashed plugin to be unregistered from manager")
	}

	// Core remains healthy and manager is accessible
	remaining := mgr.ListByDomainAndCapability(pluginv1.Domain_DOMAIN_CINEMA, pluginv1.Capability_CAPABILITY_SEARCH)
	if len(remaining) != 0 {
		t.Fatalf("expected 0 remaining plugins, got %d", len(remaining))
	}
}
