<script setup lang="ts">
import { computed, onMounted, ref, shallowRef } from 'vue'
import { useRouter } from 'vue-router'
import { ChevronLeft, ChevronRight, Info, Play, RefreshCw, Star, X } from 'lucide-vue-next'
import { isTauriRuntime, vesselApi } from '@/api/vessel'
import MediaCard from '@/components/media/MediaCard.vue'
import { useI18n } from '@/composables/useI18n'
import { usePlayerSession } from '@/composables/usePlayerSession'
import { useUiPreferences } from '@/composables/useUiPreferences'
import { pushToast } from '@/composables/useVesselEvents'
import type { ContinueWatchingEntry, HomeShelfResult, MediaSummary } from '@/types/vessel'

const router = useRouter()
const { t } = useI18n()
const { setSession } = usePlayerSession()
const { uiPreferences } = useUiPreferences()

const shelves = ref<HomeShelfResult[]>([])
const continueWatching = ref<ContinueWatchingEntry[]>([])
const loading = shallowRef(false)
const resumingKey = shallowRef<string | null>(null)
const heroIndex = shallowRef(0)

const spotlightPool = computed<Array<MediaSummary & { resolvedPluginId: string }>>(() => {
  const collected: Array<MediaSummary & { resolvedPluginId: string }> = []
  for (const shelf of shelves.value) {
    for (const section of shelf.sections) {
      for (const item of section.items) {
        collected.push({
          ...item,
          resolvedPluginId: item.pluginId ?? shelf.pluginId,
        })
        if (collected.length >= 6) return collected
      }
    }
  }
  return collected
})

const featuredItem = computed(() => {
  if (!spotlightPool.value.length) return null
  const idx = heroIndex.value % spotlightPool.value.length
  return spotlightPool.value[idx]
})

async function loadHome(): Promise<void> {
  if (!isTauriRuntime()) return
  loading.value = true
  try {
    const [feeds, cw] = await Promise.all([
      vesselApi.getHomeFeeds(),
      vesselApi.listContinueWatching(16).catch(() => [] as ContinueWatchingEntry[]),
    ])
    shelves.value = feeds
    continueWatching.value = cw
    heroIndex.value = 0
  } catch (err) {
    pushToast('error', String(err))
  } finally {
    loading.value = false
  }
}

function progressPercent(entry: ContinueWatchingEntry): number {
  if (!entry.durationMs || entry.durationMs <= 0) return 15
  return Math.min(100, Math.max(4, Math.round((entry.positionMs / entry.durationMs) * 100)))
}

function formatRemaining(entry: ContinueWatchingEntry): string {
  if (!entry.durationMs || entry.durationMs <= entry.positionMs) return 'Resume'
  const remSec = Math.max(0, Math.floor((entry.durationMs - entry.positionMs) / 1000))
  const mins = Math.floor(remSec / 60)
  if (mins >= 60) {
    const hrs = Math.floor(mins / 60)
    return `${hrs}h ${mins % 60}m left`
  }
  if (mins >= 1) return `${mins}m left`
  return `${remSec}s left`
}

async function resumeContinueWatching(entry: ContinueWatchingEntry): Promise<void> {
  const key = `${entry.media.pluginId}:${entry.media.nativeId}`
  if (resumingKey.value) return
  resumingKey.value = key
  try {
    const [detail, streamRes] = await Promise.all([
      vesselApi.getDetailsWithEnrichment(entry.media.pluginId, entry.media.nativeId, 'en'),
      vesselApi.getStreams(entry.media.pluginId, entry.streamId),
    ])
    const ep =
      detail.episodes.find((item) => item.streamId === entry.streamId) ??
      detail.episodes.find((item) => item.season === entry.season && item.episode === entry.episode) ??
      detail.episodes[0]

    if (!ep || !streamRes.streams.length) {
      openDetails(entry.media.pluginId, entry.media.nativeId)
      return
    }

    setSession({
      pluginId: entry.media.pluginId,
      nativeId: entry.media.nativeId,
      detail,
      episode: ep,
      streams: streamRes.streams,
      subtitles: streamRes.subtitles,
      activeStreamIndex: 0,
      startPositionMs: entry.positionMs,
    })

    await router.push({
      name: 'player',
      params: {
        pluginId: entry.media.pluginId,
        nativeId: entry.media.nativeId,
      },
      query: {
        streamId: ep.streamId,
        streamIndex: '0',
      },
    })
  } catch (err) {
    pushToast('warn', String(err))
    openDetails(entry.media.pluginId, entry.media.nativeId)
  } finally {
    resumingKey.value = null
  }
}

async function dismissContinueWatching(entry: ContinueWatchingEntry): Promise<void> {
  try {
    await vesselApi.deleteWatchProgress(entry.media.pluginId, entry.media.nativeId)
    continueWatching.value = continueWatching.value.filter(
      (item) =>
        !(
          item.media.pluginId === entry.media.pluginId &&
          item.media.nativeId === entry.media.nativeId
        ),
    )
  } catch (err) {
    pushToast('error', String(err))
  }
}

function openDetails(pluginId: string, nativeId: string): void {
  void router.push({ name: 'details', params: { pluginId, nativeId } })
}

function scrollRail(railId: string, dir: -1 | 1): void {
  const el = document.getElementById(railId)
  if (el) {
    el.scrollBy({ left: dir * 580, behavior: 'smooth' })
  }
}

onMounted(() => {
  void loadHome()
})
</script>

<template>
  <section class="nuvio-home">
    <!-- Full-bleed Nuvio Hero Showcase -->
    <header
      v-if="uiPreferences.home.heroBanner !== 'hidden'"
      class="nuvio-hero"
      :class="{ 'is-compact': uiPreferences.home.heroBanner === 'compact' }"
    >
      <template v-if="featuredItem">
        <img
          v-if="featuredItem.poster"
          :src="featuredItem.poster"
          :alt="featuredItem.title"
          width="1440"
          height="680"
          fetchpriority="high"
          class="hero-art"
        />
        <div class="hero-vignette" aria-hidden="true" />

        <div class="hero-content">
          <div class="hero-meta-line">
            <span class="type-pill">{{ featuredItem.isAnime ? 'ANIME' : featuredItem.type.toUpperCase() }}</span>
            <span v-if="featuredItem.year">{{ featuredItem.year }}</span>
            <span v-if="featuredItem.rating !== undefined" class="hero-rating">
              <Star :size="13" fill="currentColor" aria-hidden="true" />
              {{ featuredItem.rating.toFixed(1) }}
            </span>
            <span class="v-muted">{{ featuredItem.resolvedPluginId }}</span>
          </div>

          <h1 class="hero-title">{{ featuredItem.title }}</h1>
          <p class="hero-sub">{{ t('home.hero_subtitle') }}</p>

          <div class="hero-actions">
            <button
              type="button"
              class="v-btn v-btn-primary"
              @click="openDetails(featuredItem.resolvedPluginId, featuredItem.nativeId)"
            >
              <Play :size="16" fill="currentColor" aria-hidden="true" /> Watch Now
            </button>
            <button
              type="button"
              class="v-btn"
              @click="openDetails(featuredItem.resolvedPluginId, featuredItem.nativeId)"
            >
              <Info :size="16" aria-hidden="true" /> More Info
            </button>
            <button
              type="button"
              class="v-btn v-btn-icon"
              :aria-label="t('common.refresh')"
              :disabled="loading"
              @click="loadHome"
            >
              <RefreshCw :size="15" :class="{ spin: loading }" aria-hidden="true" />
            </button>
          </div>
        </div>

        <div v-if="spotlightPool.length > 1" class="hero-indicators">
          <button
            v-for="(item, idx) in spotlightPool"
            :key="`${item.resolvedPluginId}:${item.nativeId}`"
            type="button"
            class="indicator-bar"
            :class="{ 'is-active': idx === heroIndex }"
            :aria-label="`Spotlight ${item.title}`"
            @click="heroIndex = idx"
          />
        </div>
      </template>

      <template v-else>
        <div class="hero-vignette" aria-hidden="true" />
        <div class="hero-content">
          <h1 class="hero-title">{{ t('home.hero_title') }}</h1>
          <p class="hero-sub">{{ t('home.hero_subtitle') }}</p>
          <div class="hero-actions">
            <RouterLink to="/extensions" class="v-btn v-btn-primary">
              {{ t('nav.extensions') }}
            </RouterLink>
            <button type="button" class="v-btn" :disabled="loading" @click="loadHome">
              <RefreshCw :size="15" :class="{ spin: loading }" aria-hidden="true" />
              {{ t('common.refresh') }}
            </button>
          </div>
        </div>
      </template>
    </header>

    <!-- Main horizontal rails area -->
    <div class="home-rails-body">
      <!-- Continue Watching Rail -->
      <section
        v-if="uiPreferences.home.showContinueWatching && continueWatching.length"
        class="nuvio-rail continue-rail"
      >
        <div class="rail-head">
          <div class="rail-title-wrap">
            <h2 class="rail-title">Continue Watching</h2>
            <span class="rail-source v-muted">{{ continueWatching.length }} active</span>
          </div>
          <div class="rail-arrows">
            <button
              type="button"
              class="v-btn v-btn-icon"
              aria-label="Scroll continue watching left"
              @click="scrollRail('rail-continue-watching', -1)"
            >
              <ChevronLeft :size="16" aria-hidden="true" />
            </button>
            <button
              type="button"
              class="v-btn v-btn-icon"
              aria-label="Scroll continue watching right"
              @click="scrollRail('rail-continue-watching', 1)"
            >
              <ChevronRight :size="16" aria-hidden="true" />
            </button>
          </div>
        </div>

        <div id="rail-continue-watching" class="rail-track">
          <article
            v-for="entry in continueWatching"
            :key="`${entry.media.pluginId}:${entry.media.nativeId}`"
            class="cw-card"
            :class="{ 'is-loading': resumingKey === `${entry.media.pluginId}:${entry.media.nativeId}` }"
            @click="resumeContinueWatching(entry)"
          >
            <div class="cw-thumb">
              <img
                v-if="entry.backdrop ?? entry.poster"
                :src="entry.backdrop ?? entry.poster"
                :alt="entry.title"
                class="cw-img"
              />
              <div class="cw-shade" />
              <span class="cw-play-badge">
                <Play :size="18" fill="currentColor" aria-hidden="true" />
              </span>
              <button
                type="button"
                class="cw-dismiss"
                title="Remove from Continue Watching"
                aria-label="Remove from Continue Watching"
                @click.stop="dismissContinueWatching(entry)"
              >
                <X :size="14" aria-hidden="true" />
              </button>
              <span class="cw-remaining">{{ formatRemaining(entry) }}</span>
              <div class="cw-progress-track">
                <div class="cw-progress-fill" :style="{ width: `${progressPercent(entry)}%` }" />
              </div>
            </div>
            <div class="cw-meta">
              <div class="cw-title">{{ entry.title }}</div>
              <div class="cw-sub">
                <span>S{{ entry.season }} · E{{ entry.episode }}</span>
                <template v-if="entry.episodeTitle"> · {{ entry.episodeTitle }}</template>
              </div>
            </div>
          </article>
        </div>
      </section>

      <div v-if="loading" class="rail-track">
        <div v-for="n in 7" :key="n" class="v-skeleton rail-skeleton" />
      </div>

      <div v-else-if="!shelves.length" class="v-card empty-box">
        <p class="v-muted empty-text">{{ t('home.empty_plugins') }}</p>
        <RouterLink to="/extensions" class="v-btn v-btn-primary">
          {{ t('nav.extensions') }}
        </RouterLink>
      </div>

      <div v-else class="shelf-stack">
        <div v-for="shelf in shelves" :key="shelf.pluginId" class="plugin-block">
          <div v-if="shelf.error" class="v-card error-shelf">
            <strong>{{ shelf.pluginName }}</strong> — {{ t('home.plugin_error') }}:
            <span class="v-muted">{{ shelf.error }}</span>
          </div>

          <template v-for="section in shelf.sections" :key="`${shelf.pluginId}:${section.id}`">
            <section
              v-if="!uiPreferences.home.hiddenSections.includes(section.id)"
              class="nuvio-rail"
            >
              <div class="rail-head">
                <div class="rail-title-wrap">
                  <h2 class="rail-title">{{ section.title }}</h2>
                  <span class="rail-source v-muted">{{ shelf.pluginName }}</span>
                </div>
                <div class="rail-arrows">
                  <button
                    type="button"
                    class="v-btn v-btn-icon"
                    aria-label="Scroll shelf left"
                    @click="scrollRail(`rail-${shelf.pluginId}-${section.id}`, -1)"
                  >
                    <ChevronLeft :size="16" aria-hidden="true" />
                  </button>
                  <button
                    type="button"
                    class="v-btn v-btn-icon"
                    aria-label="Scroll shelf right"
                    @click="scrollRail(`rail-${shelf.pluginId}-${section.id}`, 1)"
                  >
                    <ChevronRight :size="16" aria-hidden="true" />
                  </button>
                </div>
              </div>

              <div :id="`rail-${shelf.pluginId}-${section.id}`" class="rail-track">
                <div
                  v-for="item in section.items"
                  :key="`${shelf.pluginId}:${item.nativeId}`"
                  class="rail-card-slot"
                >
                  <MediaCard
                    :item="{ ...item, pluginId: item.pluginId ?? shelf.pluginId }"
                    :cards="uiPreferences.cards"
                    @open="openDetails"
                  />
                </div>
              </div>
            </section>
          </template>
        </div>
      </div>
    </div>
  </section>
</template>

<style scoped>
.nuvio-home {
  display: flex;
  flex-direction: column;
  gap: 1.5rem;
}

.nuvio-hero {
  position: relative;
  min-height: 390px;
  display: flex;
  align-items: flex-end;
  justify-content: space-between;
  padding: 2.5rem 2.75rem;
  overflow: hidden;
}

.nuvio-hero.is-compact {
  min-height: 220px;
  padding: 1.75rem 2.5rem;
}

.hero-art {
  position: absolute;
  inset: 0;
  width: 100%;
  height: 100%;
  object-fit: cover;
  object-position: center 20%;
  filter: blur(14px) saturate(1.25);
  transform: scale(1.08);
  opacity: 0.62;
  pointer-events: none;
}

.hero-vignette {
  position: absolute;
  inset: 0;
  background:
    linear-gradient(
      90deg,
      var(--vessel-bg-base) 0%,
      rgba(10, 11, 16, 0.78) 46%,
      rgba(10, 11, 16, 0.28) 100%
    ),
    linear-gradient(
      180deg,
      rgba(10, 11, 16, 0.25) 0%,
      rgba(10, 11, 16, 0.68) 62%,
      var(--vessel-bg-base) 100%
    );
  pointer-events: none;
}

.hero-content {
  position: relative;
  z-index: 2;
  display: flex;
  flex-direction: column;
  gap: 0.8rem;
  max-width: 620px;
}

.hero-meta-line {
  display: flex;
  align-items: center;
  gap: 0.75rem;
  font-size: 0.85rem;
  font-weight: 600;
  letter-spacing: 0.02em;
  font-variant-numeric: tabular-nums;
}

.type-pill {
  padding: 0.14rem 0.55rem;
  border-radius: 6px;
  background: rgba(255, 255, 255, 0.14);
  font-size: 0.72rem;
  letter-spacing: 0.08em;
}

.hero-rating {
  display: inline-flex;
  align-items: center;
  gap: 0.28rem;
  color: var(--vessel-warning);
}

.hero-title {
  margin: 0;
  font-size: 2.75rem;
  font-weight: 850;
  letter-spacing: -0.035em;
  line-height: 1.06;
  text-wrap: balance;
}

.hero-sub {
  margin: 0;
  color: rgba(244, 246, 252, 0.78);
  font-size: 0.98rem;
  max-width: 52ch;
}

.hero-actions {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 0.75rem;
  margin-top: 0.35rem;
}

.hero-indicators {
  position: relative;
  z-index: 2;
  display: flex;
  align-items: center;
  gap: 0.45rem;
}

.indicator-bar {
  width: 18px;
  height: 4px;
  padding: 0;
  border: none;
  border-radius: 999px;
  background: rgba(255, 255, 255, 0.25);
  cursor: pointer;
  transition: width 0.22s ease, background-color 0.22s ease;
}

.indicator-bar.is-active {
  width: 34px;
  background: #ffffff;
}

.home-rails-body {
  padding: 0 2.75rem 2.75rem;
  display: flex;
  flex-direction: column;
  gap: 2rem;
}

.shelf-stack,
.plugin-block,
.nuvio-rail {
  display: flex;
  flex-direction: column;
  gap: 0.85rem;
}

.rail-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.75rem;
}

.rail-title-wrap {
  display: flex;
  align-items: baseline;
  gap: 0.75rem;
}

.rail-title {
  margin: 0;
  font-size: 1.25rem;
  font-weight: 700;
  letter-spacing: -0.02em;
}

.rail-source {
  font-size: 0.8rem;
  text-transform: uppercase;
  letter-spacing: 0.06em;
}

.rail-arrows {
  display: flex;
  gap: 0.4rem;
}

.rail-track {
  display: flex;
  gap: var(--vessel-gap);
  overflow-x: auto;
  padding: 0.45rem 0.2rem 0.85rem;
  scroll-snap-type: x proximity;
}

.rail-card-slot,
.rail-skeleton {
  flex: 0 0 186px;
  scroll-snap-align: start;
}

.rail-skeleton {
  aspect-ratio: 2 / 3;
}

/* Continue Watching Landscape Cards */
.cw-card {
  flex: 0 0 280px;
  scroll-snap-align: start;
  display: flex;
  flex-direction: column;
  gap: 0.55rem;
  cursor: pointer;
  transition: transform 0.2s ease, opacity 0.2s ease;
}

.cw-card:hover {
  transform: translateY(-3px);
}

.cw-card.is-loading {
  opacity: 0.65;
  pointer-events: none;
}

.cw-thumb {
  position: relative;
  aspect-ratio: 16 / 9;
  border-radius: var(--vessel-radius);
  overflow: hidden;
  background: linear-gradient(135deg, rgba(167, 139, 250, 0.2), rgba(20, 22, 34, 0.95));
  border: 1px solid rgba(255, 255, 255, 0.08);
}

.cw-img {
  width: 100%;
  height: 100%;
  object-fit: cover;
  display: block;
}

.cw-shade {
  position: absolute;
  inset: 0;
  background: linear-gradient(180deg, rgba(0, 0, 0, 0.1) 35%, rgba(0, 0, 0, 0.85) 100%);
}

.cw-play-badge {
  position: absolute;
  top: 50%;
  left: 50%;
  transform: translate(-50%, -50%);
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 44px;
  height: 44px;
  border-radius: 999px;
  background: rgba(12, 13, 22, 0.78);
  border: 1px solid rgba(255, 255, 255, 0.24);
  color: #fff;
  transition: transform 0.18s ease, background 0.18s ease;
}

.cw-card:hover .cw-play-badge {
  transform: translate(-50%, -50%) scale(1.08);
  background: var(--vessel-accent, #a78bfa);
  color: #0b0b12;
  border-color: transparent;
}

.cw-dismiss {
  position: absolute;
  top: 0.5rem;
  right: 0.5rem;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 26px;
  height: 26px;
  border-radius: 999px;
  border: 1px solid rgba(255, 255, 255, 0.2);
  background: rgba(10, 10, 18, 0.78);
  color: #fff;
  opacity: 0;
  cursor: pointer;
  transition: opacity 0.16s ease, background 0.16s ease;
}

.cw-card:hover .cw-dismiss {
  opacity: 1;
}

.cw-dismiss:hover {
  background: rgba(239, 68, 68, 0.88);
}

.cw-remaining {
  position: absolute;
  left: 0.65rem;
  bottom: 0.55rem;
  font-size: 0.74rem;
  font-weight: 600;
  color: rgba(255, 255, 255, 0.92);
}

.cw-progress-track {
  position: absolute;
  inset: auto 0 0 0;
  height: 4px;
  background: rgba(255, 255, 255, 0.22);
}

.cw-progress-fill {
  height: 100%;
  background: var(--vessel-accent, #a78bfa);
}

.cw-meta {
  display: flex;
  flex-direction: column;
  gap: 0.15rem;
  padding: 0 0.15rem;
}

.cw-title {
  font-size: 0.92rem;
  font-weight: 700;
  color: var(--vessel-text-primary);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.cw-sub {
  font-size: 0.78rem;
  color: var(--vessel-text-secondary);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.empty-box {
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  gap: 0.85rem;
}

.empty-text {
  margin: 0;
}

.error-shelf {
  border-color: var(--vessel-danger);
}

.spin {
  animation: v-spin 0.9s linear infinite;
}

@keyframes v-spin {
  to {
    transform: rotate(360deg);
  }
}
</style>
