import { ref, shallowRef } from 'vue'
import { listenVesselEvents, vesselApi } from '@/api/vessel'
import type {
  DownloadProgressSnapshot,
  EngineEvent,
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
const playerState = ref<PlayerStateSnapshot | null>(null)
const pendingDomain = ref<PendingDomainPrompt | null>(null)
const downloadProgress = ref<Record<string, DownloadProgressSnapshot>>({})
const lastEnrichedDetail = ref<{ pluginId: string; nativeId: string; detail: MediaDetail } | null>(
  null,
)
const toasts = ref<ToastItem[]>([])
let unlisten: (() => void) | null = null

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

  async function resolveDomainPrompt(allow: boolean, permanent: boolean): Promise<void> {
    const prompt = pendingDomain.value
    pendingDomain.value = null
    if (!prompt || !allow) return
    await vesselApi.grantPluginDomain(prompt.pluginId, prompt.domain, permanent)
    pushToast('info', `Granted ${prompt.domain} to ${prompt.pluginId}`)
  }

  return {
    activeSearchId,
    isSearching,
    searchBuckets,
    playerState,
    pendingDomain,
    downloadProgress,
    lastEnrichedDetail,
    toasts,
    startListening,
    stopListening,
    startSearch,
    resolveDomainPrompt,
    pushToast,
  }
}
