package debrid

import (
	"context"
	"fmt"
	"strings"
	"sync"
)

// SettingsStore abstracts the key-value persistence needed for debrid credentials.
type SettingsStore interface {
	GetSetting(ctx context.Context, key string) (string, error)
	SetSetting(ctx context.Context, key, value string) error
	DeleteSetting(ctx context.Context, key string) error
}

type ProviderEntry struct {
	Provider Provider
	Enabled  bool
}

type Manager struct {
	providers map[string]*ProviderEntry
	store     SettingsStore
	mu        sync.RWMutex
}

func NewManager(store SettingsStore) *Manager {
	m := &Manager{
		providers: make(map[string]*ProviderEntry),
		store:     store,
	}

	// Register known providers
	m.RegisterProvider(NewRealDebridProvider(""), true)
	m.RegisterProvider(NewTorBoxProvider(""), true)

	// Load stored configurations
	if store != nil {
		m.loadPersistedConfig(context.Background())
	}

	return m
}

func (m *Manager) RegisterProvider(provider Provider, defaultEnabled bool) {
	m.mu.Lock()
	defer m.mu.Unlock()
	m.providers[provider.Name()] = &ProviderEntry{
		Provider: provider,
		Enabled:  defaultEnabled,
	}
}

func (m *Manager) loadPersistedConfig(ctx context.Context) {
	m.mu.Lock()
	defer m.mu.Unlock()

	for name, entry := range m.providers {
		tokenKey := fmt.Sprintf("debrid:%s:token", name)
		enabledKey := fmt.Sprintf("debrid:%s:enabled", name)

		token, err := m.store.GetSetting(ctx, tokenKey)
		if err == nil && token != "" {
			entry.Provider.SetAPIKey(token)
		}

		enabledStr, err := m.store.GetSetting(ctx, enabledKey)
		if err == nil {
			entry.Enabled = enabledStr != "false"
		}
	}
}

func (m *Manager) Configure(ctx context.Context, providerName, apiKey string, enabled bool) (*AccountStatus, error) {
	m.mu.Lock()
	entry, ok := m.providers[strings.ToLower(providerName)]
	m.mu.Unlock()

	if !ok {
		return nil, fmt.Errorf("unsupported debrid provider: %s", providerName)
	}

	apiKey = strings.TrimSpace(apiKey)
	entry.Provider.SetAPIKey(apiKey)
	entry.Enabled = enabled

	var status *AccountStatus
	var err error

	if apiKey != "" && enabled {
		// Test credentials
		status, err = entry.Provider.GetAccountStatus(ctx)
		if err != nil {
			return nil, fmt.Errorf("failed to authenticate with %s: %w", providerName, err)
		}
	} else {
		status = &AccountStatus{
			Provider: providerName,
			Status:   "disabled",
		}
	}

	// Persist to store
	if m.store != nil {
		tokenKey := fmt.Sprintf("debrid:%s:token", providerName)
		enabledKey := fmt.Sprintf("debrid:%s:enabled", providerName)

		if apiKey != "" {
			_ = m.store.SetSetting(ctx, tokenKey, apiKey)
		} else {
			_ = m.store.DeleteSetting(ctx, tokenKey)
		}

		enabledVal := "true"
		if !enabled {
			enabledVal = "false"
		}
		_ = m.store.SetSetting(ctx, enabledKey, enabledVal)
	}

	return status, nil
}

func (m *Manager) GetStatuses(ctx context.Context, filterProvider string) ([]*AccountStatus, error) {
	m.mu.RLock()
	defer m.mu.RUnlock()

	var results []*AccountStatus
	for name, entry := range m.providers {
		if filterProvider != "" && !strings.EqualFold(name, filterProvider) {
			continue
		}

		if !entry.Provider.IsConfigured() {
			results = append(results, &AccountStatus{
				Provider: name,
				Status:   "not_configured",
			})
			continue
		}

		status, err := entry.Provider.GetAccountStatus(ctx)
		if err != nil {
			results = append(results, &AccountStatus{
				Provider: name,
				Status:   fmt.Sprintf("error: %v", err),
			})
			continue
		}
		results = append(results, status)
	}

	return results, nil
}

// ResolveMagnet attempts to resolve a magnet link using available and enabled debrid providers.
func (m *Manager) ResolveMagnet(ctx context.Context, magnet string, season, episode int, preferredProvider string) (*StreamResult, error) {
	m.mu.RLock()
	defer m.mu.RUnlock()

	// 1. Try preferred provider if specified
	if preferredProvider != "" {
		if entry, ok := m.providers[strings.ToLower(preferredProvider)]; ok && entry.Enabled && entry.Provider.IsConfigured() {
			res, err := entry.Provider.ResolveMagnet(ctx, magnet, season, episode)
			if err == nil && res != nil {
				return res, nil
			}
		}
	}

	// 2. Try priority list (realdebrid, torbox, etc.)
	priorityOrder := []string{"realdebrid", "torbox"}
	for _, name := range priorityOrder {
		if strings.EqualFold(name, preferredProvider) {
			continue // already tried
		}
		entry, ok := m.providers[name]
		if !ok || !entry.Enabled || !entry.Provider.IsConfigured() {
			continue
		}

		res, err := entry.Provider.ResolveMagnet(ctx, magnet, season, episode)
		if err == nil && res != nil {
			return res, nil
		}
	}

	return nil, ErrNotCached
}
