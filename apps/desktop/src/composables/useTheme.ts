import { computed, ref, shallowRef } from 'vue'
import { isTauriRuntime, vesselApi } from '@/api/vessel'
import type { ThemeSpec } from '@/types/vessel'

const DEFAULT_MOCHA: ThemeSpec = {
  schemaVersion: 1,
  id: 'catppuccin-mocha',
  name: 'Catppuccin Mocha',
  author: 'Vessel',
  isDark: true,
  colors: {
    bgBase: '#1e1e2e',
    bgSurface: '#181825',
    bgElevated: '#313244',
    textPrimary: '#cdd6f4',
    textMuted: '#a6adc8',
    accent: '#cba6f7',
    accentSecondary: '#89b4fa',
    border: '#45475a',
    success: '#a6e3a1',
    warning: '#f9e2af',
    danger: '#f38ba8',
  },
}

const themes = ref<ThemeSpec[]>([DEFAULT_MOCHA])
const activeThemeId = shallowRef<string>(
  (typeof localStorage !== 'undefined' && localStorage.getItem('vessel_theme_id')) ||
    DEFAULT_MOCHA.id,
)

let customStyleEl: HTMLStyleElement | null = null

export function applyThemeCssVars(theme: ThemeSpec): void {
  if (typeof document === 'undefined') return
  const root = document.documentElement.style
  root.setProperty('--vessel-bg-base', theme.colors.bgBase)
  root.setProperty('--vessel-bg-surface', theme.colors.bgSurface)
  root.setProperty('--vessel-bg-elevated', theme.colors.bgElevated)
  root.setProperty('--vessel-text-primary', theme.colors.textPrimary)
  root.setProperty('--vessel-text-muted', theme.colors.textMuted)
  root.setProperty('--vessel-accent', theme.colors.accent)
  root.setProperty('--vessel-accent-secondary', theme.colors.accentSecondary)
  root.setProperty('--vessel-border', theme.colors.border)
  root.setProperty('--vessel-success', theme.colors.success)
  root.setProperty('--vessel-warning', theme.colors.warning)
  root.setProperty('--vessel-danger', theme.colors.danger)
  document.documentElement.dataset.theme = theme.id
  document.documentElement.style.colorScheme = theme.isDark ? 'dark' : 'light'
}

export function applyCustomCss(enabled: boolean, css: string): void {
  if (typeof document === 'undefined') return
  if (!enabled || !css.trim()) {
    if (customStyleEl) {
      customStyleEl.textContent = ''
    }
    return
  }
  if (!customStyleEl) {
    customStyleEl = document.createElement('style')
    customStyleEl.id = 'vessel-custom-user-css'
    document.head.appendChild(customStyleEl)
  }
  customStyleEl.textContent = css
}

export function useTheme() {
  const activeTheme = computed(
    () => themes.value.find((t) => t.id === activeThemeId.value) ?? themes.value[0] ?? DEFAULT_MOCHA,
  )

  async function refreshThemes(): Promise<void> {
    if (isTauriRuntime()) {
      themes.value = await vesselApi.listThemes()
    }
    applyThemeCssVars(activeTheme.value)
  }

  function selectTheme(id: string): void {
    activeThemeId.value = id
    if (typeof localStorage !== 'undefined') {
      localStorage.setItem('vessel_theme_id', id)
    }
    applyThemeCssVars(activeTheme.value)
  }

  async function importTheme(json: string): Promise<ThemeSpec> {
    const imported = await vesselApi.importThemeJson(json)
    await refreshThemes()
    selectTheme(imported.id)
    return imported
  }

  return {
    themes,
    activeThemeId,
    activeTheme,
    refreshThemes,
    selectTheme,
    importTheme,
  }
}
