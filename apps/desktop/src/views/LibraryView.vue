<script setup lang="ts">
import { onActivated, onMounted, shallowRef } from 'vue'
import { useRouter } from 'vue-router'
import { X } from 'lucide-vue-next'
import { isTauriRuntime, vesselApi } from '@/api/vessel'
import MediaCard from '@/components/media/MediaCard.vue'
import { useI18n } from '@/composables/useI18n'
import { useUiPreferences } from '@/composables/useUiPreferences'
import { pushToast } from '@/composables/useVesselEvents'
import type { LibraryItem, LibraryStatus } from '@/types/vessel'

const router = useRouter()
const { t } = useI18n()
const { uiPreferences } = useUiPreferences()

const activeFilter = shallowRef<LibraryStatus | 'all'>('all')
const items = shallowRef<LibraryItem[]>([])
const loading = shallowRef(isTauriRuntime())

defineOptions({ name: 'LibraryView' })

const FILTERS: Array<{ value: LibraryStatus | 'all'; key: string }> = [
  { value: 'all', key: 'library.all' },
  { value: 'watching', key: 'library.watching' },
  { value: 'plan_to_watch', key: 'library.plan_to_watch' },
  { value: 'completed', key: 'library.completed' },
  { value: 'on_hold', key: 'library.on_hold' },
  { value: 'dropped', key: 'library.dropped' },
]

async function loadLibrary(filter = activeFilter.value): Promise<void> {
  activeFilter.value = filter
  if (!isTauriRuntime()) {
    loading.value = false
    return
  }
  loading.value = true
  try {
    items.value = await vesselApi.listLibrary(filter === 'all' ? undefined : filter)
  } catch (err) {
    pushToast('error', String(err))
  } finally {
    loading.value = false
  }
}

function openDetails(pluginId: string, nativeId: string): void {
  void router.push({ name: 'details', params: { pluginId, nativeId } })
}

async function removeEntry(entry: LibraryItem): Promise<void> {
  try {
    await vesselApi.deleteLibraryItem(entry.media.pluginId, entry.media.nativeId)
    items.value = items.value.filter(
      (i) => !(i.media.pluginId === entry.media.pluginId && i.media.nativeId === entry.media.nativeId),
    )
  } catch (err) {
    pushToast('error', String(err))
  }
}

onMounted(() => {
  void loadLibrary('all')
})

onActivated(() => {
  void loadLibrary(activeFilter.value)
})
</script>

<template>
  <section class="library-view v-page-container">
    <header class="v-page-header">
      <h1 class="v-page-title">{{ t('library.title') }}</h1>
      <div class="tab-row">
        <button
          v-for="tab in FILTERS"
          :key="tab.value"
          class="v-btn"
          :class="{ 'v-btn-primary': activeFilter === tab.value }"
          @click="loadLibrary(tab.value)"
        >
          {{ t(tab.key) }}
        </button>
      </div>
    </header>

    <div v-if="loading && !items.length" class="v-grid">
      <div v-for="n in 8" :key="n" class="lib-skeleton-card">
        <div class="v-skeleton lib-skeleton-poster" />
        <div class="v-skeleton lib-skeleton-line" />
        <div class="v-skeleton lib-skeleton-line short" />
      </div>
    </div>

    <div v-else-if="!items.length" class="v-card v-muted">{{ t('common.empty') }}</div>

    <div v-else class="v-grid">
      <div
        v-for="entry in items"
        :key="`${entry.media.pluginId}:${entry.media.nativeId}`"
        class="lib-card-wrap"
      >
        <MediaCard
          :item="{
            pluginId: entry.media.pluginId,
            nativeId: entry.media.nativeId,
            type: entry.mediaType,
            isAnime: entry.isAnime,
            title: entry.title,
            poster: entry.posterUrl,
            year: entry.year,
          }"
          :cards="uiPreferences.cards"
          @open="openDetails"
        />
        <button
          type="button"
          class="lib-remove-btn"
          :title="t('library.remove_item')"
          :aria-label="t('library.remove_item')"
          @click.stop="removeEntry(entry)"
        >
          <X :size="14" aria-hidden="true" />
        </button>
      </div>
    </div>
  </section>
</template>

<style scoped>
.library-view {
  display: flex;
  flex-direction: column;
  gap: var(--vessel-gap);
}

.tab-row {
  display: flex;
  gap: 0.45rem;
  flex-wrap: wrap;
}

.lib-card-wrap {
  position: relative;
}

.lib-skeleton-card {
  display: flex;
  flex-direction: column;
  gap: 0.45rem;
}

.lib-skeleton-poster {
  aspect-ratio: 2 / 3;
}

.lib-skeleton-line {
  width: 82%;
  height: 13px;
  border-radius: 6px;
}

.lib-skeleton-line.short {
  width: 50%;
  height: 11px;
}

.lib-remove-btn {
  position: absolute;
  top: 0.45rem;
  right: 0.45rem;
  z-index: 6;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 26px;
  height: 26px;
  border-radius: 999px;
  border: 1px solid rgba(255, 255, 255, 0.22);
  background: rgba(10, 11, 18, 0.84);
  color: #ffffff;
  opacity: 0;
  cursor: pointer;
  transition: opacity 0.18s ease, background-color 0.18s ease;
}

.lib-card-wrap:hover .lib-remove-btn {
  opacity: 1;
}

.lib-remove-btn:hover {
  background: rgba(239, 68, 68, 0.92);
}
</style>
