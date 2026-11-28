<script setup lang="ts">
import { computed, onActivated, onMounted, ref, shallowRef, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import {
  AlertTriangle,
  AlignJustify,
  ArrowLeft,
  BookmarkPlus,
  Check,
  CheckCircle2,
  Cloud,
  Download,
  Layers,
  LayoutGrid,
  List,
  Loader2,
  MoreVertical,
  Play,
  RefreshCw,
  Search,
  Server,
  Star,
  Subtitles,
  Trash2,
  Wrench,
  X,
  Zap,
} from 'lucide-vue-next'
import { vesselApi } from '@/api/vessel'
import { formatBytes } from '@/composables/useAppUpdater'
import { useI18n } from '@/composables/useI18n'
import { useLibraryState } from '@/composables/useLibraryState'
import { usePlayerSession } from '@/composables/usePlayerSession'
import { useUiPreferences } from '@/composables/useUiPreferences'
import { pushToast, useVesselEvents } from '@/composables/useVesselEvents'
import { isLiveMediaIdentifier, isLiveTvPlugin } from '@/utils/liveTv'
import { resolvePreferredLiveStreamIndex } from '@/utils/liveStreamMemory'
import { extractYouTubeVideoId, youtubeAmbientEmbedUrl } from '@/utils/trailer'
import type {
  ContinueWatchingEntry,
  Episode,
  EpisodeListLayout,
  LibraryStatus,
  LinkedMediaSource,
  MediaDetail,
  MediaSummary,
  MetadataProviderFilter,
  MetadataSearchHit,
  PlaybackProgress,
  PluginListEntry,
  SkipSegment,
  StreamCandidate,
  SubtitleTrack,
} from '@/types/vessel'

const props = defineProps<{
  pluginId: string
  nativeId: string
}>()

const route = useRoute()
const router = useRouter()
const { t, currentLang } = useI18n()
const { getMediaStatus, setMediaStatus, refreshLibraryState } = useLibraryState()
const { setSession } = usePlayerSession()
const { uiPreferences, savePreferences } = useUiPreferences()
const { lastEnrichedDetail } = useVesselEvents()

const detail = ref<MediaDetail | null>(null)
const resumeEntry = ref<ContinueWatchingEntry | null>(null)
const mediaProgressList = ref<PlaybackProgress[]>([])
const selectedSeason = shallowRef<number>(1)
const loading = shallowRef(false)

const currentLibraryStatus = computed(() => getMediaStatus(props.pluginId, props.nativeId))

function findEpisodeProgress(ep: Episode): PlaybackProgress | null {
  return (
    mediaProgressList.value.find((p) => p.streamId === ep.streamId) ??
    mediaProgressList.value.find((p) => p.season === ep.season && p.episode === ep.episode) ??
    null
  )
}

function episodeProgressPercent(ep: Episode): number {
  const prog = findEpisodeProgress(ep)
  if (!prog) return 0
  if (prog.completed) return 100
  if (!prog.durationMs || prog.durationMs <= 0) return prog.positionMs > 0 ? 12 : 0
  return Math.min(99, Math.max(4, Math.round((prog.positionMs / prog.durationMs) * 100)))
}

// Fix Match Modal state (interactive 6-provider search + manual ID fallback)
const showFixMatchModal = shallowRef(false)
const fixProvider = shallowRef<'tmdb_tv' | 'tmdb_movie' | 'imdb' | 'simkl' | 'anilist' | 'mal' | 'kitsu'>('tmdb_tv')
const fixIdInput = shallowRef('')
const fixDisableEnrichment = shallowRef(false)
const savingFixMatch = shallowRef(false)
const fixSearchQuery = shallowRef('')
const fixSearchFilter = shallowRef<MetadataProviderFilter>('all')
const fixSearchLoading = shallowRef(false)
const fixSearchHits = ref<MetadataSearchHit[]>([])
const fixSearchNotes = ref<string[]>([])
const showManualFixId = shallowRef(false)

// Top-right 3-dot menu & linked external stream sources ("Include Other Sources")
const showMoreMenu = shallowRef(false)
const showIncludeSourcesModal = shallowRef(false)
const linkedSources = ref<LinkedMediaSource[]>([])
const availablePlugins = ref<PluginListEntry[]>([])
const includeSearchQuery = shallowRef('')
const activeTargetSearchPlugin = shallowRef<string | null>(null)
const targetPluginHits = ref<Record<string, MediaSummary[]>>({})
const targetPluginSearching = ref<Record<string, boolean>>({})
const savingLinkedSources = shallowRef(false)

// CloudStream-style source resolver modal state
const resolverEpisode = ref<Episode | null>(null)
const resolvingStreams = shallowRef(false)
const resolverError = shallowRef<string>('')
const streamChoices = ref<StreamCandidate[]>([])
const episodeSubtitles = ref<SubtitleTrack[]>([])
const episodeSkipSegments = ref<SkipSegment[]>([])
const resolvingTorrentKey = shallowRef('')
const debridPrompt = ref<
  | {
      kind: 'needs_confirmation'
      ep: Episode
      candidate: StreamCandidate
      action: 'play' | 'download'
      providerLabel: string
      target: string
      progressPct: number
    }
  | { kind: 'no_provider'; message: string }
  | null
>(null)
const queueingDebridCloud = shallowRef(false)

function cleanSynopsis(raw?: string): string {
  if (!raw) return ''
  return raw
    .replace(/<br\s*\/?>/gi, '\n')
    .replace(/<\/?[^>]+(>|$)/g, '')
    .replace(/\n{3,}/g, '\n\n')
    .trim()
}

const formattedSynopsis = computed(() => cleanSynopsis(detail.value?.synopsis))

const seasonNumbers = computed(() => {
  if (!detail.value) return []
  const set = new Set<number>(detail.value.episodes.map((e) => e.season))
  return Array.from(set).sort((a, b) => a - b)
})

const filteredEpisodes = computed(() => {
  if (!detail.value) return []
  if (seasonNumbers.value.length <= 1) return detail.value.episodes
  return detail.value.episodes.filter((ep) => ep.season === selectedSeason.value)
})

const firstEpisode = computed(() => filteredEpisodes.value[0] ?? detail.value?.episodes[0] ?? null)
const resumeEpisode = computed(() => {
  const entry = resumeEntry.value
  const eps = detail.value?.episodes ?? []
  if (!entry || !eps.length) return firstEpisode.value
  return (
    eps.find((ep) => ep.streamId === entry.streamId) ??
    eps.find((ep) => ep.season === entry.season && ep.episode === entry.episode) ??
    firstEpisode.value
  )
})
const ctaEpisode = computed(() => (resumeEntry.value ? resumeEpisode.value : firstEpisode.value))
const episodeLayout = computed(() => uiPreferences.value.details.episodeLayout)

watch(lastEnrichedDetail, (enriched) => {
  if (
    enriched &&
    enriched.pluginId === props.pluginId &&
    enriched.nativeId === props.nativeId
  ) {
    detail.value = enriched.detail
  }
})

const detailsTrailerReady = shallowRef(false)
const detailsTrailerEmbedUrl = computed(() => {
  if (uiPreferences.value.details.autoPlayTrailer === false || !detail.value) return ''
  const id = extractYouTubeVideoId(detail.value.trailerUrl)
  return id ? youtubeAmbientEmbedUrl(id) : ''
})
watch(detailsTrailerEmbedUrl, () => {
  detailsTrailerReady.value = false
})

async function setEpisodeLayout(layout: EpisodeListLayout): Promise<void> {
  const next = {
    ...uiPreferences.value,
    details: { ...uiPreferences.value.details, episodeLayout: layout },
  }
  try {
    await savePreferences(next)
  } catch (err) {
    pushToast('error', String(err))
  }
}

async function refreshResumeState(): Promise<void> {
  try {
    const [cw, epProgress] = await Promise.all([
      vesselApi.listContinueWatching(50),
      vesselApi.listMediaWatchProgress(props.pluginId, props.nativeId),
    ])
    mediaProgressList.value = epProgress
    const match = cw.find(
      (entry) =>
        entry.media.pluginId === props.pluginId &&
        entry.media.nativeId === props.nativeId,
    )
    resumeEntry.value = match ?? null
    if (match && detail.value?.episodes.some((ep) => ep.season === match.season)) {
      selectedSeason.value = match.season
    }
  } catch {
    resumeEntry.value = null
  }
}

const isLiveMedia = computed(() => {
  const p = availablePlugins.value.find((pl) => pl.manifest.id === props.pluginId) ?? null
  return isLiveMediaIdentifier(
    detail.value?.type,
    props.pluginId,
    detail.value?.episodes[0]?.streamId,
    props.nativeId,
    p,
  )
})

async function redirectLiveToPlayer(res: MediaDetail): Promise<void> {
  const ep = res.episodes[0] ?? {
    streamId: props.nativeId,
    season: 1,
    episode: 1,
    title: res.title,
    thumbnail: res.poster,
    tags: [],
  }
  const streamRes = await vesselApi.getStreams(props.pluginId, ep.streamId)
  if (!streamRes.streams.length) {
    pushToast('warn', t('details.no_streams'))
    router.back()
    return
  }
  const preferredIdx = await resolvePreferredLiveStreamIndex(
    props.pluginId,
    props.nativeId,
    ep.streamId,
    streamRes.streams,
  )
  setSession({
    pluginId: props.pluginId,
    nativeId: props.nativeId,
    detail: { ...res, type: 'live' },
    episode: ep,
    streams: streamRes.streams,
    subtitles: streamRes.subtitles ?? [],
    activeStreamIndex: preferredIdx,
  })
  await router.replace({
    name: 'player',
    params: { pluginId: props.pluginId, nativeId: props.nativeId },
    query: { streamId: ep.streamId, streamIndex: String(preferredIdx) },
  })
}

async function loadDetails(): Promise<void> {
  loading.value = true
  try {
    const isVirtualMeta = props.pluginId === 'vessel.metadata' || props.nativeId.startsWith('virtual::')
    const titleHint = typeof route.query.title === 'string' ? route.query.title : undefined
    const mediaTypeHint =
      route.query.mediaType === 'series' || route.query.mediaType === 'movie'
        ? route.query.mediaType
        : undefined
    const detailPromise = isVirtualMeta
      ? vesselApi.getVirtualMetadataDetail(
          props.nativeId.replace(/^virtual::/, ''),
          titleHint,
          mediaTypeHint,
          currentLang.value,
        )
      : vesselApi.getDetailsWithEnrichment(props.pluginId, props.nativeId, currentLang.value)

    const [res, linked, pluginsList] = await Promise.all([
      detailPromise,
      isVirtualMeta ? Promise.resolve([]) : vesselApi.getLinkedSources(props.pluginId, props.nativeId).catch(() => []),
      availablePlugins.value.length ? Promise.resolve(availablePlugins.value) : vesselApi.listPlugins().catch(() => []),
      refreshResumeState(),
      refreshLibraryState(),
    ])
    if (pluginsList.length) availablePlugins.value = pluginsList
    const plug = pluginsList.find((pl) => pl.manifest.id === props.pluginId) ?? null
    if (isLiveMediaIdentifier(res.type, props.pluginId, res.episodes[0]?.streamId, props.nativeId, plug)) {
      await redirectLiveToPlayer(res)
      return
    }
    detail.value = res
    linkedSources.value = linked
    if (res.episodes.length) {
      const match = resumeEntry.value
      selectedSeason.value =
        match && res.episodes.some((ep) => ep.season === match.season)
          ? match.season
          : res.episodes[0].season
    }
  } catch (err) {
    pushToast('error', String(err))
  } finally {
    loading.value = false
  }
}

async function saveLibrary(status: LibraryStatus): Promise<void> {
  if (!detail.value || !status) return
  try {
    await setMediaStatus({
      pluginId: props.pluginId,
      nativeId: props.nativeId,
      mediaType: detail.value.type,
      isAnime: detail.value.isAnime,
      title: detail.value.title,
      posterUrl: detail.value.poster,
      year: detail.value.year,
      status,
    })
    pushToast('info', `${detail.value.title} · ${t(`library.${status}`)}`)
  } catch (err) {
    pushToast('error', String(err))
  }
}

async function runFixMatchSearch(): Promise<void> {
  const q = fixSearchQuery.value.trim()
  if (!q) {
    fixSearchHits.value = []
    fixSearchNotes.value = []
    return
  }
  fixSearchLoading.value = true
  try {
    const res = await vesselApi.searchMetadata(q, fixSearchFilter.value, currentLang.value)
    fixSearchHits.value = res.hits
    fixSearchNotes.value = res.notes
  } catch (err) {
    fixSearchHits.value = []
    fixSearchNotes.value = [String(err)]
  } finally {
    fixSearchLoading.value = false
  }
}

async function applyMetadataHit(hit: MetadataSearchHit): Promise<void> {
  savingFixMatch.value = true
  try {
    await vesselApi.setMetadataOverride(props.pluginId, props.nativeId, hit.canonicalId, false)
    showFixMatchModal.value = false
    await loadDetails()
    pushToast('info', `${t('details.match_updated')}: ${hit.title}`)
  } catch (err) {
    pushToast('error', String(err))
  } finally {
    savingFixMatch.value = false
  }
}

function openFixMatchModal(): void {
  const ids = detail.value?.externalIds
  if (ids?.tmdb) {
    fixProvider.value = detail.value?.type === 'movie' ? 'tmdb_movie' : 'tmdb_tv'
    fixIdInput.value = String(ids.tmdb)
  } else if (ids?.anilist) {
    fixProvider.value = 'anilist'
    fixIdInput.value = String(ids.anilist)
  } else if (ids?.mal) {
    fixProvider.value = 'mal'
    fixIdInput.value = String(ids.mal)
  } else if (ids?.kitsu) {
    fixProvider.value = 'kitsu'
    fixIdInput.value = String(ids.kitsu)
  } else if (ids?.simkl) {
    fixProvider.value = 'simkl'
    fixIdInput.value = String(ids.simkl)
  } else if (ids?.imdb) {
    fixProvider.value = 'imdb'
    fixIdInput.value = ids.imdb
  } else {
    fixIdInput.value = ''
  }
  fixDisableEnrichment.value = false
  showManualFixId.value = false
  fixSearchQuery.value = detail.value?.title ?? ''
  fixSearchFilter.value = 'all'
  fixSearchHits.value = []
  fixSearchNotes.value = []
  showFixMatchModal.value = true
  void runFixMatchSearch()
}

function buildCanonicalId(): string | null {
  const raw = fixIdInput.value.trim()
  if (!raw) return null
  if (raw.includes(':')) return raw
  switch (fixProvider.value) {
    case 'anilist':
      return `anilist:${raw}`
    case 'mal':
      return `mal:${raw}`
    case 'kitsu':
      return `kitsu:${raw}`
    case 'simkl':
      return `simkl:${raw}`
    case 'tmdb_tv':
      return `tmdb:tv:${raw}`
    case 'tmdb_movie':
      return `tmdb:movie:${raw}`
    case 'imdb':
      return `imdb:${raw}`
  }
}

async function applyFixMatch(): Promise<void> {
  savingFixMatch.value = true
  try {
    const canonical = fixDisableEnrichment.value ? null : buildCanonicalId()
    await vesselApi.setMetadataOverride(
      props.pluginId,
      props.nativeId,
      canonical,
      fixDisableEnrichment.value,
    )
    showFixMatchModal.value = false
    await loadDetails()
    pushToast('info', fixDisableEnrichment.value ? t('details.match_disabled') : t('details.match_updated'))
  } catch (err) {
    pushToast('error', String(err))
  } finally {
    savingFixMatch.value = false
  }
}

const selectableSourcePlugins = computed(() => {
  return availablePlugins.value.filter(
    (p) =>
      p.enabled &&
      p.manifest.id !== props.pluginId &&
      !isLiveTvPlugin(p),
  )
})

function findLinkedFor(pluginId: string): LinkedMediaSource | undefined {
  return linkedSources.value.find((s) => s.pluginId === pluginId)
}

async function openIncludeSourcesModal(): Promise<void> {
  showMoreMenu.value = false
  includeSearchQuery.value = detail.value?.title ?? ''
  activeTargetSearchPlugin.value = null
  showIncludeSourcesModal.value = true
  try {
    const [all, currentLinked] = await Promise.all([
      vesselApi.listPlugins(),
      vesselApi.getLinkedSources(props.pluginId, props.nativeId),
    ])
    availablePlugins.value = all
    linkedSources.value = currentLinked
  } catch (err) {
    pushToast('error', String(err))
  }
}

async function searchInTargetPlugin(pluginId: string): Promise<void> {
  const q = includeSearchQuery.value.trim()
  if (!q) return
  targetPluginSearching.value = { ...targetPluginSearching.value, [pluginId]: true }
  activeTargetSearchPlugin.value = pluginId
  try {
    const page = await vesselApi.searchPluginSync(pluginId, q, 1)
    targetPluginHits.value = { ...targetPluginHits.value, [pluginId]: page.items }
  } catch (err) {
    pushToast('error', String(err))
  } finally {
    targetPluginSearching.value = { ...targetPluginSearching.value, [pluginId]: false }
  }
}

async function toggleLinkedPlugin(plugin: PluginListEntry): Promise<void> {
  const existing = findLinkedFor(plugin.manifest.id)
  if (existing) {
    savingLinkedSources.value = true
    try {
      linkedSources.value = await vesselApi.setLinkedSources(
        props.pluginId,
        props.nativeId,
        linkedSources.value.filter((s) => s.pluginId !== plugin.manifest.id),
      )
    } catch (err) {
      pushToast('error', String(err))
    } finally {
      savingLinkedSources.value = false
    }
    return
  }

  await searchInTargetPlugin(plugin.manifest.id)
  const hits = targetPluginHits.value[plugin.manifest.id] ?? []
  if (hits.length > 0) {
    await bindLinkedSourceHit(plugin, hits[0])
  } else {
    pushToast('warn', `${plugin.manifest.name}: ${t('common.empty')}`)
  }
}

async function bindLinkedSourceHit(plugin: PluginListEntry, item: MediaSummary): Promise<void> {
  savingLinkedSources.value = true
  try {
    const entry: LinkedMediaSource = {
      pluginId: plugin.manifest.id,
      pluginName: plugin.manifest.name,
      nativeId: item.nativeId,
      title: item.title,
      poster: item.poster,
    }
    const next = [...linkedSources.value.filter((s) => s.pluginId !== plugin.manifest.id), entry]
    linkedSources.value = await vesselApi.setLinkedSources(props.pluginId, props.nativeId, next)
    pushToast('info', `${plugin.manifest.name}: ${item.title}`)
  } catch (err) {
    pushToast('error', String(err))
  } finally {
    savingLinkedSources.value = false
  }
}

/** Opens the CloudStream-style source resolver sheet (merging primary + linked providers). */
async function openSourceResolver(ep: Episode): Promise<void> {
  resolverEpisode.value = ep
  resolvingStreams.value = true
  resolverError.value = ''
  streamChoices.value = []
  episodeSubtitles.value = []
  episodeSkipSegments.value = []

  try {
    const res = await vesselApi.getStreamsWithLinked({
      pluginId: props.pluginId,
      nativeId: props.nativeId,
      episodeId: ep.streamId,
      season: ep.season,
      episode: ep.episode,
      absoluteEpisode: ep.absoluteEpisode,
      isMovie: detail.value?.type === 'movie',
      externalIds: detail.value?.externalIds,
    })
    if (resolverEpisode.value?.streamId !== ep.streamId) return
    streamChoices.value = res.streams
    episodeSubtitles.value = res.subtitles
    episodeSkipSegments.value = res.skipSegments ?? []
    if (!res.streams.length) {
      resolverError.value = `No playable streams found from '${props.pluginId}' for Episode ${ep.episode}.`
    }
  } catch (err) {
    if (resolverEpisode.value?.streamId !== ep.streamId) return
    resolverError.value = String(err)
  } finally {
    if (resolverEpisode.value?.streamId === ep.streamId) {
      resolvingStreams.value = false
    }
  }
}

function closeSourceResolver(): void {
  resolverEpisode.value = null
  resolvingStreams.value = false
  resolverError.value = ''
  streamChoices.value = []
  episodeSubtitles.value = []
  episodeSkipSegments.value = []
}

function isTorrentCandidate(cand: StreamCandidate): boolean {
  return cand.kind === 'torrent' || cand.url.trim().toLowerCase().startsWith('magnet:?')
}

function inferResolvedKind(url: string): StreamCandidate['kind'] {
  const lower = url.toLowerCase()
  if (lower.includes('.m3u8')) return 'hls'
  if (lower.includes('.mkv')) return 'mkv'
  if (lower.includes('.mp4')) return 'mp4'
  if (lower.includes('.webm')) return 'webm'
  return 'auto'
}

async function resolveTorrentForAction(
  ep: Episode,
  candidate: StreamCandidate,
  action: 'play' | 'download',
): Promise<StreamCandidate | null> {
  if (!isTorrentCandidate(candidate)) return candidate
  resolvingTorrentKey.value = `${candidate.url}:${candidate.label}`
  try {
    const res = await vesselApi.resolveTorrentCandidate({
      url: candidate.url,
      torrent: candidate.torrent,
      preferredTarget: candidate.debridCache?.target,
      allowUncachedQueue: false,
      season: ep.season,
      episode: ep.episode,
    })
    if (res.status === 'ready') {
      return {
        ...candidate,
        url: res.url,
        kind: inferResolvedKind(res.url),
        headers: res.headers ?? {},
        torrent: candidate.torrent
          ? { ...candidate.torrent, sizeBytes: res.sizeBytes ?? candidate.torrent.sizeBytes }
          : undefined,
      }
    }
    if (res.status === 'needs_confirmation') {
      debridPrompt.value = {
        kind: 'needs_confirmation',
        ep,
        candidate,
        action,
        providerLabel: res.providerLabel,
        target: res.target,
        progressPct: res.progressPct,
      }
      return null
    }
    debridPrompt.value = {
      kind: 'no_provider',
      message: res.message,
    }
    return null
  } catch (err) {
    pushToast('error', String(err))
    return null
  } finally {
    resolvingTorrentKey.value = ''
  }
}

async function confirmDebridCloudQueue(): Promise<void> {
  if (!debridPrompt.value || debridPrompt.value.kind !== 'needs_confirmation') return
  const prompt = debridPrompt.value
  queueingDebridCloud.value = true
  try {
    const res = await vesselApi.resolveTorrentCandidate({
      url: prompt.candidate.url,
      torrent: prompt.candidate.torrent,
      preferredTarget: prompt.target,
      allowUncachedQueue: true,
      season: prompt.ep.season,
      episode: prompt.ep.episode,
    })
    if (res.status === 'ready') {
      debridPrompt.value = null
      const resolvedCand: StreamCandidate = {
        ...prompt.candidate,
        url: res.url,
        kind: inferResolvedKind(res.url),
        headers: res.headers ?? {},
      }
      if (prompt.action === 'download') {
        await downloadCandidate(prompt.ep, resolvedCand)
      } else {
        await startStream(prompt.ep, resolvedCand)
      }
      return
    }
    if (res.status === 'needs_confirmation') {
      pushToast(
        'info',
        `${res.providerLabel}: ${res.cloudStatus.toUpperCase()} (${Math.round(res.progressPct)}%). Track progress in Settings > Debrid.`,
      )
      debridPrompt.value = null
    }
  } catch (err) {
    pushToast('error', String(err))
  } finally {
    queueingDebridCloud.value = false
  }
}

async function startStream(ep: Episode, candidate: StreamCandidate): Promise<void> {
  if (!detail.value) return
  const resolvedCandidate = await resolveTorrentForAction(ep, candidate, 'play')
  if (!resolvedCandidate) return
  const rawIndex = streamChoices.value.findIndex((item) => item.url === candidate.url && item.label === candidate.label)
  const candidateIndex = Math.max(0, rawIndex)
  const nextStreams = streamChoices.value.length ? [...streamChoices.value] : [resolvedCandidate]
  if (rawIndex >= 0) {
    nextStreams[rawIndex] = resolvedCandidate
  }
  setSession({
    pluginId: props.pluginId,
    nativeId: props.nativeId,
    detail: detail.value,
    episode: ep,
    streams: nextStreams,
    subtitles: [...episodeSubtitles.value],
    skipSegments: [...episodeSkipSegments.value],
    activeStreamIndex: candidateIndex,
  })
  closeSourceResolver()
  await router.push({
    name: 'player',
    params: {
      pluginId: props.pluginId,
      nativeId: props.nativeId,
    },
    query: {
      streamId: ep.streamId,
      streamIndex: String(candidateIndex),
    },
  })
}

async function downloadCandidate(ep: Episode, candidate: StreamCandidate): Promise<void> {
  if (!detail.value) return
  const resolvedCandidate = await resolveTorrentForAction(ep, candidate, 'download')
  if (!resolvedCandidate) return
  const isMovie = detail.value.type === 'movie'
  try {
    await vesselApi.enqueueDownload({
      pluginId: props.pluginId,
      nativeId: props.nativeId,
      title: isMovie ? detail.value.title : `${detail.value.title} — S${ep.season}E${ep.episode}`,
      season: isMovie ? undefined : ep.season,
      episode: isMovie ? undefined : ep.episode,
      streamLabel: resolvedCandidate.label,
      kind: resolvedCandidate.kind === 'hls' ? 'hls' : 'direct',
      sourceUrl: resolvedCandidate.url,
      headers: resolvedCandidate.headers,
      subtitles: [...resolvedCandidate.subtitles, ...episodeSubtitles.value],
    })
    pushToast(
      'info',
      isMovie
        ? `Queued download: ${detail.value.title}`
        : `Queued download: S${ep.season}E${ep.episode}`,
    )
  } catch (err) {
    pushToast('error', String(err))
  }
}

onMounted(() => {
  void loadDetails()
})

onActivated(() => {
  if (detail.value) {
    void refreshResumeState()
  }
})
</script>

<template>
  <section class="nuvio-details">
    <Transition name="page-fade" mode="out-in">
      <div v-if="loading" key="details-loading" class="hero-skeleton-wrap">
      <div class="details-sk-hero">
        <div class="v-skeleton details-sk-poster" />
        <div class="details-sk-meta">
          <div class="details-sk-chips">
            <span class="v-skeleton details-sk-chip" />
            <span class="v-skeleton details-sk-chip sm" />
            <span class="v-skeleton details-sk-chip sm" />
          </div>
          <div class="v-skeleton details-sk-title" />
          <div class="v-skeleton details-sk-sub" />
          <div class="v-skeleton details-sk-para" />
          <div class="v-skeleton details-sk-para short" />
          <div class="details-sk-chips" style="margin-top: 0.5rem;">
            <span class="v-skeleton details-sk-btn" />
            <span class="v-skeleton details-sk-btn secondary" />
          </div>
        </div>
      </div>

      <div class="details-sk-eps">
        <div v-for="n in 6" :key="n" class="details-sk-ep-card">
          <div class="v-skeleton details-sk-ep-thumb" />
          <div class="v-skeleton details-sk-ep-line" />
          <div class="v-skeleton details-sk-ep-line short" />
        </div>
      </div>
    </div>

    <div v-else-if="detail" key="details-loaded" class="details-loaded-shell">
      <!-- Dissolving Full-Bleed Atmospheric Backdrop extending past Hero -->
      <div
        v-if="(uiPreferences.details.showBackdrop && (detail.backdrop || detail.poster)) || detailsTrailerEmbedUrl"
        class="hero-backdrop-layer"
        aria-hidden="true"
      >
        <img
          v-if="uiPreferences.details.showBackdrop && (detail.backdrop || detail.poster)"
          :src="detail.backdrop || detail.poster"
          :alt="detail.title"
          width="1440"
          height="680"
          fetchpriority="high"
          class="hero-backdrop"
        />
        <iframe
          v-if="detailsTrailerEmbedUrl"
          :key="detailsTrailerEmbedUrl"
          :src="detailsTrailerEmbedUrl"
          title="Trailer"
          tabindex="-1"
          aria-hidden="true"
          allow="autoplay; encrypted-media"
          referrerpolicy="strict-origin-when-cross-origin"
          class="hero-backdrop-trailer"
          :class="{ 'is-ready': detailsTrailerReady }"
          @load="detailsTrailerReady = true"
        />
        <div
          class="hero-scrim"
          :style="{
            '--scrim-dim': `${Math.max(58, uiPreferences.details.backdropDimPercent) / 100}`,
          }"
        />
      </div>

      <!-- Deep Cinematic Nuvio Hero Stage -->
      <header class="detail-hero">

        <div class="hero-nav-row">
          <button type="button" class="v-btn v-btn-sm" @click="router.back()">
            <ArrowLeft :size="14" aria-hidden="true" /> {{ t('common.back') }}
          </button>

          <div class="hero-top-actions">
            <button
              v-if="linkedSources.length"
              type="button"
              class="v-btn v-btn-sm"
              @click="openIncludeSourcesModal"
            >
              <Layers :size="14" aria-hidden="true" />
              +{{ linkedSources.length }} {{ t('details.linked_sources_badge') }}
            </button>

            <div class="more-menu-wrap">
              <button
                type="button"
                class="v-btn v-btn-icon v-btn-sm"
                :aria-label="t('details.more_actions')"
                @click="showMoreMenu = !showMoreMenu"
              >
                <MoreVertical :size="16" aria-hidden="true" />
              </button>
              <div v-if="showMoreMenu" class="v-card hero-more-dropdown">
                <button type="button" class="hero-menu-item" @click="openIncludeSourcesModal">
                  <Layers :size="14" aria-hidden="true" />
                  <span>{{ t('details.include_sources') }}</span>
                </button>
                <button
                  v-if="uiPreferences.details.showFixMatch"
                  type="button"
                  class="hero-menu-item"
                  @click="showMoreMenu = false; openFixMatchModal()"
                >
                  <Wrench :size="14" aria-hidden="true" />
                  <span>{{ t('details.fix_match') }}</span>
                </button>
              </div>
            </div>
          </div>
        </div>

        <div class="hero-stage">
          <img
            v-if="detail.poster"
            :src="detail.poster"
            :alt="detail.title"
            width="216"
            height="324"
            class="hero-poster"
          />

          <div class="hero-meta-col">
            <div class="meta-line">
              <span class="source-tag">{{ pluginId }}</span>
              <span v-if="detail.year">{{ detail.year }}</span>
              <span v-if="detail.rating !== undefined" class="rating-text">
                <Star :size="13" fill="currentColor" aria-hidden="true" />
                {{ detail.rating.toFixed(1) }}
              </span>
              <span v-if="detail.status">{{ detail.status }}</span>
              <template v-if="uiPreferences.details.showExternalIdBadges">
                <span v-if="detail.externalIds.anilist">AniList #{{ detail.externalIds.anilist }}</span>
                <span v-if="detail.externalIds.mal">MAL #{{ detail.externalIds.mal }}</span>
                <span v-if="detail.externalIds.kitsu">Kitsu #{{ detail.externalIds.kitsu }}</span>
                <span v-if="detail.externalIds.tmdb">TMDB #{{ detail.externalIds.tmdb }}</span>
                <span v-if="detail.externalIds.imdb">IMDb {{ detail.externalIds.imdb }}</span>
                <span v-if="detail.externalIds.simkl">Simkl #{{ detail.externalIds.simkl }}</span>
              </template>
            </div>

            <div class="title-block">
              <h1 class="detail-title">{{ detail.title }}</h1>
              <p
                v-if="detail.originalTitle && detail.originalTitle !== detail.title"
                class="orig-title v-muted"
              >
                {{ detail.originalTitle }}
              </p>
            </div>

            <div v-if="detail.genres.length" class="genres-line">
              <span v-for="genre in detail.genres.slice(0, 6)" :key="genre" class="v-badge">
                {{ genre }}
              </span>
            </div>

            <p v-if="formattedSynopsis" class="synopsis">{{ formattedSynopsis }}</p>

            <div class="cta-row">
              <button
                v-if="ctaEpisode"
                type="button"
                class="v-btn v-btn-primary"
                @click="openSourceResolver(ctaEpisode)"
              >
                <Play :size="16" fill="currentColor" aria-hidden="true" />
                {{ resumeEntry && resumeEntry.positionMs > 0 ? t('details.resume') : t('details.watch') }}
                <template v-if="detail.type !== 'movie'">
                  S{{ ctaEpisode.season }}:E{{ ctaEpisode.episode }}
                </template>
              </button>

              <div class="library-picker">
                <BookmarkPlus :size="15" class="picker-icon" aria-hidden="true" />
                <select
                  aria-label="Library status"
                  class="v-select status-select"
                  :value="currentLibraryStatus ?? ''"
                  @change="saveLibrary(($event.target as HTMLSelectElement).value as LibraryStatus)"
                >
                  <option value="" disabled>{{ t('details.library_status') }}</option>
                  <option value="watching">{{ t('library.watching') }}</option>
                  <option value="plan_to_watch">{{ t('library.plan_to_watch') }}</option>
                  <option value="completed">{{ t('library.completed') }}</option>
                  <option value="on_hold">{{ t('library.on_hold') }}</option>
                  <option value="dropped">{{ t('library.dropped') }}</option>
                </select>
              </div>
            </div>
          </div>
        </div>
      </header>

      <!-- Nuvio Episodes Section with Layout Switcher -->
      <div class="episodes-stage">
        <div class="episodes-head">
          <div class="ep-head-title">
            <h2>{{ t('details.episodes') }}</h2>
            <span class="v-badge">{{ filteredEpisodes.length }}</span>
          </div>

          <div class="ep-head-controls">
            <div v-if="seasonNumbers.length > 1" class="season-tabs">
              <button
                v-for="s in seasonNumbers"
                :key="s"
                type="button"
                class="v-btn v-btn-sm"
                :class="{ 'v-btn-primary': selectedSeason === s }"
                @click="selectedSeason = s"
              >
                {{ t('details.season') }} {{ s }}
              </button>
            </div>

            <div class="layout-switcher" role="group" aria-label="Episode layout mode">
              <button
                type="button"
                class="layout-btn"
                :class="{ 'is-active': episodeLayout === 'grid_cards' }"
                aria-label="Grid cards layout"
                title="Grid Cards"
                @click="setEpisodeLayout('grid_cards')"
              >
                <LayoutGrid :size="15" aria-hidden="true" />
              </button>
              <button
                type="button"
                class="layout-btn"
                :class="{ 'is-active': episodeLayout === 'detailed_list' }"
                aria-label="Detailed list layout"
                title="Detailed List"
                @click="setEpisodeLayout('detailed_list')"
              >
                <List :size="15" aria-hidden="true" />
              </button>
              <button
                type="button"
                class="layout-btn"
                :class="{ 'is-active': episodeLayout === 'compact_rows' }"
                aria-label="Compact rows layout"
                title="Compact Rows"
                @click="setEpisodeLayout('compact_rows')"
              >
                <AlignJustify :size="15" aria-hidden="true" />
              </button>
            </div>
          </div>
        </div>

        <div v-if="!filteredEpisodes.length" class="v-card v-muted">
          {{ t('details.no_episodes') }}
        </div>

        <div v-else class="episodes-container" :class="`mode-${episodeLayout}`">
          <article
            v-for="ep in filteredEpisodes"
            :key="ep.streamId"
            class="nuvio-ep-item"
          >
            <button
              v-if="episodeLayout !== 'compact_rows'"
              type="button"
              class="ep-thumb-btn"
              :aria-label="`Find sources for Episode ${ep.episode}`"
              @click="openSourceResolver(ep)"
            >
              <img
                v-if="!uiPreferences.details.hideEpisodeThumbnails && (ep.thumbnail || detail.backdrop || detail.poster)"
                :src="ep.thumbnail || detail.backdrop || detail.poster"
                :alt="`Episode ${ep.episode}`"
                width="320"
                height="180"
                loading="lazy"
                class="ep-thumb-img"
              />
              <div class="ep-thumb-scrim" aria-hidden="true">
                <span class="ep-play-circle">
                  <Play :size="16" fill="currentColor" />
                </span>
              </div>
              <span v-if="detail.type !== 'movie'" class="ep-corner-tag">
                S{{ ep.season }}·E{{ ep.episode }}
              </span>
              <span
                v-if="findEpisodeProgress(ep)?.completed"
                class="ep-watched-corner"
                :title="t('details.watched_badge')"
              >
                <CheckCircle2 :size="15" aria-hidden="true" />
              </span>
              <div v-if="episodeProgressPercent(ep) > 0" class="ep-thumb-progress">
                <div
                  class="ep-thumb-progress-fill"
                  :class="{ 'is-done': findEpisodeProgress(ep)?.completed }"
                  :style="{ width: `${episodeProgressPercent(ep)}%` }"
                />
              </div>
            </button>

            <button type="button" class="ep-content-btn" @click="openSourceResolver(ep)">
              <span class="ep-title">
                <span v-if="episodeLayout === 'compact_rows' && detail.type !== 'movie'" class="compact-num">
                  E{{ String(ep.episode).padStart(2, '0') }}
                </span>
                <template v-else-if="detail.type !== 'movie'">{{ ep.episode }}. </template>
                {{ ep.title?.trim() || (detail.type === 'movie' ? detail.title : `Episode ${ep.episode}`) }}
                <CheckCircle2
                  v-if="findEpisodeProgress(ep)?.completed"
                  :size="15"
                  class="ep-watched-inline"
                  :title="t('details.watched_badge')"
                  aria-hidden="true"
                />
              </span>
              <span
                v-if="ep.overview && episodeLayout !== 'compact_rows' && !uiPreferences.details.hideEpisodeOverviews"
                class="ep-overview v-muted"
              >
                {{ ep.overview }}
              </span>
              <div v-if="episodeProgressPercent(ep) > 0" class="ep-progress-row">
                <div class="ep-progress-track">
                  <div
                    class="ep-progress-fill"
                    :class="{ 'is-done': findEpisodeProgress(ep)?.completed }"
                    :style="{ width: `${episodeProgressPercent(ep)}%` }"
                  />
                </div>
                <span
                  class="ep-progress-label"
                  :class="{ 'is-done': findEpisodeProgress(ep)?.completed }"
                >
                  {{
                    findEpisodeProgress(ep)?.completed
                      ? t('details.watched_badge')
                      : `${episodeProgressPercent(ep)}%`
                  }}
                </span>
              </div>
            </button>

            <div class="ep-controls">
              <button
                type="button"
                class="v-btn v-btn-sm"
                :aria-label="`Find sources for Episode ${ep.episode}`"
                @click="openSourceResolver(ep)"
              >
                <Server :size="14" aria-hidden="true" /> {{ t('details.sources') }}
              </button>
            </div>
          </article>
        </div>
      </div>

      <!-- Fix Match Modal (6-provider title search + manual ID fallback) -->
      <Teleport to="body">
        <Transition name="modal-pop">
          <div
            v-if="showFixMatchModal"
            class="stream-backdrop"
            role="dialog"
            aria-modal="true"
            :aria-label="t('details.fix_match')"
          >
            <div class="v-card stream-modal fix-modal-wide">
              <div class="picker-head">
                <div class="picker-title-group">
                  <h3 class="picker-title">{{ t('details.fix_match') }}</h3>
                  <span class="v-muted picker-sub">
                    {{ t('details.fix_match_desc') }}
                  </span>
                </div>
                <button
                  type="button"
                  class="v-btn v-btn-icon"
                  aria-label="Close Fix Match dialog"
                  @click="showFixMatchModal = false"
                >
                  <X :size="16" aria-hidden="true" />
                </button>
              </div>

              <div class="fix-search-bar">
                <input
                  v-model="fixSearchQuery"
                  type="search"
                  class="v-input"
                  :placeholder="t('details.match_search_placeholder')"
                  @keydown.enter.prevent="runFixMatchSearch"
                />
                <button
                  type="button"
                  class="v-btn v-btn-primary"
                  :disabled="fixSearchLoading"
                  @click="runFixMatchSearch"
                >
                  <Loader2 v-if="fixSearchLoading" :size="14" class="spin" aria-hidden="true" />
                  <Search v-else :size="14" aria-hidden="true" />
                  {{ t('common.search') }}
                </button>
              </div>

              <div class="fix-provider-chips">
                <button
                  v-for="opt in [
                    { key: 'all', label: t('common.all') },
                    { key: 'tmdb', label: 'TMDB' },
                    { key: 'imdb', label: 'IMDb' },
                    { key: 'simkl', label: 'Simkl' },
                    { key: 'anilist', label: 'AniList' },
                    { key: 'mal', label: 'MyAnimeList' },
                    { key: 'kitsu', label: 'Kitsu' },
                  ]"
                  :key="opt.key"
                  type="button"
                  class="v-btn v-btn-sm"
                  :class="{ 'v-btn-primary': fixSearchFilter === opt.key }"
                  @click="fixSearchFilter = opt.key as MetadataProviderFilter; void runFixMatchSearch()"
                >
                  {{ opt.label }}
                </button>
              </div>

              <div v-if="fixSearchNotes.length" class="fix-notes-box">
                <p v-for="(n, idx) in fixSearchNotes" :key="idx" class="v-muted fix-note-line">{{ n }}</p>
              </div>

              <div v-if="fixSearchLoading" class="resolver-state compact">
                <Loader2 :size="22" class="spin accent-icon" aria-hidden="true" />
                <span class="v-muted">{{ t('search.searching') }}</span>
              </div>

              <div v-else-if="fixSearchHits.length" class="fix-hits-list">
                <button
                  v-for="hit in fixSearchHits"
                  :key="hit.canonicalId"
                  type="button"
                  class="fix-hit-card"
                  :disabled="savingFixMatch"
                  @click="applyMetadataHit(hit)"
                >
                  <img
                    v-if="hit.poster"
                    :src="hit.poster"
                    :alt="hit.title"
                    loading="lazy"
                    class="fix-hit-poster"
                  />
                  <div v-else class="fix-hit-poster placeholder" />
                  <div class="fix-hit-body">
                    <div class="fix-hit-top">
                      <strong class="fix-hit-title">{{ hit.title }}</strong>
                      <span class="v-badge v-badge-accent">{{ hit.provider.toUpperCase() }}</span>
                    </div>
                    <div class="v-muted fix-hit-meta">
                      <span v-if="hit.year">{{ hit.year }}</span>
                      <span v-if="hit.mediaType">· {{ hit.mediaType }}</span>
                      <span>· <code>{{ hit.canonicalId }}</code></span>
                    </div>
                    <p v-if="hit.synopsis" class="v-muted fix-hit-syn">{{ hit.synopsis }}</p>
                  </div>
                </button>
              </div>

              <div v-else class="v-muted fix-empty-hits">
                {{ t('details.match_no_hits') }}
              </div>

              <div class="manual-id-section">
                <button
                  type="button"
                  class="v-btn v-btn-sm"
                  @click="showManualFixId = !showManualFixId"
                >
                  {{ t('details.match_manual_toggle') }}
                </button>

                <div v-if="showManualFixId" class="fix-form">
                  <div class="fix-row">
                    <select v-model="fixProvider" aria-label="Metadata source" class="v-select fix-provider">
                      <option value="tmdb_tv">TMDB TV</option>
                      <option value="tmdb_movie">TMDB Movie</option>
                      <option value="imdb">IMDb</option>
                      <option value="simkl">Simkl</option>
                      <option value="anilist">AniList</option>
                      <option value="mal">MyAnimeList</option>
                      <option value="kitsu">Kitsu</option>
                    </select>
                    <input
                      v-model="fixIdInput"
                      name="canonicalId"
                      autocomplete="off"
                      aria-label="External metadata ID"
                      class="v-input"
                      placeholder="e.g. 1399, 918, tt0944947…"
                    />
                  </div>
                </div>
              </div>

              <label class="v-switch">
                <input v-model="fixDisableEnrichment" type="checkbox" class="v-switch-input" />
                <span class="v-switch-track" aria-hidden="true" />
                <span>{{ t('details.fix_match_disable') }}</span>
              </label>

              <div class="fix-actions">
                <button type="button" class="v-btn" @click="showFixMatchModal = false">
                  {{ t('common.cancel') }}
                </button>
                <button
                  v-if="showManualFixId || fixDisableEnrichment"
                  type="button"
                  class="v-btn v-btn-primary"
                  :disabled="savingFixMatch"
                  @click="applyFixMatch"
                >
                  {{ savingFixMatch ? t('common.loading') : t('details.fix_match_apply') }}
                </button>
              </div>
            </div>
          </div>
        </Transition>
      </Teleport>

      <!-- Include Streams From Other Sources Modal -->
      <Teleport to="body">
        <Transition name="modal-pop">
          <div
            v-if="showIncludeSourcesModal"
            class="stream-backdrop"
            role="dialog"
            aria-modal="true"
            :aria-label="t('details.include_sources')"
          >
            <div class="v-card stream-modal fix-modal-wide">
              <div class="picker-head">
                <div class="picker-title-group">
                  <h3 class="picker-title">{{ t('details.include_sources') }}</h3>
                  <span class="v-muted picker-sub">
                    {{ t('details.include_sources_desc') }}
                  </span>
                </div>
                <button
                  type="button"
                  class="v-btn v-btn-icon"
                  aria-label="Close Include Sources dialog"
                  @click="showIncludeSourcesModal = false"
                >
                  <X :size="16" aria-hidden="true" />
                </button>
              </div>

              <div class="fix-search-bar">
                <input
                  v-model="includeSearchQuery"
                  type="search"
                  class="v-input"
                  :placeholder="t('details.include_search_placeholder')"
                />
              </div>

              <div class="linked-plugins-list">
                <div
                  v-for="plug in selectableSourcePlugins"
                  :key="plug.manifest.id"
                  class="linked-plugin-item"
                >
                  <div class="linked-plugin-row">
                    <div class="linked-plugin-info">
                      <strong>{{ plug.manifest.name }}</strong>
                      <span v-if="findLinkedFor(plug.manifest.id)" class="v-muted linked-match-line">
                        {{ t('details.linked_to') }}: <strong>{{ findLinkedFor(plug.manifest.id)?.title }}</strong>
                      </span>
                    </div>
                    <div class="linked-plugin-btns">
                      <button
                        v-if="findLinkedFor(plug.manifest.id)"
                        type="button"
                        class="v-btn v-btn-sm"
                        :disabled="targetPluginSearching[plug.manifest.id]"
                        @click="searchInTargetPlugin(plug.manifest.id)"
                      >
                        <Search :size="13" aria-hidden="true" />
                        {{ t('details.change_linked_match') }}
                      </button>
                      <button
                        type="button"
                        class="v-btn v-btn-sm"
                        :class="findLinkedFor(plug.manifest.id) ? 'v-btn-danger' : 'v-btn-primary'"
                        :disabled="savingLinkedSources || targetPluginSearching[plug.manifest.id]"
                        @click="toggleLinkedPlugin(plug)"
                      >
                        <Loader2
                          v-if="targetPluginSearching[plug.manifest.id]"
                          :size="13"
                          class="spin"
                          aria-hidden="true"
                        />
                        <Trash2
                          v-else-if="findLinkedFor(plug.manifest.id)"
                          :size="13"
                          aria-hidden="true"
                        />
                        <Check v-else :size="13" aria-hidden="true" />
                        {{ findLinkedFor(plug.manifest.id) ? t('common.remove') : t('details.include_btn') }}
                      </button>
                    </div>
                  </div>

                  <div
                    v-if="activeTargetSearchPlugin === plug.manifest.id && (targetPluginHits[plug.manifest.id]?.length ?? 0) > 0"
                    class="linked-hits-strip"
                  >
                    <button
                      v-for="cand in targetPluginHits[plug.manifest.id]"
                      :key="cand.nativeId"
                      type="button"
                      class="v-btn v-btn-sm"
                      :class="{ 'v-btn-primary': findLinkedFor(plug.manifest.id)?.nativeId === cand.nativeId }"
                      @click="bindLinkedSourceHit(plug, cand)"
                    >
                      {{ cand.title }}
                    </button>
                  </div>
                </div>
                <div v-if="!selectableSourcePlugins.length" class="v-muted fix-empty-hits">
                  {{ t('common.empty') }}
                </div>
              </div>

              <div class="fix-actions">
                <button type="button" class="v-btn v-btn-primary" @click="showIncludeSourcesModal = false">
                  {{ t('common.close') }}
                </button>
              </div>
            </div>
          </div>
        </Transition>
      </Teleport>

      <!-- CloudStream-style Source Resolver & Picker Modal -->
      <Teleport to="body">
        <Transition name="modal-pop">
          <div
            v-if="resolverEpisode"
            class="stream-backdrop"
            role="dialog"
            aria-modal="true"
            aria-label="Stream source resolver"
          >
            <div class="v-card stream-modal">
              <div class="picker-head">
                <div class="picker-title-group">
                  <h3 class="picker-title">
                    S{{ resolverEpisode.season }}:E{{ resolverEpisode.episode }} —
                    {{ resolverEpisode.title ?? `Episode ${resolverEpisode.episode}` }}
                  </h3>
                  <span class="v-muted picker-sub">
                    Provider: <strong>{{ pluginId }}</strong>
                  </span>
                </div>
                <button
                  type="button"
                  class="v-btn v-btn-icon"
                  aria-label="Close source resolver"
                  @click="closeSourceResolver"
                >
                  <X :size="16" aria-hidden="true" />
                </button>
              </div>

              <div v-if="resolvingStreams" class="resolver-state">
                <Loader2 :size="28" class="spin accent-icon" aria-hidden="true" />
                <strong>{{ t('details.searching_streams') }}</strong>
                <span class="v-muted">{{ pluginId }}</span>
              </div>

              <div v-else-if="resolverError || !streamChoices.length" class="resolver-state">
                <AlertTriangle :size="28" class="warn-icon" aria-hidden="true" />
                <strong>{{ t('details.no_streams') }}</strong>
                <p class="v-muted resolver-err-msg">
                  {{ resolverError || t('details.no_streams') }}
                </p>
                <div class="resolver-err-actions">
                  <button
                    type="button"
                    class="v-btn v-btn-sm"
                    @click="openSourceResolver(resolverEpisode)"
                  >
                    <RefreshCw :size="14" aria-hidden="true" /> {{ t('common.retry') }}
                  </button>
                  <button type="button" class="v-btn v-btn-sm" @click="closeSourceResolver">
                    {{ t('common.cancel') }}
                  </button>
                </div>
              </div>

              <div v-else class="stream-list">
                <div
                  v-for="(cand, idx) in streamChoices"
                  :key="idx"
                  class="cs-source-row"
                >
                  <button
                    type="button"
                    class="cs-source-play"
                    @click="startStream(resolverEpisode, cand)"
                  >
                    <span class="cs-play-badge" aria-hidden="true">
                      <Play :size="14" fill="currentColor" />
                    </span>
                    <div class="cs-source-info">
                      <span class="stream-label">{{ cand.label }}</span>
                      <div class="cs-source-tags">
                        <span v-if="cand.pluginName || cand.pluginId" class="v-badge v-badge-accent">
                          {{ cand.pluginName || cand.pluginId }}
                        </span>
                        <span class="v-badge v-badge-accent">{{ cand.kind.toUpperCase() }}</span>
                        <span v-if="cand.quality" class="v-badge">{{ cand.quality }}</span>
                        <span
                          v-if="cand.debridCache?.state === 'cached'"
                          class="v-badge v-badge-success"
                        >
                          <Zap :size="11" aria-hidden="true" /> {{ cand.debridCache.label }} {{ t('debrid.cached_badge') }}
                        </span>
                        <span
                          v-else-if="cand.debridCache?.state === 'uncached'"
                          class="v-badge v-badge-warn"
                        >
                          <Cloud :size="11" aria-hidden="true" /> {{ t('debrid.uncached_badge') }}
                        </span>
                        <span v-if="cand.torrent?.sizeBytes" class="v-badge">
                          {{ formatBytes(cand.torrent.sizeBytes) }}
                        </span>
                        <span v-if="cand.torrent?.seeders !== undefined" class="v-badge">
                          👤 {{ cand.torrent.seeders }}
                        </span>
                        <span v-if="cand.torrent?.trackerSource" class="v-badge">
                          {{ cand.torrent.trackerSource }}
                        </span>
                        <span v-if="cand.torrent?.hdr" class="v-badge v-badge-warn">
                          {{ cand.torrent.hdr }}
                        </span>
                        <span v-if="cand.audioType" class="v-badge v-badge-warn">
                          {{ cand.audioType.toUpperCase() }}
                        </span>
                        <span
                          v-if="cand.subtitles.length || episodeSubtitles.length"
                          class="v-badge v-badge-success"
                        >
                          <Subtitles :size="11" aria-hidden="true" />
                          {{ cand.subtitles.length + episodeSubtitles.length }}
                        </span>
                      </div>
                    </div>
                  </button>

                  <button
                    v-if="!isLiveMedia"
                    type="button"
                    class="v-btn v-btn-icon"
                    :aria-label="`Download ${cand.label}`"
                    title="Download source"
                    @click="downloadCandidate(resolverEpisode, cand)"
                  >
                    <Download :size="15" aria-hidden="true" />
                  </button>
                </div>
              </div>
            </div>
          </div>
        </Transition>
      </Teleport>

      <!-- Debrid Confirmation / Missing Provider Modal -->
      <Teleport to="body">
        <Transition name="modal-pop">
          <div
            v-if="debridPrompt"
            class="stream-backdrop"
            role="dialog"
            aria-modal="true"
            aria-label="Debrid confirmation"
          >
            <div class="v-card fix-modal">
              <div class="picker-head">
                <h3 class="picker-title">
                  {{
                    debridPrompt.kind === 'needs_confirmation'
                      ? t('debrid.uncached_title')
                      : t('debrid.no_provider_title')
                  }}
                </h3>
                <button type="button" class="v-btn v-btn-icon" @click="debridPrompt = null">
                  <X :size="16" />
                </button>
              </div>
              <p class="v-muted" style="margin: 0.5rem 0 1rem">
                {{
                  debridPrompt.kind === 'needs_confirmation'
                    ? t('debrid.uncached_desc', { provider: debridPrompt.providerLabel })
                    : debridPrompt.message
                }}
              </p>
              <div class="fix-actions">
                <button type="button" class="v-btn" @click="debridPrompt = null">
                  {{ t('common.cancel') }}
                </button>
                <button
                  v-if="debridPrompt.kind === 'needs_confirmation'"
                  type="button"
                  class="v-btn v-btn-primary"
                  :disabled="queueingDebridCloud"
                  @click="confirmDebridCloudQueue"
                >
                  <Loader2 v-if="queueingDebridCloud" :size="14" class="spin" />
                  {{ t('debrid.queue_cloud') }}
                </button>
                <button
                  v-else
                  type="button"
                  class="v-btn v-btn-primary"
                  @click="debridPrompt = null; closeSourceResolver(); router.push({ name: 'settings' })"
                >
                  {{ t('debrid.open_debrid_settings') }}
                </button>
              </div>
            </div>
          </div>
        </Transition>
      </Teleport>
    </div>
    </Transition>
  </section>
</template>

<style scoped>
.nuvio-details,
.details-loaded-shell {
  position: relative;
  display: flex;
  flex-direction: column;
  gap: 1.5rem;
}

.hero-backdrop-layer {
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
  height: min(82vh, 720px);
  overflow: hidden;
  pointer-events: none;
  z-index: 0;
  -webkit-mask-image: linear-gradient(180deg, rgba(0, 0, 0, 1) 0%, rgba(0, 0, 0, 0.78) 54%, transparent 100%);
  mask-image: linear-gradient(180deg, rgba(0, 0, 0, 1) 0%, rgba(0, 0, 0, 0.78) 54%, transparent 100%);
}

.hero-skeleton-wrap {
  padding: 2rem 2.75rem;
  display: flex;
  flex-direction: column;
  gap: 2.25rem;
}

.details-sk-hero {
  display: flex;
  gap: 2.25rem;
  align-items: flex-end;
  flex-wrap: wrap;
}

.details-sk-poster {
  width: 216px;
  aspect-ratio: 2 / 3;
  flex-shrink: 0;
}

.details-sk-meta {
  flex: 1;
  min-width: 280px;
  display: flex;
  flex-direction: column;
  gap: 0.85rem;
}

.details-sk-chips {
  display: flex;
  gap: 0.6rem;
}

.details-sk-chip {
  width: 92px;
  height: 22px;
  border-radius: 6px;
}

.details-sk-chip.sm {
  width: 56px;
}

.details-sk-title {
  width: min(420px, 75vw);
  height: 40px;
  border-radius: 10px;
}

.details-sk-sub {
  width: min(260px, 55vw);
  height: 18px;
  border-radius: 6px;
}

.details-sk-para {
  width: min(640px, 90%);
  height: 15px;
  border-radius: 6px;
}

.details-sk-para.short {
  width: min(420px, 65%);
}

.details-sk-btn {
  width: 148px;
  height: 40px;
  border-radius: 999px;
}

.details-sk-btn.secondary {
  width: 126px;
}

.details-sk-eps {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(260px, 1fr));
  gap: 1.15rem;
}

.details-sk-ep-card {
  display: flex;
  flex-direction: column;
  gap: 0.45rem;
}

.details-sk-ep-thumb {
  width: 100%;
  aspect-ratio: 16 / 9;
}

.details-sk-ep-line {
  width: 78%;
  height: 14px;
  border-radius: 6px;
}

.details-sk-ep-line.short {
  width: 52%;
  height: 12px;
}

.detail-hero {
  position: relative;
  z-index: 1;
  min-height: 440px;
  padding: 1.75rem 2.75rem 1.75rem;
  display: flex;
  flex-direction: column;
  justify-content: space-between;
}

.hero-backdrop {
  position: absolute;
  inset: 0;
  width: 100%;
  height: 100%;
  object-fit: cover;
  object-position: center 20%;
  transform: scale(1.04);
  filter: saturate(1.1);
  pointer-events: none;
}

.hero-backdrop-trailer {
  position: absolute;
  top: 50%;
  left: 50%;
  width: max(100%, 177.78vh);
  height: max(100%, 56.25vw);
  transform: translate(-50%, -50%) scale(1.06);
  border: 0;
  pointer-events: none;
  opacity: 0;
  transition: opacity 0.65s ease;
}

.hero-backdrop-trailer.is-ready {
  opacity: 0.92;
}

.hero-scrim {
  position: absolute;
  inset: 0;
  background:
    radial-gradient(circle at 80% 25%, color-mix(in srgb, var(--vessel-accent) 12%, transparent), transparent 48%),
    linear-gradient(
      90deg,
      var(--vessel-bg-base) 0%,
      color-mix(in srgb, var(--vessel-bg-base) 86%, transparent) 46%,
      color-mix(in srgb, var(--vessel-bg-base) 30%, transparent) 100%
    ),
    linear-gradient(
      180deg,
      color-mix(in srgb, var(--vessel-bg-base) 22%, transparent) 0%,
      color-mix(in srgb, var(--vessel-bg-base) 74%, transparent) 66%,
      var(--vessel-bg-base) 100%
    );
  pointer-events: none;
}

.hero-nav-row {
  position: relative;
  z-index: 4;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.75rem;
  margin-bottom: 1.5rem;
}

.hero-top-actions {
  display: flex;
  align-items: center;
  gap: 0.5rem;
}

.more-menu-wrap {
  position: relative;
}

.hero-more-dropdown {
  position: absolute;
  top: calc(100% + 0.4rem);
  right: 0;
  min-width: 210px;
  padding: 0.35rem;
  display: flex;
  flex-direction: column;
  gap: 0.2rem;
  z-index: 30;
}

.hero-menu-item {
  display: flex;
  align-items: center;
  gap: 0.55rem;
  width: 100%;
  padding: 0.5rem 0.7rem;
  border: none;
  border-radius: 8px;
  background: transparent;
  color: var(--vessel-text-primary);
  font-size: 0.85rem;
  font-weight: 600;
  text-align: left;
  cursor: pointer;
}

.hero-menu-item:hover {
  background: var(--vessel-surface-elevated);
}

.hero-stage {
  position: relative;
  z-index: 2;
  display: flex;
  gap: 2.25rem;
  align-items: flex-end;
  flex-wrap: wrap;
}

.hero-poster {
  width: 216px;
  aspect-ratio: 2 / 3;
  object-fit: cover;
  border-radius: var(--vessel-radius);
  box-shadow:
    0 0 0 1px rgba(255, 255, 255, 0.16),
    0 28px 60px rgba(0, 0, 0, 0.78);
  flex-shrink: 0;
}

.hero-meta-col {
  flex: 1;
  min-width: 280px;
  display: flex;
  flex-direction: column;
  gap: 0.95rem;
}

.meta-line {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 0.8rem;
  font-size: 0.86rem;
  font-weight: 600;
  color: var(--vessel-text-muted);
  font-variant-numeric: tabular-nums;
}

.episodes-stage {
  position: relative;
  z-index: 1;
}

.source-tag {
  padding: 0.15rem 0.6rem;
  border-radius: 6px;
  background: rgba(203, 166, 247, 0.22);
  color: var(--vessel-accent);
  text-transform: uppercase;
  font-size: 0.72rem;
  letter-spacing: 0.06em;
}

.rating-text {
  display: inline-flex;
  align-items: center;
  gap: 0.26rem;
  color: var(--vessel-warning);
}

.title-block {
  display: flex;
  flex-direction: column;
  gap: 0.22rem;
}

.detail-title {
  margin: 0;
  font-size: 2.85rem;
  font-weight: 850;
  letter-spacing: -0.035em;
  line-height: 1.06;
  text-wrap: balance;
}

.orig-title {
  margin: 0;
  font-size: 0.96rem;
}

.genres-line {
  display: flex;
  flex-wrap: wrap;
  gap: 0.45rem;
}

.synopsis {
  margin: 0;
  color: rgba(244, 246, 252, 0.88);
  line-height: 1.65;
  max-width: 74ch;
  white-space: pre-line;
  display: -webkit-box;
  -webkit-line-clamp: 4;
  -webkit-box-orient: vertical;
  overflow: hidden;
}

.cta-row {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 0.75rem;
  margin-top: 0.35rem;
}

.library-picker {
  position: relative;
  display: flex;
  align-items: center;
}

.picker-icon {
  position: absolute;
  left: 0.9rem;
  color: var(--vessel-text-muted);
  pointer-events: none;
  z-index: 1;
}

.status-select {
  height: 40px;
  width: 205px;
  padding-left: 2.35rem;
  border-radius: 999px;
}

.episodes-stage {
  padding: 0 2.75rem 3rem;
  display: flex;
  flex-direction: column;
  gap: 1.1rem;
}

.episodes-head {
  display: flex;
  justify-content: space-between;
  align-items: center;
  flex-wrap: wrap;
  gap: 0.75rem;
}

.ep-head-title,
.ep-head-controls {
  display: flex;
  align-items: center;
  gap: 0.65rem;
  flex-wrap: wrap;
}

.ep-head-title h2 {
  margin: 0;
  font-size: 1.35rem;
  font-weight: 700;
}

.season-tabs {
  display: flex;
  gap: 0.45rem;
  flex-wrap: wrap;
}

.layout-switcher {
  display: inline-flex;
  align-items: center;
  padding: 0.2rem;
  border-radius: 999px;
  background: var(--vessel-surface-input);
  border: 1px solid var(--vessel-border);
  gap: 0.15rem;
}

.layout-btn {
  width: 30px;
  height: 28px;
  border-radius: 999px;
  border: none;
  background: transparent;
  color: var(--vessel-text-muted);
  display: flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;
  transition: background-color 0.16s ease, color 0.16s ease;
}

.layout-btn:hover {
  color: var(--vessel-text-primary);
}

.layout-btn.is-active {
  background: rgba(203, 166, 247, 0.22);
  color: var(--vessel-accent);
}

/* Mode 1: Detailed List (Nuvio default horizontal row) */
.episodes-container {
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
}

/* Mode 2: Grid Cards */
.episodes-container.mode-grid_cards {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(290px, 1fr));
  gap: 1rem;
}

.episodes-container.mode-grid_cards .nuvio-ep-item {
  flex-direction: column;
  align-items: stretch;
  padding: 0.85rem;
}

.episodes-container.mode-grid_cards .ep-thumb-btn {
  width: 100%;
}

.episodes-container.mode-grid_cards .ep-controls {
  justify-content: flex-end;
}

/* Mode 3: Compact Rows */
.episodes-container.mode-compact_rows .nuvio-ep-item {
  padding: 0.6rem 0.95rem;
}

.compact-num {
  color: var(--vessel-accent);
  font-weight: 700;
  margin-right: 0.45rem;
  font-variant-numeric: tabular-nums;
}

.nuvio-ep-item {
  display: flex;
  align-items: center;
  gap: 1.15rem;
  padding: 0.75rem;
  border-radius: calc(var(--vessel-radius) + 2px);
  background: var(--vessel-surface-card);
  border: 1px solid var(--vessel-border);
  transition:
    background-color 0.22s ease,
    border-color 0.22s ease,
    transform 0.22s cubic-bezier(0.22, 1, 0.36, 1);
}

.nuvio-ep-item:hover {
  background: var(--vessel-surface-elevated);
  border-color: var(--vessel-border-strong);
  transform: translateY(-1px);
}

.ep-thumb-btn {
  position: relative;
  width: 176px;
  aspect-ratio: 16 / 9;
  border-radius: 10px;
  overflow: hidden;
  border: none;
  padding: 0;
  background: var(--vessel-bg-elevated);
  flex-shrink: 0;
  cursor: pointer;
}

.ep-thumb-img {
  width: 100%;
  height: 100%;
  object-fit: cover;
  display: block;
  transition: transform 0.28s ease;
}

.ep-thumb-btn:hover .ep-thumb-img {
  transform: scale(1.06);
}

.ep-thumb-scrim {
  position: absolute;
  inset: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  background: rgba(10, 11, 16, 0.38);
  transition: background-color 0.2s ease;
}

.ep-thumb-btn:hover .ep-thumb-scrim {
  background: rgba(10, 11, 16, 0.56);
}

.ep-play-circle {
  width: 36px;
  height: 36px;
  border-radius: 999px;
  display: flex;
  align-items: center;
  justify-content: center;
  background: rgba(255, 255, 255, 0.92);
  color: #0a0b10;
  box-shadow: 0 6px 16px rgba(0, 0, 0, 0.45);
}

.ep-corner-tag {
  position: absolute;
  left: 0.45rem;
  bottom: 0.4rem;
  padding: 0.12rem 0.45rem;
  border-radius: 6px;
  background: rgba(10, 11, 16, 0.78);
  color: #ffffff;
  font-size: 0.72rem;
  font-weight: 700;
  font-variant-numeric: tabular-nums;
}

.ep-content-btn {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 0.3rem;
  border: none;
  background: transparent;
  padding: 0;
  text-align: left;
  cursor: pointer;
}

.ep-title {
  display: inline-flex;
  align-items: center;
  gap: 0.4rem;
  font-size: 1.02rem;
  font-weight: 650;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.ep-watched-corner {
  position: absolute;
  top: 0.4rem;
  right: 0.4rem;
  width: 24px;
  height: 24px;
  border-radius: 999px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  background: rgba(16, 185, 129, 0.92);
  color: #ffffff;
  box-shadow: 0 3px 10px rgba(0, 0, 0, 0.4);
}

.ep-watched-inline {
  color: var(--vessel-success);
  flex-shrink: 0;
}

.ep-thumb-progress {
  position: absolute;
  inset: auto 0 0 0;
  height: 3.5px;
  background: rgba(255, 255, 255, 0.22);
}

.ep-thumb-progress-fill,
.ep-progress-fill {
  height: 100%;
  background: linear-gradient(90deg, var(--vessel-accent-secondary), var(--vessel-accent));
  border-radius: 999px;
}

.ep-thumb-progress-fill.is-done,
.ep-progress-fill.is-done {
  background: var(--vessel-success);
}

.ep-progress-row {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  margin-top: 0.15rem;
}

.ep-progress-track {
  flex: 1;
  max-width: 180px;
  height: 4px;
  border-radius: 999px;
  background: var(--vessel-border);
  overflow: hidden;
}

.ep-progress-label {
  font-size: 0.72rem;
  font-weight: 650;
  color: var(--vessel-text-muted);
  font-variant-numeric: tabular-nums;
}

.ep-progress-label.is-done {
  color: var(--vessel-success);
}

.ep-overview {
  font-size: 0.86rem;
  line-height: 1.45;
  display: -webkit-box;
  -webkit-line-clamp: 3;
  -webkit-box-orient: vertical;
  overflow: hidden;
}

.ep-controls {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  flex-shrink: 0;
}

.stream-backdrop {
  position: fixed;
  inset: 0;
  z-index: 65;
  display: flex;
  align-items: center;
  justify-content: center;
  background: rgba(10, 11, 16, 0.78);
  backdrop-filter: blur(14px);
  -webkit-backdrop-filter: blur(14px);
  padding: 1.25rem;
  overscroll-behavior: contain;
}

.stream-modal {
  width: min(580px, 100%);
  display: flex;
  flex-direction: column;
  gap: 1.05rem;
  background: var(--vessel-surface-overlay);
  border-color: var(--vessel-border-strong);
}

.picker-head {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  gap: 0.75rem;
}

.picker-title-group {
  display: flex;
  flex-direction: column;
  gap: 0.2rem;
}

.picker-title {
  margin: 0;
  font-size: 1.15rem;
}

.picker-sub {
  font-size: 0.84rem;
}

.fix-form {
  display: flex;
  flex-direction: column;
  gap: 0.9rem;
}

.fix-row {
  display: flex;
  gap: 0.6rem;
}

.fix-provider {
  width: 160px;
  flex-shrink: 0;
}

.fix-actions {
  display: flex;
  justify-content: flex-end;
  gap: 0.55rem;
}

.resolver-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  text-align: center;
  gap: 0.55rem;
  padding: 1.75rem 1rem;
}

.resolver-err-msg {
  margin: 0;
  max-width: 44ch;
  font-size: 0.88rem;
}

.resolver-err-actions {
  display: flex;
  gap: 0.5rem;
  margin-top: 0.4rem;
}

.accent-icon {
  color: var(--vessel-accent);
}

.warn-icon {
  color: var(--vessel-warning);
}

.stream-list {
  display: flex;
  flex-direction: column;
  gap: 0.55rem;
  max-height: 380px;
  overflow-y: auto;
}

.cs-source-row {
  display: flex;
  align-items: center;
  gap: 0.55rem;
  padding: 0.55rem 0.65rem;
  border-radius: var(--vessel-radius);
  border: 1px solid var(--vessel-border);
  background: var(--vessel-surface-card);
  transition: border-color 0.18s ease, background-color 0.18s ease;
}

.cs-source-row:hover {
  border-color: color-mix(in srgb, var(--vessel-accent) 55%, transparent);
  background: var(--vessel-surface-elevated);
}

.cs-source-play {
  flex: 1;
  min-width: 0;
  display: flex;
  align-items: center;
  gap: 0.8rem;
  border: none;
  background: transparent;
  padding: 0.25rem;
  text-align: left;
  cursor: pointer;
}

.cs-play-badge {
  width: 34px;
  height: 34px;
  border-radius: 999px;
  display: flex;
  align-items: center;
  justify-content: center;
  background: #f4f6fc;
  color: #0a0b10;
  flex-shrink: 0;
}

.cs-source-info {
  display: flex;
  flex-direction: column;
  gap: 0.3rem;
  min-width: 0;
}

.stream-label {
  font-weight: 600;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.cs-source-tags {
  display: flex;
  flex-wrap: wrap;
  gap: 0.35rem;
}

.spin {
  animation: v-spin 0.9s linear infinite;
}

@keyframes v-spin {
  to {
    transform: rotate(360deg);
  }
}

.fix-modal-wide {
  width: min(640px, 100%);
}

.fix-search-bar {
  display: flex;
  gap: 0.55rem;
}

.fix-search-bar .v-input {
  flex: 1;
}

.fix-provider-chips {
  display: flex;
  flex-wrap: wrap;
  gap: 0.35rem;
}

.fix-notes-box {
  padding: 0.5rem 0.75rem;
  border-radius: 10px;
  background: var(--vessel-surface-input);
  border: 1px solid var(--vessel-border);
}

.fix-note-line {
  margin: 0;
  font-size: 0.8rem;
}

.fix-hits-list,
.linked-plugins-list {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
  max-height: 280px;
  overflow-y: auto;
}

.fix-hit-card {
  display: flex;
  align-items: flex-start;
  gap: 0.75rem;
  padding: 0.55rem;
  border-radius: 12px;
  border: 1px solid var(--vessel-border);
  background: var(--vessel-surface-card);
  text-align: left;
  cursor: pointer;
}

.fix-hit-card:hover {
  border-color: var(--vessel-accent);
  background: var(--vessel-surface-elevated);
}

.fix-hit-poster {
  width: 48px;
  height: 70px;
  border-radius: 7px;
  object-fit: cover;
  flex-shrink: 0;
  background: var(--vessel-surface-input);
}

.fix-hit-body {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 0.22rem;
}

.fix-hit-top {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.45rem;
}

.fix-hit-title {
  font-size: 0.9rem;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.fix-hit-meta {
  font-size: 0.76rem;
}

.fix-hit-syn {
  margin: 0;
  font-size: 0.76rem;
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
}

.fix-empty-hits {
  font-size: 0.84rem;
  padding: 0.65rem 0;
}

.manual-id-section {
  display: flex;
  flex-direction: column;
  gap: 0.6rem;
  align-items: flex-start;
}

.linked-plugin-item {
  display: flex;
  flex-direction: column;
  gap: 0.45rem;
  padding: 0.65rem 0.75rem;
  border-radius: 12px;
  border: 1px solid var(--vessel-border);
  background: var(--vessel-surface-card);
}

.linked-plugin-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.65rem;
  flex-wrap: wrap;
}

.linked-plugin-info {
  display: flex;
  flex-direction: column;
  gap: 0.15rem;
  min-width: 0;
}

.linked-match-line {
  font-size: 0.78rem;
}

.linked-plugin-btns,
.linked-hits-strip {
  display: flex;
  flex-wrap: wrap;
  gap: 0.4rem;
}
</style>
