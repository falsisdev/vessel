package plugin

import (
	"errors"
	"fmt"
	"sync"

	pluginv1 "github.com/falsisdev/vessel/proto/gen/go/plugin/v1"
)

var (
	ErrPluginNotFound    = errors.New("plugin not found")
	ErrDuplicatePlugin   = errors.New("plugin already registered")
	ErrNilPluginClient   = errors.New("plugin client is nil")
	ErrNilPluginManifest = errors.New("plugin manifest is nil")
)

type Manager struct {
	mu      sync.RWMutex
	plugins map[string]Client
}

func NewManager() *Manager {
	return &Manager{
		plugins: make(map[string]Client),
	}
}

func (m *Manager) Register(client Client) error {
	if client == nil {
		return ErrNilPluginClient
	}
	manifest := client.Manifest()
	if manifest == nil || manifest.Id == "" {
		return ErrNilPluginManifest
	}

	m.mu.Lock()
	defer m.mu.Unlock()

	if _, exists := m.plugins[manifest.Id]; exists {
		return fmt.Errorf("%w: %s", ErrDuplicatePlugin, manifest.Id)
	}

	m.plugins[manifest.Id] = client
	return nil
}

func (m *Manager) Get(id string) (Client, error) {
	m.mu.RLock()
	defer m.mu.RUnlock()

	client, exists := m.plugins[id]
	if !exists {
		return nil, fmt.Errorf("%w: %s", ErrPluginNotFound, id)
	}
	return client, nil
}

func (m *Manager) Unregister(id string) error {
	m.mu.Lock()
	defer m.mu.Unlock()

	client, exists := m.plugins[id]
	if !exists {
		return fmt.Errorf("%w: %s", ErrPluginNotFound, id)
	}

	delete(m.plugins, id)
	return client.Close()
}

func (m *Manager) Has(id string) bool {
	m.mu.RLock()
	defer m.mu.RUnlock()
	_, exists := m.plugins[id]
	return exists
}

func (m *Manager) ListByDomainAndCapability(domain pluginv1.Domain, cap pluginv1.Capability) []Client {
	m.mu.RLock()
	defer m.mu.RUnlock()

	var result []Client
	for _, client := range m.plugins {
		manifest := client.Manifest()
		if manifest.Domain != domain {
			continue
		}

		for _, c := range manifest.Capabilities {
			if c == cap {
				result = append(result, client)
				break
			}
		}
	}
	return result
}

func (m *Manager) ListAll() []Client {
	m.mu.RLock()
	defer m.mu.RUnlock()

	result := make([]Client, 0, len(m.plugins))
	for _, client := range m.plugins {
		result = append(result, client)
	}
	return result
}

func (m *Manager) Close() error {
	m.mu.Lock()
	defer m.mu.Unlock()

	var errs []error
	for id, client := range m.plugins {
		if err := client.Close(); err != nil {
			errs = append(errs, fmt.Errorf("error closing plugin %s: %w", id, err))
		}
		delete(m.plugins, id)
	}

	if len(errs) > 0 {
		return fmt.Errorf("errors while closing plugins: %v", errs)
	}
	return nil
}
