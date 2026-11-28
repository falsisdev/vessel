<script setup lang="ts">
import { computed, onMounted, onUnmounted, shallowRef, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { isTauriRuntime, vesselApi } from '@/api/vessel'
import AppNav from '@/components/layout/AppNav.vue'
import DomainPermissionModal from '@/components/overlays/DomainPermissionModal.vue'
import NowPlayingBar from '@/components/overlays/NowPlayingBar.vue'
import SplashOverlay from '@/components/overlays/SplashOverlay.vue'
import ToastContainer from '@/components/overlays/ToastContainer.vue'
import UpdateModal from '@/components/overlays/UpdateModal.vue'
import { useAppUpdater } from '@/composables/useAppUpdater'
import { useI18n } from '@/composables/useI18n'
import { useTheme } from '@/composables/useTheme'
import { useUiPreferences } from '@/composables/useUiPreferences'
import { useVesselEvents } from '@/composables/useVesselEvents'

const route = useRoute()
const router = useRouter()
const isPlayerRoute = computed(() => route.name === 'player')
const showSplash = shallowRef(true)
const { refreshThemes } = useTheme()
const { refreshLocales, currentLang, setLocale } = useI18n()
const { uiPreferences, loadPreferences } = useUiPreferences()
const { autoCheckEnabled, loadUpdatePreferences, checkForUpdates } = useAppUpdater()
let updateCheckTimer: number | null = null
const {
  playerState,
  pendingDomain,
  toasts,
  startListening,
  stopListening,
  resolveDomainPrompt,
} = useVesselEvents()

const shellClasses = computed(() => [
  `density-${uiPreferences.value.shell.density}`,
  `corners-${uiPreferences.value.shell.cornerStyle}`,
  {
    'is-top-nav': uiPreferences.value.shell.layout === 'top_navbar',
    'is-floating-bottom': uiPreferences.value.shell.layout === 'floating_bottom',
    'reduced-motion': uiPreferences.value.shell.reducedMotion,
  },
])

function handleGlobalShortcut(ev: KeyboardEvent): void {
  if ((ev.metaKey || ev.ctrlKey) && ev.key.toLowerCase() === 'k') {
    ev.preventDefault()
    void router.push({ name: 'search' })
  }
}

onMounted(async () => {
  window.addEventListener('keydown', handleGlobalShortcut)
  await startListening()
  await Promise.all([
    refreshThemes(),
    refreshLocales().then(() => setLocale(currentLang.value)),
    loadPreferences(),
    loadUpdatePreferences(),
  ])
  if (route.name === 'home') {
    const startPref = uiPreferences.value.shell.startTab ?? 'home'
    let targetRoute: 'home' | 'search' | 'library' | 'iptv' = 'home'
    if (startPref === 'search' || startPref === 'library') {
      targetRoute = startPref
    } else if (startPref === 'live_tv') {
      targetRoute = 'iptv'
    } else if (startPref === 'last_visited') {
      let savedTab: string | null = null
      if (isTauriRuntime()) {
        try {
          const raw = await vesselApi.getAppSetting('app_setting:last_visited_tab')
          if (raw) savedTab = JSON.parse(raw) as string
        } catch {
          // Non-fatal
        }
      }
      if (!savedTab && typeof localStorage !== 'undefined') {
        savedTab = localStorage.getItem('vessel_last_visited_tab')
      }
      if (savedTab === 'search' || savedTab === 'library') targetRoute = savedTab
      else if (savedTab === 'live_tv' || savedTab === 'iptv') targetRoute = 'iptv'
    }
    if (targetRoute !== 'home') {
      void router.replace({ name: targetRoute })
    }
  }
  if (autoCheckEnabled.value) {
    updateCheckTimer = window.setTimeout(() => {
      void checkForUpdates(true)
    }, 2500)
  }
})

watch(
  () => route.name,
  (name) => {
    const tabKey =
      name === 'home' || name === 'search' || name === 'library'
        ? name
        : name === 'iptv'
          ? 'live_tv'
          : null
    if (!tabKey) return
    if (typeof localStorage !== 'undefined') {
      localStorage.setItem('vessel_last_visited_tab', tabKey)
    }
    if (isTauriRuntime()) {
      void vesselApi.setAppSetting('app_setting:last_visited_tab', JSON.stringify(tabKey)).catch(() => {})
    }
  },
)

onUnmounted(() => {
  if (updateCheckTimer !== null) {
    window.clearTimeout(updateCheckTimer)
  }
  window.removeEventListener('keydown', handleGlobalShortcut)
  stopListening()
})
</script>

<template>
  <div class="vessel-shell" :class="shellClasses">
    <a href="#main-content" class="skip-link">Skip to content</a>

    <div class="shell-body">
      <AppNav
        v-if="!isPlayerRoute"
        :layout="uiPreferences.shell.layout"
        :visible-modules="uiPreferences.shell.visibleModules"
      />
      <main id="main-content" class="shell-main" tabindex="-1">
        <RouterView v-slot="{ Component, route: currentRoute }">
          <Transition name="page-fade" mode="out-in" appear>
            <KeepAlive :include="['HomeView', 'SearchView', 'LibraryView', 'ExtensionsView']">
              <component
                :is="Component"
                :key="
                  currentRoute.name === 'details' || currentRoute.name === 'player'
                    ? currentRoute.fullPath
                    : String(currentRoute.name)
                "
              />
            </KeepAlive>
          </Transition>
        </RouterView>
      </main>
    </div>

    <NowPlayingBar
      v-if="!isPlayerRoute"
      :state="playerState"
      :visible="uiPreferences.playback.showLiveControlBar"
    />

    <DomainPermissionModal
      :prompt="pendingDomain"
      @resolve="(allow, permanent) => resolveDomainPrompt(allow, permanent)"
    />

    <UpdateModal />

    <ToastContainer :items="toasts" />

    <SplashOverlay v-if="showSplash" @done="showSplash = false" />
  </div>
</template>

<style scoped>
.vessel-shell {
  display: flex;
  flex-direction: column;
  height: 100%;
  background: var(--vessel-bg-base);
  color: var(--vessel-text-primary);
  overflow: hidden;
}

.shell-body {
  position: relative;
  display: flex;
  flex: 1;
  min-height: 0;
}

.vessel-shell.is-top-nav .shell-body {
  flex-direction: column;
}

/* Full-bleed main stage so Hero banners reach every edge cleanly like Nuvio */
.shell-main {
  flex: 1;
  min-width: 0;
  overflow-y: auto;
  overflow-x: hidden;
  padding: 0;
}

.vessel-shell.is-floating-bottom .shell-main {
  padding-bottom: 5.5rem;
}
</style>
