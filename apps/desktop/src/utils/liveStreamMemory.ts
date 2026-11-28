import { isTauriRuntime, vesselApi } from '@/api/vessel'
import type { StreamCandidate } from '@/types/vessel'

const LIVE_STREAM_PREFS_KEY = 'live_channel_preferred_streams'
const LOCAL_CACHE_KEY = 'vessel_live_preferred_streams'

export interface LiveChannelStreamPref {
  url: string
  label?: string
  index: number
  updatedAtEpochMs: number
}

let memoryMap: Record<string, LiveChannelStreamPref> | null = null
let loadPromise: Promise<Record<string, LiveChannelStreamPref>> | null = null

export function buildLiveChannelMemoryKey(
  pluginId: string,
  nativeId?: string | null,
  streamId?: string | null,
): string {
  const id = (streamId?.trim() || nativeId?.trim() || '').trim()
  return `${pluginId}::${id}`
}

function readLocalCache(): Record<string, LiveChannelStreamPref> {
  try {
    const raw = typeof localStorage !== 'undefined' ? localStorage.getItem(LOCAL_CACHE_KEY) : null
    if (!raw) return {}
    const parsed = JSON.parse(raw)
    return parsed && typeof parsed === 'object' ? (parsed as Record<string, LiveChannelStreamPref>) : {}
  } catch {
    return {}
  }
}

export async function loadLiveStreamMemory(): Promise<Record<string, LiveChannelStreamPref>> {
  if (memoryMap) return memoryMap
  if (loadPromise) return loadPromise
  loadPromise = (async () => {
    const local = readLocalCache()
    if (isTauriRuntime()) {
      try {
        const raw = await vesselApi.getAppSetting(LIVE_STREAM_PREFS_KEY)
        if (raw) {
          const remote = JSON.parse(raw) as Record<string, LiveChannelStreamPref>
          memoryMap = { ...local, ...remote }
          return memoryMap
        }
      } catch {
        // Non-fatal fallback to local cache
      }
    }
    memoryMap = local
    return memoryMap
  })()
  return loadPromise
}

export async function resolvePreferredLiveStreamIndex(
  pluginId: string,
  nativeId: string | null | undefined,
  streamId: string | null | undefined,
  streams: StreamCandidate[],
): Promise<number> {
  if (!streams.length) return 0
  const fallbackIdx = Math.max(0, streams.findIndex((s) => s.isDefault))
  const map = await loadLiveStreamMemory()
  const key = buildLiveChannelMemoryKey(pluginId, nativeId, streamId)
  const altKey = nativeId && streamId && nativeId !== streamId
    ? buildLiveChannelMemoryKey(pluginId, nativeId, nativeId)
    : ''
  const pref = map[key] ?? (altKey ? map[altKey] : undefined)
  if (!pref) return fallbackIdx

  if (pref.url) {
    const byUrl = streams.findIndex((s) => s.url === pref.url)
    if (byUrl >= 0) return byUrl
  }
  if (pref.label) {
    const norm = pref.label.trim().toLowerCase()
    const byLabel = streams.findIndex((s) => s.label.trim().toLowerCase() === norm)
    if (byLabel >= 0) return byLabel
  }
  if (Number.isFinite(pref.index) && pref.index >= 0 && pref.index < streams.length) {
    return pref.index
  }
  return fallbackIdx
}

export async function savePreferredLiveStream(
  pluginId: string,
  nativeId: string | null | undefined,
  streamId: string | null | undefined,
  candidate: StreamCandidate | undefined,
  index: number,
): Promise<void> {
  if (!candidate) return
  const current = { ...(await loadLiveStreamMemory()) }
  const key = buildLiveChannelMemoryKey(pluginId, nativeId, streamId)
  const entry: LiveChannelStreamPref = {
    url: candidate.url,
    label: candidate.label,
    index,
    updatedAtEpochMs: Date.now(),
  }
  current[key] = entry
  if (Object.keys(current).length > 350) {
    const trimmed = Object.entries(current)
      .sort((a, b) => (b[1].updatedAtEpochMs ?? 0) - (a[1].updatedAtEpochMs ?? 0))
      .slice(0, 300)
    memoryMap = Object.fromEntries(trimmed)
  } else {
    memoryMap = current
  }
  const encoded = JSON.stringify(memoryMap)
  try {
    if (typeof localStorage !== 'undefined') {
      localStorage.setItem(LOCAL_CACHE_KEY, encoded)
    }
  } catch {
    // Ignore storage quota issues
  }
  if (isTauriRuntime()) {
    try {
      await vesselApi.setAppSetting(LIVE_STREAM_PREFS_KEY, encoded)
    } catch {
      // Non-fatal
    }
  }
}
