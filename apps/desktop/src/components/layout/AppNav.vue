<script setup lang="ts">
import { computed, shallowRef } from 'vue'
import { useRoute } from 'vue-router'
import {
  Download,
  Home,
  Library,
  Puzzle,
  Search,
  Settings,
  Tv,
} from 'lucide-vue-next'
import VesselLogo from '@/components/brand/VesselLogo.vue'
import { useI18n } from '@/composables/useI18n'
import type { NavLayout, NavModule } from '@/types/vessel'

const props = defineProps<{
  layout: NavLayout
  visibleModules: NavModule[]
}>()

const route = useRoute()
const { t } = useI18n()
const hovering = shallowRef(false)

const MODULE_META: Record<
  NavModule,
  { path: string; labelKey: string; icon: typeof Home }
> = {
  home: { path: '/', labelKey: 'nav.home', icon: Home },
  search: { path: '/search', labelKey: 'nav.search', icon: Search },
  library: { path: '/library', labelKey: 'nav.library', icon: Library },
  live_tv: { path: '/iptv', labelKey: 'nav.live_tv', icon: Tv },
  downloads: { path: '/downloads', labelKey: 'nav.downloads', icon: Download },
  extensions: { path: '/extensions', labelKey: 'nav.extensions', icon: Puzzle },
}

const navItems = computed(() =>
  props.visibleModules.map((mod) => ({
    key: mod,
    ...MODULE_META[mod],
  })),
)

const isTop = computed(() => props.layout === 'top_navbar')
const isPinned = computed(() => props.layout === 'pinned_sidebar')
const isFloatingBottom = computed(() => props.layout === 'floating_bottom')
const isSidebar = computed(() => !isTop.value && !isFloatingBottom.value)
const showFooterExtensions = computed(
  () => isSidebar.value && props.visibleModules.includes('extensions'),
)
const mainNavItems = computed(() =>
  isSidebar.value
    ? navItems.value.filter((item) => item.key !== 'extensions')
    : navItems.value,
)
const isExpanded = computed(
  () => isPinned.value || (props.layout === 'expanded_sidebar' && hovering.value),
)
const showLabels = computed(() => isTop.value || isExpanded.value)
</script>

<template>
  <aside
    v-if="layout !== 'hidden'"
    class="nuvio-nav"
    :class="{
      'is-top': isTop,
      'is-floating-bottom': isFloatingBottom,
      'is-expanded': isExpanded,
      'is-pinned': isPinned,
    }"
    aria-label="Primary navigation"
    @mouseenter="hovering = true"
    @mouseleave="hovering = false"
  >
    <div v-if="!isFloatingBottom" class="brand" translate="no">
      <div class="brand-mark" aria-hidden="true">
        <VesselLogo :size="30" />
      </div>
      <span v-if="showLabels" class="brand-name">{{ t('app.name') }}</span>
    </div>

    <nav class="nav-list">
      <RouterLink
        v-for="item in mainNavItems"
        :key="item.key"
        :to="item.path"
        class="nav-pill"
        :class="{ 'is-active': route.path === item.path }"
        :aria-label="t(item.labelKey)"
        :title="t(item.labelKey)"
      >
        <component :is="item.icon" :size="19" class="nav-icon" aria-hidden="true" />
        <span v-if="showLabels" class="nav-label">{{ t(item.labelKey) }}</span>
      </RouterLink>
    </nav>

    <div class="nav-footer">
      <RouterLink
        v-if="showFooterExtensions"
        to="/extensions"
        class="nav-pill"
        :class="{ 'is-active': route.path === '/extensions' }"
        :aria-label="t('nav.extensions')"
        :title="t('nav.extensions')"
      >
        <Puzzle :size="19" class="nav-icon" aria-hidden="true" />
        <span v-if="showLabels" class="nav-label">{{ t('nav.extensions') }}</span>
      </RouterLink>
      <RouterLink
        to="/settings"
        class="nav-pill"
        :class="{ 'is-active': route.path === '/settings' }"
        :aria-label="t('nav.settings')"
        :title="t('nav.settings')"
      >
        <Settings :size="19" class="nav-icon" aria-hidden="true" />
        <span v-if="showLabels" class="nav-label">{{ t('nav.settings') }}</span>
      </RouterLink>
    </div>
  </aside>
</template>

<style scoped>
/* Nuvio floating sleek glass rail that expands smoothly on hover without crushing page width */
.nuvio-nav {
  display: flex;
  flex-direction: column;
  align-items: center;
  width: 72px;
  flex-shrink: 0;
  background: var(--vessel-surface-nav);
  border-right: 1px solid var(--vessel-border);
  padding: 1.15rem 0.65rem;
  gap: 1.4rem;
  z-index: 30;
  transition: width 0.28s cubic-bezier(0.22, 1, 0.36, 1);
}

.nuvio-nav.is-expanded {
  width: 220px;
  align-items: stretch;
  padding: 1.15rem 0.85rem;
}

.nuvio-nav.is-top {
  width: 100%;
  flex-direction: row;
  align-items: center;
  border-right: none;
  border-bottom: 1px solid var(--vessel-border);
  padding: 0.7rem 1.5rem;
}

.nuvio-nav.is-floating-bottom {
  position: fixed;
  left: 50%;
  bottom: 1.1rem;
  transform: translateX(-50%);
  width: auto;
  max-width: calc(100vw - 2rem);
  flex-direction: row;
  align-items: center;
  gap: 0.45rem;
  padding: 0.45rem 0.7rem;
  border-radius: 999px;
  border: 1px solid var(--vessel-border-strong);
  background: rgba(14, 15, 24, 0.96);
  box-shadow: 0 18px 44px rgba(0, 0, 0, 0.36);
  z-index: 45;
}

.brand {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 0.7rem;
  padding: 0.25rem;
  font-weight: 800;
  font-size: 1.12rem;
  letter-spacing: -0.025em;
}

.nuvio-nav.is-expanded .brand,
.nuvio-nav.is-top .brand {
  justify-content: flex-start;
  padding-left: 0.45rem;
}

.brand-mark {
  display: flex;
  align-items: center;
  justify-content: center;
  background: transparent;
  border: none;
  box-shadow: none;
  color: var(--vessel-text-primary);
  flex-shrink: 0;
}

.nav-list {
  display: flex;
  flex-direction: column;
  gap: 0.45rem;
  flex: 1;
}

.nuvio-nav.is-top .nav-list,
.nuvio-nav.is-floating-bottom .nav-list {
  flex-direction: row;
  flex: 1;
  overflow-x: auto;
}

.nuvio-nav.is-floating-bottom .nav-pill {
  height: 40px;
  min-width: 40px;
  padding: 0 0.65rem;
  border-radius: 999px;
}

.nuvio-nav.is-floating-bottom .nav-pill:hover {
  transform: translateY(-2px);
}

.nuvio-nav.is-floating-bottom .nav-pill.is-active::before {
  left: 12px;
  right: 12px;
  top: auto;
  bottom: 2px;
  width: auto;
  height: 3px;
}

.nuvio-nav.is-floating-bottom .nav-footer {
  padding-top: 0;
  padding-left: 0.45rem;
  border-top: none;
  border-left: 1px solid var(--vessel-border);
}

.nav-pill {
  position: relative;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 0.8rem;
  height: 44px;
  min-width: 44px;
  padding: 0 0.75rem;
  border-radius: 13px;
  color: var(--vessel-text-muted);
  font-weight: 500;
  transition:
    background-color 0.18s ease,
    color 0.18s ease,
    transform 0.18s cubic-bezier(0.22, 1, 0.36, 1);
  white-space: nowrap;
}

.nuvio-nav.is-expanded .nav-pill,
.nuvio-nav.is-top .nav-pill {
  justify-content: flex-start;
  padding: 0 0.9rem;
}

.nav-pill:hover {
  background: var(--vessel-surface-hover);
  color: var(--vessel-text-primary);
  transform: translateX(2px);
}

.nuvio-nav.is-top .nav-pill:hover {
  transform: translateY(-1px);
}

.nav-pill.is-active {
  background: linear-gradient(90deg, color-mix(in srgb, var(--vessel-accent) 22%, transparent), color-mix(in srgb, var(--vessel-accent-secondary) 10%, transparent));
  color: var(--vessel-text-primary);
  font-weight: 600;
  box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--vessel-accent) 34%, transparent);
}

.nav-pill.is-active::before {
  content: '';
  position: absolute;
  left: -4px;
  top: 10px;
  bottom: 10px;
  width: 3px;
  border-radius: 999px;
  background: var(--vessel-accent);
  box-shadow: 0 0 14px var(--vessel-accent);
}

.nuvio-nav.is-top .nav-pill.is-active::before {
  left: 12px;
  right: 12px;
  top: auto;
  bottom: -2px;
  width: auto;
  height: 3px;
}

.nav-icon {
  flex-shrink: 0;
}

.nav-footer {
  display: flex;
  flex-direction: column;
  gap: 0.45rem;
  padding-top: 0.65rem;
  border-top: 1px solid var(--vessel-border);
}

.nuvio-nav.is-top .nav-footer {
  padding-top: 0;
  border-top: none;
}
</style>
