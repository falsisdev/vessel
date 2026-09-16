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
		if rgb := hexToRGBString(tokens[k]); rgb != "" {
			sb.WriteString(fmt.Sprintf("  --%s-rgb: %s;\n", cssVarName, rgb))
		}
	}
	sb.WriteString("}\n")

	if strings.TrimSpace(customCSS) != "" {
		sb.WriteString("\n/* Custom Theme Styles */\n")
		sb.WriteString(strings.TrimSpace(customCSS))
		sb.WriteString("\n")
	}

	return sb.String()
}

// hexToRGBString converts "#rrggbb" (or "#rgb") into "r, g, b" so the value can
// be used directly inside rgba(var(--x-rgb, fallback), alpha). Returns "" for
// non-hex values such as rgba()/rgb() literals.
func hexToRGBString(hex string) string {
	if len(hex) != 4 && len(hex) != 7 {
		return ""
	}
	if hex[0] != '#' {
		return ""
	}
	hexDigit := func(nibble byte) int {
		if nibble >= '0' && nibble <= '9' {
			return int(nibble - '0')
		}
		if nibble >= 'a' && nibble <= 'f' {
			return int(nibble-'a') + 10
		}
		if nibble >= 'A' && nibble <= 'F' {
			return int(nibble-'A') + 10
		}
		return -1
	}
	var r, g, b int
	if len(hex) == 4 {
		r = hexDigit(hex[1])*16 + hexDigit(hex[1])
		g = hexDigit(hex[2])*16 + hexDigit(hex[2])
		b = hexDigit(hex[3])*16 + hexDigit(hex[3])
	} else {
		r = hexDigit(hex[1])*16 + hexDigit(hex[2])
		g = hexDigit(hex[3])*16 + hexDigit(hex[4])
		b = hexDigit(hex[5])*16 + hexDigit(hex[6])
	}
	if r < 0 || g < 0 || b < 0 {
		return ""
	}
	return fmt.Sprintf("%d, %d, %d", r, g, b)
}
