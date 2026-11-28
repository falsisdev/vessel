import { ref, shallowRef } from 'vue'
import { listenVesselEvents, vesselApi } from '@/api/vessel'
import type {
  DownloadProgressSnapshot,
  EngineEvent,
  HomeShelfResult,
  MediaDetail,
  MediaSummary,
  PlayerStateSnapshot,
} from '@/types/vessel'

export interface PendingDomainPrompt {
  requestId: string
  pluginId: string
  domain: string
}

export interface ToastItem {
  id: number
  tone: 'info' | 'warn' | 'error'
  message: string
}

let toastSeq = 1

const activeSearchId = shallowRef<string>('')
const isSearching = shallowRef(false)
const searchBuckets = ref<Record<string, MediaSummary[]>>({})
const activeHomeRequestId = shallowRef<string>('')
const isLoadingHome = shallowRef(false)
const homeStreamShelves = ref<HomeShelfResult[]>([])
const playerState = ref<PlayerStateSnapshot | null>(null)
const pendingDomain = ref<PendingDomainPrompt | null>(null)
const downloadProgress = ref<Record<string, DownloadProgressSnapshot>>({})
const lastEnrichedDetail = ref<{ pluginId: string; nativeId: string; detail: MediaDetail } | null>(
  null,
)
export const lastOAuthEvent = ref<{ provider: string; success: boolean; error?: string; seq: number } | null>(
  null,
)
let oauthSeq = 1
const toasts = ref<ToastItem[]>([])
let unlisten: (() => void) | null = null
const pendingHomeShelves = new Map<string, HomeShelfResult>()
let homeFlushTimer: ReturnType<typeof setTimeout> | null = null

function flushPendingHomeShelves(): void {
  if (homeFlushTimer !== null) {
    clearTimeout(homeFlushTimer)
    homeFlushTimer = null
  }
  if (pendingHomeShelves.size === 0) return
  const merged = new Map<string, HomeShelfResult>()
  for (const shelf of homeStreamShelves.value) {
    merged.set(shelf.pluginId, shelf)
  }
  for (const [pid, shelf] of pendingHomeShelves) {
    merged.set(pid, shelf)
  }
  pendingHomeShelves.clear()
  homeStreamShelves.value = Array.from(merged.values())
}

export function pushToast(tone: ToastItem['tone'], message: string): void {
  const id = toastSeq++
  toasts.value = [...toasts.value.slice(-4), { id, tone, message }]
  setTimeout(() => {
    toasts.value = toasts.value.filter((t) => t.id !== id)
  }, 4500)
}

function handleEngineEvent(event: EngineEvent): void {
  switch (event.type) {
    case 'search_partial_result': {
      if (event.searchId !== activeSearchId.value) return
      searchBuckets.value = {
        ...searchBuckets.value,
        [event.pluginId]: event.items,
      }
      break
    }
    case 'search_completed': {
      if (event.searchId === activeSearchId.value) {
        isSearching.value = false
      }
      break
    }
    case 'home_feed_partial_result': {
      if (event.requestId !== activeHomeRequestId.value) return
      for (const sec of event.sections) {
        for (const item of sec.items) {
          if (!item.pluginId) {
            item.pluginId = event.pluginId
          }
        }
      }
      const shelf: HomeShelfResult = {
        pluginId: event.pluginId,
        pluginName: event.pluginName,
        sections: event.sections,
        error: event.error,
      }
      pendingHomeShelves.set(event.pluginId, shelf)
      if (homeStreamShelves.value.length === 0) {
        flushPendingHomeShelves()
      } else if (homeFlushTimer === null) {
        homeFlushTimer = setTimeout(flushPendingHomeShelves, 50)
      }
      break
    }
    case 'home_feed_completed': {
      if (event.requestId === activeHomeRequestId.value) {
        flushPendingHomeShelves()
        isLoadingHome.value = false
      }
      break
    }
    case 'metadata_enriched': {
      lastEnrichedDetail.value = {
        pluginId: event.pluginId,
        nativeId: event.nativeId,
        detail: event.detail,
      }
      break
    }
    case 'player_state_changed': {
      playerState.value = event
      break
    }
    case 'domain_permission_requested': {
      pendingDomain.value = {
        requestId: event.requestId,
        pluginId: event.pluginId,
        domain: event.domain,
      }
      break
    }
    case 'download_progress': {
      downloadProgress.value = {
        ...downloadProgress.value,
        [event.downloadId]: event,
      }
      break
    }
    case 'plugin_update_available': {
      pushToast(
        'info',
        `${event.pluginId}: update available (${event.currentVersion} → ${event.latestVersion})`,
      )
      break
    }
    case 'plugin_log': {
      if (event.level === 'error' || event.level === 'warn') {
        pushToast(event.level === 'error' ? 'error' : 'warn', `[${event.pluginId}] ${event.message}`)
      }
      break
    }
    case 'oauth_completed': {
      lastOAuthEvent.value = {
        provider: event.provider,
        success: event.success,
        error: event.error,
        seq: oauthSeq++,
      }
      if (event.success) {
        pushToast('info', `${event.provider}: connected`)
      } else if (event.error) {
        pushToast('error', `${event.provider}: ${event.error}`)
      }
      break
    }
  }
}

export function useVesselEvents() {
  async function startListening(): Promise<void> {
    if (unlisten) return
    unlisten = await listenVesselEvents(handleEngineEvent)
  }

  function stopListening(): void {
    if (unlisten) {
      unlisten()
      unlisten = null
    }
  }

  async function startSearch(
    query: string,
    filters?: Record<string, string>,
    pluginIds?: string[],
  ): Promise<void> {
    searchBuckets.value = {}
    isSearching.value = true
    try {
      const searchId = await vesselApi.searchAll(query, filters, 1, pluginIds)
      activeSearchId.value = searchId
    } catch (err) {
      isSearching.value = false
      throw err
    }
  }

  function resetSearch(): void {
    const wasSearching = isSearching.value
    activeSearchId.value = ''
    isSearching.value = false
    searchBuckets.value = {}
    if (wasSearching) {
      void vesselApi.cancelSearch().catch(() => {})
    }
  }

  async function startHomeFeedsStream(pluginIds?: string[] | null): Promise<string> {
    homeStreamShelves.value = []
    isLoadingHome.value = true
    try {
      const requestId = await vesselApi.startHomeFeeds(pluginIds)
      activeHomeRequestId.value = requestId
      return requestId
    } catch (err) {
      isLoadingHome.value = false
      throw err
    }
  }

  async function resolveDomainPrompt(allow: boolean, permanent: boolean): Promise<void> {
    const prompt = pendingDomain.value
    pendingDomain.value = null
    if (!prompt || !allow) return
    await vesselApi.grantPluginDomain(prompt.pluginId, prompt.domain, permanent)
    pushToast('info', `Granted ${prompt.domain} to ${prompt.pluginId}`)
  }

  function setDownloadSnapshot(snapshot: DownloadProgressSnapshot): void {
    downloadProgress.value = {
      ...downloadProgress.value,
      [snapshot.downloadId]: snapshot,
    }
  }

  function clearDownloadProgress(downloadId: string): void {
    if (!(downloadId in downloadProgress.value)) return
    const copy = { ...downloadProgress.value }
    delete copy[downloadId]
    downloadProgress.value = copy
  }

  function pruneStaleDownloadProgress(validIds: Set<string>): void {
    const next: Record<string, DownloadProgressSnapshot> = {}
    for (const [id, snap] of Object.entries(downloadProgress.value)) {
      if (validIds.has(id)) {
        next[id] = snap
      }
    }
    downloadProgress.value = next
  }

  return {
    activeSearchId,
    isSearching,
    searchBuckets,
    activeHomeRequestId,
    isLoadingHome,
    homeStreamShelves,
    playerState,
    pendingDomain,
    downloadProgress,
    lastEnrichedDetail,
    lastOAuthEvent,
    toasts,
    startListening,
    stopListening,
    startSearch,
    resetSearch,
    startHomeFeedsStream,
    resolveDomainPrompt,
    setDownloadSnapshot,
    clearDownloadProgress,
    pruneStaleDownloadProgress,
    pushToast,
  }
}
