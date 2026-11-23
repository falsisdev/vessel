<script setup lang="ts">
import { computed } from 'vue'
import { Pause, Play, Square, Subtitles } from 'lucide-vue-next'
import { vesselApi } from '@/api/vessel'
import { useI18n } from '@/composables/useI18n'
import { pushToast } from '@/composables/useVesselEvents'
import type { PlayerStateSnapshot } from '@/types/vessel'

const props = defineProps<{
  state: PlayerStateSnapshot | null
  visible: boolean
}>()

const { t } = useI18n()

function formatMs(ms: number): string {
  const secs = Math.floor((ms || 0) / 1000)
  const m = Math.floor(secs / 60)
  const s = secs % 60
  return `${m}:${String(s).padStart(2, '0')}`
}

const active = computed(() => props.visible && props.state && (props.state.isPlaying || props.state.isPaused))

async function togglePlayPause(): Promise<void> {
  if (!props.state) return
  try {
    if (props.state.isPaused) {
      await vesselApi.playerPlay()
    } else {
      await vesselApi.playerPause()
    }
  } catch (err) {
    pushToast('error', String(err))
  }
}

async function stopPlayback(): Promise<void> {
  try {
    await vesselApi.playerStop()
  } catch (err) {
    pushToast('error', String(err))
  }
}

async function shiftDelay(deltaMs: number): Promise<void> {
  const next = (props.state?.subtitleDelayMs ?? 0) + deltaMs
  try {
    await vesselApi.setSubtitleDelay(next)
  } catch (err) {
    pushToast('error', String(err))
  }
}
</script>

<template>
  <Transition name="page-fade">
    <div v-if="active && state" class="now-playing">
      <div class="np-info">
        <span class="v-badge v-badge-accent">{{ t('overlay.now_playing') }}</span>
        <span class="np-time">{{ formatMs(state.positionMs) }} / {{ formatMs(state.durationMs) }}</span>
        <span class="np-url v-muted">{{ state.currentStreamUrl ?? 'mpv' }}</span>
      </div>

      <div class="np-actions">
        <button
          type="button"
          class="v-btn v-btn-icon"
          :aria-label="state.isPaused ? t('common.play') : t('common.pause')"
          @click="togglePlayPause"
        >
          <Play v-if="state.isPaused" :size="15" aria-hidden="true" />
          <Pause v-else :size="15" aria-hidden="true" />
        </button>
        <button
          type="button"
          class="v-btn v-btn-icon v-btn-danger"
          :aria-label="t('common.stop')"
          @click="stopPlayback"
        >
          <Square :size="14" aria-hidden="true" />
        </button>
        <div class="sub-delay">
          <Subtitles :size="15" aria-hidden="true" />
          <span class="v-muted">{{ t('overlay.subtitle_delay') }}: {{ state.subtitleDelayMs }}ms</span>
          <button type="button" class="v-btn v-btn-sm" @click="shiftDelay(-100)">-100ms</button>
          <button type="button" class="v-btn v-btn-sm" @click="shiftDelay(100)">+100ms</button>
        </div>
      </div>
    </div>
  </Transition>
</template>

<style scoped>
.now-playing {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  justify-content: space-between;
  gap: 0.75rem;
  padding: 0.65rem 1.35rem;
  background: var(--vessel-surface-overlay);
  border-top: 1px solid var(--vessel-border);
}

.np-info,
.np-actions,
.sub-delay {
  display: flex;
  align-items: center;
  gap: 0.55rem;
  min-width: 0;
}

.np-url {
  max-width: 280px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-size: 0.84rem;
}

.np-time {
  font-variant-numeric: tabular-nums;
  font-weight: 600;
}
</style>
