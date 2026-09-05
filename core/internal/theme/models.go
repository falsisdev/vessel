package theme

import (
	"fmt"
	"sort"
	"strings"
)

type Variant struct {
	ID     string            `json:"id"`
	Name   string            `json:"name"`
	IsDark bool              `json:"is_dark"`
	Tokens map[string]string `json:"tokens"`
}

type ThemeManifest struct {
	ID             string    `json:"id"`
	Name           string    `json:"name"`
	Version        string    `json:"version"`
	Description    string    `json:"description"`
	Author         string    `json:"author"`
	DefaultVariant string    `json:"default_variant"`
	Variants       []Variant `json:"variants"`
	CSSPath        string    `json:"css_path,omitempty"`
}

type Theme struct {
	Manifest  ThemeManifest
	IsBuiltin bool
	Dir       string
	CustomCSS string
}

func (t *Theme) GetVariant(variantID string) (*Variant, error) {
	if variantID == "" {
		variantID = t.Manifest.DefaultVariant
	}
	for _, v := range t.Manifest.Variants {
		if v.ID == variantID {
			return &v, nil
		}
	}
	if len(t.Manifest.Variants) > 0 {
		return &t.Manifest.Variants[0], nil
	}
	return nil, fmt.Errorf("no variants defined in theme %s", t.Manifest.ID)
}

type ActiveTheme struct {
	Theme       *Theme
	Variant     Variant
	Tokens      map[string]string
	CompiledCSS string
}

func CompileTokensToCSS(tokens map[string]string, customCSS string) string {
	var keys []string
	for k := range tokens {
		keys = append(keys, k)
	}
	sort.Strings(keys)

	var sb strings.Builder
	sb.WriteString(":root {\n")
	for _, k := range keys {
		cssVarName := strings.ReplaceAll(strings.ToLower(k), "_", "-")
		if !strings.HasPrefix(cssVarName, "v-") {
			cssVarName = "v-" + cssVarName
		}
		sb.WriteString(fmt.Sprintf("  --%s: %s;\n", cssVarName, tokens[k]))
	}
	sb.WriteString("}\n")

	if strings.TrimSpace(customCSS) != "" {
		sb.WriteString("\n/* Custom Theme Styles */\n")
		sb.WriteString(strings.TrimSpace(customCSS))
		sb.WriteString("\n")
	}

	return sb.String()
}
