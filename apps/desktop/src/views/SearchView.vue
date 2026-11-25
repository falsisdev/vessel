<script setup lang="ts">
import { computed, onActivated, onMounted, ref, shallowRef } from 'vue'
import { useRouter } from 'vue-router'
import { History, Loader2, Search as SearchIcon, Trash2, X } from 'lucide-vue-next'
import { isTauriRuntime, vesselApi } from '@/api/vessel'
import MediaCard from '@/components/media/MediaCard.vue'
import { useI18n } from '@/composables/useI18n'
import { useLibraryState } from '@/composables/useLibraryState'
import { usePlayerSession } from '@/composables/usePlayerSession'
import { useUiPreferences } from '@/composables/useUiPreferences'
import { pushToast, useVesselEvents } from '@/composables/useVesselEvents'
import { isLiveMediaSummary, isLiveTvPlugin } from '@/utils/liveTv'
import type { MediaSummary, PluginListEntry } from '@/types/vessel'

defineOptions({ name: 'SearchView' })

const router = useRouter()
const { t } = useI18n()
const { uiPreferences, savePreferences } = useUiPreferences()
const { refreshLibraryState } = useLibraryState()
const { setSession } = usePlayerSession()
const { isSearching, searchBuckets, startSearch } = useVesselEvents()

const MAX_SEARCH_HISTORY = 10
const query = shallowRef('')
const mediaTypeFilter = shallowRef('')
const selectedPluginId = shallowRef('')
const plugins = ref<PluginListEntry[]>([])
const launchingLiveKey = shallowRef('')

const searchHistory = computed(() => (uiPreferences.value.searchHistory ?? []).slice(0, MAX_SEARCH_HISTORY))

function recordSearchHistory(term: string): void {
  const clean = term.trim()
  if (!clean) return
  const prev = uiPreferences.value.searchHistory ?? []
  const next = [clean, ...prev.filter((item) => item.toLowerCase() !== clean.toLowerCase())].slice(
    0,
    MAX_SEARCH_HISTORY,
  )
  uiPreferences.value = { ...uiPreferences.value, searchHistory: next }
  void savePreferences(uiPreferences.value)
}

function removeHistoryItem(term: string): void {
  const next = (uiPreferences.value.searchHistory ?? []).filter((item) => item !== term)
  uiPreferences.value = { ...uiPreferences.value, searchHistory: next }
  void savePreferences(uiPreferences.value)
}

function clearSearchHistory(): void {
  uiPreferences.value = { ...uiPreferences.value, searchHistory: [] }
  void savePreferences(uiPreferences.value)
}

function applyHistorySearch(term: string): void {
  query.value = term
  void runSearch()
}

const enabledPlugins = computed(() => plugins.value.filter((p) => p.enabled))
const pluginMap = computed(() => {
  const m = new Map<string, PluginListEntry>()
  for (const p of plugins.value) {
    m.set(p.manifest.id, p)
  }
  return m
})

const TYPE_PILLS = [
  { value: '', labelKey: 'search.filter_all' },
  { value: 'movie', labelKey: 'search.filter_movies' },
  { value: 'series', labelKey: 'search.filter_series' },
  { value: 'anime', labelKey: 'search.filter_anime' },
]

const pluginNameMap = computed(() => {
  const m = new Map<string, string>()
  for (const p of plugins.value) {
    m.set(p.manifest.id, p.manifest.name)
  }
  return m
})

const bucketEntries = computed(() => {
  const filter = mediaTypeFilter.value
  const targetPlugin = selectedPluginId.value
  return Object.entries(searchBuckets.value)
    .filter(([pluginId]) => !targetPlugin || pluginId === targetPlugin)
    .map(([pluginId, items]) => {
      const filtered = !filter
        ? items
        : items.filter((item) => {
            if (filter === 'movie') return item.type === 'movie'
            if (filter === 'anime') return item.isAnime === true
            if (filter === 'series') return item.type === 'series' && !item.isAnime
            return true
          })
      return [pluginId, filtered] as const
    })
    .filter(([, items]) => items.length > 0)
})

async function refreshPlugins(): Promise<void> {
  if (!isTauriRuntime()) return
  try {
    plugins.value = await vesselApi.listPlugins()
    if (
      selectedPluginId.value &&
      !plugins.value.some((p) => p.enabled && p.manifest.id === selectedPluginId.value)
    ) {
      selectedPluginId.value = ''
    }
  } catch {
    // Non-fatal
  }
}

function onPluginFilterChange(): void {
  if (query.value.trim()) {
    void runSearch()
  }
}

async function runSearch(): Promise<void> {
  const trimmed = query.value.trim()
  if (!trimmed) return
  recordSearchHistory(trimmed)
  try {
    const filters = mediaTypeFilter.value ? { mediaType: mediaTypeFilter.value } : undefined
    const targetPlugins = selectedPluginId.value ? [selectedPluginId.value] : undefined
    await startSearch(trimmed, filters, targetPlugins)
  } catch (err) {
    pushToast('error', String(err))
  }
}

onMounted(() => {
  void refreshPlugins()
  void refreshLibraryState()
})

onActivated(() => {
  void refreshPlugins()
  void refreshLibraryState()
})

async function stopSearch(): Promise<void> {
  try {
    await vesselApi.cancelSearch()
    isSearching.value = false
  } catch (err) {
    pushToast('error', String(err))
  }
}

async function handleCardOpen(pluginId: string, nativeId: string): Promise<void> {
  const items = searchBuckets.value[pluginId] ?? []
  const matched = items.find((it) => it.nativeId === nativeId)
  const plug = pluginMap.value.get(pluginId)
  const isLive = (plug ? isLiveTvPlugin(plug) : false) || (matched ? isLiveMediaSummary(matched, plug) : false)
  if (!isLive) {
    void router.push({ name: 'details', params: { pluginId, nativeId } })
    return
  }
  const key = `${pluginId}:${nativeId}`
  if (launchingLiveKey.value) return
  launchingLiveKey.value = key
  try {
    const detail = await vesselApi.getDetailsWithEnrichment(pluginId, nativeId, 'en')
    const ep = detail.episodes[0] ?? {
      streamId: nativeId,
      season: 1,
      episode: 1,
      title: matched?.title ?? detail.title,
      thumbnail: matched?.poster ?? detail.poster,
      tags: [],
    }
    const streamRes = await vesselApi.getStreams(pluginId, ep.streamId)
    if (!streamRes.streams.length) {
      pushToast('warn', t('details.no_streams'))
      return
    }
    setSession({
      pluginId,
      nativeId,
      detail: {
        ...detail,
        type: 'live',
        title: detail.title || matched?.title || nativeId,
        poster: detail.poster ?? matched?.poster,
        backdrop: detail.backdrop ?? matched?.poster,
      },
      episode: ep,
      streams: streamRes.streams,
      subtitles: streamRes.subtitles ?? [],
      activeStreamIndex: 0,
    })
    await router.push({
      name: 'player',
      params: { pluginId, nativeId },
      query: { streamId: ep.streamId },
    })
  } catch (err) {
    pushToast('error', String(err))
  } finally {
    launchingLiveKey.value = ''
  }
}

function cardItemFor(pluginId: string, item: MediaSummary): MediaSummary {
  const plug = pluginMap.value.get(pluginId)
  const forceLive = plug ? isLiveTvPlugin(plug) : false
  const nextType = forceLive ? 'live' : item.type
  if (item.pluginId === pluginId && item.type === nextType) {
    return item
  }
  return {
    ...item,
    pluginId: item.pluginId ?? pluginId,
    type: nextType,
  }
}
</script>

<template>
  <section class="search-view v-page-container">
    <header class="v-page-header">
      <h1 class="v-page-title">{{ t('search.title') }}</h1>
      <span v-if="isSearching" class="v-badge v-badge-accent searching-pill">
        <Loader2 :size="13" class="spin" aria-hidden="true" />
        <span>{{ t('search.searching') }}</span>
      </span>
    </header>

    <form class="search-bar" @submit.prevent="runSearch">
      <input
        v-model="query"
        type="search"
        name="searchQuery"
        autocomplete="off"
        aria-label="Search media"
        class="v-input"
        :placeholder="t('search.placeholder')"
      />
      <select
        v-if="enabledPlugins.length"
        v-model="selectedPluginId"
        aria-label="Filter by plugin"
        class="v-select search-plugin-select"
        @change="onPluginFilterChange"
      >
        <option value="">{{ t('search.all_plugins') }}</option>
        <option v-for="p in enabledPlugins" :key="p.manifest.id" :value="p.manifest.id">
          {{ p.manifest.name }}
        </option>
      </select>
      <button type="submit" class="v-btn v-btn-primary" :disabled="isSearching">
        <Loader2 v-if="isSearching" :size="15" class="spin" aria-hidden="true" />
        <SearchIcon v-else :size="15" aria-hidden="true" />
        <span>{{ isSearching ? t('search.searching') : t('common.search') }}</span>
      </button>
      <button v-if="isSearching" type="button" class="v-btn" @click="stopSearch">
        <X :size="15" aria-hidden="true" /> {{ t('common.cancel') }}
      </button>
    </form>

    <div v-if="isSearching" class="search-progress-bar" aria-hidden="true" />

    <div class="type-pills" role="group" aria-label="Filter by media type">
      <button
        v-for="pill in TYPE_PILLS"
        :key="pill.value"
        type="button"
        class="v-btn v-btn-sm"
        :class="{ 'v-btn-primary': mediaTypeFilter === pill.value }"
        @click="mediaTypeFilter = pill.value"
      >
        {{ t(pill.labelKey) }}
      </button>
    </div>

    <div v-if="searchHistory.length && !bucketEntries.length && !isSearching" class="v-card history-card">
      <div class="history-head">
        <span class="history-title">
          <History :size="15" aria-hidden="true" />
          {{ t('search.recent_searches') }}
        </span>
        <button type="button" class="v-btn v-btn-sm" @click="clearSearchHistory">
          <Trash2 :size="13" aria-hidden="true" /> {{ t('search.clear_history') }}
        </button>
      </div>
      <div class="history-chips">
        <div v-for="term in searchHistory" :key="term" class="history-chip">
          <button type="button" class="history-chip-trigger" @click="applyHistorySearch(term)">
            {{ term }}
          </button>
          <button
            type="button"
            class="history-chip-remove"
            :aria-label="t('common.remove')"
            @click="removeHistoryItem(term)"
          >
            <X :size="12" aria-hidden="true" />
          </button>
        </div>
      </div>
    </div>

    <div v-if="isSearching && !bucketEntries.length" class="v-grid">
      <div v-for="n in 8" :key="n" class="search-skeleton-card">
        <div class="v-skeleton search-skeleton" />
        <div class="v-skeleton search-skeleton-line" />
        <div class="v-skeleton search-skeleton-line short" />
      </div>
    </div>

    <div v-else-if="!bucketEntries.length" class="v-card v-muted">
      {{ t('search.empty') }}
    </div>

    <div v-for="[pluginId, items] in bucketEntries" :key="pluginId" class="bucket">
      <div class="bucket-head">
        <h2 class="bucket-title">{{ pluginNameMap.get(pluginId) ?? pluginId }}</h2>
        <span class="v-badge">{{ items.length }}</span>
      </div>
      <div class="v-grid">
        <MediaCard
          v-for="item in items"
          :key="`${pluginId}:${item.nativeId}`"
          :item="cardItemFor(pluginId, item)"
          :cards="uiPreferences.cards"
          @open="handleCardOpen"
        />
      </div>
    </div>
  </section>
</template>

<style scoped>
.search-view,
.bucket {
  display: flex;
  flex-direction: column;
  gap: var(--vessel-gap);
}

.bucket {
  content-visibility: auto;
  contain-intrinsic-size: auto 360px;
  contain: layout style;
}

.search-bar {
  display: flex;
  gap: 0.65rem;
  flex-wrap: wrap;
}

.search-bar .v-input {
  flex: 1;
  min-width: 240px;
}

.search-plugin-select {
  width: auto;
  min-width: 175px;
  max-width: 240px;
}

.type-pills {
  display: flex;
  gap: 0.45rem;
  flex-wrap: wrap;
}

.bucket-head {
  display: flex;
  align-items: center;
  gap: 0.55rem;
}

.search-skeleton-card {
  display: flex;
  flex-direction: column;
  gap: 0.45rem;
}

.search-skeleton {
  aspect-ratio: 2 / 3;
  min-height: 240px;
}

.search-skeleton-line {
  width: 80%;
  height: 13px;
  border-radius: 6px;
}

.search-skeleton-line.short {
  width: 48%;
  height: 11px;
}

.searching-pill {
  display: inline-flex;
  align-items: center;
  gap: 0.4rem;
}

.search-progress-bar {
  height: 2.5px;
  border-radius: 999px;
  background: linear-gradient(
    90deg,
    transparent 0%,
    var(--vessel-accent) 45%,
    var(--vessel-accent-secondary) 75%,
    transparent 100%
  );
  background-size: 200% 100%;
  animation: searchProgressSlide 1.05s linear infinite;
}

.spin {
  animation: searchSpin 0.85s linear infinite;
}

@keyframes searchSpin {
  to {
    transform: rotate(360deg);
  }
}

@keyframes searchProgressSlide {
  0% {
    background-position: 120% 0;
  }
  100% {
    background-position: -80% 0;
  }
}

.bucket-title {
  margin: 0;
  font-size: 1.1rem;
}

.history-card {
  display: flex;
  flex-direction: column;
  gap: 0.7rem;
}

.history-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.75rem;
}

.history-title {
  display: inline-flex;
  align-items: center;
  gap: 0.45rem;
  font-weight: 600;
  font-size: 0.92rem;
}

.history-chips {
  display: flex;
  flex-wrap: wrap;
  gap: 0.45rem;
}

.history-chip {
  display: inline-flex;
  align-items: center;
  border: 1px solid var(--vessel-border);
  background: var(--vessel-bg-elevated, var(--vessel-surface-input));
  border-radius: 999px;
  overflow: hidden;
}

.history-chip-trigger {
  border: 0;
  background: transparent;
  color: var(--vessel-text);
  padding: 0.32rem 0.55rem 0.32rem 0.75rem;
  font-size: 0.85rem;
  cursor: pointer;
}

.history-chip-trigger:hover {
  color: var(--vessel-accent);
}

.history-chip-remove {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  border: 0;
  background: transparent;
  color: var(--vessel-text-muted);
  padding: 0.32rem 0.55rem 0.32rem 0.25rem;
  cursor: pointer;
}

.history-chip-remove:hover {
  color: var(--vessel-danger);
}
</style>
