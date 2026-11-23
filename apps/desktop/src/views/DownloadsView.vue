<script setup lang="ts">
import { computed, onMounted, ref, shallowRef } from 'vue'
import { Pause, Play, RefreshCw, Trash2 } from 'lucide-vue-next'
import { isTauriRuntime, vesselApi } from '@/api/vessel'
import { useI18n } from '@/composables/useI18n'
import { pushToast, useVesselEvents } from '@/composables/useVesselEvents'
import type { StoredDownload } from '@/types/vessel'

const { t } = useI18n()
const { downloadProgress } = useVesselEvents()

const downloads = ref<StoredDownload[]>([])
const confirmDeleteId = shallowRef<string>('')

const mergedDownloads = computed(() =>
  downloads.value.map((dl) => {
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
  }),
)

function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`
}

async function refresh(): Promise<void> {
  if (!isTauriRuntime()) return
  try {
    downloads.value = await vesselApi.listDownloads()
  } catch (err) {
    pushToast('error', String(err))
  }
}

async function pauseItem(id: string): Promise<void> {
  try {
    await vesselApi.pauseDownload(id)
    await refresh()
  } catch (err) {
    pushToast('error', String(err))
  }
}

async function resumeItem(id: string): Promise<void> {
  try {
    await vesselApi.resumeDownload(id)
    await refresh()
  } catch (err) {
    pushToast('error', String(err))
  }
}

async function deleteItem(id: string): Promise<void> {
  if (confirmDeleteId.value !== id) {
    confirmDeleteId.value = id
    setTimeout(() => {
      if (confirmDeleteId.value === id) confirmDeleteId.value = ''
    }, 3000)
    return
  }
  confirmDeleteId.value = ''
  try {
    await vesselApi.deleteDownload(id)
    await refresh()
  } catch (err) {
    pushToast('error', String(err))
  }
}

onMounted(() => {
  void refresh()
})
</script>

<template>
  <section class="downloads-view v-page-container">
    <header class="v-page-header">
      <h1 class="v-page-title">{{ t('downloads.title') }}</h1>
      <button class="v-btn" @click="refresh">
        <RefreshCw :size="15" /> {{ t('common.refresh') }}
      </button>
    </header>

    <div v-if="!mergedDownloads.length" class="v-card v-muted">{{ t('downloads.empty') }}</div>

    <div v-else class="dl-stack">
      <div v-for="dl in mergedDownloads" :key="dl.id" class="v-card dl-row">
        <div class="dl-info">
          <div class="dl-head">
            <strong>{{ dl.title }}</strong>
            <span class="v-badge v-badge-accent">{{ dl.status }}</span>
            <span class="v-badge">{{ dl.kind }}</span>
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
            v-if="dl.status === 'downloading' || dl.status === 'queued'"
            class="v-btn"
            @click="pauseItem(dl.id)"
          >
            <Pause :size="14" /> {{ t('common.pause') }}
          </button>
          <button
            v-if="dl.status === 'paused' || dl.status === 'failed'"
            class="v-btn v-btn-primary"
            @click="resumeItem(dl.id)"
          >
            <Play :size="14" /> {{ t('common.resume') }}
          </button>
          <button type="button" class="v-btn v-btn-danger" @click="deleteItem(dl.id)">
            <Trash2 :size="14" aria-hidden="true" />
            {{ confirmDeleteId === dl.id ? 'Confirm Delete' : t('common.delete') }}
          </button>
        </div>
      </div>
    </div>
  </section>
</template>

<style scoped>
.downloads-view,
.dl-stack {
  display: flex;
  flex-direction: column;
  gap: var(--vessel-gap);
}

.dl-row {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 0.75rem;
  flex-wrap: wrap;
}

.dl-info {
  display: flex;
  flex-direction: column;
  gap: 0.3rem;
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
