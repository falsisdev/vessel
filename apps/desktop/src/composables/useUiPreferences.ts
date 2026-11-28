import { ref, shallowRef, watch } from 'vue'
import { isTauriRuntime, vesselApi } from '@/api/vessel'
import { applyCustomCss } from '@/composables/useTheme'
import type { UiPreferences } from '@/types/vessel'

export function defaultUiPreferences(): UiPreferences {
  return {
    shell: {
      layout: 'expanded_sidebar',
      mobileNavLayout: 'standard',
      tvNavLayout: 'collapsed_rail',
      startTab: 'home',
      visibleModules: ['home', 'search', 'library', 'live_tv', 'downloads', 'extensions'],
      density: 'comfortable',
      fontScalePercent: 100,
      cornerStyle: 'rounded',
      reducedMotion: false,
    },
    home: {
      preset: 'default',
      heroBanner: 'full',
      heroAutoPlayTrailer: true,
      sectionViewMode: 'rail',
      showContinueWatching: true,
      continueWatchingLimit: 12,
      showRecentIptv: true,
      hiddenSections: [],
      catalogOrder: [],
      selectedPlugins: null,
      heroPlugins: null,
    },
    cards: {
      aspectRatio: 'poster_2x3',
      scalePercent: 100,
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
      autoPlayTrailer: true,
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
      naturalTouchpadScroll: false,
    },
    subtitles: {
      fontSizePx: 28,
      fontFamily: 'system-ui',
      textColor: '#ffffff',
      backgroundOpacity: 72,
      outlineStyle: 'outline',
      bottomOffsetPx: 56,
      bold: false,
    },
    advanced: {
      customCssEnabled: false,
      customCss: '',
    },
    liveTv: {
      pluginOrder: [],
      sectionOrderByPlugin: {},
      favoriteChannels: [],
      viewMode: 'grid',
      density: 'comfortable',
      columns: undefined,
      showLogos: true,
      showCategories: true,
      logoStyle: 'rounded',
      cardAspect: 'auto',
      showChannelNumbers: true,
      showFavoritesBar: true,
      showSectionHeaders: true,
      showActionButtons: true,
      favoritesOnly: false,
      pluginFavoritesOnly: false,
      titleLines: 1,
    },
  }
}

const uiPreferences = ref<UiPreferences>(defaultUiPreferences())
const preferencesLoaded = shallowRef(false)
let preferencesPromise: Promise<void> | null = null
let watcherBound = false

function applyRootPreferences(prefs: UiPreferences): void {
  if (typeof document === 'undefined') return
  const scale = Math.min(135, Math.max(85, prefs.shell.fontScalePercent || 100)) / 100
  const cardScale = Math.min(150, Math.max(70, prefs.cards.scalePercent || 100)) / 100
  document.documentElement.style.setProperty('--vessel-font-scale', String(scale))
  document.documentElement.style.setProperty('--vessel-card-scale', String(cardScale))
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
    if (!isTauriRuntime()) {
      preferencesLoaded.value = true
      return
    }
    if (preferencesPromise) {
      return preferencesPromise
    }
    preferencesPromise = (async () => {
      try {
        const loaded = await vesselApi.getUiPreferences()
        const defaults = defaultUiPreferences()
        uiPreferences.value = {
          ...defaults,
          ...loaded,
          shell: { ...defaults.shell, ...(loaded.shell ?? {}) },
          home: { ...defaults.home, ...(loaded.home ?? {}) },
          cards: { ...defaults.cards, ...(loaded.cards ?? {}) },
          details: { ...defaults.details, ...(loaded.details ?? {}) },
          playback: { ...defaults.playback, ...(loaded.playback ?? {}) },
          subtitles: { ...defaults.subtitles, ...(loaded.subtitles ?? {}) },
          advanced: { ...defaults.advanced, ...(loaded.advanced ?? {}) },
          liveTv: {
            pluginOrder: [...(loaded.liveTv?.pluginOrder ?? defaults.liveTv.pluginOrder)],
            sectionOrderByPlugin: {
              ...defaults.liveTv.sectionOrderByPlugin,
              ...(loaded.liveTv?.sectionOrderByPlugin ?? {}),
            },
            favoriteChannels: [
              ...(loaded.liveTv?.favoriteChannels ?? defaults.liveTv.favoriteChannels),
            ],
            viewMode: loaded.liveTv?.viewMode ?? defaults.liveTv.viewMode,
            density: loaded.liveTv?.density ?? defaults.liveTv.density,
            columns: loaded.liveTv?.columns ?? defaults.liveTv.columns,
            showLogos: loaded.liveTv?.showLogos ?? defaults.liveTv.showLogos,
            showCategories: loaded.liveTv?.showCategories ?? defaults.liveTv.showCategories,
            logoStyle: loaded.liveTv?.logoStyle ?? defaults.liveTv.logoStyle,
            cardAspect: loaded.liveTv?.cardAspect ?? defaults.liveTv.cardAspect,
            showChannelNumbers:
              loaded.liveTv?.showChannelNumbers ?? defaults.liveTv.showChannelNumbers,
            showFavoritesBar: loaded.liveTv?.showFavoritesBar ?? defaults.liveTv.showFavoritesBar,
            showSectionHeaders:
              loaded.liveTv?.showSectionHeaders ?? defaults.liveTv.showSectionHeaders,
            showActionButtons:
              loaded.liveTv?.showActionButtons ?? defaults.liveTv.showActionButtons,
            favoritesOnly: loaded.liveTv?.favoritesOnly ?? defaults.liveTv.favoritesOnly,
            pluginFavoritesOnly:
              loaded.liveTv?.pluginFavoritesOnly ?? defaults.liveTv.pluginFavoritesOnly,
            titleLines: loaded.liveTv?.titleLines ?? defaults.liveTv.titleLines,
          },
        }
      } finally {
        preferencesLoaded.value = true
      }
    })()
    return preferencesPromise
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
    preferencesLoaded,
    loadPreferences,
    savePreferences,
    resetAllPreferences,
  }
}
