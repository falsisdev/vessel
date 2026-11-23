<script setup lang="ts">
import { onMounted, ref, shallowRef } from 'vue'
import { useRouter } from 'vue-router'
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
const items = ref<LibraryItem[]>([])

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
  if (!isTauriRuntime()) return
  try {
    items.value = await vesselApi.listLibrary(filter === 'all' ? undefined : filter)
  } catch (err) {
    pushToast('error', String(err))
  }
}

function openDetails(pluginId: string, nativeId: string): void {
  void router.push({ name: 'details', params: { pluginId, nativeId } })
}

onMounted(() => {
  void loadLibrary('all')
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

    <div v-if="!items.length" class="v-card v-muted">{{ t('common.empty') }}</div>

    <div v-else class="v-grid">
      <MediaCard
        v-for="entry in items"
        :key="`${entry.media.pluginId}:${entry.media.nativeId}`"
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
</style>
