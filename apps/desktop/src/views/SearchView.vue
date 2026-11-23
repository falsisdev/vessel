<script setup lang="ts">
import { computed, shallowRef } from 'vue'
import { useRouter } from 'vue-router'
import { Search as SearchIcon, X } from 'lucide-vue-next'
import { vesselApi } from '@/api/vessel'
import MediaCard from '@/components/media/MediaCard.vue'
import { useI18n } from '@/composables/useI18n'
import { useUiPreferences } from '@/composables/useUiPreferences'
import { pushToast, useVesselEvents } from '@/composables/useVesselEvents'

defineOptions({ name: 'SearchView' })

const router = useRouter()
const { t } = useI18n()
const { uiPreferences } = useUiPreferences()
const { isSearching, searchBuckets, startSearch } = useVesselEvents()

const query = shallowRef('')
const mediaTypeFilter = shallowRef('')

const TYPE_PILLS = [
  { value: '', label: 'All' },
  { value: 'movie', label: 'Movies' },
  { value: 'series', label: 'Series' },
  { value: 'anime', label: 'Anime' },
]

const bucketEntries = computed(() =>
  Object.entries(searchBuckets.value).filter(([, items]) => items.length > 0),
)

async function runSearch(): Promise<void> {
  const trimmed = query.value.trim()
  if (!trimmed) return
  try {
    const filters = mediaTypeFilter.value ? { mediaType: mediaTypeFilter.value } : undefined
    await startSearch(trimmed, filters)
  } catch (err) {
    pushToast('error', String(err))
  }
}

async function stopSearch(): Promise<void> {
  try {
    await vesselApi.cancelSearch()
    isSearching.value = false
  } catch (err) {
    pushToast('error', String(err))
  }
}

function openDetails(pluginId: string, nativeId: string): void {
  void router.push({ name: 'details', params: { pluginId, nativeId } })
}
</script>

<template>
  <section class="search-view v-page-container">
    <header class="v-page-header">
      <h1 class="v-page-title">{{ t('search.title') }}</h1>
      <span v-if="isSearching" class="v-badge v-badge-accent">{{ t('search.searching') }}</span>
    </header>

    <form class="search-bar" @submit.prevent="runSearch">
      <input
        v-model="query"
        type="search"
        name="searchQuery"
        autocomplete="off"
        aria-label="Search media"
        class="v-input"
        :placeholder="t('search.placeholder')"
      />
      <button type="submit" class="v-btn v-btn-primary">
        <SearchIcon :size="15" aria-hidden="true" /> {{ t('common.search') }}
      </button>
      <button v-if="isSearching" type="button" class="v-btn" @click="stopSearch">
        <X :size="15" aria-hidden="true" /> {{ t('common.cancel') }}
      </button>
    </form>

    <div class="type-pills" role="group" aria-label="Filter by media type">
      <button
        v-for="pill in TYPE_PILLS"
        :key="pill.label"
        type="button"
        class="v-btn v-btn-sm"
        :class="{ 'v-btn-primary': mediaTypeFilter === pill.value }"
        @click="mediaTypeFilter = pill.value"
      >
        {{ pill.label }}
      </button>
    </div>

    <div v-if="!bucketEntries.length && !isSearching" class="v-card v-muted">
      {{ t('search.empty') }}
    </div>

    <div v-for="[pluginId, items] in bucketEntries" :key="pluginId" class="bucket">
      <div class="bucket-head">
        <h2 class="bucket-title">{{ pluginId }}</h2>
        <span class="v-badge">{{ items.length }}</span>
      </div>
      <div class="v-grid">
        <MediaCard
          v-for="item in items"
          :key="`${pluginId}:${item.nativeId}`"
          :item="{ ...item, pluginId: item.pluginId ?? pluginId }"
          :cards="uiPreferences.cards"
          @open="openDetails"
        />
      </div>
    </div>
  </section>
</template>

<style scoped>
.search-view,
.bucket {
  display: flex;
  flex-direction: column;
  gap: var(--vessel-gap);
}

.search-bar {
  display: flex;
  gap: 0.65rem;
  flex-wrap: wrap;
}

.search-bar .v-input {
  flex: 1;
  min-width: 240px;
}

.type-pills {
  display: flex;
  gap: 0.45rem;
  flex-wrap: wrap;
}

.bucket-head {
  display: flex;
  align-items: center;
  gap: 0.55rem;
}

.bucket-title {
  margin: 0;
  font-size: 1.1rem;
}
</style>
