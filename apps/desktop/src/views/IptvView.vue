<script setup lang="ts">
import { computed, nextTick, onActivated, onMounted, ref, shallowRef } from 'vue'
import { useRouter } from 'vue-router'
import {
  ArrowDown,
  ArrowUp,
  GripVertical,
  ListOrdered,
  Package,
  Play,
  Radio,
  RefreshCw,
  RotateCcw,
  SlidersHorizontal,
  Star,
  Trash2,
  Tv,
} from 'lucide-vue-next'
import { isTauriRuntime, vesselApi } from '@/api/vessel'
import IptvProfileForm from '@/components/iptv/IptvProfileForm.vue'
import { useI18n } from '@/composables/useI18n'
import { usePlayerSession } from '@/composables/usePlayerSession'
import { useUiPreferences } from '@/composables/useUiPreferences'
import { pushToast } from '@/composables/useVesselEvents'
import { isLiveTvPlugin, resolveLiveGroupName } from '@/utils/liveTv'
import type {
  EpgNowNext,
  Episode,
  FavoriteLiveChannel,
  HomeSection,
  IptvKind,
  LiveTvCardAspect,
  LiveTvLogoStyle,
  LiveTvViewMode,
  MediaSummary,
  PluginListEntry,
  StoredIptvChannel,
  StoredIptvProfile,
  StreamCandidate,
  UiDensity,
} from '@/types/vessel'

const router = useRouter()
const { t } = useI18n()
const { setSession } = usePlayerSession()
const { uiPreferences, preferencesLoaded, loadPreferences, savePreferences } = useUiPreferences()

const activeTab = shallowRef<'playlists' | 'plugins'>('plugins')
const livePlugins = ref<PluginListEntry[]>([])
const selectedLivePluginId = shallowRef<string>('')
const pluginLiveSections = ref<HomeSection[]>([])
const selectedSectionId = shallowRef('')
const loadingPluginLive = shallowRef(false)
const pluginChannelFilter = shallowRef('')
const pluginFavoritesOnly = shallowRef(false)
const launchingChannelId = shallowRef('')
const failedPluginLogos = ref<Record<string, boolean>>({})
const showReorderPanel = shallowRef(false)
const showViewPanel = shallowRef(false)
const pluginOrderListRef = ref<HTMLElement | null>(null)
const sectionOrderListRef = ref<HTMLElement | null>(null)
const draggingPluginIndex = shallowRef<number | null>(null)
const draggingSectionIndex = shallowRef<number | null>(null)

const profiles = ref<StoredIptvProfile[]>([])
const activeProfileId = shallowRef<string>('')
const groups = ref<string[]>([])
const selectedGroup = shallowRef<string>('')
const searchQuery = shallowRef('')
const favoritesOnly = shallowRef(false)
const channels = ref<StoredIptvChannel[]>([])
const favoriteIptvChannels = ref<StoredIptvChannel[]>([])
const selectedChannel = ref<StoredIptvChannel | null>(null)
const epg = ref<EpgNowNext | null>(null)
const syncingProfileId = shallowRef<string>('')
const failedLogos = ref<Record<number, boolean>>({})

const livePrefs = computed(() => uiPreferences.value.liveTv)
const pluginFavorites = computed(() => livePrefs.value.favoriteChannels ?? [])

const liveGridStyle = computed(() => {
  const cols = livePrefs.value.columns
  if (cols && cols >= 1 && cols <= 8) {
    return { gridTemplateColumns: `repeat(${cols}, minmax(0, 1fr))` }
  }
  return undefined
})

const liveContainerClasses = computed(() => [
  `mode-${livePrefs.value.viewMode || 'grid'}`,
  `density-${livePrefs.value.density || 'comfortable'}`,
  `logo-${livePrefs.value.logoStyle || 'rounded'}`,
  `aspect-${livePrefs.value.cardAspect || 'auto'}`,
  `lines-${livePrefs.value.titleLines || 1}`,
  {
    'hide-logos': !livePrefs.value.showLogos,
    'hide-categories': !livePrefs.value.showCategories,
    'hide-actions': !livePrefs.value.showActionButtons,
    'hide-numbers': !livePrefs.value.showChannelNumbers,
  },
])

function isEnabledLiveTvPlugin(p: PluginListEntry): boolean {
  return p.enabled && isLiveTvPlugin(p)
}

function favoriteKey(pluginId: string, nativeId: string): string {
  return `${pluginId}::${nativeId}`
}

const pluginFavoriteSet = computed(() => {
  const s = new Set<string>()
  for (const f of pluginFavorites.value) {
    s.add(favoriteKey(f.pluginId, f.nativeId))
  }
  return s
})

function isPluginChannelFavorite(pluginId: string, nativeId: string): boolean {
  return pluginFavoriteSet.value.has(favoriteKey(pluginId, nativeId))
}

function orderBySavedIds<T>(items: T[], savedOrder: string[], getId: (item: T) => string): T[] {
  if (!savedOrder.length || items.length < 2) return items
  const rank = new Map<string, number>()
  savedOrder.forEach((id, idx) => rank.set(id, idx))
  return items
    .map((item, idx) => ({ item, idx, order: rank.get(getId(item)) ?? Number.MAX_SAFE_INTEGER }))
    .sort((a, b) => (a.order !== b.order ? a.order - b.order : a.idx - b.idx))
    .map((entry) => entry.item)
}

const orderedLivePlugins = computed(() =>
  orderBySavedIds(livePlugins.value, livePrefs.value.pluginOrder ?? [], (p) => p.manifest.id),
)

const groupedLivePlugins = computed(() => {
  const buckets = new Map<string, PluginListEntry[]>()
  for (const p of orderedLivePlugins.value) {
    const group = resolveLiveGroupName(p)
    const arr = buckets.get(group)
    if (arr) arr.push(p)
    else buckets.set(group, [p])
  }
  return Array.from(buckets.entries()).map(([name, items]) => ({ name, items }))
})

const orderedPluginLiveSections = computed(() => {
  const pid = selectedLivePluginId.value
  const savedOrder = pid ? (livePrefs.value.sectionOrderByPlugin?.[pid] ?? []) : []
  return orderBySavedIds(pluginLiveSections.value, savedOrder, (sec) => sec.id)
})

const filteredPluginLiveSections = computed(() => {
  const q = pluginChannelFilter.value.trim().toLowerCase()
  const pid = selectedLivePluginId.value
  const secFilter = selectedSectionId.value
  return orderedPluginLiveSections.value
    .filter((sec) => !secFilter || sec.id === secFilter)
    .map((sec) => ({
      ...sec,
      items: sec.items.filter((it) => {
        const matchQ = !q || it.title.toLowerCase().includes(q)
        const resolvedPid = it.pluginId || pid
        const matchFav =
          !pluginFavoritesOnly.value || isPluginChannelFavorite(resolvedPid, it.nativeId)
        return matchQ && matchFav
      }),
    }))
    .filter((sec) => sec.items.length > 0)
})

const pluginSiblingItems = computed(() =>
  filteredPluginLiveSections.value.flatMap((sec) =>
    sec.items.map((item) => ({ sectionTitle: sec.title, item })),
  ),
)

const favoriteSiblingItems = computed(() =>
  filteredPluginFavorites.value.map((f) => ({
    sectionTitle: f.sectionTitle ?? f.pluginId,
    item: {
      nativeId: f.nativeId,
      pluginId: f.pluginId,
      type: 'live' as const,
      isAnime: false,
      title: f.title,
      poster: f.poster,
    },
  })),
)

const filteredPluginFavorites = computed(() => {
  const q = pluginChannelFilter.value.trim().toLowerCase()
  if (!q) return pluginFavorites.value
  return pluginFavorites.value.filter(
    (f) =>
      f.title.toLowerCase().includes(q) ||
      (f.sectionTitle ?? '').toLowerCase().includes(q),
  )
})

const totalFavoritesCount = computed(
  () => pluginFavorites.value.length + favoriteIptvChannels.value.length,
)

async function patchLivePrefs(patch: Partial<typeof uiPreferences.value.liveTv>): Promise<void> {
  const next = {
    ...uiPreferences.value,
    liveTv: {
      ...uiPreferences.value.liveTv,
      ...patch,
    },
  }
  try {
    await savePreferences(next)
  } catch (err) {
    pushToast('error', String(err))
  }
}

async function togglePluginFavorite(item: MediaSummary, sectionTitle?: string): Promise<void> {
  const pluginId = item.pluginId || selectedLivePluginId.value
  if (!pluginId) return
  const current = (uiPreferences.value.liveTv.favoriteChannels ?? []).slice()
  const idx = current.findIndex(
    (f) => f.pluginId === pluginId && f.nativeId === item.nativeId,
  )
  if (idx >= 0) {
    current.splice(idx, 1)
  } else {
    const entry: FavoriteLiveChannel = {
      pluginId,
      nativeId: item.nativeId,
      title: item.title,
      poster: item.poster,
      sectionTitle,
    }
    current.unshift(entry)
  }
  await patchLivePrefs({ favoriteChannels: current })
}

async function removeFavoritePluginEntry(fav: FavoriteLiveChannel): Promise<void> {
  const current = (uiPreferences.value.liveTv.favoriteChannels ?? []).filter(
    (f) => !(f.pluginId === fav.pluginId && f.nativeId === fav.nativeId),
  )
  await patchLivePrefs({ favoriteChannels: current })
}

async function playFavoritePluginEntry(fav: FavoriteLiveChannel): Promise<void> {
  await playPluginLiveItem(
    {
      nativeId: fav.nativeId,
      pluginId: fav.pluginId,
      type: 'live',
      isAnime: false,
      title: fav.title,
      poster: fav.poster,
    },
    fav.sectionTitle ?? fav.pluginId,
    favoriteSiblingItems.value.filter((s) => s.item.pluginId === fav.pluginId),
  )
}

function setLiveViewMode(mode: LiveTvViewMode): void {
  void patchLivePrefs({ viewMode: mode })
}

function setLiveDensity(density: UiDensity): void {
  void patchLivePrefs({ density })
}

function setLiveColumns(cols: number | undefined): void {
  void patchLivePrefs({ columns: cols })
}

function setLiveLogoStyle(logoStyle: LiveTvLogoStyle): void {
  void patchLivePrefs({ logoStyle })
}

function setLiveCardAspect(cardAspect: LiveTvCardAspect): void {
  void patchLivePrefs({ cardAspect })
}

function setLiveTitleLines(titleLines: number): void {
  void patchLivePrefs({ titleLines })
}

async function persistLiveTvOrder(
  pluginOrder: string[],
  sectionOrderByPlugin: Record<string, string[]>,
): Promise<void> {
  await patchLivePrefs({ pluginOrder, sectionOrderByPlugin })
}

async function moveLivePlugin(pluginId: string, delta: -1 | 1): Promise<void> {
  const ids = orderedLivePlugins.value.map((p) => p.manifest.id)
  const idx = ids.indexOf(pluginId)
  const target = idx + delta
  if (idx < 0 || target < 0 || target >= ids.length) return
  ;[ids[idx], ids[target]] = [ids[target], ids[idx]]
  await persistLiveTvOrder(ids, livePrefs.value.sectionOrderByPlugin ?? {})
  const preferDir = delta === -1 ? (target > 0 ? 'up' : 'down') : (target < ids.length - 1 ? 'down' : 'up')
  await nextTick()
  const el = pluginOrderListRef.value?.querySelector(`[data-reorder-id="${CSS.escape(pluginId)}"] [data-move-dir="${preferDir}"]`)
  if (el instanceof HTMLElement) el.focus()
}

async function moveLiveSection(sectionId: string, delta: -1 | 1): Promise<void> {
  const pid = selectedLivePluginId.value
  if (!pid) return
  const ids = orderedPluginLiveSections.value.map((s) => s.id)
  const idx = ids.indexOf(sectionId)
  const target = idx + delta
  if (idx < 0 || target < 0 || target >= ids.length) return
  ;[ids[idx], ids[target]] = [ids[target], ids[idx]]
  await persistLiveTvOrder(livePrefs.value.pluginOrder ?? [], {
    ...(livePrefs.value.sectionOrderByPlugin ?? {}),
    [pid]: ids,
  })
  const preferDir = delta === -1 ? (target > 0 ? 'up' : 'down') : (target < ids.length - 1 ? 'down' : 'up')
  await nextTick()
  const el = sectionOrderListRef.value?.querySelector(`[data-reorder-id="${CSS.escape(sectionId)}"] [data-move-dir="${preferDir}"]`)
  if (el instanceof HTMLElement) el.focus()
}

function startVerticalDrag(
  kind: 'plugin' | 'section',
  index: number,
  ev: PointerEvent,
): void {
  if (ev.button !== 0) return
  ev.preventDefault()
  const activeDragging = kind === 'plugin' ? draggingPluginIndex : draggingSectionIndex
  const listElRef = kind === 'plugin' ? pluginOrderListRef : sectionOrderListRef
  activeDragging.value = index

  const onMove = (moveEv: PointerEvent) => {
    const listEl = listElRef.value
    const curIdx = activeDragging.value
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
      if (kind === 'plugin') {
        const ids = orderedLivePlugins.value.map((p) => p.manifest.id)
        const [moved] = ids.splice(curIdx, 1)
        if (!moved) return
        ids.splice(targetIdx, 0, moved)
        uiPreferences.value = {
          ...uiPreferences.value,
          liveTv: { ...uiPreferences.value.liveTv, pluginOrder: ids },
        }
      } else {
        const pid = selectedLivePluginId.value
        if (!pid) return
        const ids = orderedPluginLiveSections.value.map((s) => s.id)
        const [moved] = ids.splice(curIdx, 1)
        if (!moved) return
        ids.splice(targetIdx, 0, moved)
        uiPreferences.value = {
          ...uiPreferences.value,
          liveTv: {
            ...uiPreferences.value.liveTv,
            sectionOrderByPlugin: {
              ...(uiPreferences.value.liveTv.sectionOrderByPlugin ?? {}),
              [pid]: ids,
            },
          },
        }
      }
      activeDragging.value = targetIdx
    }
  }

  const onUp = () => {
    window.removeEventListener('pointermove', onMove)
    window.removeEventListener('pointerup', onUp)
    window.removeEventListener('pointercancel', onUp)
    activeDragging.value = null
    void savePreferences(uiPreferences.value)
  }

  window.addEventListener('pointermove', onMove)
  window.addEventListener('pointerup', onUp)
  window.addEventListener('pointercancel', onUp)
}

async function resetLiveOrder(): Promise<void> {
  const pid = selectedLivePluginId.value
  const map = { ...(uiPreferences.value.liveTv.sectionOrderByPlugin ?? {}) }
  if (pid) delete map[pid]
  await persistLiveTvOrder([], map)
}

async function refreshLivePlugins(): Promise<void> {
  if (!isTauriRuntime()) return
  try {
    const all = await vesselApi.listPlugins()
    livePlugins.value = all.filter(isEnabledLiveTvPlugin)
    const firstId = orderedLivePlugins.value[0]?.manifest.id
    if (
      firstId &&
      (!selectedLivePluginId.value ||
        !livePlugins.value.some((p) => p.manifest.id === selectedLivePluginId.value))
    ) {
      selectedLivePluginId.value = firstId
    }
  } catch {
    livePlugins.value = []
  }
}

async function refreshFavoriteIptv(): Promise<void> {
  if (!isTauriRuntime()) return
  try {
    favoriteIptvChannels.value = await vesselApi.searchIptvChannels({
      favoritesOnly: true,
      limit: 200,
    })
  } catch {
    favoriteIptvChannels.value = []
  }
}

async function selectLivePlugin(pluginId: string): Promise<void> {
  if (selectedLivePluginId.value !== pluginId) {
    selectedSectionId.value = ''
  }
  selectedLivePluginId.value = pluginId
  if (!pluginId || !isTauriRuntime()) return
  loadingPluginLive.value = true
  try {
    const res = await vesselApi.getHomeFeeds([pluginId])
    const feed = res[0]
    if (feed?.error) {
      pushToast('warn', `${feed.pluginName}: ${feed.error}`)
    }
    pluginLiveSections.value = feed?.sections ?? []
  } catch (err) {
    pushToast('error', String(err))
    pluginLiveSections.value = []
  } finally {
    loadingPluginLive.value = false
  }
}

async function switchIptvTab(tab: 'playlists' | 'plugins'): Promise<void> {
  activeTab.value = tab
  if (tab === 'plugins') {
    await refreshLivePlugins()
    if (selectedLivePluginId.value && !pluginLiveSections.value.length) {
      await selectLivePlugin(selectedLivePluginId.value)
    }
  }
}

async function playPluginLiveItem(
  item: MediaSummary,
  sectionTitle: string,
  siblings: Array<{ sectionTitle: string; item: MediaSummary }> = pluginSiblingItems.value,
): Promise<void> {
  const pluginId = item.pluginId || selectedLivePluginId.value
  if (!pluginId || launchingChannelId.value) return
  launchingChannelId.value = item.nativeId
  try {
    const detail = await vesselApi.getDetailsWithEnrichment(pluginId, item.nativeId, 'en')
    const siblingEpisodes: Episode[] = siblings.map((entry, idx) => ({
      streamId: entry.item.nativeId,
      season: 1,
      episode: idx + 1,
      title: entry.item.title,
      overview: entry.sectionTitle,
      thumbnail: entry.item.poster,
      tags: [],
    }))
    const ep =
      siblingEpisodes.find((e) => e.streamId === item.nativeId) ??
      detail.episodes[0] ?? {
        streamId: item.nativeId,
        season: 1,
        episode: 1,
        title: item.title,
        overview: sectionTitle,
        thumbnail: item.poster,
        tags: [],
      }
    const finalEpisodes = siblingEpisodes.some((e) => e.streamId === ep.streamId)
      ? siblingEpisodes
      : detail.episodes.length
        ? detail.episodes
        : [ep]
    const streamRes = await vesselApi.getStreams(pluginId, ep.streamId)
    if (!streamRes.streams.length) {
      pushToast('warn', `No playable live streams returned for ${item.title}`)
      return
    }
    setSession({
      pluginId,
      nativeId: item.nativeId,
      detail: {
        ...detail,
        type: 'live',
        title: detail.title || item.title,
        poster: detail.poster ?? item.poster,
        backdrop: detail.backdrop ?? item.poster,
        episodes: finalEpisodes,
      },
      episode: ep,
      streams: streamRes.streams,
      subtitles: streamRes.subtitles ?? [],
      activeStreamIndex: 0,
    })
    await router.push({
      name: 'player',
      params: { pluginId, nativeId: item.nativeId },
      query: { streamId: ep.streamId },
    })
  } catch (err) {
    pushToast('error', String(err))
  } finally {
    launchingChannelId.value = ''
  }
}

async function refreshProfiles(): Promise<void> {
  if (!isTauriRuntime()) return
  profiles.value = await vesselApi.listIptvProfiles()
  if (!activeProfileId.value && profiles.value.length) {
    activeProfileId.value = profiles.value[0].id
  }
  await refreshGroupsAndChannels()
}

async function refreshGroupsAndChannels(): Promise<void> {
  if (!isTauriRuntime()) return
  if (activeProfileId.value) {
    groups.value = await vesselApi.listIptvGroups(activeProfileId.value)
  } else {
    groups.value = []
  }
  await runChannelSearch()
}

async function runChannelSearch(): Promise<void> {
  if (!isTauriRuntime()) return
  channels.value = await vesselApi.searchIptvChannels({
    profileId: activeProfileId.value || undefined,
    query: searchQuery.value.trim() || undefined,
    groupTitle: selectedGroup.value || undefined,
    favoritesOnly: favoritesOnly.value,
    limit: 200,
  })
}

async function createProfile(payload: {
  id: string
  name: string
  kind: IptvKind
  serverUrl: string
  username?: string
  password?: string
  epgUrl?: string
  userAgent?: string
}): Promise<void> {
  try {
    const saved = await vesselApi.upsertIptvProfile(payload)
    activeProfileId.value = saved.id
    await refreshProfiles()
    pushToast('info', `Added IPTV profile ${saved.name} — syncing channels…`)
    await syncChannels(saved.id)
  } catch (err) {
    pushToast('error', String(err))
  }
}

async function syncChannels(profileId: string): Promise<void> {
  if (syncingProfileId.value) return
  syncingProfileId.value = profileId
  try {
    const count = await vesselApi.syncIptvProfile(profileId)
    failedLogos.value = {}
    await refreshGroupsAndChannels()
    pushToast('info', `Synced ${count} channels`)
  } catch (err) {
    pushToast('error', String(err))
  } finally {
    syncingProfileId.value = ''
  }
}

async function syncEpg(profileId: string): Promise<void> {
  try {
    const count = await vesselApi.syncIptvEpg(profileId)
    pushToast('info', `Synced ${count} EPG slots`)
  } catch (err) {
    pushToast('error', String(err))
  }
}

async function removeProfile(profileId: string): Promise<void> {
  try {
    await vesselApi.deleteIptvProfile(profileId)
    if (activeProfileId.value === profileId) {
      activeProfileId.value = ''
    }
    await refreshProfiles()
  } catch (err) {
    pushToast('error', String(err))
  }
}

async function selectChannel(ch: StoredIptvChannel): Promise<void> {
  selectedChannel.value = ch
  epg.value = null
  if (ch.tvgId) {
    try {
      const nowTs = Math.floor(Date.now() / 1000)
      epg.value = await vesselApi.getEpgNowNext(ch.profileId, ch.tvgId, nowTs)
    } catch {
      epg.value = null
    }
  }
}

async function toggleFavorite(ch: StoredIptvChannel): Promise<void> {
  try {
    await vesselApi.setIptvFavorite(ch.id, !ch.isFavorite)
    await Promise.all([runChannelSearch(), refreshFavoriteIptv()])
  } catch (err) {
    pushToast('error', String(err))
  }
}

async function playChannel(ch: StoredIptvChannel): Promise<void> {
  const buildChannelCandidates = (item: StoredIptvChannel): StreamCandidate[] => {
    const itemProf = profiles.value.find((p) => p.id === item.profileId)
    const itemHeaders: Record<string, string> = {}
    if (itemProf?.userAgent) itemHeaders['User-Agent'] = itemProf.userAgent

    const rawUrl = item.streamUrl
    const lowerUrl = rawUrl.toLowerCase()
    const isXtreamLive = item.streamId.startsWith('xtream-') && lowerUrl.includes('/live/')
    const list: StreamCandidate[] = []

    if (isXtreamLive && lowerUrl.endsWith('.ts')) {
      const hlsUrl = `${rawUrl.slice(0, -3)}.m3u8`
      list.push(
        {
          url: hlsUrl,
          kind: 'hls',
          label: `${item.name} (HLS)`,
          flags: ['requires_proxy'],
          headers: itemHeaders,
          subtitles: [],
        },
        {
          url: rawUrl,
          kind: 'mpeg_ts',
          label: `${item.name} (MPEG-TS)`,
          flags: ['requires_proxy'],
          headers: itemHeaders,
          subtitles: [],
        },
      )
    } else if (isXtreamLive && lowerUrl.endsWith('.m3u8')) {
      const tsUrl = `${rawUrl.slice(0, -5)}.ts`
      list.push(
        {
          url: rawUrl,
          kind: 'hls',
          label: `${item.name} (HLS)`,
          flags: ['requires_proxy'],
          headers: itemHeaders,
          subtitles: [],
        },
        {
          url: tsUrl,
          kind: 'mpeg_ts',
          label: `${item.name} (MPEG-TS)`,
          flags: ['requires_proxy'],
          headers: itemHeaders,
          subtitles: [],
        },
      )
    } else {
      const isHls = lowerUrl.includes('.m3u8')
      list.push({
        url: rawUrl,
        kind: isHls ? 'hls' : 'mpeg_ts',
        label: isHls ? `${item.name} (HLS)` : `${item.name} (MPEG-TS)`,
        flags: ['requires_proxy'],
        headers: itemHeaders,
        subtitles: [],
      })
    }
    return list
  }

  const channelStreams: Record<string, StreamCandidate> = {}
  const episodes = channels.value.map((item, idx) => {
    const candidates = buildChannelCandidates(item)
    channelStreams[item.streamId] = candidates[0]
    return {
      streamId: item.streamId,
      season: 1,
      episode: idx + 1,
      title: item.name,
      overview: item.groupTitle || undefined,
      thumbnail: item.logoUrl || undefined,
      tags: [],
    }
  })
  const activeEp =
    episodes.find((e) => e.streamId === ch.streamId) ?? {
      streamId: ch.streamId,
      season: 1,
      episode: 1,
      title: ch.name,
      overview: ch.groupTitle || undefined,
      thumbnail: ch.logoUrl || undefined,
      tags: [],
    }
  const activeCandidates = buildChannelCandidates(ch)
  if (!channelStreams[ch.streamId]) {
    channelStreams[ch.streamId] = activeCandidates[0]
  }

  setSession({
    pluginId: 'iptv',
    nativeId: ch.streamId,
    detail: {
      nativeId: ch.streamId,
      pluginId: 'iptv',
      type: 'live',
      isAnime: false,
      title: ch.name,
      poster: ch.logoUrl || undefined,
      backdrop: ch.logoUrl || undefined,
      synopsis: epg.value?.now ? `Now: ${epg.value.now.title}` : ch.groupTitle || undefined,
      genres: ch.groupTitle ? [ch.groupTitle] : ['Live TV'],
      externalIds: {},
      episodes,
    },
    episode: activeEp,
    streams: activeCandidates,
    subtitles: [],
    activeStreamIndex: 0,
    channelStreams,
  })

  await router.push({
    name: 'player',
    params: { pluginId: 'iptv', nativeId: ch.streamId },
    query: { streamId: ch.streamId },
  })
}

onMounted(async () => {
  if (!preferencesLoaded.value) {
    await loadPreferences()
  }
  void refreshProfiles()
  void refreshFavoriteIptv()
  await refreshLivePlugins()
  if (selectedLivePluginId.value && !pluginLiveSections.value.length) {
    await selectLivePlugin(selectedLivePluginId.value)
  }
})

onActivated(() => {
  void refreshFavoriteIptv()
  void refreshLivePlugins()
})
</script>

<template>
  <section class="iptv-view v-page-container">
    <header class="v-page-header">
      <h1 class="v-page-title">{{ t('iptv.title') }}</h1>
      <div class="iptv-tabs" role="tablist">
        <button
          type="button"
          role="tab"
          :aria-selected="activeTab === 'plugins'"
          class="v-btn"
          :class="{ 'v-btn-primary': activeTab === 'plugins' }"
          @click="switchIptvTab('plugins')"
        >
          <Radio :size="15" aria-hidden="true" />
          <span>{{ t('iptv.tab_plugins') }}</span>
          <span v-if="livePlugins.length" class="v-badge">{{ livePlugins.length }}</span>
        </button>
        <button
          type="button"
          role="tab"
          :aria-selected="activeTab === 'playlists'"
          class="v-btn"
          :class="{ 'v-btn-primary': activeTab === 'playlists' }"
          @click="switchIptvTab('playlists')"
        >
          <Tv :size="15" aria-hidden="true" />
          <span>{{ t('iptv.tab_playlists') }}</span>
        </button>
      </div>
    </header>

    <!-- Top Unified Favorites Section (always immediately visible when opening Live TV unless hidden) -->
    <div v-if="!livePrefs.showFavoritesBar" class="live-section-head" style="justify-content: flex-end;">
      <button
        type="button"
        class="v-btn v-btn-sm"
        :class="{ 'v-btn-primary': showViewPanel }"
        @click="showViewPanel = !showViewPanel"
      >
        <SlidersHorizontal :size="14" aria-hidden="true" />
        <span>{{ t('iptv.customize_view') }}</span>
      </button>
    </div>

    <section class="v-card live-favorites-box">
      <div v-if="livePrefs.showFavoritesBar" class="live-section-head" style="justify-content: space-between;">
        <div class="live-section-head">
          <Star :size="16" fill="currentColor" style="color: var(--vessel-warning);" aria-hidden="true" />
          <strong>{{ t('iptv.favorites_section') }}</strong>
          <span class="v-badge">{{ totalFavoritesCount }}</span>
        </div>
        <button
          type="button"
          class="v-btn v-btn-sm"
          :class="{ 'v-btn-primary': showViewPanel }"
          @click="showViewPanel = !showViewPanel"
        >
          <SlidersHorizontal :size="14" aria-hidden="true" />
          <span>{{ t('iptv.customize_view') }}</span>
        </button>
      </div>

      <div v-if="showViewPanel" class="live-view-customizer">
        <div class="view-pref-group">
          <span class="v-muted view-pref-label">{{ t('iptv.view_layout') }}</span>
          <div class="view-pref-chips">
            <button
              v-for="m in (['grid', 'compact_list', 'showcase_tiles', 'guide_rows', 'logo_wall', 'split_directory'] as const)"
              :key="m"
              type="button"
              class="v-btn v-btn-sm"
              :class="{ 'v-btn-primary': (livePrefs.viewMode || 'grid') === m }"
              @click="setLiveViewMode(m)"
            >
              {{ t(`iptv.layout_${m}`) }}
            </button>
          </div>
        </div>
        <div class="view-pref-group">
          <span class="v-muted view-pref-label">{{ t('settings.density') }}</span>
          <div class="view-pref-chips">
            <button
              v-for="d in (['compact', 'comfortable', 'spacious'] as const)"
              :key="d"
              type="button"
              class="v-btn v-btn-sm"
              :class="{ 'v-btn-primary': (livePrefs.density || 'comfortable') === d }"
              @click="setLiveDensity(d)"
            >
              {{ t(`settings.density_${d}`) }}
            </button>
          </div>
        </div>
        <div class="view-pref-group">
          <span class="v-muted view-pref-label">{{ t('iptv.columns_label') }}</span>
          <div class="view-pref-chips">
            <button
              type="button"
              class="v-btn v-btn-sm"
              :class="{ 'v-btn-primary': !livePrefs.columns }"
              @click="setLiveColumns(undefined)"
            >
              {{ t('iptv.columns_auto') }}
            </button>
            <button
              v-for="c in [2, 3, 4, 5, 6]"
              :key="c"
              type="button"
              class="v-btn v-btn-sm"
              :class="{ 'v-btn-primary': livePrefs.columns === c }"
              @click="setLiveColumns(c)"
            >
              {{ c }}
            </button>
          </div>
        </div>
        <div class="view-pref-group">
          <span class="v-muted view-pref-label">{{ t('iptv.logo_style') }}</span>
          <div class="view-pref-chips">
            <button
              v-for="ls in (['rounded', 'circle', 'square', 'plain'] as const)"
              :key="ls"
              type="button"
              class="v-btn v-btn-sm"
              :class="{ 'v-btn-primary': (livePrefs.logoStyle || 'rounded') === ls }"
              @click="setLiveLogoStyle(ls)"
            >
              {{ t(`iptv.logo_${ls}`) }}
            </button>
          </div>
        </div>
        <div class="view-pref-group">
          <span class="v-muted view-pref-label">{{ t('iptv.card_aspect') }}</span>
          <div class="view-pref-chips">
            <button
              v-for="ar in (['auto', 'square', 'wide_16x9', 'classic_4x3'] as const)"
              :key="ar"
              type="button"
              class="v-btn v-btn-sm"
              :class="{ 'v-btn-primary': (livePrefs.cardAspect || 'auto') === ar }"
              @click="setLiveCardAspect(ar)"
            >
              {{ t(`iptv.aspect_${ar}`) }}
            </button>
          </div>
        </div>
        <div class="view-pref-group">
          <span class="v-muted view-pref-label">{{ t('iptv.title_lines') }}</span>
          <div class="view-pref-chips">
            <button
              v-for="ln in [1, 2]"
              :key="ln"
              type="button"
              class="v-btn v-btn-sm"
              :class="{ 'v-btn-primary': (livePrefs.titleLines || 1) === ln }"
              @click="setLiveTitleLines(ln)"
            >
              {{ t(`iptv.lines_${ln}`) }}
            </button>
          </div>
        </div>
        <div class="view-pref-toggles">
          <label class="v-switch">
            <input
              :checked="livePrefs.showLogos"
              type="checkbox"
              class="v-switch-input"
              @change="patchLivePrefs({ showLogos: ($event.target as HTMLInputElement).checked })"
            />
            <span class="v-switch-track" aria-hidden="true" />
            <span>{{ t('iptv.show_logos') }}</span>
          </label>
          <label class="v-switch">
            <input
              :checked="livePrefs.showCategories"
              type="checkbox"
              class="v-switch-input"
              @change="patchLivePrefs({ showCategories: ($event.target as HTMLInputElement).checked })"
            />
            <span class="v-switch-track" aria-hidden="true" />
            <span>{{ t('iptv.show_categories') }}</span>
          </label>
          <label class="v-switch">
            <input
              :checked="livePrefs.showChannelNumbers"
              type="checkbox"
              class="v-switch-input"
              @change="patchLivePrefs({ showChannelNumbers: ($event.target as HTMLInputElement).checked })"
            />
            <span class="v-switch-track" aria-hidden="true" />
            <span>{{ t('iptv.show_channel_numbers') }}</span>
          </label>
          <label class="v-switch">
            <input
              :checked="livePrefs.showFavoritesBar"
              type="checkbox"
              class="v-switch-input"
              @change="patchLivePrefs({ showFavoritesBar: ($event.target as HTMLInputElement).checked })"
            />
            <span class="v-switch-track" aria-hidden="true" />
            <span>{{ t('iptv.show_favorites_bar') }}</span>
          </label>
          <label class="v-switch">
            <input
              :checked="livePrefs.showSectionHeaders"
              type="checkbox"
              class="v-switch-input"
              @change="patchLivePrefs({ showSectionHeaders: ($event.target as HTMLInputElement).checked })"
            />
            <span class="v-switch-track" aria-hidden="true" />
            <span>{{ t('iptv.show_section_headers') }}</span>
          </label>
          <label class="v-switch">
            <input
              :checked="livePrefs.showActionButtons"
              type="checkbox"
              class="v-switch-input"
              @change="patchLivePrefs({ showActionButtons: ($event.target as HTMLInputElement).checked })"
            />
            <span class="v-switch-track" aria-hidden="true" />
            <span>{{ t('iptv.show_action_buttons') }}</span>
          </label>
        </div>
      </div>

      <template v-if="livePrefs.showFavoritesBar">
      <p v-if="!totalFavoritesCount" class="v-muted" style="margin: 0; font-size: 0.85rem;">
        {{ t('iptv.favorites_empty_hint') }}
      </p>

      <div
        v-else
        class="channels-grid"
        :class="liveContainerClasses"
        :style="liveGridStyle"
      >
        <div
          v-for="fav in filteredPluginFavorites"
          :key="`fav-plug:${fav.pluginId}:${fav.nativeId}`"
          class="v-card channel-card"
        >
          <div v-if="livePrefs.showLogos" class="ch-logo-box">
            <img
              v-if="fav.poster && !failedPluginLogos[fav.nativeId]"
              :src="fav.poster"
              :alt="fav.title"
              loading="lazy"
              decoding="async"
              class="ch-logo-img"
              @error="failedPluginLogos = { ...failedPluginLogos, [fav.nativeId]: true }"
            />
            <Tv v-else :size="18" class="ch-logo-fallback" aria-hidden="true" />
          </div>
          <button
            type="button"
            class="ch-info-btn"
            @click="playFavoritePluginEntry(fav)"
          >
            <strong class="ch-name">{{ fav.title }}</strong>
            <span v-if="livePrefs.showCategories" class="v-muted ch-group">
              {{ fav.sectionTitle || fav.pluginId }}
            </span>
          </button>
          <div v-if="livePrefs.showActionButtons" class="ch-actions">
            <button
              type="button"
              class="v-btn v-btn-icon"
              :aria-label="t('iptv.unfavorite_channel')"
              @click.stop="removeFavoritePluginEntry(fav)"
            >
              <Star :size="14" fill="currentColor" style="color: var(--vessel-warning);" aria-hidden="true" />
            </button>
            <button
              type="button"
              class="v-btn v-btn-icon v-btn-primary"
              :disabled="launchingChannelId === fav.nativeId"
              :aria-label="`Play ${fav.title}`"
              @click="playFavoritePluginEntry(fav)"
            >
              <RefreshCw v-if="launchingChannelId === fav.nativeId" :size="14" class="spin" aria-hidden="true" />
              <Play v-else :size="14" aria-hidden="true" />
            </button>
          </div>
        </div>

        <div
          v-for="ch in favoriteIptvChannels"
          :key="`fav-iptv:${ch.id}`"
          class="v-card channel-card"
        >
          <div v-if="livePrefs.showLogos" class="ch-logo-box">
            <img
              v-if="ch.logoUrl && !failedLogos[ch.id]"
              :src="ch.logoUrl"
              :alt="ch.name"
              loading="lazy"
              decoding="async"
              class="ch-logo-img"
              @error="failedLogos = { ...failedLogos, [ch.id]: true }"
            />
            <Tv v-else :size="18" class="ch-logo-fallback" aria-hidden="true" />
          </div>
          <button type="button" class="ch-info-btn" @click="playChannel(ch)">
            <strong class="ch-name">{{ ch.name }}</strong>
            <span v-if="livePrefs.showCategories" class="v-muted ch-group">{{ ch.groupTitle }}</span>
          </button>
          <div v-if="livePrefs.showActionButtons" class="ch-actions">
            <button
              type="button"
              class="v-btn v-btn-icon"
              :aria-label="t('iptv.unfavorite_channel')"
              @click.stop="toggleFavorite(ch)"
            >
              <Star :size="14" fill="currentColor" style="color: var(--vessel-warning);" aria-hidden="true" />
            </button>
            <button
              type="button"
              class="v-btn v-btn-icon v-btn-primary"
              :aria-label="`Play ${ch.name}`"
              @click="playChannel(ch)"
            >
              <Play :size="14" aria-hidden="true" />
            </button>
          </div>
        </div>
      </div>
      </template>
    </section>

    <template v-if="activeTab === 'playlists'">
      <IptvProfileForm @submit="createProfile" />

    <div v-if="profiles.length" class="profiles-strip">
      <div
        v-for="prof in profiles"
        :key="prof.id"
        class="v-card profile-pill"
        :class="{ 'is-selected': activeProfileId === prof.id }"
      >
        <button class="profile-select" @click="activeProfileId = prof.id; refreshGroupsAndChannels()">
          <strong>{{ prof.name }}</strong>
          <span class="v-badge">{{ prof.kind }}</span>
        </button>
        <div class="profile-actions">
          <button
            class="v-btn"
            :disabled="syncingProfileId === prof.id"
            @click="syncChannels(prof.id)"
          >
            <RefreshCw
              :size="14"
              :class="{ spin: syncingProfileId === prof.id }"
              aria-hidden="true"
            />
            {{ syncingProfileId === prof.id ? 'Syncing…' : t('iptv.sync_channels') }}
          </button>
          <button class="v-btn" @click="syncEpg(prof.id)">{{ t('iptv.sync_epg') }}</button>
          <button
            type="button"
            class="v-btn v-btn-icon v-btn-danger"
            :aria-label="`Delete IPTV profile ${prof.name}`"
            @click="removeProfile(prof.id)"
          >
            <Trash2 :size="14" aria-hidden="true" />
          </button>
        </div>
      </div>
    </div>

    <div class="controls-bar">
      <input
        v-model="searchQuery"
        type="search"
        name="iptvChannelSearch"
        autocomplete="off"
        aria-label="Search IPTV channels"
        class="v-input"
        :placeholder="t('iptv.search_channels')"
        @input="runChannelSearch"
      />
      <select
        v-model="selectedGroup"
        aria-label="Channel group filter"
        class="v-select group-select"
        @change="runChannelSearch"
      >
        <option value="">{{ t('iptv.all_groups') }}</option>
        <option v-for="g in groups" :key="g" :value="g">{{ g }}</option>
      </select>
      <label class="v-switch">
        <input
          v-model="favoritesOnly"
          type="checkbox"
          class="v-switch-input"
          @change="runChannelSearch"
        />
        <span class="v-switch-track" aria-hidden="true" />
        <span>{{ t('iptv.favorites_only') }}</span>
      </label>
    </div>

    <div v-if="selectedChannel && epg" class="v-card epg-banner">
      <strong>{{ selectedChannel.name }}</strong>
      <div class="v-muted">
        <span>{{ t('iptv.now') }}: {{ epg.now?.title ?? '—' }}</span> ·
        <span>{{ t('iptv.next') }}: {{ epg.next?.title ?? '—' }}</span>
      </div>
    </div>

    <div v-if="!channels.length" class="v-card v-muted">{{ t('common.empty') }}</div>

    <div v-else-if="livePrefs.viewMode === 'split_directory' && groups.length" class="split-directory-shell">
      <aside class="v-card split-dir-sidebar">
        <button
          type="button"
          class="split-dir-btn"
          :class="{ 'is-active': !selectedGroup }"
          @click="selectedGroup = ''; runChannelSearch()"
        >
          <span>{{ t('iptv.all_groups') }}</span>
        </button>
        <button
          v-for="g in groups"
          :key="`iptv-split-${g}`"
          type="button"
          class="split-dir-btn"
          :class="{ 'is-active': selectedGroup === g }"
          @click="selectedGroup = g; runChannelSearch()"
        >
          <span class="split-dir-title">{{ g }}</span>
        </button>
      </aside>
      <div class="channels-grid" :class="liveContainerClasses" :style="[liveGridStyle, { flex: '1', minWidth: '0' }]">
        <div
          v-for="(ch, chIdx) in channels"
          :key="ch.id"
          class="v-card channel-card"
        >
          <span v-if="livePrefs.showChannelNumbers" class="ch-num-badge">{{ String(chIdx + 1).padStart(2, '0') }}</span>
          <div v-if="livePrefs.showLogos" class="ch-logo-box">
            <img
              v-if="ch.logoUrl && !failedLogos[ch.id]"
              :src="ch.logoUrl"
              :alt="ch.name"
              loading="lazy"
              decoding="async"
              class="ch-logo-img"
              @error="failedLogos = { ...failedLogos, [ch.id]: true }"
            />
            <Tv v-else :size="18" class="ch-logo-fallback" aria-hidden="true" />
          </div>
          <button type="button" class="ch-info-btn" @click="selectChannel(ch)">
            <strong class="ch-name">{{ ch.name }}</strong>
            <span v-if="livePrefs.showCategories" class="v-muted ch-group">{{ ch.groupTitle }}</span>
          </button>
          <div v-if="livePrefs.showActionButtons" class="ch-actions">
            <button
              type="button"
              class="v-btn v-btn-icon"
              :aria-label="ch.isFavorite ? `Unfavorite ${ch.name}` : `Favorite ${ch.name}`"
              @click.stop="toggleFavorite(ch)"
            >
              <Star :size="14" :fill="ch.isFavorite ? 'currentColor' : 'none'" :style="ch.isFavorite ? 'color: var(--vessel-warning);' : ''" aria-hidden="true" />
            </button>
            <button
              type="button"
              class="v-btn v-btn-icon v-btn-primary"
              :aria-label="`Play ${ch.name}`"
              @click="playChannel(ch)"
            >
              <Play :size="14" aria-hidden="true" />
            </button>
          </div>
        </div>
      </div>
    </div>

    <div v-else class="channels-grid" :class="liveContainerClasses" :style="liveGridStyle">
      <div
        v-for="(ch, chIdx) in channels"
        :key="ch.id"
        class="v-card channel-card"
      >
        <span v-if="livePrefs.showChannelNumbers" class="ch-num-badge">{{ String(chIdx + 1).padStart(2, '0') }}</span>
        <div v-if="livePrefs.showLogos" class="ch-logo-box">
          <img
            v-if="ch.logoUrl && !failedLogos[ch.id]"
            :src="ch.logoUrl"
            :alt="ch.name"
            loading="lazy"
            decoding="async"
            class="ch-logo-img"
            @error="failedLogos = { ...failedLogos, [ch.id]: true }"
          />
          <Tv v-else :size="18" class="ch-logo-fallback" aria-hidden="true" />
        </div>
        <button type="button" class="ch-info-btn" @click="selectChannel(ch)">
          <strong class="ch-name">{{ ch.name }}</strong>
          <span v-if="livePrefs.showCategories" class="v-muted ch-group">{{ ch.groupTitle }}</span>
        </button>
        <div v-if="livePrefs.showActionButtons" class="ch-actions">
          <button
            type="button"
            class="v-btn v-btn-icon"
            :aria-label="ch.isFavorite ? `Unfavorite ${ch.name}` : `Favorite ${ch.name}`"
            @click.stop="toggleFavorite(ch)"
          >
            <Star :size="14" :fill="ch.isFavorite ? 'currentColor' : 'none'" :style="ch.isFavorite ? 'color: var(--vessel-warning);' : ''" aria-hidden="true" />
          </button>
          <button
            type="button"
            class="v-btn v-btn-icon v-btn-primary"
            :aria-label="`Play ${ch.name}`"
            @click="playChannel(ch)"
          >
            <Play :size="14" aria-hidden="true" />
          </button>
        </div>
      </div>
    </div>
    </template>

    <template v-else>
      <div v-if="!livePlugins.length" class="v-card v-muted">
        {{ t('iptv.no_live_plugins') }}
      </div>

      <template v-else>
        <div class="plugin-repos-stack">
          <div v-for="group in groupedLivePlugins" :key="group.name" class="v-card live-repo-card">
            <div class="live-repo-head">
              <Package :size="16" class="repo-icon" aria-hidden="true" />
              <strong>{{ group.name }}</strong>
              <span class="v-badge">{{ group.items.length }}</span>
            </div>
            <div class="live-plugin-pills">
              <button
                v-for="plug in group.items"
                :key="plug.manifest.id"
                type="button"
                class="v-btn v-btn-sm"
                :class="{ 'v-btn-primary': selectedLivePluginId === plug.manifest.id }"
                @click="selectLivePlugin(plug.manifest.id)"
              >
                <Radio :size="13" aria-hidden="true" />
                <span>{{ plug.manifest.name }}</span>
              </button>
            </div>
          </div>
        </div>

        <div class="controls-bar">
          <input
            v-model="pluginChannelFilter"
            type="search"
            aria-label="Filter plugin channels"
            class="v-input"
            :placeholder="t('iptv.filter_channels')"
          />
          <label class="v-switch">
            <input
              v-model="pluginFavoritesOnly"
              type="checkbox"
              class="v-switch-input"
            />
            <span class="v-switch-track" aria-hidden="true" />
            <span>{{ t('iptv.favorites_only') }}</span>
          </label>
          <button
            type="button"
            class="v-btn"
            :class="{ 'v-btn-primary': showReorderPanel }"
            @click="showReorderPanel = !showReorderPanel"
          >
            <ListOrdered :size="14" aria-hidden="true" />
            <span>{{ t('iptv.reorder') }}</span>
          </button>
          <button
            type="button"
            class="v-btn"
            :disabled="loadingPluginLive"
            @click="selectLivePlugin(selectedLivePluginId)"
          >
            <RefreshCw :size="14" :class="{ spin: loadingPluginLive }" aria-hidden="true" />
            <span>{{ t('common.refresh') }}</span>
          </button>
        </div>

        <div v-if="showReorderPanel" class="v-card reorder-panel">
          <div class="reorder-head">
            <strong>{{ t('iptv.reorder_title') }} · {{ t('home.shelf_order_hint') }}</strong>
            <button type="button" class="v-btn v-btn-sm" @click="resetLiveOrder">
              <RotateCcw :size="13" aria-hidden="true" />
              <span>{{ t('iptv.reset_order') }}</span>
            </button>
          </div>
          <div class="reorder-columns">
            <div v-if="orderedLivePlugins.length > 1" class="reorder-col">
              <span class="v-muted reorder-subhead">{{ t('iptv.plugin_order') }}</span>
              <div ref="pluginOrderListRef" class="reorder-list-wrap">
                <div
                  v-for="(plug, pIdx) in orderedLivePlugins"
                  :key="plug.manifest.id"
                  :data-reorder-id="plug.manifest.id"
                  class="reorder-row"
                  :class="{ 'is-dragging': draggingPluginIndex === pIdx }"
                >
                  <button
                    type="button"
                    class="drag-handle"
                    :aria-label="t('home.shelf_order_hint')"
                    @pointerdown="startVerticalDrag('plugin', pIdx, $event)"
                  >
                    <GripVertical :size="14" aria-hidden="true" />
                  </button>
                  <span class="reorder-label">{{ plug.manifest.name }}</span>
                  <div class="reorder-btns">
                    <button
                      type="button"
                      class="v-btn v-btn-icon v-btn-sm"
                      data-move-dir="up"
                      :disabled="pIdx === 0"
                      :aria-label="t('iptv.move_up')"
                      @click="moveLivePlugin(plug.manifest.id, -1)"
                    >
                      <ArrowUp :size="13" aria-hidden="true" />
                    </button>
                    <button
                      type="button"
                      class="v-btn v-btn-icon v-btn-sm"
                      data-move-dir="down"
                      :disabled="pIdx === orderedLivePlugins.length - 1"
                      :aria-label="t('iptv.move_down')"
                      @click="moveLivePlugin(plug.manifest.id, 1)"
                    >
                      <ArrowDown :size="13" aria-hidden="true" />
                    </button>
                  </div>
                </div>
              </div>
            </div>

            <div v-if="orderedPluginLiveSections.length > 1" class="reorder-col">
              <span class="v-muted reorder-subhead">{{ t('iptv.section_order') }}</span>
              <div ref="sectionOrderListRef" class="reorder-list-wrap">
                <div
                  v-for="(sec, sIdx) in orderedPluginLiveSections"
                  :key="sec.id"
                  :data-reorder-id="sec.id"
                  class="reorder-row"
                  :class="{ 'is-dragging': draggingSectionIndex === sIdx }"
                >
                  <button
                    type="button"
                    class="drag-handle"
                    :aria-label="t('home.shelf_order_hint')"
                    @pointerdown="startVerticalDrag('section', sIdx, $event)"
                  >
                    <GripVertical :size="14" aria-hidden="true" />
                  </button>
                  <span class="reorder-label">{{ sec.title }}</span>
                  <div class="reorder-btns">
                    <button
                      type="button"
                      class="v-btn v-btn-icon v-btn-sm"
                      data-move-dir="up"
                      :disabled="sIdx === 0"
                      :aria-label="t('iptv.move_up')"
                      @click="moveLiveSection(sec.id, -1)"
                    >
                      <ArrowUp :size="13" aria-hidden="true" />
                    </button>
                    <button
                      type="button"
                      class="v-btn v-btn-icon v-btn-sm"
                      data-move-dir="down"
                      :disabled="sIdx === orderedPluginLiveSections.length - 1"
                      :aria-label="t('iptv.move_down')"
                      @click="moveLiveSection(sec.id, 1)"
                    >
                      <ArrowDown :size="13" aria-hidden="true" />
                    </button>
                  </div>
                </div>
              </div>
            </div>
          </div>
        </div>

        <div v-if="loadingPluginLive" class="channels-grid" :class="liveContainerClasses" :style="liveGridStyle">
          <div v-for="n in 8" :key="n" class="v-skeleton live-skeleton" />
        </div>

        <div v-else-if="!filteredPluginLiveSections.length" class="v-card v-muted">
          {{ t('common.empty') }}
        </div>

        <div v-else-if="livePrefs.viewMode === 'split_directory' && orderedPluginLiveSections.length > 1" class="split-directory-shell">
          <aside class="v-card split-dir-sidebar">
            <button
              type="button"
              class="split-dir-btn"
              :class="{ 'is-active': !selectedSectionId }"
              @click="selectedSectionId = ''"
            >
              <span>{{ t('common.all') }}</span>
            </button>
            <button
              v-for="sec in orderedPluginLiveSections"
              :key="`split-${sec.id}`"
              type="button"
              class="split-dir-btn"
              :class="{ 'is-active': selectedSectionId === sec.id }"
              @click="selectedSectionId = sec.id"
            >
              <span class="split-dir-title">{{ sec.title }}</span>
              <span class="v-badge">{{ sec.items.length }}</span>
            </button>
          </aside>
          <div class="plugin-sections-stack" style="flex: 1; min-width: 0;">
            <section v-for="sec in filteredPluginLiveSections" :key="sec.id" class="live-section">
              <div v-if="livePrefs.showSectionHeaders" class="live-section-head">
                <h2 class="live-section-title">{{ sec.title }}</h2>
                <span class="v-badge">{{ sec.items.length }}</span>
              </div>
              <div class="channels-grid" :class="liveContainerClasses" :style="liveGridStyle">
                <div
                  v-for="(item, cIdx) in sec.items"
                  :key="item.nativeId"
                  class="v-card channel-card"
                >
                  <span v-if="livePrefs.showChannelNumbers" class="ch-num-badge">{{ String(cIdx + 1).padStart(2, '0') }}</span>
                  <div v-if="livePrefs.showLogos" class="ch-logo-box">
                    <img
                      v-if="item.poster && !failedPluginLogos[item.nativeId]"
                      :src="item.poster"
                      :alt="item.title"
                      loading="lazy"
                      decoding="async"
                      class="ch-logo-img"
                      @error="failedPluginLogos = { ...failedPluginLogos, [item.nativeId]: true }"
                    />
                    <Tv v-else :size="18" class="ch-logo-fallback" aria-hidden="true" />
                  </div>
                  <button
                    type="button"
                    class="ch-info-btn"
                    @click="playPluginLiveItem(item, sec.title)"
                  >
                    <strong class="ch-name">{{ item.title }}</strong>
                    <span v-if="livePrefs.showCategories" class="v-muted ch-group">{{ sec.title }}</span>
                  </button>
                  <div v-if="livePrefs.showActionButtons" class="ch-actions">
                    <button
                      type="button"
                      class="v-btn v-btn-icon"
                      :aria-label="isPluginChannelFavorite(item.pluginId || selectedLivePluginId, item.nativeId) ? t('iptv.unfavorite_channel') : t('iptv.favorite_channel')"
                      @click.stop="togglePluginFavorite(item, sec.title)"
                    >
                      <Star
                        :size="14"
                        :fill="isPluginChannelFavorite(item.pluginId || selectedLivePluginId, item.nativeId) ? 'currentColor' : 'none'"
                        :style="isPluginChannelFavorite(item.pluginId || selectedLivePluginId, item.nativeId) ? 'color: var(--vessel-warning);' : ''"
                        aria-hidden="true"
                      />
                    </button>
                    <button
                      type="button"
                      class="v-btn v-btn-icon v-btn-primary"
                      :disabled="launchingChannelId === item.nativeId"
                      :aria-label="`Play ${item.title}`"
                      @click="playPluginLiveItem(item, sec.title)"
                    >
                      <RefreshCw v-if="launchingChannelId === item.nativeId" :size="14" class="spin" aria-hidden="true" />
                      <Play v-else :size="14" aria-hidden="true" />
                    </button>
                  </div>
                </div>
              </div>
            </section>
          </div>
        </div>

        <div v-else class="plugin-sections-stack">
          <section v-for="sec in filteredPluginLiveSections" :key="sec.id" class="live-section">
            <div v-if="livePrefs.showSectionHeaders" class="live-section-head">
              <h2 class="live-section-title">{{ sec.title }}</h2>
              <span class="v-badge">{{ sec.items.length }}</span>
            </div>
            <div class="channels-grid" :class="liveContainerClasses" :style="liveGridStyle">
              <div
                v-for="(item, cIdx) in sec.items"
                :key="item.nativeId"
                class="v-card channel-card"
              >
                <span v-if="livePrefs.showChannelNumbers" class="ch-num-badge">{{ String(cIdx + 1).padStart(2, '0') }}</span>
                <div v-if="livePrefs.showLogos" class="ch-logo-box">
                  <img
                    v-if="item.poster && !failedPluginLogos[item.nativeId]"
                    :src="item.poster"
                    :alt="item.title"
                    loading="lazy"
                    decoding="async"
                    class="ch-logo-img"
                    @error="failedPluginLogos = { ...failedPluginLogos, [item.nativeId]: true }"
                  />
                  <Tv v-else :size="18" class="ch-logo-fallback" aria-hidden="true" />
                </div>
                <button
                  type="button"
                  class="ch-info-btn"
                  @click="playPluginLiveItem(item, sec.title)"
                >
                  <strong class="ch-name">{{ item.title }}</strong>
                  <span v-if="livePrefs.showCategories" class="v-muted ch-group">{{ sec.title }}</span>
                </button>
                <div v-if="livePrefs.showActionButtons" class="ch-actions">
                  <button
                    type="button"
                    class="v-btn v-btn-icon"
                    :aria-label="isPluginChannelFavorite(item.pluginId || selectedLivePluginId, item.nativeId) ? t('iptv.unfavorite_channel') : t('iptv.favorite_channel')"
                    @click.stop="togglePluginFavorite(item, sec.title)"
                  >
                    <Star
                      :size="14"
                      :fill="isPluginChannelFavorite(item.pluginId || selectedLivePluginId, item.nativeId) ? 'currentColor' : 'none'"
                      :style="isPluginChannelFavorite(item.pluginId || selectedLivePluginId, item.nativeId) ? 'color: var(--vessel-warning);' : ''"
                      aria-hidden="true"
                    />
                  </button>
                  <button
                    type="button"
                    class="v-btn v-btn-icon v-btn-primary"
                    :disabled="launchingChannelId === item.nativeId"
                    :aria-label="`Play ${item.title}`"
                    @click="playPluginLiveItem(item, sec.title)"
                  >
                    <RefreshCw v-if="launchingChannelId === item.nativeId" :size="14" class="spin" aria-hidden="true" />
                    <Play v-else :size="14" aria-hidden="true" />
                  </button>
                </div>
              </div>
            </div>
          </section>
        </div>
      </template>
    </template>
  </section>
</template>

<style scoped>
.iptv-view {
  display: flex;
  flex-direction: column;
  gap: var(--vessel-gap);
}

.profiles-strip {
  display: flex;
  flex-wrap: wrap;
  gap: 0.65rem;
}

.profile-pill {
  display: flex;
  align-items: center;
  gap: 0.75rem;
  flex-wrap: wrap;
}

.profile-pill.is-selected {
  border-color: var(--vessel-accent);
}

.profile-select {
  background: transparent;
  border: none;
  display: flex;
  align-items: center;
  gap: 0.5rem;
  cursor: pointer;
}

.profile-actions,
.controls-bar,
.ch-actions {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  flex-wrap: wrap;
}

.controls-bar .v-input {
  flex: 1;
  min-width: 200px;
}

.group-select {
  width: 200px;
}

.fav-toggle {
  display: flex;
  align-items: center;
  gap: 0.45rem;
}

.channels-grid {
  display: grid;
  gap: 0.65rem;
  grid-template-columns: repeat(auto-fill, minmax(270px, 1fr));
}

.channel-card {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 0.75rem;
  cursor: pointer;
}

.ch-logo-box {
  width: 40px;
  height: 40px;
  border-radius: 10px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  background: var(--vessel-surface-input);
  border: 1px solid var(--vessel-border);
  overflow: hidden;
  padding: 4px;
  flex-shrink: 0;
}

.ch-logo-img {
  width: 100%;
  height: 100%;
  object-fit: contain;
}

.ch-logo-fallback {
  color: var(--vessel-text-muted);
}

.ch-info-btn {
  display: flex;
  flex-direction: column;
  gap: 0.15rem;
  min-width: 0;
  flex: 1;
  border: none;
  background: transparent;
  padding: 0;
  text-align: left;
  cursor: pointer;
}

.ch-name,
.ch-group {
  font-size: 0.8rem;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.ch-name {
  font-size: 0.92rem;
}

.spin {
  animation: iptv-spin 0.9s linear infinite;
}

@keyframes iptv-spin {
  to {
    transform: rotate(360deg);
  }
}

.epg-banner {
  border-color: var(--vessel-accent-secondary);
  display: flex;
  flex-direction: column;
  gap: 0.25rem;
}

.iptv-tabs {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  flex-wrap: wrap;
}

.plugin-repos-stack,
.plugin-sections-stack {
  display: flex;
  flex-direction: column;
  gap: 1rem;
}

.live-repo-card {
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
}

.live-repo-head,
.live-plugin-pills,
.live-section-head {
  display: flex;
  align-items: center;
  gap: 0.55rem;
  flex-wrap: wrap;
}

.repo-icon {
  color: var(--vessel-accent);
}

.live-section {
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
}

.live-section-title {
  margin: 0;
  font-size: 1.08rem;
  font-weight: 700;
}

.live-skeleton {
  height: 70px;
}

.reorder-panel {
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
}

.reorder-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.5rem;
}

.reorder-columns {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(240px, 1fr));
  gap: 0.85rem;
}

.reorder-col {
  display: flex;
  flex-direction: column;
  gap: 0.35rem;
}

.reorder-subhead {
  font-size: 0.78rem;
  font-weight: 600;
}

.reorder-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.5rem;
  padding: 0.35rem 0.55rem;
  border-radius: 10px;
  background: var(--vessel-surface-input);
  border: 1px solid var(--vessel-border);
}

.reorder-label {
  font-size: 0.84rem;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.reorder-btns {
  display: flex;
  align-items: center;
  gap: 0.25rem;
  flex-shrink: 0;
}

.live-favorites-box {
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
  border-color: color-mix(in srgb, var(--vessel-warning) 28%, var(--vessel-border));
}

.live-view-customizer {
  display: flex;
  flex-wrap: wrap;
  gap: 0.9rem 1.4rem;
  padding: 0.75rem 0.9rem;
  border-radius: 10px;
  background: var(--vessel-surface-input);
  border: 1px solid var(--vessel-border);
}

.view-pref-group {
  display: flex;
  flex-direction: column;
  gap: 0.35rem;
}

.view-pref-label {
  font-size: 0.75rem;
  font-weight: 600;
}

.view-pref-chips,
.view-pref-toggles {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 0.4rem 0.75rem;
}

.reorder-list-wrap {
  display: flex;
  flex-direction: column;
  gap: 0.35rem;
}

.reorder-row.is-dragging {
  border-color: var(--vessel-accent);
  background: rgba(203, 166, 247, 0.16);
  transform: scale(1.012);
}

.drag-handle {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 24px;
  height: 24px;
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

.channels-grid.mode-compact_list {
  grid-template-columns: minmax(0, 1fr);
}

.channels-grid.mode-showcase_tiles {
  grid-template-columns: repeat(auto-fill, minmax(196px, 1fr));
}

.channels-grid.mode-showcase_tiles .channel-card {
  flex-direction: column;
  align-items: stretch;
  text-align: center;
  gap: 0.55rem;
}

.channels-grid.mode-showcase_tiles .ch-logo-box {
  width: 100%;
  height: 76px;
}

.channels-grid.mode-showcase_tiles .ch-info-btn {
  text-align: center;
  align-items: center;
}

.channels-grid.mode-showcase_tiles .ch-actions {
  justify-content: center;
}

.channels-grid.density-compact {
  gap: 0.42rem;
}

.channels-grid.density-compact .channel-card {
  padding: 0.55rem 0.7rem;
}

.channels-grid.density-compact .ch-logo-box {
  width: 32px;
  height: 32px;
}

.channels-grid.density-spacious {
  gap: 0.9rem;
}

.channels-grid.density-spacious .channel-card {
  padding: 1rem 1.1rem;
}

.channels-grid.density-spacious .ch-logo-box {
  width: 48px;
  height: 48px;
}

.ch-num-badge {
  font-size: 0.72rem;
  font-weight: 700;
  padding: 0.16rem 0.42rem;
  border-radius: 6px;
  background: color-mix(in srgb, var(--vessel-accent) 16%, transparent);
  color: var(--vessel-accent);
  flex-shrink: 0;
}

.channels-grid.logo-circle .ch-logo-box {
  border-radius: 999px;
}

.channels-grid.logo-square .ch-logo-box {
  border-radius: 3px;
}

.channels-grid.logo-plain .ch-logo-box {
  background: transparent;
  border-color: transparent;
}

.channels-grid.aspect-square .channel-card {
  aspect-ratio: 1 / 1;
}

.channels-grid.aspect-wide_16x9 .channel-card {
  aspect-ratio: 16 / 9;
}

.channels-grid.aspect-classic_4x3 .channel-card {
  aspect-ratio: 4 / 3;
}

.channels-grid.mode-compact_list.aspect-square .channel-card,
.channels-grid.mode-compact_list.aspect-wide_16x9 .channel-card,
.channels-grid.mode-compact_list.aspect-classic_4x3 .channel-card,
.channels-grid.mode-guide_rows.aspect-square .channel-card,
.channels-grid.mode-guide_rows.aspect-wide_16x9 .channel-card,
.channels-grid.mode-guide_rows.aspect-classic_4x3 .channel-card {
  aspect-ratio: auto;
}

.channels-grid.lines-2 .ch-name {
  white-space: normal;
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
}

.channels-grid.mode-guide_rows {
  grid-template-columns: minmax(0, 1fr);
}

.channels-grid.mode-guide_rows .channel-card {
  padding: 0.85rem 1rem;
}

.channels-grid.mode-guide_rows .ch-logo-box {
  width: 52px;
  height: 52px;
}

.channels-grid.mode-logo_wall {
  grid-template-columns: repeat(auto-fill, minmax(150px, 1fr));
}

.channels-grid.mode-logo_wall .channel-card {
  flex-direction: column;
  align-items: center;
  text-align: center;
  gap: 0.45rem;
}

.channels-grid.mode-logo_wall .ch-logo-box {
  width: 100%;
  height: 82px;
}

.channels-grid.mode-logo_wall .ch-info-btn {
  align-items: center;
  text-align: center;
}

.split-directory-shell {
  display: flex;
  gap: 1rem;
  align-items: flex-start;
}

.split-dir-sidebar {
  width: 220px;
  flex-shrink: 0;
  display: flex;
  flex-direction: column;
  gap: 0.3rem;
  padding: 0.65rem;
}

.split-dir-btn {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.4rem;
  width: 100%;
  border: none;
  border-radius: 9px;
  padding: 0.5rem 0.7rem;
  background: transparent;
  color: var(--vessel-text-primary);
  font-size: 0.84rem;
  font-weight: 550;
  text-align: left;
  cursor: pointer;
}

.split-dir-btn:hover {
  background: rgba(255, 255, 255, 0.06);
}

.split-dir-btn.is-active {
  background: color-mix(in srgb, var(--vessel-accent) 20%, transparent);
  color: var(--vessel-accent);
  font-weight: 700;
}

.split-dir-title {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
</style>
