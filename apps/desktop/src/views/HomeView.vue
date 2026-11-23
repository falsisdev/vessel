<script setup lang="ts">
import { computed, onActivated, onDeactivated, onMounted, onUnmounted, ref, shallowRef } from 'vue'
import { useRouter } from 'vue-router'
import {
  ChevronLeft,
  ChevronRight,
  GripVertical,
  Info,
  Layers,
  Play,
  RefreshCw,
  SlidersHorizontal,
  Sparkles,
  Star,
  X,
} from 'lucide-vue-next'
import { isTauriRuntime, vesselApi } from '@/api/vessel'
import MediaCard from '@/components/media/MediaCard.vue'
import { useHomeCache } from '@/composables/useHomeCache'
import { useI18n } from '@/composables/useI18n'
import { usePlayerSession } from '@/composables/usePlayerSession'
import { useUiPreferences } from '@/composables/useUiPreferences'
import { pushToast } from '@/composables/useVesselEvents'
import type { ContinueWatchingEntry, MediaSummary, PluginListEntry } from '@/types/vessel'

const router = useRouter()
const { t } = useI18n()
const { setSession } = usePlayerSession()
const { uiPreferences, savePreferences } = useUiPreferences()
const {
  cachedShelves: shelves,
  cachedContinueWatching: continueWatching,
  cachedPlugins: allPlugins,
  cachedFilterKey,
  hasLoadedOnce,
} = useHomeCache()

const loading = shallowRef(false)
const resumingKey = shallowRef<string | null>(null)
const heroIndex = shallowRef(0)
const showSourcePicker = shallowRef(false)
const orderListRef = ref<HTMLElement | null>(null)
const draggingPluginIndex = shallowRef<number | null>(null)

let heroAutoTimer: number | null = null

defineOptions({ name: 'HomeView' })

const enabledPlugins = computed(() => allPlugins.value.filter((p) => p.enabled))

const activeHomePluginIds = computed<string[]>(() => {
  const enabledSet = new Set(enabledPlugins.value.map((p) => p.manifest.id))
  const sel = uiPreferences.value.home.selectedPlugins
  if (sel && sel.length > 0) {
    return sel.filter((id) => enabledSet.has(id))
  }
  if (sel && sel.length === 0) {
    return []
  }
  // Default when user hasn't picked explicitly yet: smart curated starter set <= 5 plugins
  const preferred = ['anizium', 'diziwatch', 'yabancidizi', 'animecix', 'seicode', 'anthology_spor']
  const picked: string[] = []
  for (const id of preferred) {
    if (enabledSet.has(id) && picked.length < 5) picked.push(id)
  }
  for (const p of enabledPlugins.value) {
    if (!picked.includes(p.manifest.id) && picked.length < 5) {
      picked.push(p.manifest.id)
    }
  }
  return picked
})

const activeHeroPluginIds = computed<string[]>(() => {
  const enabledSet = new Set(enabledPlugins.value.map((p) => p.manifest.id))
  const sel = uiPreferences.value.home.heroPlugins
  if (!sel || sel.length === 0) return []
  return sel.filter((id) => enabledSet.has(id))
})

function resolveGroupName(p: PluginListEntry): string {
  if (p.repoId?.trim()) return p.repoId.trim()
  const domains = p.allowedDomains ?? []
  if (
    domains.includes('raw.githubusercontent.com') &&
    (p.manifest.author === 'falsisdev' ||
      p.manifest.author === 'metemel' ||
      p.manifest.id.startsWith('anthology') ||
      p.manifest.id === 'm3u.anthology.addon')
  ) {
    return 'Anthology'
  }
  if (p.manifest.id.startsWith('community.anthology')) return 'Anthology'
  if (p.sourceType === 'dev_dir') return 'Local Dev Plugins'
  return 'Standalone Plugins'
}

const groupedEnabledPlugins = computed(() => {
  const buckets = new Map<string, PluginListEntry[]>()
  for (const p of enabledPlugins.value) {
    const group = resolveGroupName(p)
    const arr = buckets.get(group)
    if (arr) arr.push(p)
    else buckets.set(group, [p])
  }
  return Array.from(buckets.entries()).map(([name, items]) => ({ name, items }))
})

const pluginNameById = computed(() => {
  const map = new Map<string, string>()
  for (const p of allPlugins.value) {
    map.set(p.manifest.id, p.manifest.name)
  }
  return map
})

const orderedShelves = computed(() => {
  const order = activeHomePluginIds.value
  const allowed = new Set(order)
  return shelves.value
    .filter((s) => allowed.has(s.pluginId))
    .slice()
    .sort((a, b) => {
      const ia = order.indexOf(a.pluginId)
      const ib = order.indexOf(b.pluginId)
      return (ia === -1 ? 999 : ia) - (ib === -1 ? 999 : ib)
    })
})

const spotlightPool = computed<Array<MediaSummary & { resolvedPluginId: string }>>(() => {
  const heroIds = activeHeroPluginIds.value
  const sourceShelves =
    heroIds.length > 0
      ? shelves.value.filter((s) => heroIds.includes(s.pluginId))
      : orderedShelves.value
  const candidateShelves = sourceShelves.length > 0 ? sourceShelves : shelves.value

  const collected: Array<MediaSummary & { resolvedPluginId: string }> = []
  for (const shelf of candidateShelves) {
    for (const section of shelf.sections) {
      for (const item of section.items) {
        if (!item.poster) continue
        collected.push({
          ...item,
          resolvedPluginId: item.pluginId ?? shelf.pluginId,
        })
        if (collected.length >= 6) return collected
      }
    }
  }
  if (!collected.length) {
    for (const shelf of candidateShelves) {
      for (const section of shelf.sections) {
        for (const item of section.items) {
          collected.push({
            ...item,
            resolvedPluginId: item.pluginId ?? shelf.pluginId,
          })
          if (collected.length >= 6) return collected
        }
      }
    }
  }
  return collected
})

const featuredItem = computed(() => {
  if (!spotlightPool.value.length) return null
  const idx = heroIndex.value % spotlightPool.value.length
  return spotlightPool.value[idx]
})

async function loadHome(forceRefresh = false): Promise<void> {
  if (!isTauriRuntime()) return
  try {
    allPlugins.value = await vesselApi.listPlugins()
  } catch {
    // Non-fatal
  }

  const targetIds = Array.from(new Set([...activeHomePluginIds.value, ...activeHeroPluginIds.value]))
  const targetKey = `${activeHomePluginIds.value.slice().sort().join(',')}|${activeHeroPluginIds.value.slice().sort().join(',')}`

  // Always silently refresh Continue Watching so Home return feels instant and accurate
  const cwPromise = vesselApi
    .listContinueWatching(uiPreferences.value.home.continueWatchingLimit || 12)
    .then((cw) => {
      continueWatching.value = cw
    })
    .catch(() => {})

  if (!forceRefresh && hasLoadedOnce.value && cachedFilterKey.value === targetKey) {
    await cwPromise
    return
  }

  loading.value = true
  try {
    if (forceRefresh) {
      await vesselApi.clearRuntimeCaches().catch(() => {})
    }
    const [feeds] = await Promise.all([
      targetIds.length > 0 ? vesselApi.getHomeFeeds(targetIds) : Promise.resolve([]),
      cwPromise,
    ])
    shelves.value = feeds
    cachedFilterKey.value = targetKey
    hasLoadedOnce.value = true
    heroIndex.value = 0
  } catch (err) {
    pushToast('error', String(err))
  } finally {
    loading.value = false
  }
}

function isHomeSelected(pluginId: string): boolean {
  return activeHomePluginIds.value.includes(pluginId)
}

function isHeroSelected(pluginId: string): boolean {
  return activeHeroPluginIds.value.includes(pluginId)
}

async function toggleHomePlugin(pluginId: string): Promise<void> {
  const next = activeHomePluginIds.value.slice()
  const idx = next.indexOf(pluginId)
  if (idx >= 0) next.splice(idx, 1)
  else next.push(pluginId)
  uiPreferences.value.home.selectedPlugins = next
  await savePreferences(uiPreferences.value)
}

async function toggleHeroPlugin(pluginId: string): Promise<void> {
  const next = activeHeroPluginIds.value.slice()
  const idx = next.indexOf(pluginId)
  if (idx >= 0) next.splice(idx, 1)
  else next.push(pluginId)
  uiPreferences.value.home.heroPlugins = next.length > 0 ? next : null
  heroIndex.value = 0
  await savePreferences(uiPreferences.value)
}

async function clearHeroPlugins(): Promise<void> {
  uiPreferences.value.home.heroPlugins = null
  heroIndex.value = 0
  await savePreferences(uiPreferences.value)
}

function startOrderDrag(index: number, ev: PointerEvent): void {
  if (ev.button !== 0) return
  ev.preventDefault()
  draggingPluginIndex.value = index

  const onMove = (moveEv: PointerEvent) => {
    const listEl = orderListRef.value
    const curIdx = draggingPluginIndex.value
    if (!listEl || curIdx === null) return
    const rows = Array.from(listEl.children) as HTMLElement[]
    let targetIdx = curIdx
    for (let i = 0; i < rows.length; i++) {
      const rect = rows[i].getBoundingClientRect()
      const midY = rect.top + rect.height / 2
      if (moveEv.clientY < midY) {
        targetIdx = i
        break
      }
      targetIdx = i
    }
    if (targetIdx !== curIdx) {
      const next = activeHomePluginIds.value.slice()
      const [moved] = next.splice(curIdx, 1)
      if (!moved) return
      next.splice(targetIdx, 0, moved)
      uiPreferences.value.home.selectedPlugins = next
      draggingPluginIndex.value = targetIdx
    }
  }

  const onUp = () => {
    window.removeEventListener('pointermove', onMove)
    window.removeEventListener('pointerup', onUp)
    window.removeEventListener('pointercancel', onUp)
    draggingPluginIndex.value = null
    void savePreferences(uiPreferences.value)
  }

  window.addEventListener('pointermove', onMove)
  window.addEventListener('pointerup', onUp)
  window.addEventListener('pointercancel', onUp)
}

async function applyHomeSources(): Promise<void> {
  showSourcePicker.value = false
  await loadHome(true)
}

async function toggleGroupOnHome(items: PluginListEntry[], enable: boolean): Promise<void> {
  const next = new Set(activeHomePluginIds.value)
  for (const item of items) {
    if (enable) next.add(item.manifest.id)
    else next.delete(item.manifest.id)
  }
  uiPreferences.value.home.selectedPlugins = Array.from(next)
  await savePreferences(uiPreferences.value)
}

function progressPercent(entry: ContinueWatchingEntry): number {
  if (!entry.durationMs || entry.durationMs <= 0) return 15
  return Math.min(100, Math.max(4, Math.round((entry.positionMs / entry.durationMs) * 100)))
}

function formatRemaining(entry: ContinueWatchingEntry): string {
  if (!entry.durationMs || entry.durationMs <= entry.positionMs) return 'Resume'
  const remSec = Math.max(0, Math.floor((entry.durationMs - entry.positionMs) / 1000))
  const mins = Math.floor(remSec / 60)
  if (mins >= 60) {
    const hrs = Math.floor(mins / 60)
    return `${hrs}h ${mins % 60}m left`
  }
  if (mins >= 1) return `${mins}m left`
  return `${remSec}s left`
}

async function resumeContinueWatching(entry: ContinueWatchingEntry): Promise<void> {
  const key = `${entry.media.pluginId}:${entry.media.nativeId}`
  if (resumingKey.value) return
  resumingKey.value = key
  try {
    const [detail, streamRes] = await Promise.all([
      vesselApi.getDetailsWithEnrichment(entry.media.pluginId, entry.media.nativeId, 'en'),
      vesselApi.getStreams(entry.media.pluginId, entry.streamId),
    ])
    const ep =
      detail.episodes.find((item) => item.streamId === entry.streamId) ??
      detail.episodes.find((item) => item.season === entry.season && item.episode === entry.episode) ??
      detail.episodes[0]

    if (!ep || !streamRes.streams.length) {
      openDetails(entry.media.pluginId, entry.media.nativeId)
      return
    }

    setSession({
      pluginId: entry.media.pluginId,
      nativeId: entry.media.nativeId,
      detail,
      episode: ep,
      streams: streamRes.streams,
      subtitles: streamRes.subtitles,
      activeStreamIndex: 0,
      startPositionMs: entry.positionMs,
    })

    await router.push({
      name: 'player',
      params: {
        pluginId: entry.media.pluginId,
        nativeId: entry.media.nativeId,
      },
      query: {
        streamId: ep.streamId,
        streamIndex: '0',
      },
    })
  } catch (err) {
    pushToast('warn', String(err))
    openDetails(entry.media.pluginId, entry.media.nativeId)
  } finally {
    resumingKey.value = null
  }
}

async function dismissContinueWatching(entry: ContinueWatchingEntry): Promise<void> {
  try {
    await vesselApi.deleteWatchProgress(entry.media.pluginId, entry.media.nativeId)
    continueWatching.value = continueWatching.value.filter(
      (item) =>
        !(
          item.media.pluginId === entry.media.pluginId &&
          item.media.nativeId === entry.media.nativeId
        ),
    )
  } catch (err) {
    pushToast('error', String(err))
  }
}

function openDetails(pluginId: string, nativeId: string): void {
  void router.push({ name: 'details', params: { pluginId, nativeId } })
}

function scrollRail(railId: string, dir: -1 | 1): void {
  const el = document.getElementById(railId)
  if (el) {
    el.scrollBy({ left: dir * 640, behavior: 'smooth' })
  }
}

function startHeroTimer(): void {
  if (heroAutoTimer !== null) window.clearInterval(heroAutoTimer)
  heroAutoTimer = window.setInterval(() => {
    if (spotlightPool.value.length > 1 && !showSourcePicker.value) {
      heroIndex.value = (heroIndex.value + 1) % spotlightPool.value.length
    }
  }, 9000)
}

onMounted(() => {
  void loadHome(false)
  startHeroTimer()
})

onActivated(() => {
  void loadHome(false)
  startHeroTimer()
})

onDeactivated(() => {
  if (heroAutoTimer !== null) {
    window.clearInterval(heroAutoTimer)
    heroAutoTimer = null
  }
})

onUnmounted(() => {
  if (heroAutoTimer !== null) window.clearInterval(heroAutoTimer)
})
</script>

<template>
  <section class="nuvio-home">
    <!-- Full-bleed Luxury Cinema Spotlight Hero -->
    <header
      v-if="uiPreferences.home.heroBanner !== 'hidden'"
      class="nuvio-hero"
      :class="{ 'is-compact': uiPreferences.home.heroBanner === 'compact' }"
    >
      <Transition name="hero-swap" mode="out-in">
        <div v-if="featuredItem" :key="`${featuredItem.resolvedPluginId}:${featuredItem.nativeId}`" class="hero-stage">
          <img
            v-if="featuredItem.poster"
            :src="featuredItem.poster"
            :alt="featuredItem.title"
            width="1440"
            height="680"
            fetchpriority="high"
            decoding="async"
            class="hero-art"
          />
          <div class="hero-vignette" aria-hidden="true" />

          <div class="hero-content">
            <div class="hero-meta-line">
              <span class="type-pill">
                <Sparkles :size="11" aria-hidden="true" />
                {{ featuredItem.isAnime ? 'ANIME' : featuredItem.type.toUpperCase() }}
              </span>
              <span v-if="featuredItem.year">{{ featuredItem.year }}</span>
              <span v-if="featuredItem.rating !== undefined" class="hero-rating">
                <Star :size="13" fill="currentColor" aria-hidden="true" />
                {{ featuredItem.rating.toFixed(1) }}
              </span>
              <span class="source-tag">{{ featuredItem.resolvedPluginId }}</span>
            </div>

            <h1 class="hero-title">{{ featuredItem.title }}</h1>
            <p class="hero-sub">{{ t('home.hero_subtitle') }}</p>

            <div class="hero-actions">
              <button
                type="button"
                class="v-btn v-btn-primary"
                @click="openDetails(featuredItem.resolvedPluginId, featuredItem.nativeId)"
              >
                <Play :size="16" fill="currentColor" aria-hidden="true" /> Watch Now
              </button>
              <button
                type="button"
                class="v-btn"
                @click="openDetails(featuredItem.resolvedPluginId, featuredItem.nativeId)"
              >
                <Info :size="16" aria-hidden="true" /> More Info
              </button>
              <button
                v-if="enabledPlugins.length"
                type="button"
                class="v-btn"
                @click="showSourcePicker = true"
              >
                <SlidersHorizontal :size="15" aria-hidden="true" />
                Feeds ({{ activeHomePluginIds.length }}/{{ enabledPlugins.length }})
              </button>
              <button
                type="button"
                class="v-btn v-btn-icon"
                :aria-label="t('common.refresh')"
                :disabled="loading"
                @click="loadHome(true)"
              >
                <RefreshCw :size="15" :class="{ spin: loading }" aria-hidden="true" />
              </button>
            </div>
          </div>
        </div>

        <div v-else key="empty-hero" class="hero-stage">
          <div class="hero-vignette" aria-hidden="true" />
          <div class="hero-content">
            <h1 class="hero-title">{{ t('home.hero_title') }}</h1>
            <p class="hero-sub">{{ t('home.hero_subtitle') }}</p>
            <div class="hero-actions">
              <RouterLink to="/extensions" class="v-btn v-btn-primary">
                {{ t('nav.extensions') }}
              </RouterLink>
              <button
                v-if="enabledPlugins.length"
                type="button"
                class="v-btn"
                @click="showSourcePicker = true"
              >
                <SlidersHorizontal :size="15" aria-hidden="true" />
                Choose Home Feeds ({{ activeHomePluginIds.length }})
              </button>
              <button type="button" class="v-btn" :disabled="loading" @click="loadHome(true)">
                <RefreshCw :size="15" :class="{ spin: loading }" aria-hidden="true" />
                {{ t('common.refresh') }}
              </button>
            </div>
          </div>
        </div>
      </Transition>

      <div v-if="spotlightPool.length > 1" class="hero-indicators">
        <button
          v-for="(item, idx) in spotlightPool"
          :key="`${item.resolvedPluginId}:${item.nativeId}`"
          type="button"
          class="indicator-bar"
          :class="{ 'is-active': idx === heroIndex }"
          :aria-label="`Spotlight ${item.title}`"
          @click="heroIndex = idx"
        />
      </div>
    </header>

    <!-- Main horizontal rails area -->
    <div class="home-rails-body">
      <!-- Continue Watching Rail -->
      <section
        v-if="uiPreferences.home.showContinueWatching && continueWatching.length"
        class="nuvio-rail continue-rail"
      >
        <div class="rail-head">
          <div class="rail-title-wrap">
            <h2 class="rail-title">Continue Watching</h2>
            <span class="rail-source v-muted">{{ continueWatching.length }} active</span>
          </div>
          <div class="rail-arrows">
            <button
              type="button"
              class="v-btn v-btn-icon"
              aria-label="Scroll continue watching left"
              @click="scrollRail('rail-continue-watching', -1)"
            >
              <ChevronLeft :size="16" aria-hidden="true" />
            </button>
            <button
              type="button"
              class="v-btn v-btn-icon"
              aria-label="Scroll continue watching right"
              @click="scrollRail('rail-continue-watching', 1)"
            >
              <ChevronRight :size="16" aria-hidden="true" />
            </button>
          </div>
        </div>

        <div id="rail-continue-watching" class="rail-track">
          <article
            v-for="entry in continueWatching"
            :key="`${entry.media.pluginId}:${entry.media.nativeId}`"
            class="cw-card"
            :class="{ 'is-loading': resumingKey === `${entry.media.pluginId}:${entry.media.nativeId}` }"
            @click="resumeContinueWatching(entry)"
          >
            <div class="cw-thumb">
              <img
                v-if="entry.backdrop ?? entry.poster"
                :src="entry.backdrop ?? entry.poster"
                :alt="entry.title"
                loading="eager"
                decoding="async"
                class="cw-img"
              />
              <div class="cw-shade" />
              <span class="cw-play-badge">
                <Play :size="18" fill="currentColor" aria-hidden="true" />
              </span>
              <button
                type="button"
                class="cw-dismiss"
                title="Remove from Continue Watching"
                aria-label="Remove from Continue Watching"
                @click.stop="dismissContinueWatching(entry)"
              >
                <X :size="14" aria-hidden="true" />
              </button>
              <span class="cw-remaining">{{ formatRemaining(entry) }}</span>
              <div class="cw-progress-track">
                <div class="cw-progress-fill" :style="{ width: `${progressPercent(entry)}%` }" />
              </div>
            </div>
            <div class="cw-meta">
              <div class="cw-title">{{ entry.title }}</div>
              <div class="cw-sub">
                <span>S{{ entry.season }} · E{{ entry.episode }}</span>
                <template v-if="entry.episodeTitle"> · {{ entry.episodeTitle }}</template>
              </div>
            </div>
          </article>
        </div>
      </section>

      <div v-if="loading && !shelves.length" class="rail-track">
        <div v-for="n in 7" :key="n" class="v-skeleton rail-skeleton" />
      </div>

      <div v-else-if="!shelves.length" class="v-card empty-box">
        <p class="v-muted empty-text">
          {{
            enabledPlugins.length
              ? 'No Home feeds are currently selected. Pick your favorite plugins for Home or explore Search.'
              : t('home.empty_plugins')
          }}
        </p>
        <div class="hero-actions">
          <button
            v-if="enabledPlugins.length"
            type="button"
            class="v-btn v-btn-primary"
            @click="showSourcePicker = true"
          >
            <Layers :size="15" /> Select Home Sources
          </button>
          <RouterLink v-else to="/extensions" class="v-btn v-btn-primary">
            {{ t('nav.extensions') }}
          </RouterLink>
        </div>
      </div>

      <div v-else class="shelf-stack">
        <div v-for="shelf in orderedShelves" :key="shelf.pluginId" class="plugin-block">
          <div v-if="shelf.error" class="v-card error-shelf">
            <strong>{{ shelf.pluginName }}</strong> — {{ t('home.plugin_error') }}:
            <span class="v-muted">{{ shelf.error }}</span>
          </div>

          <template v-for="section in shelf.sections" :key="`${shelf.pluginId}:${section.id}`">
            <section
              v-if="!uiPreferences.home.hiddenSections.includes(section.id)"
              class="nuvio-rail"
            >
              <div class="rail-head">
                <div class="rail-title-wrap">
                  <h2 class="rail-title">{{ section.title }}</h2>
                  <span class="rail-source v-muted">{{ shelf.pluginName }}</span>
                </div>
                <div class="rail-arrows">
                  <button
                    type="button"
                    class="v-btn v-btn-icon"
                    aria-label="Scroll shelf left"
                    @click="scrollRail(`rail-${shelf.pluginId}-${section.id}`, -1)"
                  >
                    <ChevronLeft :size="16" aria-hidden="true" />
                  </button>
                  <button
                    type="button"
                    class="v-btn v-btn-icon"
                    aria-label="Scroll shelf right"
                    @click="scrollRail(`rail-${shelf.pluginId}-${section.id}`, 1)"
                  >
                    <ChevronRight :size="16" aria-hidden="true" />
                  </button>
                </div>
              </div>

              <div :id="`rail-${shelf.pluginId}-${section.id}`" class="rail-track">
                <div
                  v-for="item in section.items"
                  :key="`${shelf.pluginId}:${item.nativeId}`"
                  class="rail-card-slot"
                >
                  <MediaCard
                    :item="{ ...item, pluginId: item.pluginId ?? shelf.pluginId }"
                    :cards="uiPreferences.cards"
                    @open="openDetails"
                  />
                </div>
              </div>
            </section>
          </template>
        </div>
      </div>
    </div>

    <!-- Customize Home Feeds Modal -->
    <Teleport to="body">
      <Transition name="modal-pop">
        <div
          v-if="showSourcePicker"
          class="home-modal-backdrop"
          role="dialog"
          aria-modal="true"
          aria-label="Select Home feeds"
          @click.self="showSourcePicker = false"
        >
          <div class="v-card home-picker-modal">
            <div class="picker-head">
              <div>
                <h3 class="picker-title">Select Home Catalog Feeds</h3>
                <p class="v-muted picker-sub">
                  Pick which plugins show shelves on Home. Unchecked plugins stay fast and active in Search.
                </p>
              </div>
              <button
                type="button"
                class="v-btn v-btn-icon"
                aria-label="Close"
                @click="showSourcePicker = false"
              >
                <X :size="16" />
              </button>
            </div>

            <div class="picker-groups">
              <div class="hero-source-section">
                <div class="picker-group-head">
                  <strong>Spotlight Hero Source</strong>
                  <button
                    type="button"
                    class="v-btn v-btn-sm"
                    :disabled="!activeHeroPluginIds.length"
                    @click="clearHeroPlugins"
                  >
                    Auto (All Active Feeds)
                  </button>
                </div>
                <p class="v-muted picker-hero-sub">
                  Pick specific plugin(s) for the Hero banner, or leave Auto to pull from your top Home feeds.
                </p>
                <div class="hero-chip-grid">
                  <button
                    v-for="p in enabledPlugins"
                    :key="`hero-${p.manifest.id}`"
                    type="button"
                    class="hero-source-chip"
                    :class="{ checked: isHeroSelected(p.manifest.id) }"
                    @click="toggleHeroPlugin(p.manifest.id)"
                  >
                    <Sparkles :size="12" aria-hidden="true" />
                    <span>{{ p.manifest.name }}</span>
                  </button>
                </div>
              </div>

              <div v-if="activeHomePluginIds.length" class="order-section">
                <div class="picker-group-head">
                  <strong>Active Home Shelf Order (Top → Bottom)</strong>
                  <span class="v-muted group-hint">Drag handle to reorder shelves</span>
                </div>
                <div ref="orderListRef" class="order-list">
                  <div
                    v-for="(pid, idx) in activeHomePluginIds"
                    :key="pid"
                    class="order-row"
                    :class="{ 'is-dragging': draggingPluginIndex === idx }"
                  >
                    <button
                      type="button"
                      class="drag-handle"
                      title="Drag to reorder"
                      aria-label="Drag to reorder"
                      @pointerdown="startOrderDrag(idx, $event)"
                    >
                      <GripVertical :size="15" aria-hidden="true" />
                    </button>
                    <span class="order-index">{{ idx + 1 }}</span>
                    <span class="order-name">{{ pluginNameById.get(pid) ?? pid }}</span>
                    <button
                      type="button"
                      class="v-btn v-btn-icon"
                      title="Remove from Home"
                      @click="toggleHomePlugin(pid)"
                    >
                      <X :size="14" />
                    </button>
                  </div>
                </div>
              </div>

              <div v-for="group in groupedEnabledPlugins" :key="group.name" class="picker-group">
                <div class="picker-group-head">
                  <strong>{{ group.name }}</strong>
                  <div class="picker-group-btns">
                    <button
                      type="button"
                      class="v-btn v-btn-sm"
                      @click="toggleGroupOnHome(group.items, true)"
                    >
                      All
                    </button>
                    <button
                      type="button"
                      class="v-btn v-btn-sm"
                      @click="toggleGroupOnHome(group.items, false)"
                    >
                      None
                    </button>
                  </div>
                </div>

                <div class="picker-grid">
                  <label
                    v-for="p in group.items"
                    :key="p.manifest.id"
                    class="picker-item"
                    :class="{ checked: isHomeSelected(p.manifest.id) }"
                  >
                    <input
                      type="checkbox"
                      :checked="isHomeSelected(p.manifest.id)"
                      @change="toggleHomePlugin(p.manifest.id)"
                    />
                    <span class="picker-item-name">{{ p.manifest.name }}</span>
                  </label>
                </div>
              </div>
            </div>

            <div class="picker-foot">
              <span class="v-muted">{{ activeHomePluginIds.length }} plugin(s) selected for Home</span>
              <button type="button" class="v-btn v-btn-primary" @click="applyHomeSources">
                Apply & Refresh Home
              </button>
            </div>
          </div>
        </div>
      </Transition>
    </Teleport>
  </section>
</template>

<style scoped>
.nuvio-home {
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
}

.nuvio-hero {
  position: relative;
  min-height: clamp(540px, 64vh, 690px);
  display: flex;
  flex-direction: column;
  justify-content: flex-end;
  padding: 3rem 3rem 2.35rem;
  gap: 1.25rem;
  overflow: hidden;
  flex-shrink: 0;
}

.nuvio-hero.is-compact {
  min-height: 335px;
  padding: 1.9rem 2.75rem 1.45rem;
}

.hero-stage {
  display: flex;
  flex-direction: column;
  justify-content: flex-end;
  min-height: 290px;
}

.hero-art {
  position: absolute;
  inset: 0;
  width: 100%;
  height: 100%;
  object-fit: cover;
  object-position: center 22%;
  opacity: 0.92;
  filter: contrast(1.06) saturate(1.14);
  pointer-events: none;
  transform: scale(1.03);
}

.hero-vignette {
  position: absolute;
  inset: 0;
  background:
    linear-gradient(
      90deg,
      rgba(10, 11, 16, 0.88) 0%,
      rgba(10, 11, 16, 0.56) 36%,
      rgba(10, 11, 16, 0.14) 68%,
      transparent 100%
    ),
    linear-gradient(
      180deg,
      rgba(10, 11, 16, 0.12) 0%,
      rgba(10, 11, 16, 0.16) 54%,
      var(--vessel-bg-base) 100%
    );
  pointer-events: none;
}

.hero-content {
  position: relative;
  z-index: 2;
  display: flex;
  flex-direction: column;
  gap: 0.95rem;
  max-width: 760px;
  text-shadow: 0 2px 20px rgba(0, 0, 0, 0.85);
}

.hero-meta-line {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 0.7rem;
  font-size: 0.85rem;
  font-weight: 600;
  letter-spacing: 0.02em;
  font-variant-numeric: tabular-nums;
}

.type-pill {
  display: inline-flex;
  align-items: center;
  gap: 0.35rem;
  padding: 0.18rem 0.6rem;
  border-radius: 999px;
  background: rgba(203, 166, 247, 0.2);
  border: 1px solid rgba(203, 166, 247, 0.38);
  color: #f4ecff;
  font-size: 0.72rem;
  letter-spacing: 0.08em;
}

.source-tag {
  padding: 0.14rem 0.5rem;
  border-radius: 6px;
  background: rgba(255, 255, 255, 0.08);
  color: var(--vessel-text-muted);
  font-size: 0.76rem;
}

.hero-rating {
  display: inline-flex;
  align-items: center;
  gap: 0.28rem;
  color: var(--vessel-warning);
}

.hero-title {
  margin: 0;
  font-size: clamp(2.35rem, 3.8vw, 3.45rem);
  font-weight: 850;
  letter-spacing: -0.035em;
  line-height: 1.05;
  text-wrap: balance;
}

.hero-sub {
  margin: 0;
  color: rgba(244, 246, 252, 0.8);
  font-size: 0.98rem;
  max-width: 54ch;
}

.hero-actions {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 0.7rem;
  margin-top: 0.35rem;
}

.hero-indicators {
  position: relative;
  z-index: 2;
  display: flex;
  align-items: center;
  gap: 0.45rem;
}

.indicator-bar {
  width: 18px;
  height: 4px;
  padding: 0;
  border: none;
  border-radius: 999px;
  background: rgba(255, 255, 255, 0.25);
  cursor: pointer;
  transition: width 0.24s cubic-bezier(0.22, 1, 0.36, 1), background-color 0.22s ease;
}

.indicator-bar.is-active {
  width: 36px;
  background: linear-gradient(90deg, #ffffff, var(--vessel-accent));
}

.hero-swap-enter-active,
.hero-swap-leave-active {
  transition: opacity 0.28s ease, transform 0.32s cubic-bezier(0.22, 1, 0.36, 1);
}

.hero-swap-enter-from {
  opacity: 0;
  transform: scale(0.99);
}

.hero-swap-leave-to {
  opacity: 0;
}

.home-rails-body {
  padding: 0 2.75rem 2.75rem;
  display: flex;
  flex-direction: column;
  gap: 2.1rem;
}

.shelf-stack,
.plugin-block {
  display: flex;
  flex-direction: column;
  gap: 2rem;
}

/* Keep rails always painted (no content-visibility culling flicker in WKWebView) */
.nuvio-rail {
  display: flex;
  flex-direction: column;
  gap: 0.85rem;
}

.rail-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.75rem;
}

.rail-title-wrap {
  display: flex;
  align-items: baseline;
  gap: 0.75rem;
}

.rail-title {
  margin: 0;
  font-size: 1.25rem;
  font-weight: 750;
  letter-spacing: -0.02em;
}

.rail-source {
  font-size: 0.78rem;
  text-transform: uppercase;
  letter-spacing: 0.06em;
}

.rail-arrows {
  display: flex;
  gap: 0.4rem;
}

.rail-track {
  display: flex;
  gap: var(--vessel-gap);
  overflow-x: auto;
  overflow-y: visible;
  overscroll-behavior-x: contain;
  -webkit-overflow-scrolling: touch;
  padding: 0.65rem 0.35rem 1rem;
}

.rail-card-slot,
.rail-skeleton {
  flex: 0 0 186px;
}

.rail-skeleton {
  aspect-ratio: 2 / 3;
}

/* Continue Watching Landscape Cards */
.cw-card {
  flex: 0 0 280px;
  display: flex;
  flex-direction: column;
  gap: 0.55rem;
  cursor: pointer;
  transition: transform 0.22s cubic-bezier(0.22, 1, 0.36, 1), opacity 0.2s ease;
}

.cw-card:hover {
  transform: translate3d(0, -4px, 0);
}

.cw-card.is-loading {
  opacity: 0.65;
  pointer-events: none;
}

.cw-thumb {
  position: relative;
  aspect-ratio: 16 / 9;
  border-radius: var(--vessel-radius);
  overflow: hidden;
  background: linear-gradient(135deg, rgba(167, 139, 250, 0.2), rgba(20, 22, 34, 0.95));
  border: 1px solid rgba(255, 255, 255, 0.09);
}

.cw-card:hover .cw-thumb {
  border-color: var(--vessel-accent);
}

.cw-img {
  width: 100%;
  height: 100%;
  object-fit: cover;
  display: block;
}

.cw-shade {
  position: absolute;
  inset: 0;
  background: linear-gradient(180deg, rgba(0, 0, 0, 0.08) 35%, rgba(0, 0, 0, 0.85) 100%);
}

.cw-play-badge {
  position: absolute;
  top: 50%;
  left: 50%;
  transform: translate(-50%, -50%) scale(0.92);
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 46px;
  height: 46px;
  border-radius: 999px;
  background: rgba(12, 13, 22, 0.82);
  border: 1px solid rgba(255, 255, 255, 0.24);
  color: #fff;
  transition: transform 0.2s cubic-bezier(0.34, 1.56, 0.64, 1), background-color 0.18s ease;
}

.cw-card:hover .cw-play-badge {
  transform: translate(-50%, -50%) scale(1.06);
  background: linear-gradient(135deg, #fff, var(--vessel-accent));
  color: #0b0b12;
  border-color: transparent;
}

.cw-dismiss {
  position: absolute;
  top: 0.5rem;
  right: 0.5rem;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 26px;
  height: 26px;
  border-radius: 999px;
  border: 1px solid rgba(255, 255, 255, 0.2);
  background: rgba(10, 10, 18, 0.84);
  color: #fff;
  opacity: 0;
  cursor: pointer;
  transition: opacity 0.16s ease, background-color 0.16s ease;
}

.cw-card:hover .cw-dismiss {
  opacity: 1;
}

.cw-dismiss:hover {
  background: rgba(239, 68, 68, 0.9);
}

.cw-remaining {
  position: absolute;
  left: 0.65rem;
  bottom: 0.55rem;
  font-size: 0.74rem;
  font-weight: 600;
  color: rgba(255, 255, 255, 0.92);
}

.cw-progress-track {
  position: absolute;
  inset: auto 0 0 0;
  height: 4px;
  background: rgba(255, 255, 255, 0.22);
}

.cw-progress-fill {
  height: 100%;
  background: linear-gradient(90deg, var(--vessel-accent-secondary), var(--vessel-accent));
}

.cw-meta {
  display: flex;
  flex-direction: column;
  gap: 0.15rem;
  padding: 0 0.15rem;
}

.cw-title {
  font-size: 0.92rem;
  font-weight: 700;
  color: var(--vessel-text-primary);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.cw-sub {
  font-size: 0.78rem;
  color: var(--vessel-text-muted);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.empty-box {
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  gap: 0.85rem;
}

.empty-text {
  margin: 0;
}

.error-shelf {
  border-color: var(--vessel-danger);
}

/* Home Feed Source Picker Modal */
.home-modal-backdrop {
  position: fixed;
  inset: 0;
  z-index: 95;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 1.5rem;
  background: rgba(6, 7, 12, 0.76);
  backdrop-filter: blur(12px);
  -webkit-backdrop-filter: blur(12px);
}

.home-picker-modal {
  width: min(680px, 100%);
  max-height: 82vh;
  display: flex;
  flex-direction: column;
  gap: 1rem;
  border-color: rgba(203, 166, 247, 0.3);
}

.picker-head,
.picker-foot {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 1rem;
}

.picker-title {
  margin: 0;
  font-size: 1.12rem;
}

.picker-sub {
  margin: 0.2rem 0 0;
  font-size: 0.84rem;
}

.picker-groups {
  display: flex;
  flex-direction: column;
  gap: 1rem;
  overflow-y: auto;
  padding-right: 0.25rem;
}

.picker-group {
  display: flex;
  flex-direction: column;
  gap: 0.55rem;
}

.order-section,
.hero-source-section {
  display: flex;
  flex-direction: column;
  gap: 0.55rem;
  padding: 0.85rem;
  border-radius: 12px;
  background: rgba(203, 166, 247, 0.06);
  border: 1px solid rgba(203, 166, 247, 0.22);
}

.picker-hero-sub {
  margin: 0;
  font-size: 0.78rem;
}

.hero-chip-grid {
  display: flex;
  flex-wrap: wrap;
  gap: 0.45rem;
}

.hero-source-chip {
  display: inline-flex;
  align-items: center;
  gap: 0.35rem;
  padding: 0.35rem 0.7rem;
  border-radius: 999px;
  border: 1px solid rgba(255, 255, 255, 0.12);
  background: rgba(10, 11, 18, 0.65);
  color: var(--vessel-text-muted);
  font-size: 0.8rem;
  font-weight: 600;
  cursor: pointer;
  transition: border-color 0.15s ease, background-color 0.15s ease, color 0.15s ease;
}

.hero-source-chip.checked {
  border-color: var(--vessel-accent);
  background: rgba(203, 166, 247, 0.2);
  color: #ffffff;
}

.group-hint {
  font-size: 0.78rem;
}

.order-list {
  display: flex;
  flex-direction: column;
  gap: 0.4rem;
}

.order-row {
  display: flex;
  align-items: center;
  gap: 0.55rem;
  padding: 0.42rem 0.65rem;
  border-radius: 9px;
  background: rgba(10, 11, 18, 0.65);
  border: 1px solid rgba(255, 255, 255, 0.06);
  user-select: none;
  transition: border-color 0.15s ease, background-color 0.15s ease, transform 0.15s ease;
}

.order-row.is-dragging {
  border-color: var(--vessel-accent);
  background: rgba(203, 166, 247, 0.16);
  transform: scale(1.015);
  box-shadow: 0 8px 24px rgba(0, 0, 0, 0.45);
}

.drag-handle {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 26px;
  height: 26px;
  padding: 0;
  border: none;
  border-radius: 6px;
  background: transparent;
  color: var(--vessel-text-muted);
  cursor: grab;
  touch-action: none;
}

.drag-handle:hover {
  color: var(--vessel-text-primary);
  background: rgba(255, 255, 255, 0.08);
}

.drag-handle:active {
  cursor: grabbing;
}

.order-index {
  width: 22px;
  height: 22px;
  border-radius: 6px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  font-size: 0.76rem;
  font-weight: 700;
  background: rgba(203, 166, 247, 0.18);
  color: var(--vessel-accent);
  font-variant-numeric: tabular-nums;
}

.order-name {
  flex: 1;
  font-size: 0.88rem;
  font-weight: 600;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.order-btns {
  display: flex;
  gap: 0.3rem;
}

.picker-group-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.5rem;
}

.picker-group-btns {
  display: flex;
  gap: 0.4rem;
}

.picker-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(180px, 1fr));
  gap: 0.5rem;
}

.picker-item {
  display: flex;
  align-items: center;
  gap: 0.55rem;
  padding: 0.55rem 0.75rem;
  border-radius: 10px;
  border: 1px solid rgba(255, 255, 255, 0.08);
  background: rgba(10, 11, 18, 0.55);
  cursor: pointer;
  font-size: 0.86rem;
  font-weight: 500;
  transition: border-color 0.16s ease, background-color 0.16s ease;
}

.picker-item.checked {
  border-color: var(--vessel-accent);
  background: rgba(203, 166, 247, 0.14);
}

.picker-item input {
  accent-color: var(--vessel-accent);
}

.picker-item-name {
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.spin {
  animation: v-spin 0.9s linear infinite;
}

@keyframes v-spin {
  to {
    transform: rotate(360deg);
  }
}
</style>
