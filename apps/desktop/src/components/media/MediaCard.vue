<script setup lang="ts">
import { computed } from 'vue'
import { Film, Play, Star } from 'lucide-vue-next'
import type { CardPreferences, MediaSummary } from '@/types/vessel'

const props = defineProps<{
  item: MediaSummary
  cards: CardPreferences
}>()

const emit = defineEmits<{
  open: [pluginId: string, nativeId: string]
}>()

const ratioClass = computed(() => {
  if (props.cards.aspectRatio === 'wide_16x9') return 'ratio-wide'
  if (props.cards.aspectRatio === 'square') return 'ratio-square'
  return 'ratio-poster'
})

const ratingText = computed(() =>
  props.item.rating !== undefined ? props.item.rating.toFixed(1) : null,
)

function handleOpen(): void {
  if (props.item.pluginId) {
    emit('open', props.item.pluginId, props.item.nativeId)
  }
}
</script>

<template>
  <button
    type="button"
    class="nuvio-card"
    :aria-label="`Open ${item.title}`"
    @click="handleOpen"
  >
    <div class="poster-frame" :class="ratioClass">
      <img
        v-if="item.poster"
        :src="item.poster"
        :alt="item.title"
        width="300"
        height="450"
        loading="lazy"
        class="poster-img"
      />
      <div v-else class="poster-fallback" aria-hidden="true">
        <Film :size="30" />
      </div>

      <div class="poster-overlay" aria-hidden="true">
        <span class="nuvio-play-pill">
          <Play :size="16" fill="currentColor" />
        </span>
      </div>

      <span v-if="cards.showRating && ratingText" class="rating-chip">
        <Star :size="11" fill="currentColor" aria-hidden="true" /> {{ ratingText }}
      </span>
    </div>

    <div class="card-caption">
      <span class="card-title">{{ item.title }}</span>
      <div class="card-meta v-muted">
        <span v-if="cards.showYear && item.year">{{ item.year }}</span>
        <span v-if="cards.showYear && item.year && cards.showContentType" aria-hidden="true">·</span>
        <span v-if="cards.showContentType">{{ item.isAnime ? 'Anime' : item.type }}</span>
        <template v-if="cards.showPluginBadge && item.pluginId">
          <span aria-hidden="true">·</span>
          <span>{{ item.pluginId }}</span>
        </template>
      </div>
    </div>
  </button>
</template>

<style scoped>
/* Nuvio clean frameless poster cards with crisp focus/hover halo */
.nuvio-card {
  display: flex;
  flex-direction: column;
  gap: 0.55rem;
  width: 100%;
  padding: 0;
  border: none;
  background: transparent;
  text-align: left;
  cursor: pointer;
}

.poster-frame {
  position: relative;
  width: 100%;
  overflow: hidden;
  border-radius: var(--vessel-radius);
  background: var(--vessel-bg-surface);
  box-shadow:
    inset 0 0 0 1px rgba(255, 255, 255, 0.08),
    0 12px 26px rgba(0, 0, 0, 0.38);
  transition:
    transform 0.26s cubic-bezier(0.22, 1, 0.36, 1),
    box-shadow 0.26s ease;
}

.nuvio-card:hover .poster-frame,
.nuvio-card:focus-visible .poster-frame {
  transform: translateY(-4px) scale(1.02);
  box-shadow:
    0 0 0 2px rgba(255, 255, 255, 0.85),
    0 20px 42px rgba(0, 0, 0, 0.62);
}

.ratio-poster {
  aspect-ratio: 2 / 3;
}

.ratio-wide {
  aspect-ratio: 16 / 9;
}

.ratio-square {
  aspect-ratio: 1 / 1;
}

.poster-img {
  width: 100%;
  height: 100%;
  object-fit: cover;
  display: block;
  transition: transform 0.38s cubic-bezier(0.22, 1, 0.36, 1);
}

.nuvio-card:hover .poster-img {
  transform: scale(1.04);
}

.poster-fallback {
  width: 100%;
  height: 100%;
  display: flex;
  align-items: center;
  justify-content: center;
  color: var(--vessel-text-muted);
}

.poster-overlay {
  position: absolute;
  inset: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  background: linear-gradient(180deg, transparent 40%, rgba(10, 11, 16, 0.72) 100%);
  opacity: 0;
  transition: opacity 0.2s ease;
}

.nuvio-card:hover .poster-overlay,
.nuvio-card:focus-visible .poster-overlay {
  opacity: 1;
}

.nuvio-play-pill {
  width: 46px;
  height: 46px;
  border-radius: 999px;
  display: flex;
  align-items: center;
  justify-content: center;
  background: rgba(255, 255, 255, 0.92);
  color: #0a0b10;
  transform: scale(0.85);
  transition: transform 0.22s cubic-bezier(0.22, 1, 0.36, 1);
  box-shadow: 0 10px 26px rgba(0, 0, 0, 0.55);
}

.nuvio-card:hover .nuvio-play-pill,
.nuvio-card:focus-visible .nuvio-play-pill {
  transform: scale(1);
}

.rating-chip {
  position: absolute;
  top: 0.5rem;
  right: 0.5rem;
  display: inline-flex;
  align-items: center;
  gap: 0.25rem;
  padding: 0.18rem 0.5rem;
  border-radius: 999px;
  font-size: 0.74rem;
  font-weight: 700;
  color: var(--vessel-warning);
  background: rgba(10, 11, 16, 0.72);
  backdrop-filter: blur(8px);
  -webkit-backdrop-filter: blur(8px);
  font-variant-numeric: tabular-nums;
}

.card-caption {
  display: flex;
  flex-direction: column;
  gap: 0.15rem;
  padding: 0 0.1rem;
  min-width: 0;
}

.card-title {
  font-size: 0.94rem;
  font-weight: 600;
  line-height: 1.28;
  display: -webkit-box;
  -webkit-line-clamp: 1;
  -webkit-box-orient: vertical;
  overflow: hidden;
}

.card-meta {
  display: flex;
  gap: 0.3rem;
  font-size: 0.78rem;
  text-transform: capitalize;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  font-variant-numeric: tabular-nums;
}
</style>
