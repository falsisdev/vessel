package theme

import (
	"encoding/json"
	"errors"
	"fmt"
	"os"
	"path/filepath"
	"sync"
)

var (
	ErrThemeNotFound   = errors.New("theme not found")
	ErrVariantNotFound = errors.New("variant not found")
)

type Manager struct {
	mu              sync.RWMutex
	themes          map[string]*Theme
	themeOrder      []string
	activeThemeID   string
	activeVariantID string
	customDirs      []string
}

func NewManager(customDirs ...string) *Manager {
	m := &Manager{
		themes:          make(map[string]*Theme),
		activeThemeID:   "catppuccin",
		activeVariantID: "mocha",
		customDirs:      customDirs,
	}

	for _, t := range GetBuiltinThemes() {
		m.themes[t.Manifest.ID] = t
		m.themeOrder = append(m.themeOrder, t.Manifest.ID)
	}

	_ = m.Discover()
	return m
}

func (m *Manager) Discover() error {
	m.mu.Lock()
	defer m.mu.Unlock()

	for _, dir := range m.customDirs {
		if dir == "" {
			continue
		}
		entries, err := os.ReadDir(dir)
		if err != nil {
			continue
		}

		for _, e := range entries {
			if !e.IsDir() {
				continue
			}
			themeDir := filepath.Join(dir, e.Name())
			manifestPath := filepath.Join(themeDir, "theme.json")

			data, err := os.ReadFile(manifestPath)
			if err != nil {
				continue
			}

			var manifest ThemeManifest
			if err := json.Unmarshal(data, &manifest); err != nil {
				continue
			}
			if manifest.ID == "" {
				manifest.ID = e.Name()
			}
			if manifest.DefaultVariant == "" && len(manifest.Variants) > 0 {
				manifest.DefaultVariant = manifest.Variants[0].ID
			}

			var customCSS string
			cssPath := filepath.Join(themeDir, "user.css")
			if manifest.CSSPath != "" {
				if filepath.IsAbs(manifest.CSSPath) {
					cssPath = manifest.CSSPath
				} else {
					cssPath = filepath.Join(themeDir, manifest.CSSPath)
				}
			}
			if cssData, err := os.ReadFile(cssPath); err == nil {
				customCSS = string(cssData)
			}

			theme := &Theme{
				Manifest:  manifest,
				IsBuiltin: false,
				Dir:       themeDir,
				CustomCSS: customCSS,
			}

			if _, exists := m.themes[manifest.ID]; !exists {
				m.themeOrder = append(m.themeOrder, manifest.ID)
			}
			m.themes[manifest.ID] = theme
		}
	}

	return nil
}

func (m *Manager) List() []*Theme {
	m.mu.RLock()
	defer m.mu.RUnlock()

	var result []*Theme
	for _, id := range m.themeOrder {
		if t, ok := m.themes[id]; ok {
			result = append(result, t)
		}
	}
	return result
}

func (m *Manager) Get(id string) (*Theme, error) {
	m.mu.RLock()
	defer m.mu.RUnlock()

	t, ok := m.themes[id]
	if !ok {
		return nil, fmt.Errorf("%w: %s", ErrThemeNotFound, id)
	}
	return t, nil
}

func (m *Manager) GetActive() (*ActiveTheme, error) {
	m.mu.RLock()
	defer m.mu.RUnlock()

	themeID := m.activeThemeID
	if themeID == "" {
		themeID = "catppuccin"
	}

	t, ok := m.themes[themeID]
	if !ok {
		// Fallback to first available
		if len(m.themeOrder) > 0 {
			t = m.themes[m.themeOrder[0]]
		} else {
			return nil, ErrThemeNotFound
		}
	}

	variantID := m.activeVariantID
	variant, err := t.GetVariant(variantID)
	if err != nil {
		return nil, err
	}

	compiledCSS := CompileTokensToCSS(variant.Tokens, t.CustomCSS)

	return &ActiveTheme{
		Theme:       t,
		Variant:     *variant,
		Tokens:      variant.Tokens,
		CompiledCSS: compiledCSS,
	}, nil
}

func (m *Manager) SetActive(themeID, variantID string) (*ActiveTheme, error) {
	m.mu.Lock()
	defer m.mu.Unlock()

	t, ok := m.themes[themeID]
	if !ok {
		return nil, fmt.Errorf("%w: %s", ErrThemeNotFound, themeID)
	}

	if variantID == "" {
		variantID = t.Manifest.DefaultVariant
	}

	variant, err := t.GetVariant(variantID)
	if err != nil {
		return nil, fmt.Errorf("%w: %s in theme %s", ErrVariantNotFound, variantID, themeID)
	}

	m.activeThemeID = themeID
	m.activeVariantID = variant.ID

	compiledCSS := CompileTokensToCSS(variant.Tokens, t.CustomCSS)

	return &ActiveTheme{
		Theme:       t,
		Variant:     *variant,
		Tokens:      variant.Tokens,
		CompiledCSS: compiledCSS,
	}, nil
}

func (m *Manager) CompileCSS(themeID, variantID string) (string, error) {
	t, err := m.Get(themeID)
	if err != nil {
		return "", err
	}
	variant, err := t.GetVariant(variantID)
	if err != nil {
		return "", err
	}
	return CompileTokensToCSS(variant.Tokens, t.CustomCSS), nil
}
