import { ref, watch } from 'vue'
import { isTauriRuntime, vesselApi } from '@/api/vessel'
import { applyCustomCss } from '@/composables/useTheme'
import type { UiPreferences } from '@/types/vessel'

export function defaultUiPreferences(): UiPreferences {
  return {
    shell: {
      layout: 'expanded_sidebar',
      visibleModules: ['home', 'search', 'library', 'live_tv', 'downloads', 'extensions'],
      density: 'comfortable',
      fontScalePercent: 100,
      cornerStyle: 'rounded',
      reducedMotion: false,
    },
    home: {
      preset: 'default',
      heroBanner: 'full',
      showContinueWatching: true,
      continueWatchingLimit: 12,
      showRecentIptv: true,
      hiddenSections: [],
      selectedPlugins: null,
    },
    cards: {
      aspectRatio: 'poster_2x3',
      columns: undefined,
      showPluginBadge: true,
      showRating: true,
      showYear: true,
      showContentType: true,
      showWatchedCheck: true,
      showProgressBar: true,
      showHoverActions: true,
      blurNsfwPosters: true,
    },
    details: {
      showBackdrop: true,
      backdropDimPercent: 65,
      showExternalIdBadges: true,
      showFixMatch: true,
      showSwitchSource: true,
      episodeLayout: 'grid_cards',
      hideEpisodeThumbnails: false,
      hideEpisodeOverviews: false,
    },
    playback: {
      clickBehavior: 'quick_start_best',
      showLiveControlBar: true,
      autoNextDelaySecs: 10,
      watchedThresholdPercent: 90,
    },
    advanced: {
      customCssEnabled: false,
      customCss: '',
    },
  }
}

const uiPreferences = ref<UiPreferences>(defaultUiPreferences())
let watcherBound = false

function applyRootPreferences(prefs: UiPreferences): void {
  if (typeof document === 'undefined') return
  const scale = Math.min(135, Math.max(85, prefs.shell.fontScalePercent || 100)) / 100
  document.documentElement.style.setProperty('--vessel-font-scale', String(scale))
  applyCustomCss(prefs.advanced.customCssEnabled, prefs.advanced.customCss)
}

export function useUiPreferences() {
  if (!watcherBound) {
    watcherBound = true
    watch(
      uiPreferences,
      (val) => {
        applyRootPreferences(val)
      },
      { deep: true, immediate: true },
    )
  }

  async function loadPreferences(): Promise<void> {
    if (!isTauriRuntime()) return
    const loaded = await vesselApi.getUiPreferences()
    uiPreferences.value = loaded
  }

  async function savePreferences(next: UiPreferences): Promise<void> {
    uiPreferences.value = next
    if (isTauriRuntime()) {
      await vesselApi.setUiPreferences(next)
    }
  }

  async function resetAllPreferences(): Promise<void> {
    await savePreferences(defaultUiPreferences())
  }

  return {
    uiPreferences,
    loadPreferences,
    savePreferences,
    resetAllPreferences,
  }
}
