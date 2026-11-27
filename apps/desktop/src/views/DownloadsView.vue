<script setup lang="ts">
import { computed, nextTick, onMounted, ref, shallowRef } from 'vue'
import { useRouter } from 'vue-router'
import {
  ArrowDown,
  ArrowUp,
  CheckCircle2,
  ChevronDown,
  ChevronRight,
  Film,
  FolderOpen,
  GripVertical,
  HardDriveDownload,
  Pause,
  Play,
  RefreshCw,
  Trash2,
} from 'lucide-vue-next'
import { isTauriRuntime, vesselApi } from '@/api/vessel'
import { useI18n } from '@/composables/useI18n'
import { usePlayerSession } from '@/composables/usePlayerSession'
import { pushToast, useVesselEvents } from '@/composables/useVesselEvents'
import type { MediaDetail, PlaybackProgress, StoredDownload } from '@/types/vessel'

defineOptions({ name: 'DownloadsView' })

interface EpisodeGroup {
  key: string
  season: number
  episode: number
  items: StoredDownload[]
  downloadedBytes: number
  totalBytes: number
  activeCount: number
  progress: PlaybackProgress | null
}

interface SeasonGroup {
  key: string
  season: number
  episodes: EpisodeGroup[]
  totalStreams: number
  downloadedBytes: number
  totalBytes: number
  activeCount: number
  allWatched: boolean
}

interface ContentGroup {
  key: string
  pluginId: string
  nativeId: string
  displayTitle: string
  isMovie: boolean
  seasons: SeasonGroup[]
  movieStreams: StoredDownload[]
  episodeCount: number
  totalStreams: number
  downloadedBytes: number
  totalBytes: number
  activeCount: number
  allWatched: boolean
  movieProgress: PlaybackProgress | null
}

const router = useRouter()
const { t } = useI18n()
const { setSession } = usePlayerSession()
const {
  downloadProgress,
  setDownloadSnapshot,
  clearDownloadProgress,
  pruneStaleDownloadProgress,
} = useVesselEvents()

const downloads = ref<StoredDownload[]>([])
const progressList = ref<PlaybackProgress[]>([])
const downloadDir = shallowRef('')
const expandedContents = ref<Set<string>>(new Set())
const expandedSeasons = ref<Set<string>>(new Set())
const expandedEpisodes = ref<Set<string>>(new Set())
const queueListRef = ref<HTMLElement | null>(null)
const draggingQueueId = shallowRef<string | null>(null)
const launchingDownloadId = shallowRef<string | null>(null)
const busyIds = ref<Set<string>>(new Set())

const mergedDownloads = computed(() =>
  downloads.value
    .map((dl) => {
      const live = downloadProgress.value[dl.id]
      if (!live) return dl
      return {
        ...dl,
        status: live.status,
        bytesDownloaded: live.bytesDownloaded,
        totalBytes: live.totalBytes ?? dl.totalBytes,
        segmentsDone: live.segmentsDone,
        segmentsTotal: live.segmentsTotal,
      }
    })
    .sort((a, b) => {
      const ao = a.queueOrder > 0 ? a.queueOrder : Number.MAX_SAFE_INTEGER
      const bo = b.queueOrder > 0 ? b.queueOrder : Number.MAX_SAFE_INTEGER
      if (ao !== bo) return ao - bo
      return a.createdAt.localeCompare(b.createdAt)
    }),
)

const queueDownloads = computed(() =>
  mergedDownloads.value.filter(
    (dl) =>
      dl.status === 'queued' ||
      dl.status === 'downloading' ||
      dl.status === 'paused' ||
      dl.status === 'failed',
  ),
)

function progressKeyFor(
  pluginId: string,
  nativeId: string,
  season?: number,
  episode?: number,
): string {
  return `${pluginId}:${nativeId}:${season ?? 0}:${episode ?? 0}`
}

const progressByCoord = computed(() => {
  const map = new Map<string, PlaybackProgress>()
  for (const row of progressList.value) {
    map.set(
      progressKeyFor(row.media.pluginId, row.media.nativeId, row.season, row.episode),
      row,
    )
  }
  return map
})

function findProgressForDownload(dl: StoredDownload): PlaybackProgress | null {
  return (
    progressByCoord.value.get(
      progressKeyFor(dl.pluginId, dl.nativeId, dl.season, dl.episode),
    ) ?? null
  )
}

function cleanDisplayTitle(raw: string): string {
  return raw.replace(/\s+—\s+S\d+E\d+$/i, '').trim() || raw
}

/** 4-level hierarchy: Content -> Season -> Episode -> Stream candidates (movies go directly Content -> Stream). */
const contentHierarchy = computed<ContentGroup[]>(() => {
  const contentMap = new Map<
    string,
    {
      key: string
      pluginId: string
      nativeId: string
      displayTitle: string
      hasEpisodes: boolean
      seasonMap: Map<number, Map<number, StoredDownload[]>>
      movieStreams: StoredDownload[]
    }
  >()

  for (const dl of mergedDownloads.value) {
    const cKey = `${dl.pluginId}:${dl.nativeId}`
    let cBucket = contentMap.get(cKey)
    if (!cBucket) {
      cBucket = {
        key: cKey,
        pluginId: dl.pluginId,
        nativeId: dl.nativeId,
        displayTitle: cleanDisplayTitle(dl.title),
        hasEpisodes: dl.season !== undefined || dl.episode !== undefined,
        seasonMap: new Map(),
        movieStreams: [],
      }
      contentMap.set(cKey, cBucket)
    }

    if (dl.season === undefined && dl.episode === undefined) {
      cBucket.movieStreams.push(dl)
    } else {
      cBucket.hasEpisodes = true
      const sNum = dl.season ?? 1
      const eNum = dl.episode ?? 1
      let epMap = cBucket.seasonMap.get(sNum)
      if (!epMap) {
        epMap = new Map()
        cBucket.seasonMap.set(sNum, epMap)
      }
      let list = epMap.get(eNum)
      if (!list) {
        list = []
        epMap.set(eNum, list)
      }
      list.push(dl)
    }
  }

  const out: ContentGroup[] = []
  for (const c of contentMap.values()) {
    const isMovie = !c.hasEpisodes && c.seasonMap.size === 0
    if (isMovie) {
      const totalBytes = c.movieStreams.reduce((s, d) => s + (d.totalBytes ?? 0), 0)
      const downloadedBytes = c.movieStreams.reduce((s, d) => s + d.bytesDownloaded, 0)
      const activeCount = c.movieStreams.filter(
        (d) => d.status === 'downloading' || d.status === 'queued',
      ).length
      const movieProgress =
        progressByCoord.value.get(progressKeyFor(c.pluginId, c.nativeId, 0, 0)) ?? null
      out.push({
        key: c.key,
        pluginId: c.pluginId,
        nativeId: c.nativeId,
        displayTitle: c.displayTitle,
        isMovie: true,
        seasons: [],
        movieStreams: c.movieStreams,
        episodeCount: 0,
        totalStreams: c.movieStreams.length,
        downloadedBytes,
        totalBytes,
        activeCount,
        allWatched: Boolean(movieProgress?.completed),
        movieProgress,
      })
      continue
    }

    const seasons: SeasonGroup[] = []
    const sortedSeasonNums = Array.from(c.seasonMap.keys()).sort((a, b) => a - b)
    let totalEpCount = 0
    let totalStreamCount = 0
    let contentDownloadedBytes = 0
    let contentTotalBytes = 0
    let contentActiveCount = 0

    for (const sNum of sortedSeasonNums) {
      const epMap = c.seasonMap.get(sNum)!
      const sortedEpNums = Array.from(epMap.keys()).sort((a, b) => a - b)
      const episodes: EpisodeGroup[] = []
      const seasonKey = `${c.key}:s:${sNum}`
      let sStreamCount = 0
      let sDownloadedBytes = 0
      let sTotalBytes = 0
      let sActiveCount = 0

      for (const eNum of sortedEpNums) {
        const items = epMap.get(eNum)!
        const epKey = `${seasonKey}:e:${eNum}`
        const epDownloaded = items.reduce((s, d) => s + d.bytesDownloaded, 0)
        const epTotal = items.reduce((s, d) => s + (d.totalBytes ?? 0), 0)
        const epActive = items.filter(
          (d) => d.status === 'downloading' || d.status === 'queued',
        ).length
        const prog =
          progressByCoord.value.get(progressKeyFor(c.pluginId, c.nativeId, sNum, eNum)) ?? null

        episodes.push({
          key: epKey,
          season: sNum,
          episode: eNum,
          items,
          downloadedBytes: epDownloaded,
          totalBytes: epTotal,
          activeCount: epActive,
          progress: prog,
        })

        sStreamCount += items.length
        sDownloadedBytes += epDownloaded
        sTotalBytes += epTotal
        sActiveCount += epActive
      }

      const seasonWatched = episodes.length > 0 && episodes.every((e) => Boolean(e.progress?.completed))
      seasons.push({
        key: seasonKey,
        season: sNum,
        episodes,
        totalStreams: sStreamCount,
        downloadedBytes: sDownloadedBytes,
        totalBytes: sTotalBytes,
        activeCount: sActiveCount,
        allWatched: seasonWatched,
      })

      totalEpCount += episodes.length
      totalStreamCount += sStreamCount
      contentDownloadedBytes += sDownloadedBytes
      contentTotalBytes += sTotalBytes
      contentActiveCount += sActiveCount
    }

    const allWatched = seasons.length > 0 && seasons.every((s) => s.allWatched)
    out.push({
      key: c.key,
      pluginId: c.pluginId,
      nativeId: c.nativeId,
      displayTitle: c.displayTitle,
      isMovie: false,
      seasons,
      movieStreams: [],
      episodeCount: totalEpCount,
      totalStreams: totalStreamCount,
      downloadedBytes: contentDownloadedBytes,
      totalBytes: contentTotalBytes,
      activeCount: contentActiveCount,
      allWatched,
      movieProgress: null,
    })
  }

  return out
})

const continueWatchingDownloads = computed(() => {
  const seen = new Set<string>()
  const out: Array<{ download: StoredDownload; progress: PlaybackProgress }> = []
  for (const dl of mergedDownloads.value) {
    if (dl.status !== 'completed') continue
    const key = progressKeyFor(dl.pluginId, dl.nativeId, dl.season, dl.episode)
    if (seen.has(key)) continue
    const prog = findProgressForDownload(dl)
    if (prog && !prog.completed && prog.positionMs > 0) {
      seen.add(key)
      out.push({ download: dl, progress: prog })
    }
  }
  return out
})

function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`
  if (bytes < 1024 * 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(1)} MB`
  return `${(bytes / (1024 * 1024 * 1024)).toFixed(2)} GB`
}

function downloadPercent(dl: StoredDownload): number {
  if (dl.status === 'completed') return 100
  if (dl.segmentsTotal > 0) {
    return Math.min(100, Math.round((dl.segmentsDone / dl.segmentsTotal) * 100))
  }
  if (dl.totalBytes && dl.totalBytes > 0) {
    return Math.min(100, Math.round((dl.bytesDownloaded / dl.totalBytes) * 100))
  }
  return 0
}

function watchPercent(prog: PlaybackProgress): number {
  if (prog.completed) return 100
  if (!prog.durationMs || prog.durationMs <= 0) return 15
  return Math.min(99, Math.max(4, Math.round((prog.positionMs / prog.durationMs) * 100)))
}

function formatRemaining(prog: PlaybackProgress): string {
  if (!prog.durationMs || prog.durationMs <= prog.positionMs) return t('details.resume')
  const remSec = Math.max(0, Math.floor((prog.durationMs - prog.positionMs) / 1000))
  const mins = Math.floor(remSec / 60)
  if (mins >= 60) return `${Math.floor(mins / 60)}h ${mins % 60}m`
  if (mins >= 1) return `${mins}m`
  return `${remSec}s`
}

function toggleInSet(targetRef: typeof expandedContents, key: string): void {
  const next = new Set(targetRef.value)
  if (next.has(key)) next.delete(key)
  else next.add(key)
  targetRef.value = next
}

const toggleContent = (key: string): void => toggleInSet(expandedContents, key)
const toggleSeason = (key: string): void => toggleInSet(expandedSeasons, key)
const toggleEpisode = (key: string): void => toggleInSet(expandedEpisodes, key)

function firstCompletedInContent(c: ContentGroup): StoredDownload | undefined {
  if (c.isMovie) {
    return c.movieStreams.find((d) => d.status === 'completed')
  }
  for (const s of c.seasons) {
    for (const e of s.episodes) {
      const done = e.items.find((d) => d.status === 'completed')
      if (done) return done
    }
  }
  return undefined
}

async function refresh(): Promise<void> {
  if (!isTauriRuntime()) return
  try {
    const [list, info, allProg] = await Promise.all([
      vesselApi.listDownloads(),
      vesselApi.getEngineInfo().catch(() => null),
      vesselApi.listAllWatchProgress(200).catch(() => []),
    ])
    downloads.value = list
    progressList.value = allProg
    if (info) downloadDir.value = info.downloadsDir

    const validIds = new Set(list.map((d) => d.id))
    pruneStaleDownloadProgress(validIds)
    for (const dl of list) {
      setDownloadSnapshot({
        downloadId: dl.id,
        status: dl.status,
        bytesDownloaded: dl.bytesDownloaded,
        totalBytes: dl.totalBytes,
        segmentsDone: dl.segmentsDone,
        segmentsTotal: dl.segmentsTotal,
      })
    }
  } catch (err) {
    pushToast('error', String(err))
  }
}

async function commitQueueOrder(reorderedQueueIds: string[]): Promise<void> {
  const completedIds = mergedDownloads.value
    .filter((d) => !reorderedQueueIds.includes(d.id))
    .map((d) => d.id)
  const fullOrder = [...reorderedQueueIds, ...completedIds]
  const orderIndex = new Map<string, number>()
  fullOrder.forEach((id, idx) => orderIndex.set(id, idx + 1))
  downloads.value = downloads.value.map((d) => ({
    ...d,
    queueOrder: orderIndex.get(d.id) ?? d.queueOrder,
  }))
  try {
    await vesselApi.reorderDownloads(fullOrder)
  } catch (err) {
    pushToast('error', String(err))
    await refresh()
  }
}

function moveQueueItem(id: string, delta: -1 | 1): void {
  const ids = queueDownloads.value.map((d) => d.id)
  const idx = ids.indexOf(id)
  const target = idx + delta
  if (idx < 0 || target < 0 || target >= ids.length) return
  const [moved] = ids.splice(idx, 1)
  if (!moved) return
  ids.splice(target, 0, moved)
  void commitQueueOrder(ids)
  const preferDir = delta === -1 ? (target > 0 ? 'up' : 'down') : (target < ids.length - 1 ? 'down' : 'up')
  void nextTick(() => {
    const el = queueListRef.value?.querySelector(`[data-queue-id="${CSS.escape(id)}"] [data-move-dir="${preferDir}"]`)
    if (el instanceof HTMLElement) el.focus()
  })
}

function startQueueDrag(id: string, ev: PointerEvent): void {
  if (ev.button !== 0) return
  ev.preventDefault()
  draggingQueueId.value = id

  const onMove = (moveEv: PointerEvent) => {
    const listEl = queueListRef.value
    const curId = draggingQueueId.value
    if (!listEl || !curId) return
    const ids = queueDownloads.value.map((d) => d.id)
    const curIdx = ids.indexOf(curId)
    if (curIdx < 0) return

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
      const [moved] = ids.splice(curIdx, 1)
      if (!moved) return
      ids.splice(targetIdx, 0, moved)
      const orderIndex = new Map<string, number>()
      ids.forEach((itemId, idx) => orderIndex.set(itemId, idx + 1))
      const baseCompleted = ids.length
      let c = 1
      downloads.value = downloads.value.map((d) => {
        const idx = orderIndex.get(d.id)
        if (idx !== undefined) return { ...d, queueOrder: idx }
        return { ...d, queueOrder: baseCompleted + c++ }
      })
    }
  }

  const onUp = () => {
    draggingQueueId.value = null
    window.removeEventListener('pointermove', onMove)
    window.removeEventListener('pointerup', onUp)
    const finalQueueIds = queueDownloads.value.map((d) => d.id)
    void commitQueueOrder(finalQueueIds)
  }

  window.addEventListener('pointermove', onMove)
  window.addEventListener('pointerup', onUp)
}

async function playLocalDownload(
  dl: StoredDownload,
  resumeProgress?: PlaybackProgress | null,
): Promise<void> {
  if (dl.status !== 'completed' || launchingDownloadId.value) return
  launchingDownloadId.value = dl.id
  try {
    const bundle = await vesselApi.prepareLocalDownloadPlayback(dl.id)
    const prog = resumeProgress ?? findProgressForDownload(dl)
    const isMovie = dl.season === undefined && dl.episode === undefined
    const baseTitle = cleanDisplayTitle(dl.title)
    const epSeason = dl.season ?? 0
    const epEpisode = dl.episode ?? 0
    const epObj = {
      streamId: dl.id,
      season: epSeason,
      episode: epEpisode,
      title: isMovie ? baseTitle : dl.streamLabel || `${baseTitle} S${epSeason}·E${epEpisode}`,
      tags: ['offline'],
    }
    const detailStub: MediaDetail = {
      nativeId: dl.nativeId,
      pluginId: dl.pluginId,
      type: isMovie ? 'movie' : 'series',
      isAnime: false,
      title: baseTitle,
      genres: [],
      externalIds: {},
      episodes: [epObj],
    }

    setSession({
      pluginId: dl.pluginId,
      nativeId: dl.nativeId,
      detail: detailStub,
      episode: epObj,
      streams: [
        {
          label: dl.streamLabel || t('downloads.stream_default'),
          url: bundle.url,
          kind: bundle.kind,
          headers: {},
          flags: [],
          subtitles: bundle.subtitles,
          needsExtractor: false,
          isDefault: true,
        },
      ],
      subtitles: bundle.subtitles,
      activeStreamIndex: 0,
      startPositionMs: prog && !prog.completed && prog.positionMs > 0 ? prog.positionMs : undefined,
    })

    await router.push({
      name: 'player',
      params: { pluginId: dl.pluginId, nativeId: dl.nativeId },
      query: { streamId: dl.id, streamIndex: '0', offline: '1' },
    })
  } catch (err) {
    pushToast('error', String(err))
  } finally {
    launchingDownloadId.value = null
  }
}

async function pauseItem(id: string): Promise<void> {
  if (busyIds.value.has(id)) return
  const nextBusy = new Set(busyIds.value)
  nextBusy.add(id)
  busyIds.value = nextBusy

  const cur = mergedDownloads.value.find((d) => d.id === id)
  if (cur) {
    setDownloadSnapshot({
      downloadId: id,
      status: 'paused',
      bytesDownloaded: cur.bytesDownloaded,
      totalBytes: cur.totalBytes,
      segmentsDone: cur.segmentsDone,
      segmentsTotal: cur.segmentsTotal,
    })
  }
  try {
    await vesselApi.pauseDownload(id)
    await refresh()
  } catch (err) {
    pushToast('error', String(err))
    await refresh()
  } finally {
    const done = new Set(busyIds.value)
    done.delete(id)
    busyIds.value = done
  }
}

async function resumeItem(id: string): Promise<void> {
  if (busyIds.value.has(id)) return
  const nextBusy = new Set(busyIds.value)
  nextBusy.add(id)
  busyIds.value = nextBusy

  try {
    const updated = await vesselApi.resumeDownload(id)
    setDownloadSnapshot({
      downloadId: id,
      status: updated.status,
      bytesDownloaded: updated.bytesDownloaded,
      totalBytes: updated.totalBytes,
      segmentsDone: updated.segmentsDone,
      segmentsTotal: updated.segmentsTotal,
    })
    await refresh()
  } catch (err) {
    pushToast('error', String(err))
    await refresh()
  } finally {
    const done = new Set(busyIds.value)
    done.delete(id)
    busyIds.value = done
  }
}

async function deleteItem(id: string): Promise<void> {
  if (busyIds.value.has(id)) return
  const nextBusy = new Set(busyIds.value)
  nextBusy.add(id)
  busyIds.value = nextBusy

  try {
    await vesselApi.deleteDownload(id)
    clearDownloadProgress(id)
    downloads.value = downloads.value.filter((d) => d.id !== id)
    await refresh()
  } catch (err) {
    pushToast('error', String(err))
    await refresh()
  } finally {
    const done = new Set(busyIds.value)
    done.delete(id)
    busyIds.value = done
  }
}

onMounted(() => {
  void refresh()
})
</script>

<template>
  <section class="downloads-view v-page-container">
    <header class="v-page-header">
      <div>
        <h1 class="v-page-title">{{ t('downloads.title') }}</h1>
        <p v-if="downloadDir" class="v-muted dl-dir-line">
          {{ t('settings.download_dir') }}: <code>{{ downloadDir }}</code>
        </p>
      </div>
      <button type="button" class="v-btn" @click="refresh">
        <RefreshCw :size="15" aria-hidden="true" /> {{ t('common.refresh') }}
      </button>
    </header>

    <!-- Offline Continue Watching -->
    <section v-if="continueWatchingDownloads.length" class="dl-section">
      <div class="dl-sec-head">
        <h2>{{ t('downloads.continue_watching') }}</h2>
        <span class="v-badge v-badge-accent">{{ continueWatchingDownloads.length }}</span>
      </div>
      <div class="dl-cw-rail">
        <button
          v-for="item in continueWatchingDownloads"
          :key="item.download.id"
          type="button"
          class="v-card dl-cw-card"
          @click="playLocalDownload(item.download, item.progress)"
        >
          <div class="dl-cw-top">
            <span class="dl-cw-play"><Play :size="14" fill="currentColor" aria-hidden="true" /></span>
            <span class="dl-cw-rem">{{ formatRemaining(item.progress) }}</span>
          </div>
          <strong class="dl-cw-title">{{ cleanDisplayTitle(item.download.title) }}</strong>
          <span class="v-muted dl-cw-sub">
            <template v-if="item.download.season !== undefined && item.download.episode !== undefined">
              S{{ item.download.season }} · E{{ item.download.episode }} ·
            </template>
            {{ item.download.streamLabel || t('downloads.stream_default') }}
          </span>
          <div class="dl-cw-bar">
            <div class="dl-cw-bar-fill" :style="{ width: `${watchPercent(item.progress)}%` }" />
          </div>
        </button>
      </div>
    </section>

    <!-- Priority Reorderable Active / Pending Queue -->
    <section v-if="queueDownloads.length" class="dl-section">
      <div class="dl-sec-head">
        <h2>{{ t('downloads.queue_pos') }}</h2>
        <span class="v-muted dl-drag-hint">{{ t('downloads.drag_hint') }}</span>
      </div>
      <div ref="queueListRef" class="dl-queue-list">
        <div
          v-for="(dl, idx) in queueDownloads"
          :key="dl.id"
          :data-queue-id="dl.id"
          class="v-card dl-queue-row"
          :class="{ 'is-dragging': draggingQueueId === dl.id }"
        >
          <button
            type="button"
            class="dl-drag-handle"
            :aria-label="`Drag ${dl.title} to reorder`"
            @pointerdown="startQueueDrag(dl.id, $event)"
          >
            <GripVertical :size="16" aria-hidden="true" />
          </button>
          <span class="dl-queue-num">#{{ idx + 1 }}</span>

          <div class="dl-info">
            <div class="dl-head">
              <strong>{{ cleanDisplayTitle(dl.title) }}</strong>
              <span v-if="dl.season !== undefined && dl.episode !== undefined" class="v-badge">
                S{{ dl.season }}·E{{ dl.episode }}
              </span>
              <span v-if="dl.streamLabel" class="v-badge">{{ dl.streamLabel }}</span>
              <span class="v-badge v-badge-accent">{{ dl.status }} · {{ downloadPercent(dl) }}%</span>
            </div>
            <div class="v-muted dl-stats">
              <span>{{ formatBytes(dl.bytesDownloaded) }}</span>
              <span v-if="dl.totalBytes">/ {{ formatBytes(dl.totalBytes) }}</span>
              <span v-if="dl.segmentsTotal > 0">
                · {{ t('downloads.segments') }}: {{ dl.segmentsDone }}/{{ dl.segmentsTotal }}
              </span>
            </div>
            <p v-if="dl.errorMsg" class="dl-err">{{ dl.errorMsg }}</p>
          </div>

          <div class="dl-actions">
            <button
              type="button"
              class="v-btn v-btn-icon v-btn-sm"
              data-move-dir="up"
              :disabled="idx === 0"
              aria-label="Move up"
              @click="moveQueueItem(dl.id, -1)"
            >
              <ArrowUp :size="14" aria-hidden="true" />
            </button>
            <button
              type="button"
              class="v-btn v-btn-icon v-btn-sm"
              data-move-dir="down"
              :disabled="idx === queueDownloads.length - 1"
              aria-label="Move down"
              @click="moveQueueItem(dl.id, 1)"
            >
              <ArrowDown :size="14" aria-hidden="true" />
            </button>
            <button
              v-if="dl.status === 'downloading' || dl.status === 'queued'"
              type="button"
              class="v-btn v-btn-sm"
              :disabled="busyIds.has(dl.id)"
              @click="pauseItem(dl.id)"
            >
              <Pause :size="14" aria-hidden="true" /> {{ t('common.pause') }}
            </button>
            <button
              v-if="dl.status === 'paused' || dl.status === 'failed'"
              type="button"
              class="v-btn v-btn-primary v-btn-sm"
              :disabled="busyIds.has(dl.id)"
              @click="resumeItem(dl.id)"
            >
              <Play :size="14" aria-hidden="true" /> {{ t('common.resume') }}
            </button>
            <button
              type="button"
              class="v-btn v-btn-danger v-btn-sm"
              :disabled="busyIds.has(dl.id)"
              @click="deleteItem(dl.id)"
            >
              <Trash2 :size="14" aria-hidden="true" /> {{ t('common.delete') }}
            </button>
          </div>
        </div>
      </div>
    </section>

    <div v-if="!mergedDownloads.length" class="v-card v-muted">{{ t('downloads.empty') }}</div>

    <!-- 4-Level Expandable Hierarchy: Content -> Season -> Episode -> Stream -->
    <div v-else class="dl-stack">
      <div
        v-for="content in contentHierarchy"
        :key="content.key"
        class="v-card dl-content-card"
      >
        <!-- Level 1: Content -->
        <div
          role="button"
          tabindex="0"
          class="dl-group-header"
          :aria-expanded="expandedContents.has(content.key)"
          @click="toggleContent(content.key)"
          @keydown.enter.self="toggleContent(content.key)"
          @keydown.space.prevent.self="toggleContent(content.key)"
        >
          <div class="dl-group-left">
            <ChevronDown v-if="expandedContents.has(content.key)" :size="18" aria-hidden="true" />
            <ChevronRight v-else :size="18" aria-hidden="true" />
            <Film v-if="content.isMovie" :size="18" class="dl-type-icon" aria-hidden="true" />
            <HardDriveDownload v-else :size="18" class="dl-type-icon" aria-hidden="true" />

            <div class="dl-group-meta">
              <div class="dl-head">
                <strong>{{ content.displayTitle }}</strong>
                <span v-if="!content.isMovie" class="v-badge">
                  {{ content.seasons.length }} {{ t('downloads.seasons_count') }} ·
                  {{ content.episodeCount }} {{ t('downloads.episodes_count') }}
                </span>
                <span class="v-badge">
                  {{ content.totalStreams }} {{ t('downloads.streams_count') }}
                </span>
                <span v-if="content.allWatched" class="v-badge v-badge-success dl-watched-pill">
                  <CheckCircle2 :size="12" aria-hidden="true" /> {{ t('downloads.watched') }}
                </span>
                <span
                  v-else-if="content.movieProgress && content.movieProgress.positionMs > 0"
                  class="v-badge v-badge-accent"
                >
                  {{ watchPercent(content.movieProgress) }}%
                </span>
              </div>
              <div class="v-muted dl-stats">
                <span>{{ content.pluginId }}</span>
                <span>· {{ formatBytes(content.downloadedBytes) }}</span>
                <span v-if="content.totalBytes">/ {{ formatBytes(content.totalBytes) }}</span>
                <span v-if="content.activeCount > 0">· {{ content.activeCount }} active</span>
              </div>
            </div>
          </div>

          <div class="dl-actions" @click.stop>
            <button
              v-if="firstCompletedInContent(content)"
              type="button"
              class="v-btn v-btn-primary v-btn-sm"
              @click="
                playLocalDownload(
                  firstCompletedInContent(content)!,
                  content.isMovie ? content.movieProgress : undefined,
                )
              "
            >
              <Play :size="14" fill="currentColor" aria-hidden="true" />
              {{ t('downloads.play_offline') }}
            </button>
          </div>
        </div>

        <!-- Level 2: Seasons (Series) or Direct Streams (Movie) -->
        <Transition name="page-fade">
          <div v-if="expandedContents.has(content.key)" class="dl-children-level">
            <!-- Movies: Level 4 Streams directly -->
            <template v-if="content.isMovie">
              <div
                v-for="dl in content.movieStreams"
                :key="dl.id"
                class="dl-stream-row"
              >
                <button
                  type="button"
                  class="dl-stream-link"
                  :disabled="dl.status !== 'completed' || launchingDownloadId === dl.id"
                  @click="playLocalDownload(dl, content.movieProgress)"
                >
                  <Play
                    v-if="dl.status === 'completed'"
                    :size="14"
                    fill="currentColor"
                    class="dl-stream-play-icon"
                    aria-hidden="true"
                  />
                  <span class="dl-stream-name">
                    {{ dl.streamLabel || `${t('downloads.stream_default')} (${dl.kind.toUpperCase()})` }}
                  </span>
                  <CheckCircle2
                    v-if="dl.status === 'completed' && content.movieProgress?.completed"
                    :size="14"
                    class="dl-stream-watched"
                    :title="t('downloads.watched')"
                    aria-hidden="true"
                  />
                </button>

                <div class="dl-stream-meta">
                  <span class="v-badge v-badge-accent">{{ dl.status }}</span>
                  <span class="v-badge">{{ dl.kind.toUpperCase() }}</span>
                  <span class="v-muted dl-stats">
                    {{ formatBytes(dl.bytesDownloaded) }}
                    <template v-if="dl.totalBytes">/ {{ formatBytes(dl.totalBytes) }}</template>
                  </span>
                </div>

                <div class="dl-actions">
                  <button
                    v-if="dl.status === 'downloading' || dl.status === 'queued'"
                    type="button"
                    class="v-btn v-btn-sm"
                    :disabled="busyIds.has(dl.id)"
                    @click="pauseItem(dl.id)"
                  >
                    <Pause :size="13" aria-hidden="true" /> {{ t('common.pause') }}
                  </button>
                  <button
                    v-if="dl.status === 'paused' || dl.status === 'failed'"
                    type="button"
                    class="v-btn v-btn-primary v-btn-sm"
                    :disabled="busyIds.has(dl.id)"
                    @click="resumeItem(dl.id)"
                  >
                    <Play :size="13" aria-hidden="true" /> {{ t('common.resume') }}
                  </button>
                  <button
                    type="button"
                    class="v-btn v-btn-danger v-btn-sm"
                    :disabled="busyIds.has(dl.id)"
                    @click="deleteItem(dl.id)"
                  >
                    <Trash2 :size="13" aria-hidden="true" /> {{ t('common.delete') }}
                  </button>
                </div>
              </div>
            </template>

            <!-- Series: Season List -->
            <template v-else>
              <div
                v-for="seasonGrp in content.seasons"
                :key="seasonGrp.key"
                class="dl-season-box"
              >
                <div
                  role="button"
                  tabindex="0"
                  class="dl-sub-header"
                  :aria-expanded="expandedSeasons.has(seasonGrp.key)"
                  @click="toggleSeason(seasonGrp.key)"
                  @keydown.enter.self="toggleSeason(seasonGrp.key)"
                  @keydown.space.prevent.self="toggleSeason(seasonGrp.key)"
                >
                  <div class="dl-group-left">
                    <ChevronDown
                      v-if="expandedSeasons.has(seasonGrp.key)"
                      :size="16"
                      aria-hidden="true"
                    />
                    <ChevronRight v-else :size="16" aria-hidden="true" />
                    <FolderOpen :size="15" class="dl-type-icon" aria-hidden="true" />
                    <strong>{{ t('details.season') }} {{ seasonGrp.season }}</strong>
                    <span class="v-badge">
                      {{ seasonGrp.episodes.length }} {{ t('downloads.episodes_count') }}
                    </span>
                    <span class="v-badge">
                      {{ seasonGrp.totalStreams }} {{ t('downloads.streams_count') }}
                    </span>
                    <span
                      v-if="seasonGrp.allWatched"
                      class="v-badge v-badge-success dl-watched-pill"
                    >
                      <CheckCircle2 :size="12" aria-hidden="true" /> {{ t('downloads.watched') }}
                    </span>
                  </div>
                  <span class="v-muted dl-stats">{{ formatBytes(seasonGrp.downloadedBytes) }}</span>
                </div>

                <!-- Level 3: Episodes inside Season -->
                <Transition name="page-fade">
                  <div v-if="expandedSeasons.has(seasonGrp.key)" class="dl-episodes-level">
                    <div
                      v-for="epGrp in seasonGrp.episodes"
                      :key="epGrp.key"
                      class="dl-episode-box"
                    >
                      <div
                        role="button"
                        tabindex="0"
                        class="dl-sub-header dl-ep-header"
                        :aria-expanded="expandedEpisodes.has(epGrp.key)"
                        @click="toggleEpisode(epGrp.key)"
                        @keydown.enter.self="toggleEpisode(epGrp.key)"
                        @keydown.space.prevent.self="toggleEpisode(epGrp.key)"
                      >
                        <div class="dl-group-left">
                          <ChevronDown
                            v-if="expandedEpisodes.has(epGrp.key)"
                            :size="15"
                            aria-hidden="true"
                          />
                          <ChevronRight v-else :size="15" aria-hidden="true" />
                          <strong>
                            {{ t('downloads.episode_label') }} {{ epGrp.episode }}
                          </strong>
                          <span class="v-badge">
                            S{{ epGrp.season }}·E{{ epGrp.episode }}
                          </span>
                          <span class="v-badge">
                            {{ epGrp.items.length }} {{ t('downloads.streams_count') }}
                          </span>
                          <span
                            v-if="epGrp.progress?.completed"
                            class="v-badge v-badge-success dl-watched-pill"
                          >
                            <CheckCircle2 :size="12" aria-hidden="true" />
                            {{ t('downloads.watched') }}
                          </span>
                          <span
                            v-else-if="epGrp.progress && epGrp.progress.positionMs > 0"
                            class="v-badge v-badge-accent"
                          >
                            {{ watchPercent(epGrp.progress) }}%
                          </span>
                        </div>

                        <div class="dl-actions" @click.stop>
                          <span class="v-muted dl-stats">
                            {{ formatBytes(epGrp.downloadedBytes) }}
                          </span>
                          <button
                            v-if="epGrp.items.some((i) => i.status === 'completed')"
                            type="button"
                            class="v-btn v-btn-primary v-btn-sm"
                            @click="
                              playLocalDownload(
                                epGrp.items.find((i) => i.status === 'completed')!,
                                epGrp.progress,
                              )
                            "
                          >
                            <Play :size="13" fill="currentColor" aria-hidden="true" />
                            {{ t('common.play') }}
                          </button>
                        </div>
                      </div>

                      <!-- Level 4: Stream Candidates inside Episode -->
                      <Transition name="page-fade">
                        <div v-if="expandedEpisodes.has(epGrp.key)" class="dl-streams-level">
                          <div
                            v-for="dl in epGrp.items"
                            :key="dl.id"
                            class="dl-stream-row"
                          >
                            <button
                              type="button"
                              class="dl-stream-link"
                              :disabled="dl.status !== 'completed' || launchingDownloadId === dl.id"
                              @click="playLocalDownload(dl, epGrp.progress)"
                            >
                              <Play
                                v-if="dl.status === 'completed'"
                                :size="14"
                                fill="currentColor"
                                class="dl-stream-play-icon"
                                aria-hidden="true"
                              />
                              <span class="dl-stream-name">
                                {{
                                  dl.streamLabel ||
                                  `${t('downloads.stream_default')} (${dl.kind.toUpperCase()})`
                                }}
                              </span>
                              <CheckCircle2
                                v-if="dl.status === 'completed' && epGrp.progress?.completed"
                                :size="14"
                                class="dl-stream-watched"
                                :title="t('downloads.watched')"
                                aria-hidden="true"
                              />
                            </button>

                            <div class="dl-stream-meta">
                              <span class="v-badge v-badge-accent">{{ dl.status }}</span>
                              <span class="v-badge">{{ dl.kind.toUpperCase() }}</span>
                              <span class="v-muted dl-stats">
                                {{ formatBytes(dl.bytesDownloaded) }}
                                <template v-if="dl.totalBytes">
                                  / {{ formatBytes(dl.totalBytes) }}
                                </template>
                              </span>
                            </div>

                            <div class="dl-actions">
                              <button
                                v-if="dl.status === 'downloading' || dl.status === 'queued'"
                                type="button"
                                class="v-btn v-btn-sm"
                                :disabled="busyIds.has(dl.id)"
                                @click="pauseItem(dl.id)"
                              >
                                <Pause :size="13" aria-hidden="true" /> {{ t('common.pause') }}
                              </button>
                              <button
                                v-if="dl.status === 'paused' || dl.status === 'failed'"
                                type="button"
                                class="v-btn v-btn-primary v-btn-sm"
                                :disabled="busyIds.has(dl.id)"
                                @click="resumeItem(dl.id)"
                              >
                                <Play :size="13" aria-hidden="true" /> {{ t('common.resume') }}
                              </button>
                              <button
                                type="button"
                                class="v-btn v-btn-danger v-btn-sm"
                                :disabled="busyIds.has(dl.id)"
                                @click="deleteItem(dl.id)"
                              >
                                <Trash2 :size="13" aria-hidden="true" /> {{ t('common.delete') }}
                              </button>
                            </div>
                          </div>
                        </div>
                      </Transition>
                    </div>
                  </div>
                </Transition>
              </div>
            </template>
          </div>
        </Transition>
      </div>
    </div>
  </section>
</template>

<style scoped>
.downloads-view,
.dl-stack,
.dl-queue-list,
.dl-section {
  display: flex;
  flex-direction: column;
  gap: var(--vessel-gap);
}

.dl-sec-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.75rem;
}

.dl-sec-head h2 {
  margin: 0;
  font-size: 1.08rem;
}

.dl-drag-hint {
  font-size: 0.8rem;
}

.dl-cw-rail {
  display: flex;
  gap: 0.85rem;
  overflow-x: auto;
  padding-bottom: 0.25rem;
}

.dl-cw-card {
  flex: 0 0 250px;
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  gap: 0.4rem;
  text-align: left;
  cursor: pointer;
  transition: transform 0.2s ease, border-color 0.2s ease;
}

.dl-cw-card:hover {
  transform: translateY(-3px);
  border-color: var(--vessel-accent);
}

.dl-cw-top {
  width: 100%;
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.dl-cw-play {
  width: 28px;
  height: 28px;
  border-radius: 999px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  background: var(--vessel-accent);
  color: #ffffff;
}

.dl-cw-rem {
  font-size: 0.76rem;
  font-weight: 650;
  color: var(--vessel-text-muted);
}

.dl-cw-title,
.dl-cw-sub {
  width: 100%;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.dl-cw-sub {
  font-size: 0.78rem;
}

.dl-cw-bar {
  width: 100%;
  height: 4px;
  border-radius: 999px;
  background: var(--vessel-border);
  overflow: hidden;
  margin-top: 0.2rem;
}

.dl-cw-bar-fill {
  height: 100%;
  background: linear-gradient(90deg, var(--vessel-accent-secondary), var(--vessel-accent));
}

.dl-queue-row {
  display: flex;
  align-items: center;
  gap: 0.7rem;
  flex-wrap: wrap;
  transition: border-color 0.2s ease, background 0.2s ease;
}

.dl-queue-row.is-dragging {
  border-color: var(--vessel-accent);
  background: color-mix(in srgb, var(--vessel-accent) 10%, var(--vessel-surface-card));
}

.dl-drag-handle {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  padding: 0.35rem;
  border: none;
  background: transparent;
  color: var(--vessel-text-muted);
  cursor: grab;
}

.dl-drag-handle:active {
  cursor: grabbing;
  color: var(--vessel-accent);
}

.dl-queue-num {
  min-width: 1.9rem;
  font-size: 0.82rem;
  font-weight: 700;
  color: var(--vessel-accent);
  font-variant-numeric: tabular-nums;
}

.dl-content-card {
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
}

.dl-group-header,
.dl-sub-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.75rem;
  cursor: pointer;
  flex-wrap: wrap;
}

.dl-group-left {
  display: flex;
  align-items: center;
  gap: 0.55rem;
  min-width: 0;
  flex: 1;
  flex-wrap: wrap;
}

.dl-group-meta {
  display: flex;
  flex-direction: column;
  gap: 0.25rem;
  min-width: 0;
}

.dl-type-icon {
  color: var(--vessel-accent);
  flex-shrink: 0;
}

.dl-watched-pill {
  display: inline-flex;
  align-items: center;
  gap: 0.28rem;
}

.dl-children-level {
  display: flex;
  flex-direction: column;
  gap: 0.65rem;
  padding-top: 0.7rem;
  border-top: 1px solid var(--vessel-border);
}

.dl-season-box {
  display: flex;
  flex-direction: column;
  gap: 0.55rem;
  padding: 0.7rem 0.85rem;
  border-radius: var(--vessel-radius-sm);
  border: 1px solid var(--vessel-border);
  background: var(--vessel-bg-surface);
}

.dl-episodes-level {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
  padding-left: 1.15rem;
  border-left: 2px solid color-mix(in srgb, var(--vessel-accent) 35%, transparent);
}

.dl-episode-box {
  display: flex;
  flex-direction: column;
  gap: 0.45rem;
  padding: 0.55rem 0.7rem;
  border-radius: var(--vessel-radius-sm);
  background: var(--vessel-surface-elevated);
  border: 1px solid var(--vessel-border);
}

.dl-streams-level {
  display: flex;
  flex-direction: column;
  gap: 0.4rem;
  padding-top: 0.45rem;
  padding-left: 1rem;
  border-top: 1px solid var(--vessel-border);
}

.dl-stream-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.75rem;
  flex-wrap: wrap;
  padding: 0.5rem 0.7rem;
  border-radius: var(--vessel-radius-sm);
  background: var(--vessel-surface-input);
  border: 1px solid var(--vessel-border);
}

.dl-stream-link {
  display: inline-flex;
  align-items: center;
  gap: 0.45rem;
  border: none;
  background: transparent;
  padding: 0;
  color: var(--vessel-text-primary);
  font-weight: 600;
  cursor: pointer;
  text-align: left;
}

.dl-stream-link:not(:disabled):hover .dl-stream-name {
  color: var(--vessel-accent);
  text-decoration: underline;
}

.dl-stream-link:disabled {
  cursor: default;
  opacity: 0.75;
}

.dl-stream-play-icon {
  color: var(--vessel-accent);
}

.dl-stream-watched {
  color: var(--vessel-success);
}

.dl-stream-meta {
  display: flex;
  align-items: center;
  gap: 0.45rem;
  flex-wrap: wrap;
}

.dl-info {
  flex: 1;
  min-width: 220px;
  display: flex;
  flex-direction: column;
  gap: 0.25rem;
}

.dl-dir-line {
  margin: 0.25rem 0 0;
  font-size: 0.8rem;
}

.dl-head,
.dl-stats,
.dl-actions {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  flex-wrap: wrap;
}

.dl-err {
  margin: 0;
  color: var(--vessel-danger);
  font-size: 0.85rem;
}
</style>
