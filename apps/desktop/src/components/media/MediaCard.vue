<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, shallowRef, watch } from 'vue'
import {
  Bookmark,
  BookmarkCheck,
  CheckCircle2,
  Clock,
  Film,
  PauseCircle,
  Play,
  Star,
  Trash2,
  XCircle,
} from 'lucide-vue-next'
import { useI18n } from '@/composables/useI18n'
import { useLibraryState } from '@/composables/useLibraryState'
import { pushToast } from '@/composables/useVesselEvents'
import type { CardPreferences, LibraryStatus, MediaSummary } from '@/types/vessel'

const props = defineProps<{
  item: MediaSummary
  cards: CardPreferences
}>()

const emit = defineEmits<{
  open: [pluginId: string, nativeId: string]
}>()

const { t } = useI18n()
const { getMediaStatus, setMediaStatus, removeMediaStatus } = useLibraryState()

const imgRef = ref<HTMLImageElement | null>(null)
const cardRootRef = ref<HTMLElement | null>(null)
const imgLoaded = shallowRef(false)
const menuOpen = shallowRef(false)

watch(
  () => props.item.poster,
  () => {
    imgLoaded.value = false
  },
)

function handleDocPointerDown(ev: PointerEvent): void {
  if (!menuOpen.value) return
  const target = ev.target as Node | null
  if (cardRootRef.value && target && !cardRootRef.value.contains(target)) {
    menuOpen.value = false
  }
}

onMounted(() => {
  if (imgRef.value?.complete && imgRef.value.naturalWidth > 0) {
    imgLoaded.value = true
  }
  window.addEventListener('pointerdown', handleDocPointerDown)
})

onUnmounted(() => {
  window.removeEventListener('pointerdown', handleDocPointerDown)
})

const ratioClass = computed(() => {
  if (props.cards.aspectRatio === 'wide_16x9') return 'ratio-wide'
  if (props.cards.aspectRatio === 'square') return 'ratio-square'
  return 'ratio-poster'
})

const ratingText = computed(() =>
  props.item.rating !== undefined ? props.item.rating.toFixed(1) : null,
)

const currentStatus = computed<LibraryStatus | null>(() =>
  getMediaStatus(props.item.pluginId, props.item.nativeId),
)

const STATUS_OPTIONS: Array<{ value: LibraryStatus; key: string }> = [
  { value: 'watching', key: 'library.watching' },
  { value: 'plan_to_watch', key: 'library.plan_to_watch' },
  { value: 'completed', key: 'library.completed' },
  { value: 'on_hold', key: 'library.on_hold' },
  { value: 'dropped', key: 'library.dropped' },
]

function handleOpen(): void {
  menuOpen.value = false
  if (props.item.pluginId) {
    emit('open', props.item.pluginId, props.item.nativeId)
  }
}

function toggleStatusMenu(): void {
  if (!props.item.pluginId) return
  menuOpen.value = !menuOpen.value
}

async function pickStatus(status: LibraryStatus): Promise<void> {
  if (!props.item.pluginId) return
  menuOpen.value = false
  try {
    await setMediaStatus({
      pluginId: props.item.pluginId,
      nativeId: props.item.nativeId,
      mediaType: props.item.type,
      isAnime: props.item.isAnime,
      title: props.item.title,
      posterUrl: props.item.poster,
      year: props.item.year,
      status,
    })
    pushToast('info', `${props.item.title} · ${t(`library.${status}`)}`)
  } catch (err) {
    pushToast('error', String(err))
  }
}

async function clearStatus(): Promise<void> {
  if (!props.item.pluginId) return
  menuOpen.value = false
  try {
    await removeMediaStatus(props.item.pluginId, props.item.nativeId)
  } catch (err) {
    pushToast('error', String(err))
  }
}
</script>

<template>
  <div
    ref="cardRootRef"
    role="button"
    tabindex="0"
    class="nuvio-card"
    :class="{ 'has-open-menu': menuOpen }"
    :aria-label="`Open ${item.title}`"
    @click="handleOpen"
    @keydown.enter.self="handleOpen"
    @keydown.space.prevent.self="handleOpen"
  >
    <div class="card-stage">
      <div class="poster-frame" :class="ratioClass">
        <div v-if="item.poster && !imgLoaded" class="v-skeleton poster-shimmer" aria-hidden="true" />
        <img
          v-if="item.poster"
          ref="imgRef"
          :src="item.poster"
          :alt="item.title"
          width="300"
          height="450"
          loading="eager"
          decoding="async"
          class="poster-img"
          :class="{ 'is-loaded': imgLoaded }"
          @load="imgLoaded = true"
          @error="imgLoaded = true"
        />
        <div v-else class="poster-fallback" aria-hidden="true">
          <Film :size="30" />
        </div>

        <span class="poster-sheen" aria-hidden="true" />

        <div class="poster-overlay" aria-hidden="true">
          <span class="nuvio-play-pill">
            <Play :size="16" fill="currentColor" />
          </span>
        </div>

        <span v-if="cards.showRating && ratingText" class="rating-chip">
          <Star :size="11" fill="currentColor" aria-hidden="true" /> {{ ratingText }}
        </span>
      </div>

      <div v-if="item.pluginId" class="quick-status-wrap" @click.stop>
        <button
          type="button"
          class="quick-status-btn"
          :class="{ 'has-status': Boolean(currentStatus), [`status-${currentStatus}`]: Boolean(currentStatus) }"
          :title="currentStatus ? t(`library.${currentStatus}`) : t('media.quick_status')"
          :aria-label="currentStatus ? t(`library.${currentStatus}`) : t('media.quick_status')"
          @click.stop="toggleStatusMenu"
        >
          <CheckCircle2 v-if="currentStatus === 'completed'" :size="14" aria-hidden="true" />
          <BookmarkCheck v-else-if="currentStatus === 'watching'" :size="14" aria-hidden="true" />
          <Clock v-else-if="currentStatus === 'plan_to_watch'" :size="14" aria-hidden="true" />
          <PauseCircle v-else-if="currentStatus === 'on_hold'" :size="14" aria-hidden="true" />
          <XCircle v-else-if="currentStatus === 'dropped'" :size="14" aria-hidden="true" />
          <Bookmark v-else :size="14" aria-hidden="true" />
        </button>

        <Transition name="modal-pop">
          <div v-if="menuOpen" class="quick-status-menu" role="menu">
            <button
              v-for="opt in STATUS_OPTIONS"
              :key="opt.value"
              type="button"
              role="menuitem"
              class="qs-item"
              :class="{ 'is-active': currentStatus === opt.value }"
              @click.stop="pickStatus(opt.value)"
            >
              <span>{{ t(opt.key) }}</span>
            </button>
            <button
              v-if="currentStatus"
              type="button"
              role="menuitem"
              class="qs-item qs-danger"
              @click.stop="clearStatus"
            >
              <Trash2 :size="12" aria-hidden="true" />
              <span>{{ t('common.remove') }}</span>
            </button>
          </div>
        </Transition>
      </div>
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
  </div>
</template>

<style scoped>
/* Nuvio clean frameless poster cards with crisp focus/hover halo */
.nuvio-card {
  position: relative;
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

.nuvio-card.has-open-menu {
  z-index: 40;
}

.card-stage {
  position: relative;
  width: 100%;
}

.quick-status-wrap {
  position: absolute;
  top: 0.45rem;
  right: 0.45rem;
  z-index: 6;
}

.quick-status-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 30px;
  height: 30px;
  border-radius: 999px;
  border: 1px solid rgba(255, 255, 255, 0.22);
  background: rgba(10, 11, 16, 0.78);
  backdrop-filter: blur(8px);
  color: #f8fafc;
  cursor: pointer;
  opacity: 0.84;
  transition:
    opacity 0.18s ease,
    transform 0.18s ease,
    background 0.18s ease,
    border-color 0.18s ease;
}

.nuvio-card:hover .quick-status-btn,
.quick-status-btn.has-status,
.quick-status-btn:focus-visible {
  opacity: 1;
}

.quick-status-btn:hover {
  transform: scale(1.08);
  border-color: var(--vessel-accent);
}

.quick-status-btn.has-status {
  background: color-mix(in srgb, var(--vessel-accent) 88%, #090b10);
  border-color: rgba(255, 255, 255, 0.35);
}

.quick-status-btn.status-completed {
  background: color-mix(in srgb, var(--vessel-success) 88%, #090b10);
}

.quick-status-btn.status-dropped {
  background: color-mix(in srgb, var(--vessel-danger) 86%, #090b10);
}

.quick-status-menu {
  position: absolute;
  top: calc(100% + 0.35rem);
  right: 0;
  min-width: 148px;
  padding: 0.32rem;
  border-radius: 12px;
  background: var(--vessel-surface-elevated);
  border: 1px solid var(--vessel-border);
  box-shadow: 0 14px 32px rgba(0, 0, 0, 0.45);
  display: flex;
  flex-direction: column;
  gap: 0.15rem;
  z-index: 60;
}

.qs-item {
  width: 100%;
  display: flex;
  align-items: center;
  gap: 0.4rem;
  padding: 0.38rem 0.55rem;
  border: none;
  border-radius: 8px;
  background: transparent;
  color: var(--vessel-text-primary);
  font-size: 0.78rem;
  font-weight: 550;
  text-align: left;
  cursor: pointer;
}

.qs-item:hover {
  background: color-mix(in srgb, var(--vessel-accent) 16%, transparent);
}

.qs-item.is-active {
  background: color-mix(in srgb, var(--vessel-accent) 26%, transparent);
  color: var(--vessel-accent);
  font-weight: 700;
}

.qs-item.qs-danger {
  color: var(--vessel-danger);
  border-top: 1px solid var(--vessel-border);
  margin-top: 0.1rem;
  padding-top: 0.42rem;
  border-radius: 0 0 8px 8px;
}

.poster-frame {
  position: relative;
  width: 100%;
  overflow: hidden;
  border-radius: var(--vessel-radius);
  background: var(--vessel-surface-card);
  border: 1px solid var(--vessel-border);
  transition:
    transform 0.28s cubic-bezier(0.22, 1, 0.36, 1),
    border-color 0.24s ease;
}

.nuvio-card:hover .poster-frame,
.nuvio-card:focus-visible .poster-frame {
  transform: translate3d(0, -6px, 0) scale(1.025);
  border-color: var(--vessel-accent);
}

.poster-sheen {
  position: absolute;
  inset: -20% -60%;
  background: linear-gradient(
    115deg,
    transparent 35%,
    rgba(255, 255, 255, 0.16) 50%,
    transparent 65%
  );
  transform: translateX(-110%);
  pointer-events: none;
  transition: transform 0.55s cubic-bezier(0.22, 1, 0.36, 1);
}

.nuvio-card:hover .poster-sheen {
  transform: translateX(110%);
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

.poster-shimmer {
  position: absolute;
  inset: 0;
  z-index: 1;
  border-radius: 0;
  border: none;
}

.poster-img {
  width: 100%;
  height: 100%;
  object-fit: cover;
  display: block;
  opacity: 0;
  transition:
    opacity 0.28s ease,
    transform 0.38s cubic-bezier(0.22, 1, 0.36, 1);
}

.poster-img.is-loaded {
  opacity: 1;
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
  width: 48px;
  height: 48px;
  border-radius: 999px;
  display: flex;
  align-items: center;
  justify-content: center;
  background: linear-gradient(135deg, #ffffff, var(--vessel-accent));
  color: #0a0b10;
  transform: scale(0.75);
  transition: transform 0.28s cubic-bezier(0.34, 1.56, 0.64, 1);
}

.nuvio-card:hover .nuvio-play-pill,
.nuvio-card:focus-visible .nuvio-play-pill {
  transform: scale(1);
}

.rating-chip {
  position: absolute;
  top: 0.45rem;
  left: 0.45rem;
  display: inline-flex;
  align-items: center;
  gap: 0.25rem;
  padding: 0.18rem 0.52rem;
  border-radius: 999px;
  font-size: 0.74rem;
  font-weight: 700;
  color: var(--vessel-warning);
  background: rgba(10, 11, 16, 0.86);
  border: 1px solid rgba(255, 255, 255, 0.12);
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
