import { ref, shallowRef } from 'vue'
import type { ContinueWatchingEntry, HomeShelfResult, PluginListEntry } from '@/types/vessel'

const cachedShelves = ref<HomeShelfResult[]>([])
const cachedContinueWatching = ref<ContinueWatchingEntry[]>([])
const cachedPlugins = ref<PluginListEntry[]>([])
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
