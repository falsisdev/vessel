package theme

func GetBuiltinThemes() []*Theme {
	return []*Theme{
		buildVesselDarkTheme(),
		buildVesselLightTheme(),
		buildMidnightOLEDTheme(),
		buildCatppuccinTheme(),
		buildNordTheme(),
		buildDraculaTheme(),
		buildMangileDumanTheme(),
		buildMangileMauveTheme(),
		buildMangileStoneTheme(),
		buildMangileZincTheme(),
		buildMangileSlateTheme(),
		buildMangileOliveTheme(),
		buildMangileTaupeTheme(),
		buildMangileGrayTheme(),
		buildMangileNeutralTheme(),
	}
}

func buildVesselDarkTheme() *Theme {
	return &Theme{
		IsBuiltin: true,
		Manifest: ThemeManifest{
			ID:             "vessel-dark",
			Name:           "Vessel Dark (Default)",
			Version:        "1.0.0",
			Description:    "Modern dark theme with slate and indigo tones",
			Author:         "Vessel Team",
			DefaultVariant: "slate-indigo",
			Variants: []Variant{
				{
					ID:     "slate-indigo",
					Name:   "Slate Indigo",
					IsDark: true,
					Tokens: map[string]string{
						"bg-base":          "#0b0f17",
						"bg-surface":       "#131a26",
						"bg-elevated":      "#1e293b",
						"bg-overlay":       "rgba(11, 15, 23, 0.82)",
						"bg-sidebar":       "#080c13",
						"bg-player":        "#0f1522",
						"bg-card":          "#141c2b",
						"text-primary":     "#f8fafc",
						"text-secondary":   "#cbd5e1",
						"text-muted":       "#94a3b8",
						"text-inverse":     "#0b0f17",
						"accent-primary":   "#6366f1",
						"accent-secondary": "#818cf8",
						"accent-hover":     "#4f46e5",
						"accent-active":    "#4338ca",
						"border-subtle":    "rgba(255, 255, 255, 0.08)",
						"border-default":   "rgba(255, 255, 255, 0.15)",
						"border-strong":    "rgba(255, 255, 255, 0.25)",
						"status-success":   "#10b981",
						"status-warning":   "#f59e0b",
						"status-error":     "#ef4444",
						"status-info":      "#3b82f6",
						"radius-sm":        "6px",
						"radius-md":        "12px",
						"radius-lg":        "18px",
						"radius-full":      "9999px",
						"blur-amount":      "18px",
					},
				},
			},
		},
		CustomCSS: `
.vessel-card {
  transition: transform 0.2s cubic-bezier(0.16, 1, 0.3, 1), box-shadow 0.2s ease;
}
.vessel-card:hover {
  transform: translateY(-2px);
}
`,
	}
}

func buildVesselLightTheme() *Theme {
	return &Theme{
		IsBuiltin: true,
		Manifest: ThemeManifest{
			ID:             "vessel-light",
			Name:           "Vessel Light",
			Version:        "1.0.0",
			Description:    "Clean, crisp light theme with indigo accents",
			Author:         "Vessel Team",
			DefaultVariant: "clean-pearl",
			Variants: []Variant{
				{
					ID:     "clean-pearl",
					Name:   "Clean Pearl",
					IsDark: false,
					Tokens: map[string]string{
						"bg-base":          "#f8fafc",
						"bg-surface":       "#ffffff",
						"bg-elevated":      "#f1f5f9",
						"bg-overlay":       "rgba(255, 255, 255, 0.85)",
						"bg-sidebar":       "#f1f5f9",
						"bg-player":        "#ffffff",
						"bg-card":          "#ffffff",
						"text-primary":     "#0f172a",
						"text-secondary":   "#475569",
						"text-muted":       "#64748b",
						"text-inverse":     "#ffffff",
						"accent-primary":   "#4f46e5",
						"accent-secondary": "#6366f1",
						"accent-hover":     "#4338ca",
						"accent-active":    "#3730a3",
						"border-subtle":    "rgba(0, 0, 0, 0.06)",
						"border-default":   "rgba(0, 0, 0, 0.12)",
						"border-strong":    "rgba(0, 0, 0, 0.20)",
						"status-success":   "#059669",
						"status-warning":   "#d97706",
						"status-error":     "#dc2626",
						"status-info":      "#2563eb",
						"radius-sm":        "6px",
						"radius-md":        "12px",
						"radius-lg":        "18px",
						"radius-full":      "9999px",
						"blur-amount":      "18px",
					},
				},
			},
		},
	}
}

func buildMidnightOLEDTheme() *Theme {
	return &Theme{
		IsBuiltin: true,
		Manifest: ThemeManifest{
			ID:             "midnight-oled",
			Name:           "Midnight OLED",
			Version:        "1.0.0",
			Description:    "True pitch-black palette optimized for OLED displays",
			Author:         "Vessel Team",
			DefaultVariant: "pure-black",
			Variants: []Variant{
				{
					ID:     "pure-black",
					Name:   "Pure Black & Neon Cyan",
					IsDark: true,
					Tokens: map[string]string{
						"bg-base":          "#000000",
						"bg-surface":       "#050505",
						"bg-elevated":      "#101010",
						"bg-overlay":       "rgba(0, 0, 0, 0.92)",
						"bg-sidebar":       "#000000",
						"bg-player":        "#000000",
						"bg-card":          "#0a0a0a",
						"text-primary":     "#ffffff",
						"text-secondary":   "#a1a1aa",
						"text-muted":       "#71717a",
						"text-inverse":     "#000000",
						"accent-primary":   "#06b6d4",
						"accent-secondary": "#22d3ee",
						"accent-hover":     "#0891b2",
						"accent-active":    "#0e7490",
						"border-subtle":    "rgba(255, 255, 255, 0.08)",
						"border-default":   "rgba(255, 255, 255, 0.18)",
						"border-strong":    "rgba(255, 255, 255, 0.35)",
						"status-success":   "#10b981",
						"status-warning":   "#fbbf24",
						"status-error":     "#f87171",
						"status-info":      "#38bdf8",
						"radius-sm":        "4px",
						"radius-md":        "8px",
						"radius-lg":        "14px",
						"radius-full":      "9999px",
						"blur-amount":      "24px",
					},
				},
			},
		},
	}
}

func buildCatppuccinTheme() *Theme {
	return &Theme{
		IsBuiltin: true,
		Manifest: ThemeManifest{
			ID:             "catppuccin",
			Name:           "Catppuccin",
			Version:        "1.0.0",
			Description:    "Soothing warm pastel theme with Mocha and Latte variants",
			Author:         "Catppuccin Community",
			DefaultVariant: "mocha",
			Variants: []Variant{
				{
					ID:     "mocha",
					Name:   "Catppuccin Mocha (Dark)",
					IsDark: true,
					Tokens: map[string]string{
						"bg-base":          "#1e1e2e",
						"bg-surface":       "#181825",
						"bg-elevated":      "#313244",
						"bg-overlay":       "rgba(30, 30, 46, 0.88)",
						"bg-sidebar":       "#181825",
						"bg-player":        "#181825",
						"bg-card":          "#313244",
						"text-primary":     "#cdd6f4",
						"text-secondary":   "#bac2de",
						"text-muted":       "#a6adc8",
						"text-inverse":     "#11111b",
						"accent-primary":   "#cba6f7",
						"accent-secondary": "#f5c2e7",
						"accent-hover":     "#b4befe",
						"accent-active":    "#89b4fa",
						"accent-contrast":  "#11111b",
						"border-subtle":    "rgba(205, 214, 244, 0.08)",
						"border-default":   "rgba(205, 214, 244, 0.16)",
						"border-strong":    "rgba(205, 214, 244, 0.28)",
						"status-success":   "#a6e3a1",
						"status-warning":   "#f9e2af",
						"status-error":     "#f38ba8",
						"status-info":      "#89dceb",
						"radius-sm":        "8px",
						"radius-md":        "14px",
						"radius-lg":        "20px",
						"radius-full":      "9999px",
						"blur-amount":      "16px",
					},
				},
				{
					ID:     "latte",
					Name:   "Catppuccin Latte (Light)",
					IsDark: false,
					Tokens: map[string]string{
						"bg-base":          "#eff1f5",
						"bg-surface":       "#e6e9ef",
						"bg-elevated":      "#ccd0da",
						"bg-overlay":       "rgba(239, 241, 245, 0.88)",
						"bg-sidebar":       "#dce0e8",
						"bg-player":        "#e6e9ef",
						"bg-card":          "#ccd0da",
						"text-primary":     "#4c4f69",
						"text-secondary":   "#5c5f77",
						"text-muted":       "#6c6f85",
						"text-inverse":     "#eff1f5",
						"accent-primary":   "#8839ef",
						"accent-secondary": "#ea76cb",
						"accent-hover":     "#7287fd",
						"accent-active":    "#1e66f5",
						"border-subtle":    "rgba(76, 79, 105, 0.08)",
						"border-default":   "rgba(76, 79, 105, 0.16)",
						"border-strong":    "rgba(76, 79, 105, 0.28)",
						"status-success":   "#40a02b",
						"status-warning":   "#df8e1d",
						"status-error":     "#d20f39",
						"status-info":      "#04a5e5",
						"radius-sm":        "8px",
						"radius-md":        "14px",
						"radius-lg":        "20px",
						"radius-full":      "9999px",
						"blur-amount":      "16px",
					},
				},
			},
		},
	}
}

func buildNordTheme() *Theme {
	return &Theme{
		IsBuiltin: true,
		Manifest: ThemeManifest{
			ID:             "nord",
			Name:           "Nord",
			Version:        "1.0.0",
			Description:    "An arctic, north-bluish clean and elegant color palette",
			Author:         "Arctic Ice Studio",
			DefaultVariant: "arctic-dark",
			Variants: []Variant{
				{
					ID:     "arctic-dark",
					Name:   "Arctic Dark",
					IsDark: true,
					Tokens: map[string]string{
						"bg-base":          "#2e3440",
						"bg-surface":       "#3b4252",
						"bg-elevated":      "#434c5e",
						"bg-overlay":       "rgba(46, 52, 64, 0.88)",
						"bg-sidebar":       "#242933",
						"bg-player":        "#2e3440",
						"bg-card":          "#3b4252",
						"text-primary":     "#eceff4",
						"text-secondary":   "#e5e9f0",
						"text-muted":       "#d8dee9",
						"text-inverse":     "#2e3440",
						"accent-primary":   "#88c0d0",
						"accent-secondary": "#81a1c1",
						"accent-hover":     "#5e81ac",
						"accent-active":    "#8fbcbb",
						"border-subtle":    "rgba(216, 222, 233, 0.08)",
						"border-default":   "rgba(216, 222, 233, 0.16)",
						"border-strong":    "rgba(216, 222, 233, 0.28)",
						"status-success":   "#a3be8c",
						"status-warning":   "#ebcb8b",
						"status-error":     "#bf616a",
						"status-info":      "#81a1c1",
						"radius-sm":        "6px",
						"radius-md":        "10px",
						"radius-lg":        "16px",
						"radius-full":      "9999px",
						"blur-amount":      "14px",
					},
				},
			},
		},
	}
}

func buildDraculaTheme() *Theme {
	return &Theme{
		IsBuiltin: true,
		Manifest: ThemeManifest{
			ID:             "dracula",
			Name:           "Dracula",
			Version:        "1.0.0",
			Description:    "Classic gothic dark theme with purple and pink highlights",
			Author:         "Zeno Rocha",
			DefaultVariant: "vampire",
			Variants: []Variant{
				{
					ID:     "vampire",
					Name:   "Dracula Dark",
					IsDark: true,
					Tokens: map[string]string{
						"bg-base":          "#282a36",
						"bg-surface":       "#343746",
						"bg-elevated":      "#44475a",
						"bg-overlay":       "rgba(40, 42, 54, 0.88)",
						"bg-sidebar":       "#21222c",
						"bg-player":        "#282a36",
						"bg-card":          "#343746",
						"text-primary":     "#f8f8f2",
						"text-secondary":   "#bd93f9",
						"text-muted":       "#6272a4",
						"text-inverse":     "#282a36",
						"accent-primary":   "#ff79c6",
						"accent-secondary": "#bd93f9",
						"accent-hover":     "#ff92d0",
						"accent-active":    "#e065aa",
						"border-subtle":    "rgba(98, 114, 164, 0.25)",
						"border-default":   "rgba(98, 114, 164, 0.40)",
						"border-strong":    "rgba(98, 114, 164, 0.65)",
						"status-success":   "#50fa7b",
						"status-warning":   "#ffb86c",
						"status-error":     "#ff5555",
						"status-info":      "#8be9fd",
						"radius-sm":        "6px",
						"radius-md":        "12px",
						"radius-lg":        "18px",
						"radius-full":      "9999px",
						"blur-amount":      "16px",
					},
				},
			},
		},
	}
}

func buildMangileTokens(base, surface, elevated, sidebar, textPrim, textSec, textMut string) map[string]string {
	return map[string]string{
		"bg-base":          base,
		"bg-surface":       surface,
		"bg-elevated":      elevated,
		"bg-overlay":       "rgba(10, 14, 20, 0.88)",
		"bg-sidebar":       sidebar,
		"bg-player":        base,
		"bg-card":          surface,
		"text-primary":     textPrim,
		"text-secondary":   textSec,
		"text-muted":       textMut,
		"text-inverse":     base,
		"accent-primary":   "#ffffff",
		"accent-secondary": "#f1f5f9",
		"accent-hover":     "#e2e8f0",
		"accent-active":    "#cbd5e1",
		"accent-contrast":  base,
		"border-subtle":    "rgba(255, 255, 255, 0.08)",
		"border-default":   "rgba(255, 255, 255, 0.12)",
		"border-strong":    "rgba(255, 255, 255, 0.22)",
		"status-success":   "#22c55e",
		"status-warning":   "#eab308",
		"status-error":     "#ef4444",
		"status-info":      "#3b82f6",
		"radius-sm":        "6px",
		"radius-md":        "10px",
		"radius-lg":        "16px",
		"radius-full":      "9999px",
		"blur-amount":      "18px",
	}
}

func buildMangileDumanTheme() *Theme {
	return &Theme{
		IsBuiltin: true,
		Manifest: ThemeManifest{
			ID:             "mangile",
			Name:           "Mangile Duman",
			Version:        "1.0.0",
			Description:    "Puslu koyu arduvaz tonları ve beyaz vurgular (Varsayılan Mangile)",
			Author:         "Mangile",
			DefaultVariant: "dark",
			Variants: []Variant{
				{
					ID:     "dark",
					Name:   "Duman Koyu",
					IsDark: true,
					Tokens: buildMangileTokens("#0c1017", "#131a24", "#1b2533", "#090d13", "#f1f5f9", "#94a3b8", "#64748b"),
				},
			},
		},
	}
}

func buildMangileMauveTheme() *Theme {
	return &Theme{
		IsBuiltin: true,
		Manifest: ThemeManifest{
			ID:             "mangile-mauve",
			Name:           "Mangile Leylak",
			Version:        "1.0.0",
			Description:    "Zarif leylak moru koyu zemin ve beyaz vurgular",
			Author:         "Mangile",
			DefaultVariant: "dark",
			Variants: []Variant{
				{
					ID:     "dark",
					Name:   "Leylak Koyu",
					IsDark: true,
					Tokens: buildMangileTokens("#131118", "#1a1722", "#24212e", "#0e0c12", "#f5f3f7", "#d8d4df", "#9e98a8"),
				},
			},
		},
	}
}

func buildMangileStoneTheme() *Theme {
	return &Theme{
		IsBuiltin: true,
		Manifest: ThemeManifest{
			ID:             "mangile-stone",
			Name:           "Mangile Kaya",
			Version:        "1.0.0",
			Description:    "Sıcak doğal kaya ve taş tonlarında koyu zemin",
			Author:         "Mangile",
			DefaultVariant: "dark",
			Variants: []Variant{
				{
					ID:     "dark",
					Name:   "Kaya Koyu",
					IsDark: true,
					Tokens: buildMangileTokens("#141210", "#1c1917", "#292524", "#0c0a09", "#f5f5f4", "#d6d3d1", "#a8a29e"),
				},
			},
		},
	}
}

func buildMangileZincTheme() *Theme {
	return &Theme{
		IsBuiltin: true,
		Manifest: ThemeManifest{
			ID:             "mangile-zinc",
			Name:           "Mangile Çinko",
			Version:        "1.0.0",
			Description:    "Modern derin çinko koyu zemin ve beyaz vurgular",
			Author:         "Mangile",
			DefaultVariant: "dark",
			Variants: []Variant{
				{
					ID:     "dark",
					Name:   "Çinko Koyu",
					IsDark: true,
					Tokens: buildMangileTokens("#09090b", "#141417", "#18181b", "#09090b", "#fafafa", "#d4d4d8", "#a1a1aa"),
				},
			},
		},
	}
}

func buildMangileSlateTheme() *Theme {
	return &Theme{
		IsBuiltin: true,
		Manifest: ThemeManifest{
			ID:             "mangile-slate",
			Name:           "Mangile Arduvaz",
			Version:        "1.0.0",
			Description:    "Gece mavisi arduvaz koyu zemin ve beyaz vurgular",
			Author:         "Mangile",
			DefaultVariant: "dark",
			Variants: []Variant{
				{
					ID:     "dark",
					Name:   "Arduvaz Koyu",
					IsDark: true,
					Tokens: buildMangileTokens("#0b1120", "#141d2f", "#1e293b", "#080d1a", "#f8fafc", "#cbd5e1", "#94a3b8"),
				},
			},
		},
	}
}

func buildMangileOliveTheme() *Theme {
	return &Theme{
		IsBuiltin: true,
		Manifest: ThemeManifest{
			ID:             "mangile-olive",
			Name:           "Mangile Zeytin",
			Version:        "1.0.0",
			Description:    "Doğal orman zeytini koyu zemin ve beyaz vurgular",
			Author:         "Mangile",
			DefaultVariant: "dark",
			Variants: []Variant{
				{
					ID:     "dark",
					Name:   "Zeytin Koyu",
					IsDark: true,
					Tokens: buildMangileTokens("#0f120e", "#161c15", "#20291e", "#0b0e0a", "#f4f6f3", "#d2d7cf", "#9ba399"),
				},
			},
		},
	}
}

func buildMangileTaupeTheme() *Theme {
	return &Theme{
		IsBuiltin: true,
		Manifest: ThemeManifest{
			ID:             "mangile-taupe",
			Name:           "Mangile Boz",
			Version:        "1.0.0",
			Description:    "Sıcak boz gri koyu zemin tonları",
			Author:         "Mangile",
			DefaultVariant: "dark",
			Variants: []Variant{
				{
					ID:     "dark",
					Name:   "Boz Koyu",
					IsDark: true,
					Tokens: buildMangileTokens("#141211", "#1d1a19", "#2a2624", "#0d0b0a", "#f6f4f3", "#d5d0ce", "#a39d99"),
				},
			},
		},
	}
}

func buildMangileGrayTheme() *Theme {
	return &Theme{
		IsBuiltin: true,
		Manifest: ThemeManifest{
			ID:             "mangile-gray",
			Name:           "Mangile Kır",
			Version:        "1.0.0",
			Description:    "Klasik kır gri koyu zemin ve beyaz vurgular",
			Author:         "Mangile",
			DefaultVariant: "dark",
			Variants: []Variant{
				{
					ID:     "dark",
					Name:   "Kır Koyu",
					IsDark: true,
					Tokens: buildMangileTokens("#111827", "#1f2937", "#374151", "#0d1117", "#f9fafb", "#d1d5db", "#9ca3af"),
				},
			},
		},
	}
}

func buildMangileNeutralTheme() *Theme {
	return &Theme{
		IsBuiltin: true,
		Manifest: ThemeManifest{
			ID:             "mangile-neutral",
			Name:           "Mangile Yavan",
			Version:        "1.0.0",
			Description:    "Derin monokrom ve saf koyu zemin tonları",
			Author:         "Mangile",
			DefaultVariant: "dark",
			Variants: []Variant{
				{
					ID:     "dark",
					Name:   "Yavan Koyu",
					IsDark: true,
					Tokens: buildMangileTokens("#0a0a0a", "#171717", "#262626", "#050505", "#fafafa", "#e5e5e5", "#a3a3a3"),
				},
			},
		},
	}
}
