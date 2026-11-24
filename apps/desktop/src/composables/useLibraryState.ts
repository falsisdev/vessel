import { ref, shallowRef } from 'vue'
import { isTauriRuntime, vesselApi } from '@/api/vessel'
import type { LibraryItem, LibraryStatus, MediaType } from '@/types/vessel'

const libraryMap = ref<Record<string, LibraryItem>>({})
const libraryLoaded = shallowRef(false)

function mediaKey(pluginId: string, nativeId: string): string {
  return `${pluginId}:${nativeId}`
}

export function useLibraryState() {
  async function refreshLibraryState(): Promise<void> {
    if (!isTauriRuntime()) return
    try {
      const items = await vesselApi.listLibrary()
      const next: Record<string, LibraryItem> = {}
      for (const item of items) {
        next[mediaKey(item.media.pluginId, item.media.nativeId)] = item
      }
      libraryMap.value = next
      libraryLoaded.value = true
    } catch {
      // Non-fatal
    }
  }

  function getMediaStatus(pluginId: string | undefined, nativeId: string): LibraryStatus | null {
    if (!pluginId) return null
    return libraryMap.value[mediaKey(pluginId, nativeId)]?.status ?? null
  }

  async function setMediaStatus(params: {
    pluginId: string
    nativeId: string
    mediaType: MediaType
    isAnime: boolean
    title: string
    posterUrl?: string
    year?: number
    status: LibraryStatus
  }): Promise<void> {
    const key = mediaKey(params.pluginId, params.nativeId)
    const now = new Date().toISOString()
    const existing = libraryMap.value[key]
    const item: LibraryItem = {
      media: { pluginId: params.pluginId, nativeId: params.nativeId },
      canonicalId: existing?.canonicalId,
      mediaType: params.mediaType,
      isAnime: params.isAnime,
      title: params.title,
      posterUrl: params.posterUrl,
      year: params.year,
      status: params.status,
      addedAt: existing?.addedAt ?? now,
      updatedAt: now,
    }
    libraryMap.value = { ...libraryMap.value, [key]: item }
    try {
      await vesselApi.upsertLibrary(item)
    } catch (err) {
      await refreshLibraryState()
      throw err
    }
  }

  async function removeMediaStatus(pluginId: string, nativeId: string): Promise<void> {
    const key = mediaKey(pluginId, nativeId)
    const copy = { ...libraryMap.value }
    delete copy[key]
    libraryMap.value = copy
    try {
      await vesselApi.deleteLibraryItem(pluginId, nativeId)
    } catch (err) {
      await refreshLibraryState()
      throw err
    }
  }

  return {
    libraryMap,
    libraryLoaded,
    refreshLibraryState,
    getMediaStatus,
    setMediaStatus,
    removeMediaStatus,
  }
}
