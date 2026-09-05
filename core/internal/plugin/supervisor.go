package plugin

import (
	"context"
	"errors"
	"fmt"
	"log/slog"
	"sync"
	"time"
)

var (
	ErrPluginAlreadyManaged = errors.New("plugin is already managed by supervisor")
	ErrPluginNotManaged     = errors.New("plugin is not managed by supervisor")
)

type managedPlugin struct {
	proc       *Process
	manifestID string
}

type Supervisor struct {
	manager   *Manager
	mu        sync.RWMutex
	processes map[string]*managedPlugin
}

func NewSupervisor(manager *Manager) *Supervisor {
	return &Supervisor{
		manager:   manager,
		processes: make(map[string]*managedPlugin),
	}
}

func (s *Supervisor) Launch(ctx context.Context, cfg ProcessConfig) (*GRPCClient, error) {
	s.mu.Lock()
	if _, exists := s.processes[cfg.ID]; exists {
		s.mu.Unlock()
		return nil, fmt.Errorf("%w: %s", ErrPluginAlreadyManaged, cfg.ID)
	}

	proc := NewProcess(cfg, s.handleCrash)
	managed := &managedPlugin{proc: proc}
	s.processes[cfg.ID] = managed
	s.mu.Unlock()

	if err := proc.Start(ctx); err != nil {
		s.mu.Lock()
		delete(s.processes, cfg.ID)
		s.mu.Unlock()
		return nil, fmt.Errorf("failed to start process for %s: %w", cfg.ID, err)
	}

	client, err := waitForPlugin(ctx, proc, 5*time.Second)
	if err != nil {
		_ = proc.Stop(1 * time.Second)
		s.mu.Lock()
		delete(s.processes, cfg.ID)
		s.mu.Unlock()
		return nil, fmt.Errorf("handshake failed for %s at %s: %w", cfg.ID, proc.Target(), err)
	}

	manifestID := client.Manifest().Id
	s.mu.Lock()
	managed.manifestID = manifestID
	if manifestID != cfg.ID {
		s.processes[manifestID] = managed
	}
	s.mu.Unlock()

	if err := s.manager.Register(client); err != nil {
		_ = client.Close()
		_ = proc.Stop(1 * time.Second)
		s.mu.Lock()
		delete(s.processes, cfg.ID)
		if manifestID != cfg.ID {
			delete(s.processes, manifestID)
		}
		s.mu.Unlock()
		return nil, fmt.Errorf("failed to register plugin client for %s: %w", cfg.ID, err)
	}

	slog.Info("Successfully launched and registered plugin",
		"id", cfg.ID,
		"manifest_id", manifestID,
		"target", proc.Target(),
		"is_builtin", client.Manifest().IsBuiltin,
	)

	return client, nil
}

func (s *Supervisor) LaunchDescriptor(ctx context.Context, desc *PluginDescriptor) (*GRPCClient, error) {
	if !desc.Enabled {
		return nil, fmt.Errorf("plugin %s is disabled", desc.ID)
	}

	cfg := ProcessConfig{
		ID:             desc.ID,
		ExecutablePath: desc.ExecutablePath,
		Args:           desc.Args,
		Env:            desc.Env,
	}

	return s.Launch(ctx, cfg)
}

func (s *Supervisor) StopPlugin(id string, timeout time.Duration) error {
	s.mu.Lock()
	managed, exists := s.processes[id]
	if !exists {
		s.mu.Unlock()
		return fmt.Errorf("%w: %s", ErrPluginNotManaged, id)
	}
	delete(s.processes, id)
	if managed.manifestID != "" && managed.manifestID != id {
		delete(s.processes, managed.manifestID)
	}
	delete(s.processes, managed.proc.ID())
	s.mu.Unlock()

	if managed.manifestID != "" {
		_ = s.manager.Unregister(managed.manifestID)
	}
	_ = s.manager.Unregister(id)
	return managed.proc.Stop(timeout)
}

func (s *Supervisor) GetProcess(id string) (*Process, bool) {
	s.mu.RLock()
	defer s.mu.RUnlock()
	managed, exists := s.processes[id]
	if !exists {
		return nil, false
	}
	return managed.proc, true
}

func (s *Supervisor) Shutdown(timeout time.Duration) error {
	s.mu.Lock()
	uniqueProcs := make(map[*Process]string)
	for _, m := range s.processes {
		uniqueProcs[m.proc] = m.manifestID
	}
	s.processes = make(map[string]*managedPlugin)
	s.mu.Unlock()

	var wg sync.WaitGroup
	for proc, manifestID := range uniqueProcs {
		wg.Add(1)
		go func(p *Process, mID string) {
			defer wg.Done()
			if mID != "" {
				_ = s.manager.Unregister(mID)
			}
			if p.ID() != mID {
				_ = s.manager.Unregister(p.ID())
			}
			_ = p.Stop(timeout)
		}(proc, manifestID)
	}
	wg.Wait()

	return nil
}

func (s *Supervisor) handleCrash(id string, err error) {
	slog.Warn("Supervisor handling plugin crash", "id", id, "error", err)

	s.mu.Lock()
	managed, exists := s.processes[id]
	if exists {
		delete(s.processes, id)
		if managed.manifestID != "" && managed.manifestID != id {
			delete(s.processes, managed.manifestID)
		}
		delete(s.processes, managed.proc.ID())
	}
	s.mu.Unlock()

	if exists {
		if managed.manifestID != "" {
			_ = s.manager.Unregister(managed.manifestID)
		}
		if id != managed.manifestID {
			_ = s.manager.Unregister(id)
		}
	} else {
		_ = s.manager.Unregister(id)
	}
}

func waitForPlugin(ctx context.Context, proc *Process, timeout time.Duration) (*GRPCClient, error) {
	deadline := time.Now().Add(timeout)
	var lastErr error

	for time.Now().Before(deadline) {
		select {
		case <-ctx.Done():
			return nil, ctx.Err()
		default:
		}

		if proc.State() == StateCrashed || proc.State() == StateStopped {
			return nil, fmt.Errorf("plugin process %s exited prematurely: %v", proc.ID(), proc.ExitError())
		}

		client, err := Dial(ctx, proc.Target())
		if err == nil {
			return client, nil
		}
		lastErr = err
		time.Sleep(50 * time.Millisecond)
	}

	return nil, fmt.Errorf("plugin failed to respond within %v: %w", timeout, lastErr)
}
