<script setup lang="ts">
import { computed, onActivated, onDeactivated, onMounted, onUnmounted, ref, shallowRef, watch } from 'vue'
import { useRouter } from 'vue-router'
import {
  CheckCircle2,
  ChevronLeft,
  ChevronRight,
  GripVertical,
  Info,
  Layers,
  Pencil,
  Play,
  RefreshCw,
  SlidersHorizontal,
  Sparkles,
  Star,
  Trash2,
  X,
} from 'lucide-vue-next'
import { isTauriRuntime, vesselApi } from '@/api/vessel'
import MediaCard from '@/components/media/MediaCard.vue'
import { useHomeCache } from '@/composables/useHomeCache'
import { useI18n } from '@/composables/useI18n'
import { useLibraryState } from '@/composables/useLibraryState'
import { usePlayerSession } from '@/composables/usePlayerSession'
import { useUiPreferences } from '@/composables/useUiPreferences'
import { pushToast } from '@/composables/useVesselEvents'
import type { ContinueWatchingEntry, Episode, MediaSummary, PlaybackProgress, PluginListEntry } from '@/types/vessel'

const router = useRouter()
const { t, currentLang } = useI18n()
const { refreshLibraryState } = useLibraryState()
const { setSession } = usePlayerSession()
const { uiPreferences, preferencesLoaded, loadPreferences, savePreferences } = useUiPreferences()
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

const editingCwEntry = ref<ContinueWatchingEntry | null>(null)
const cwEditEpisodes = ref<Episode[]>([])
const cwEditProgressRows = ref<PlaybackProgress[]>([])
const cwEditLoadingEps = shallowRef(false)
const cwEditSaving = shallowRef(false)
const cwEditSeason = shallowRef(1)
const cwEditEpisode = shallowRef(1)
const cwEditStreamId = shallowRef('')
const cwEditEpTitle = shallowRef('')
const cwEditWatchedMin = shallowRef(0)
const cwEditDurationMs = shallowRef(24 * 60_000)
const cwEditDurationMin = computed(() => Math.max(1, Math.round(cwEditDurationMs.value / 60_000)))
const cwEditPercent = shallowRef(0)

const cwContextMenu = ref<{ entry: ContinueWatchingEntry; x: number; y: number } | null>(null)
let cwLongPressTimer: number | null = null
let cwLongPressFired = false
let cwPressStartX = 0
let cwPressStartY = 0

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
  // Default when user hasn't picked explicitly yet: smart curated starter set <= 5 VOD/anime plugins
  const isPureLivePlugin = (p: PluginListEntry) =>
    p.manifest.capabilities.includes('live_tv') && !p.manifest.capabilities.includes('vod')
  const vodEnabled = enabledPlugins.value.filter((p) => !isPureLivePlugin(p))
  const vodSet = new Set(vodEnabled.map((p) => p.manifest.id))
  const preferred = ['anizium', 'diziwatch', 'yabancidizi', 'animecix', 'seicode']
  const picked: string[] = []
  for (const id of preferred) {
    if (vodSet.has(id) && picked.length < 5) picked.push(id)
  }
  for (const p of vodEnabled) {
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

watch(
  spotlightPool,
  (pool) => {
    for (const item of pool) {
      if (item.poster) {
        const pre = new Image()
        pre.decoding = 'async'
        pre.src = item.poster
      }
    }
  },
  { immediate: true },
)

function stepHero(delta: -1 | 1): void {
  const len = spotlightPool.value.length
  if (len <= 1) return
  heroIndex.value = (heroIndex.value + delta + len) % len
  startHeroTimer()
}

async function loadHome(forceRefresh = false): Promise<void> {
  if (!isTauriRuntime()) return
  if (!preferencesLoaded.value) {
    await loadPreferences()
  }
  try {
    allPlugins.value = await vesselApi.listPlugins()
  } catch {
    // Non-fatal
  }

  const targetIds = Array.from(new Set([...activeHomePluginIds.value, ...activeHeroPluginIds.value]))
  const targetKey = `${activeHomePluginIds.value.slice().sort().join(',')}|${activeHeroPluginIds.value.slice().sort().join(',')}`

  // Always silently refresh Continue Watching & library status so Home return feels instant and accurate
  void refreshLibraryState()
  const cwPromise = vesselApi
    .listContinueWatching(uiPreferences.value.home.continueWatchingLimit || 12)
    .then((cw) => {
      continueWatching.value = cw
    })
    .catch((err) => {
      console.warn('Continue Watching failed:', err)
    })

  if (!forceRefresh && hasLoadedOnce.value && cachedFilterKey.value === targetKey) {
    await cwPromise
    return
  }

  loading.value = true
  if (forceRefresh) {
    shelves.value = []
  }
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
  if (!entry.positionMs || entry.positionMs <= 0) return 0
  if (!entry.durationMs || entry.durationMs <= 0) return 15
  return Math.min(100, Math.max(4, Math.round((entry.positionMs / entry.durationMs) * 100)))
}

function formatRemaining(entry: ContinueWatchingEntry): string {
  if (!entry.positionMs || entry.positionMs <= 0) return t('home.up_next')
  if (!entry.durationMs || entry.durationMs <= entry.positionMs) return t('details.resume')
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
      vesselApi.getDetailsWithEnrichment(entry.media.pluginId, entry.media.nativeId, currentLang.value),
      vesselApi.getStreams(entry.media.pluginId, entry.streamId),
    ])
    const ep =
      detail.episodes.find((item) => item.streamId === entry.streamId) ??
      detail.episodes.find((item) => item.season === entry.season && item.episode === entry.episode) ??
      detail.episodes[0]

    const playableIdx = streamRes.streams.findIndex((s) => !s.needsExtractor && s.kind !== 'embed')
    if (!ep || playableIdx < 0) {
      if (streamRes.streams.length > 0 && playableIdx < 0) {
        pushToast('info', t('details.select_stream'))
      }
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
      skipSegments: streamRes.skipSegments,
      activeStreamIndex: playableIdx,
      startPositionMs: entry.positionMs > 2000 ? entry.positionMs : undefined,
    })

    await router.push({
      name: 'player',
      params: {
        pluginId: entry.media.pluginId,
        nativeId: entry.media.nativeId,
      },
      query: {
        streamId: ep.streamId,
        streamIndex: String(playableIdx),
      },
    })
  } catch (err) {
    pushToast('warn', String(err))
    openDetails(entry.media.pluginId, entry.media.nativeId)
  } finally {
    resumingKey.value = null
  }
}

function applyEpisodeTimingFromProgress(season: number, episode: number, streamId: string): void {
  const entry = editingCwEntry.value
  if (!entry) return
  const matched =
    cwEditProgressRows.value.find((p) => streamId && p.streamId === streamId) ??
    cwEditProgressRows.value.find((p) => p.season === season && p.episode === episode)

  const fallbackDurMs =
    cwEditProgressRows.value.find((p) => p.durationMs > 60_000)?.durationMs ??
    (entry.durationMs > 60_000 ? entry.durationMs : entry.mediaType === 'movie' ? 110 * 60_000 : 24 * 60_000)

  const durMs = matched && matched.durationMs > 60_000 ? matched.durationMs : fallbackDurMs
  const posMs = matched ? matched.positionMs : streamId === entry.streamId ? entry.positionMs : 0
  cwEditDurationMs.value = durMs
  const maxMin = Math.max(1, Math.round(durMs / 60_000))
  cwEditWatchedMin.value = Math.min(maxMin, Math.max(0, Math.round(posMs / 60_000)))
  cwEditPercent.value =
    durMs > 0 ? Math.min(99, Math.max(0, Math.round((posMs / durMs) * 100))) : 0
}

async function openCwEditor(entry: ContinueWatchingEntry): Promise<void> {
  editingCwEntry.value = entry
  cwEditEpisodes.value = []
  cwEditProgressRows.value = []
  cwEditSeason.value = entry.season || 1
  cwEditEpisode.value = entry.episode || 1
  cwEditStreamId.value = entry.streamId
  cwEditEpTitle.value = entry.episodeTitle ?? ''
  applyEpisodeTimingFromProgress(entry.season, entry.episode, entry.streamId)

  cwEditLoadingEps.value = true
  try {
    const [detail, progRows] = await Promise.all([
      entry.mediaType !== 'movie'
        ? vesselApi.getDetailsWithEnrichment(entry.media.pluginId, entry.media.nativeId, currentLang.value)
        : Promise.resolve(null),
      vesselApi.listMediaWatchProgress(entry.media.pluginId, entry.media.nativeId).catch(() => []),
    ])
    if (editingCwEntry.value?.media.nativeId === entry.media.nativeId) {
      cwEditProgressRows.value = progRows
      if (detail) {
        cwEditEpisodes.value = detail.episodes
      }
      applyEpisodeTimingFromProgress(cwEditSeason.value, cwEditEpisode.value, cwEditStreamId.value)
    }
  } catch {
    // Allow manual editing if plugin details fail
  } finally {
    cwEditLoadingEps.value = false
  }
}

function onCwEpisodeSelect(streamId: string): void {
  const picked = cwEditEpisodes.value.find((e) => e.streamId === streamId)
  if (!picked) return
  cwEditStreamId.value = picked.streamId
  cwEditSeason.value = picked.season
  cwEditEpisode.value = picked.episode
  cwEditEpTitle.value = picked.title ?? ''
  applyEpisodeTimingFromProgress(picked.season, picked.episode, picked.streamId)
}

function onCwManualCoordChange(): void {
  const matchedEp = cwEditEpisodes.value.find(
    (e) => e.season === Number(cwEditSeason.value) && e.episode === Number(cwEditEpisode.value),
  )
  if (matchedEp) {
    cwEditStreamId.value = matchedEp.streamId
    cwEditEpTitle.value = matchedEp.title ?? ''
  }
  applyEpisodeTimingFromProgress(
    Number(cwEditSeason.value) || 1,
    Number(cwEditEpisode.value) || 1,
    cwEditStreamId.value,
  )
}

function onCwPercentInput(pct: number): void {
  cwEditPercent.value = pct
  cwEditWatchedMin.value = Math.round((cwEditDurationMin.value * pct) / 100)
}

function onCwMinutesInput(): void {
  const dur = cwEditDurationMin.value
  const watched = Math.max(0, Math.min(dur, Number(cwEditWatchedMin.value) || 0))
  cwEditWatchedMin.value = watched
  cwEditPercent.value = Math.min(99, Math.round((watched / dur) * 100))
}

async function saveCwEdit(markCompleted = false): Promise<void> {
  const entry = editingCwEntry.value
  if (!entry || cwEditSaving.value) return
  cwEditSaving.value = true
  try {
    const durMs = Math.max(60_000, cwEditDurationMs.value)
    const rawPosMs = markCompleted
      ? durMs
      : Math.min(durMs, Math.max(10_000, Math.round((cwEditDurationMs.value * cwEditPercent.value) / 100)))
    const matchedEp = cwEditEpisodes.value.find(
      (e) =>
        e.streamId === cwEditStreamId.value ||
        (e.season === Number(cwEditSeason.value) && e.episode === Number(cwEditEpisode.value)),
    )
    const season = entry.mediaType === 'movie' ? entry.season : Math.max(0, Number(cwEditSeason.value) || 1)
    const episode = entry.mediaType === 'movie' ? entry.episode : Math.max(0, Number(cwEditEpisode.value) || 1)

    await vesselApi.recordProgress({
      media: entry.media,
      streamId: matchedEp?.streamId ?? cwEditStreamId.value ?? entry.streamId,
      canonicalId: entry.canonicalId,
      season,
      episode,
      absoluteEpisode: matchedEp?.absoluteEpisode ?? entry.absoluteEpisode,
      title: entry.title,
      posterUrl: entry.poster,
      backdropUrl: entry.backdrop,
      episodeTitle: matchedEp?.title ?? (cwEditEpTitle.value || entry.episodeTitle),
      positionMs: rawPosMs,
      durationMs: durMs,
      completed: markCompleted,
      lastWatchedAt: new Date().toISOString(),
    })

    continueWatching.value = await vesselApi.listContinueWatching(
      uiPreferences.value.home.continueWatchingLimit || 12,
    )
    editingCwEntry.value = null
    pushToast('info', `${entry.title} · ${t('common.save')}`)
  } catch (err) {
    pushToast('error', String(err))
  } finally {
    cwEditSaving.value = false
  }
}

async function removeCwFromEditor(): Promise<void> {
  const entry = editingCwEntry.value
  if (!entry) return
  await dismissContinueWatching(entry)
  editingCwEntry.value = null
}

function formatCwEpisodeLine(entry: ContinueWatchingEntry): string {
  const prefix = `S${entry.season} · E${entry.episode}`
  const rawEp = entry.episodeTitle?.trim()
  if (!rawEp) return prefix
  // Strip redundant show-title prefix if the plugin included it in episode title
  const normShow = entry.title.trim().toLowerCase()
  let cleaned = rawEp
  if (normShow && rawEp.toLowerCase().startsWith(normShow)) {
    cleaned = rawEp.slice(entry.title.trim().length).replace(/^[\s:\-–—·]+/, '').trim()
  }
  if (!cleaned || cleaned.toLowerCase() === normShow) return prefix
  const full = `${prefix} · ${cleaned}`
  return full.length > 38 ? `${full.slice(0, 35).trimEnd()}...` : full
}

function openCwContextMenuAt(entry: ContinueWatchingEntry, clientX: number, clientY: number): void {
  const menuW = 210
  const menuH = 178
  const x = Math.max(8, Math.min(clientX, window.innerWidth - menuW - 10))
  const y = Math.max(8, Math.min(clientY, window.innerHeight - menuH - 10))
  cwContextMenu.value = { entry, x, y }
}

function openCwContextMenu(entry: ContinueWatchingEntry, ev: MouseEvent): void {
  openCwContextMenuAt(entry, ev.clientX, ev.clientY)
}

function startCwPointerPress(entry: ContinueWatchingEntry, ev: PointerEvent): void {
  if (ev.button !== 0) return
  cancelCwLongPress()
  cwLongPressFired = false
  cwPressStartX = ev.clientX
  cwPressStartY = ev.clientY
  cwLongPressTimer = window.setTimeout(() => {
    cwLongPressFired = true
    openCwContextMenuAt(entry, ev.clientX, ev.clientY)
  }, 520)
}

function moveCwPointerPress(ev: PointerEvent): void {
  if (cwLongPressTimer === null) return
  if (Math.abs(ev.clientX - cwPressStartX) > 10 || Math.abs(ev.clientY - cwPressStartY) > 10) {
    cancelCwLongPress()
  }
}

function cancelCwLongPress(): void {
  if (cwLongPressTimer !== null) {
    window.clearTimeout(cwLongPressTimer)
    cwLongPressTimer = null
  }
}

function handleCwCardClick(entry: ContinueWatchingEntry): void {
  if (cwLongPressFired) {
    cwLongPressFired = false
    return
  }
  if (cwContextMenu.value) {
    cwContextMenu.value = null
    return
  }
  void resumeContinueWatching(entry)
}

function closeCwContextOnDoc(ev: PointerEvent | KeyboardEvent): void {
  if (ev instanceof KeyboardEvent) {
    if (ev.key === 'Escape') cwContextMenu.value = null
    return
  }
  if (!cwContextMenu.value) return
  const target = ev.target as HTMLElement | null
  if (!target?.closest('.cw-context-menu')) {
    cwContextMenu.value = null
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
  window.addEventListener('pointerdown', closeCwContextOnDoc)
  window.addEventListener('keydown', closeCwContextOnDoc)
})

onActivated(() => {
  void loadHome(false)
  startHeroTimer()
})

onDeactivated(() => {
  cancelCwLongPress()
  cwContextMenu.value = null
  if (heroAutoTimer !== null) {
    window.clearInterval(heroAutoTimer)
    heroAutoTimer = null
  }
})

onUnmounted(() => {
  cancelCwLongPress()
  window.removeEventListener('pointerdown', closeCwContextOnDoc)
  window.removeEventListener('keydown', closeCwContextOnDoc)
  if (heroAutoTimer !== null) window.clearInterval(heroAutoTimer)
})
</script>

<template>
  <section class="nuvio-home">
    <!-- Full-bleed Netflix-style Cinema Spotlight Hero with Overlapping Crossfade -->
    <header
      v-if="uiPreferences.home.heroBanner !== 'hidden'"
      class="nuvio-hero"
      :class="{ 'is-compact': uiPreferences.home.heroBanner === 'compact' }"
    >
      <div class="hero-stage-stack">
        <Transition name="hero-crossfade">
          <div
            v-if="featuredItem"
            :key="`${featuredItem.resolvedPluginId}:${featuredItem.nativeId}`"
            class="hero-stage"
          >
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
                  <Play :size="16" fill="currentColor" aria-hidden="true" /> {{ t('home.watch_now') }}
                </button>
                <button
                  type="button"
                  class="v-btn"
                  @click="openDetails(featuredItem.resolvedPluginId, featuredItem.nativeId)"
                >
                  <Info :size="16" aria-hidden="true" /> {{ t('home.more_info') }}
                </button>
                <button
                  v-if="enabledPlugins.length"
                  type="button"
                  class="v-btn"
                  @click="showSourcePicker = true"
                >
                  <SlidersHorizontal :size="15" aria-hidden="true" />
                  {{ t('home.feeds') }} ({{ activeHomePluginIds.length }}/{{ enabledPlugins.length }})
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

          <div v-else-if="loading" key="loading-hero" class="hero-stage">
            <div class="hero-vignette" aria-hidden="true" />
            <div class="hero-content">
              <div class="hero-meta-line">
                <span class="v-skeleton hero-sk-pill" />
                <span class="v-skeleton hero-sk-tag" />
              </div>
              <div class="v-skeleton hero-sk-title" />
              <div class="v-skeleton hero-sk-sub" />
              <div class="hero-actions">
                <span class="v-skeleton hero-sk-btn" />
                <span class="v-skeleton hero-sk-btn secondary" />
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
                  {{ t('home.choose_feeds') }} ({{ activeHomePluginIds.length }})
                </button>
                <button type="button" class="v-btn" :disabled="loading" @click="loadHome(true)">
                  <RefreshCw :size="15" :class="{ spin: loading }" aria-hidden="true" />
                  {{ t('common.refresh') }}
                </button>
              </div>
            </div>
          </div>
        </Transition>
      </div>

      <div v-if="spotlightPool.length > 1" class="hero-controls-row">
        <div class="hero-indicators">
          <button
            v-for="(item, idx) in spotlightPool"
            :key="`${item.resolvedPluginId}:${item.nativeId}`"
            type="button"
            class="indicator-bar"
            :class="{ 'is-active': idx === heroIndex }"
            :aria-label="`Spotlight ${item.title}`"
            @click="heroIndex = idx; startHeroTimer()"
          />
        </div>
        <div class="hero-step-arrows">
          <button
            type="button"
            class="v-btn v-btn-icon v-btn-sm"
            aria-label="Previous featured item"
            @click="stepHero(-1)"
          >
            <ChevronLeft :size="15" aria-hidden="true" />
          </button>
          <button
            type="button"
            class="v-btn v-btn-icon v-btn-sm"
            aria-label="Next featured item"
            @click="stepHero(1)"
          >
            <ChevronRight :size="15" aria-hidden="true" />
          </button>
        </div>
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
            <h2 class="rail-title">{{ t('home.continue_watching') }}</h2>
            <span class="rail-source v-muted">{{ continueWatching.length }} {{ t('home.active_count') }}</span>
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
            @click="handleCwCardClick(entry)"
            @contextmenu.prevent="openCwContextMenu(entry, $event)"
            @pointerdown="startCwPointerPress(entry, $event)"
            @pointermove="moveCwPointerPress"
            @pointerup="cancelCwLongPress"
            @pointercancel="cancelCwLongPress"
            @pointerleave="cancelCwLongPress"
          >
            <div class="cw-thumb">
              <div
                v-if="entry.backdrop ?? entry.poster"
                class="v-skeleton cw-thumb-skeleton"
                aria-hidden="true"
              />
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
              <div class="cw-top-actions" @click.stop>
                <button
                  type="button"
                  class="cw-action-btn cw-edit"
                  :title="t('home.edit_cw')"
                  :aria-label="t('home.edit_cw')"
                  @click.stop="openCwEditor(entry)"
                >
                  <Pencil :size="13" aria-hidden="true" />
                </button>
                <button
                  type="button"
                  class="cw-action-btn cw-dismiss"
                  title="Remove from Continue Watching"
                  aria-label="Remove from Continue Watching"
                  @click.stop="dismissContinueWatching(entry)"
                >
                  <X :size="14" aria-hidden="true" />
                </button>
              </div>
              <span class="cw-remaining">{{ formatRemaining(entry) }}</span>
              <div v-if="entry.positionMs > 0" class="cw-progress-track">
                <div class="cw-progress-fill" :style="{ width: `${progressPercent(entry)}%` }" />
              </div>
            </div>
            <div class="cw-meta">
              <div class="cw-title" :title="entry.title">{{ entry.title }}</div>
              <div v-if="entry.mediaType !== 'movie'" class="cw-sub" :title="entry.episodeTitle">
                {{ formatCwEpisodeLine(entry) }}
              </div>
              <div
                v-else-if="entry.episodeTitle && entry.episodeTitle !== entry.title"
                class="cw-sub"
                :title="entry.episodeTitle"
              >
                {{ entry.episodeTitle }}
              </div>
            </div>
          </article>
        </div>
      </section>

      <div v-if="loading && !shelves.length" class="shelf-stack">
        <div v-for="rowIdx in 3" :key="rowIdx" class="nuvio-rail">
          <div class="rail-head">
            <div class="v-skeleton rail-title-skeleton" />
          </div>
          <div class="rail-track">
            <div v-for="n in 7" :key="n" class="rail-skeleton-card">
              <div class="v-skeleton rail-skeleton" />
              <div class="v-skeleton rail-skeleton-line" />
              <div class="v-skeleton rail-skeleton-line short" />
            </div>
          </div>
        </div>
      </div>

      <div v-else-if="!shelves.length" class="v-card empty-box">
        <p class="v-muted empty-text">
          {{
            enabledPlugins.length
              ? t('home.empty_selected')
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
            <Layers :size="15" /> {{ t('home.select_sources') }}
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
                <h3 class="picker-title">{{ t('home.modal_title') }}</h3>
                <p class="v-muted picker-sub">
                  {{ t('home.modal_subtitle') }}
                </p>
              </div>
              <button
                type="button"
                class="v-btn v-btn-icon"
                :aria-label="t('common.close')"
                @click="showSourcePicker = false"
              >
                <X :size="16" />
              </button>
            </div>

            <div class="picker-groups">
              <div class="hero-source-section">
                <div class="picker-group-head">
                  <strong>{{ t('home.hero_source_title') }}</strong>
                  <button
                    type="button"
                    class="v-btn v-btn-sm"
                    :disabled="!activeHeroPluginIds.length"
                    @click="clearHeroPlugins"
                  >
                    {{ t('home.hero_source_auto') }}
                  </button>
                </div>
                <p class="v-muted picker-hero-sub">
                  {{ t('home.hero_source_hint') }}
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
                  <strong>{{ t('home.shelf_order_title') }}</strong>
                  <span class="v-muted group-hint">{{ t('home.shelf_order_hint') }}</span>
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

    <!-- Manual Continue Watching Editor Modal -->
    <Teleport to="body">
      <Transition name="modal-pop">
        <div
          v-if="editingCwEntry"
          class="home-modal-backdrop"
          role="dialog"
          aria-modal="true"
          :aria-label="t('home.edit_cw')"
          @click.self="editingCwEntry = null"
        >
          <div class="v-card cw-editor-modal">
            <div class="picker-head">
              <div>
                <h3 class="picker-title">{{ t('home.edit_cw') }}</h3>
                <p class="v-muted picker-sub">{{ editingCwEntry.title }}</p>
              </div>
              <button
                type="button"
                class="v-btn v-btn-icon"
                :aria-label="t('common.close')"
                @click="editingCwEntry = null"
              >
                <X :size="16" />
              </button>
            </div>

            <div class="cw-edit-body">
              <template v-if="editingCwEntry.mediaType !== 'movie'">
                <div v-if="cwEditEpisodes.length" class="cw-field">
                  <label class="cw-label">{{ t('home.cw_episode') }}</label>
                  <select
                    :value="cwEditStreamId"
                    class="v-select"
                    @change="onCwEpisodeSelect(($event.target as HTMLSelectElement).value)"
                  >
                    <option
                      v-for="ep in cwEditEpisodes"
                      :key="ep.streamId"
                      :value="ep.streamId"
                    >
                      S{{ ep.season }} · E{{ ep.episode }}
                      <template v-if="ep.title"> — {{ ep.title }}</template>
                    </option>
                  </select>
                </div>

                <div class="cw-row-2">
                  <label class="cw-field">
                    <span class="cw-label">{{ t('home.cw_season') }}</span>
                    <input
                      v-model.number="cwEditSeason"
                      type="number"
                      min="0"
                      class="v-input"
                      @change="onCwManualCoordChange"
                    />
                  </label>
                  <label class="cw-field">
                    <span class="cw-label">{{ t('home.cw_ep_num') }}</span>
                    <input
                      v-model.number="cwEditEpisode"
                      type="number"
                      min="0"
                      class="v-input"
                      @change="onCwManualCoordChange"
                    />
                  </label>
                </div>
              </template>

              <div class="cw-field">
                <div class="cw-label-row">
                  <span class="cw-label">{{ t('home.cw_progress') }}</span>
                  <strong>{{ cwEditPercent }}%</strong>
                </div>
                <input
                  type="range"
                  min="0"
                  max="95"
                  step="1"
                  :value="cwEditPercent"
                  class="cw-slider"
                  @input="onCwPercentInput(Number(($event.target as HTMLInputElement).value))"
                />
              </div>

              <div class="cw-row-2 cw-time-row">
                <label class="cw-field">
                  <span class="cw-label">{{ t('home.cw_watched_min') }}</span>
                  <input
                    v-model.number="cwEditWatchedMin"
                    type="number"
                    min="0"
                    :max="cwEditDurationMin"
                    class="v-input"
                    @input="onCwMinutesInput"
                  />
                </label>
                <div class="cw-field">
                  <span class="cw-label">{{ t('home.cw_duration_min') }}</span>
                  <div class="v-input cw-readonly-dur">{{ cwEditDurationMin }} min</div>
                </div>
              </div>
            </div>

            <div class="cw-edit-foot">
              <button
                type="button"
                class="v-btn v-btn-danger v-btn-sm"
                :disabled="cwEditSaving"
                @click="removeCwFromEditor"
              >
                <Trash2 :size="14" aria-hidden="true" /> {{ t('common.remove') }}
              </button>
              <div class="cw-foot-right">
                <button
                  type="button"
                  class="v-btn v-btn-sm"
                  :disabled="cwEditSaving"
                  @click="saveCwEdit(true)"
                >
                  <CheckCircle2 :size="14" aria-hidden="true" /> {{ t('home.cw_mark_done') }}
                </button>
                <button
                  type="button"
                  class="v-btn v-btn-primary v-btn-sm"
                  :disabled="cwEditSaving"
                  @click="saveCwEdit(false)"
                >
                  {{ cwEditSaving ? t('common.loading') : t('common.save') }}
                </button>
              </div>
            </div>
          </div>
        </div>
      </Transition>
    </Teleport>

    <!-- Continue Watching Context Menu (Right-click or Long-press) -->
    <Teleport to="body">
      <Transition name="modal-pop">
        <div
          v-if="cwContextMenu"
          class="cw-context-menu"
          role="menu"
          :style="{ left: `${cwContextMenu.x}px`, top: `${cwContextMenu.y}px` }"
          @click.stop
        >
          <button
            type="button"
            role="menuitem"
            class="cw-ctx-item"
            @click="() => { const e = cwContextMenu!.entry; cwContextMenu = null; void resumeContinueWatching(e) }"
          >
            <Play :size="14" aria-hidden="true" />
            <span>{{ t('common.play') }}</span>
          </button>
          <button
            type="button"
            role="menuitem"
            class="cw-ctx-item"
            @click="() => { const e = cwContextMenu!.entry; cwContextMenu = null; openDetails(e.media.pluginId, e.media.nativeId) }"
          >
            <Info :size="14" aria-hidden="true" />
            <span>{{ t('home.go_to_details') }}</span>
          </button>
          <button
            type="button"
            role="menuitem"
            class="cw-ctx-item"
            @click="() => { const e = cwContextMenu!.entry; cwContextMenu = null; void openCwEditor(e) }"
          >
            <Pencil :size="14" aria-hidden="true" />
            <span>{{ t('home.edit_cw') }}</span>
          </button>
          <button
            type="button"
            role="menuitem"
            class="cw-ctx-item is-danger"
            @click="() => { const e = cwContextMenu!.entry; cwContextMenu = null; void dismissContinueWatching(e) }"
          >
            <Trash2 :size="14" aria-hidden="true" />
            <span>{{ t('common.remove') }}</span>
          </button>
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
  padding: 0;
  overflow: hidden;
  flex-shrink: 0;
}

.nuvio-hero.is-compact {
  min-height: 335px;
}

.hero-stage-stack {
  position: relative;
  flex: 1;
  display: grid;
  grid-template-areas: 'stage';
}

.hero-stage {
  grid-area: stage;
  position: relative;
  display: flex;
  flex-direction: column;
  justify-content: flex-end;
  padding: 3rem 3rem 4.85rem;
  min-height: clamp(540px, 64vh, 690px);
}

.nuvio-hero.is-compact .hero-stage {
  min-height: 335px;
  padding: 1.9rem 2.75rem 4.15rem;
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
  transform: scale(1.02);
  transition: transform 6.5s ease-out;
}

.hero-vignette {
  position: absolute;
  inset: 0;
  background:
    linear-gradient(
      90deg,
      color-mix(in srgb, var(--vessel-bg-base) 92%, transparent) 0%,
      color-mix(in srgb, var(--vessel-bg-base) 65%, transparent) 38%,
      color-mix(in srgb, var(--vessel-bg-base) 16%, transparent) 68%,
      transparent 100%
    ),
    linear-gradient(
      180deg,
      color-mix(in srgb, var(--vessel-bg-base) 14%, transparent) 0%,
      color-mix(in srgb, var(--vessel-bg-base) 22%, transparent) 54%,
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
  background: color-mix(in srgb, var(--vessel-accent) 18%, transparent);
  border: 1px solid color-mix(in srgb, var(--vessel-accent) 42%, transparent);
  color: var(--vessel-accent);
  font-size: 0.72rem;
  letter-spacing: 0.08em;
}

.source-tag {
  padding: 0.14rem 0.5rem;
  border-radius: 6px;
  background: var(--vessel-surface-subtle);
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
  color: var(--vessel-text-muted);
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

.hero-controls-row {
  position: absolute;
  left: 3rem;
  right: 3rem;
  bottom: 1.15rem;
  z-index: 4;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 1rem;
}

.hero-sk-pill {
  width: 72px;
  height: 22px;
  border-radius: 999px;
}

.hero-sk-tag {
  width: 96px;
  height: 20px;
  border-radius: 6px;
}

.hero-sk-title {
  width: min(440px, 72vw);
  height: 44px;
  border-radius: 10px;
}

.hero-sk-sub {
  width: min(520px, 84vw);
  height: 18px;
  border-radius: 6px;
}

.hero-sk-btn {
  width: 138px;
  height: 40px;
  border-radius: 999px;
}

.hero-sk-btn.secondary {
  width: 118px;
}

.hero-indicators {
  display: flex;
  align-items: center;
  gap: 0.45rem;
}

.hero-step-arrows {
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
  background: color-mix(in srgb, var(--vessel-text-muted) 40%, transparent);
  cursor: pointer;
  transition: width 0.28s cubic-bezier(0.22, 1, 0.36, 1), background-color 0.24s ease;
}

.indicator-bar.is-active {
  width: 38px;
  background: linear-gradient(90deg, var(--vessel-text-primary), var(--vessel-accent));
}

.hero-crossfade-enter-active,
.hero-crossfade-leave-active {
  transition:
    opacity 0.65s cubic-bezier(0.22, 1, 0.36, 1),
    transform 0.65s cubic-bezier(0.22, 1, 0.36, 1);
}

.hero-crossfade-enter-from {
  opacity: 0;
  transform: scale(1.03);
}

.hero-crossfade-leave-to {
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

/* Keep rails GPU-pinned to prevent WKWebView tile-cull flashes during vertical scroll */
.nuvio-rail {
  display: flex;
  flex-direction: column;
  gap: 0.85rem;
  transform: translateZ(0);
  -webkit-backface-visibility: hidden;
  backface-visibility: hidden;
}

.rail-title-skeleton {
  width: 210px;
  height: 22px;
  border-radius: 8px;
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
.rail-skeleton-card {
  flex: 0 0 186px;
  display: flex;
  flex-direction: column;
  gap: 0.45rem;
}

.rail-skeleton {
  width: 100%;
  aspect-ratio: 2 / 3;
}

.rail-skeleton-line {
  width: 82%;
  height: 13px;
  border-radius: 6px;
}

.rail-skeleton-line.short {
  width: 48%;
  height: 11px;
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
  width: 280px;
  height: 158px;
  flex-shrink: 0;
  border-radius: var(--vessel-radius);
  overflow: hidden;
  background: linear-gradient(135deg, color-mix(in srgb, var(--vessel-accent) 18%, transparent), var(--vessel-bg-surface));
  border: 1px solid var(--vessel-border);
}

.cw-card:hover .cw-thumb {
  border-color: var(--vessel-accent);
}

.cw-img {
  position: absolute;
  inset: 0;
  z-index: 1;
  width: 100%;
  height: 100%;
  object-fit: cover;
  display: block;
}

.cw-thumb-skeleton {
  position: absolute;
  inset: 0;
  z-index: 0;
  border-radius: 0;
  border: none;
}

.cw-shade {
  position: absolute;
  inset: 0;
  z-index: 2;
  background: linear-gradient(180deg, rgba(0, 0, 0, 0.08) 35%, rgba(0, 0, 0, 0.85) 100%);
}

.cw-play-badge {
  position: absolute;
  top: 50%;
  left: 50%;
  z-index: 4;
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

.cw-top-actions {
  position: absolute;
  top: 0.45rem;
  right: 0.45rem;
  z-index: 5;
  display: flex;
  align-items: center;
  gap: 0.3rem;
}

.cw-action-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 26px;
  height: 26px;
  border-radius: 999px;
  border: 1px solid rgba(255, 255, 255, 0.26);
  background: rgba(10, 10, 18, 0.84);
  color: #fff;
  opacity: 0.78;
  cursor: pointer;
  transition: opacity 0.16s ease, background-color 0.16s ease, transform 0.16s ease;
}

.cw-card:hover .cw-action-btn,
.cw-action-btn:focus-visible {
  opacity: 1;
}

.cw-action-btn:hover {
  transform: scale(1.08);
}

.cw-edit:hover {
  background: var(--vessel-accent);
  color: #0b0b12;
}

.cw-dismiss:hover {
  background: rgba(239, 68, 68, 0.9);
}

.cw-remaining {
  position: absolute;
  left: 0.65rem;
  bottom: 0.6rem;
  z-index: 4;
  padding: 0.14rem 0.48rem;
  border-radius: 6px;
  background: rgba(10, 11, 16, 0.72);
  font-size: 0.73rem;
  font-weight: 700;
  color: rgba(255, 255, 255, 0.96);
}

.cw-progress-track {
  position: absolute;
  inset: auto 0 0 0;
  z-index: 4;
  height: 5px;
  background: rgba(255, 255, 255, 0.28);
}

.cw-progress-fill {
  height: 100%;
  background: linear-gradient(90deg, var(--vessel-accent-secondary), var(--vessel-accent));
}

.cw-meta {
  display: flex;
  flex-direction: column;
  width: 280px;
  min-width: 0;
  gap: 0.15rem;
  padding: 0 0.15rem;
}

.cw-title {
  width: 100%;
  font-size: 0.92rem;
  font-weight: 700;
  color: var(--vessel-text-primary);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.cw-sub {
  width: 100%;
  font-size: 0.78rem;
  color: var(--vessel-text-muted);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.cw-context-menu {
  position: fixed;
  z-index: 120;
  min-width: 196px;
  padding: 0.35rem;
  border-radius: 12px;
  background: var(--vessel-bg-surface);
  border: 1px solid var(--vessel-border-strong);
  box-shadow: 0 18px 40px rgba(0, 0, 0, 0.55);
  display: flex;
  flex-direction: column;
  gap: 0.15rem;
}

.cw-ctx-item {
  width: 100%;
  display: flex;
  align-items: center;
  gap: 0.5rem;
  padding: 0.48rem 0.65rem;
  border: none;
  border-radius: 8px;
  background: transparent;
  color: var(--vessel-text-primary);
  font-size: 0.84rem;
  font-weight: 550;
  text-align: left;
  cursor: pointer;
}

.cw-ctx-item:hover {
  background: color-mix(in srgb, var(--vessel-accent) 18%, transparent);
}

.cw-ctx-item.is-danger {
  color: var(--vessel-danger);
  border-top: 1px solid var(--vessel-border);
  margin-top: 0.1rem;
  padding-top: 0.52rem;
  border-radius: 0 0 8px 8px;
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
  border: 1px solid var(--vessel-border);
  background: var(--vessel-surface-input);
  color: var(--vessel-text-muted);
  font-size: 0.8rem;
  font-weight: 600;
  cursor: pointer;
  transition: border-color 0.2s ease, background-color 0.2s ease, color 0.2s ease;
}

.hero-source-chip.checked {
  border-color: var(--vessel-accent);
  background: color-mix(in srgb, var(--vessel-accent) 20%, transparent);
  color: var(--vessel-text-primary);
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
  background: var(--vessel-surface-input);
  border: 1px solid var(--vessel-border);
  user-select: none;
  transition: border-color 0.18s ease, background-color 0.18s ease, transform 0.18s ease;
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
  border: 1px solid var(--vessel-border);
  background: var(--vessel-surface-input);
  cursor: pointer;
  font-size: 0.86rem;
  font-weight: 500;
  transition: border-color 0.18s ease, background-color 0.18s ease;
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

.cw-editor-modal {
  width: min(500px, 100%);
  display: flex;
  flex-direction: column;
  gap: 1rem;
  background: var(--vessel-bg-surface);
}

.cw-edit-body {
  display: flex;
  flex-direction: column;
  gap: 0.85rem;
}

.cw-row-2 {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 0.75rem;
}

.cw-field {
  display: flex;
  flex-direction: column;
  gap: 0.35rem;
}

.cw-label-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.cw-label {
  font-size: 0.8rem;
  font-weight: 650;
  color: var(--vessel-text-muted);
}

.cw-slider {
  width: 100%;
  accent-color: var(--vessel-accent);
  cursor: pointer;
}

.cw-readonly-dur {
  display: flex;
  align-items: center;
  color: var(--vessel-text-muted);
  cursor: default;
  opacity: 0.85;
}

.cw-edit-foot {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.75rem;
  flex-wrap: wrap;
  padding-top: 0.75rem;
  border-top: 1px solid var(--vessel-border);
}

.cw-foot-right {
  display: flex;
  align-items: center;
  gap: 0.5rem;
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
