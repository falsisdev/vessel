import { shallowRef } from 'vue'
import type { ContinueWatchingEntry, HomeShelfResult, PluginListEntry } from '@/types/vessel'

const cachedShelves = shallowRef<HomeShelfResult[]>([])
const cachedContinueWatching = shallowRef<ContinueWatchingEntry[]>([])
const cachedPlugins = shallowRef<PluginListEntry[]>([])
const cachedFilterKey = shallowRef<string>('')
const hasLoadedOnce = shallowRef(false)

export function useHomeCache() {
  function invalidateHomeCache(): void {
    hasLoadedOnce.value = false
    cachedFilterKey.value = ''
  }

  return {
    cachedShelves,
    cachedContinueWatching,
    cachedPlugins,
    cachedFilterKey,
    hasLoadedOnce,
    invalidateHomeCache,
  }
}
