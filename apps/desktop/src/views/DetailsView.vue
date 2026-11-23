<script setup lang="ts">
import { computed, onMounted, ref, shallowRef, watch } from 'vue'
import { useRouter } from 'vue-router'
import {
  AlertTriangle,
  AlignJustify,
  ArrowLeft,
  BookmarkPlus,
  Download,
  LayoutGrid,
  List,
  Loader2,
  Play,
  RefreshCw,
  Server,
  Star,
  Subtitles,
  Wrench,
  X,
} from 'lucide-vue-next'
import { vesselApi } from '@/api/vessel'
import { useI18n } from '@/composables/useI18n'
import { usePlayerSession } from '@/composables/usePlayerSession'
import { useUiPreferences } from '@/composables/useUiPreferences'
import { pushToast, useVesselEvents } from '@/composables/useVesselEvents'
import type {
  Episode,
  EpisodeListLayout,
  LibraryStatus,
  MediaDetail,
  StreamCandidate,
  SubtitleTrack,
} from '@/types/vessel'

const props = defineProps<{
  pluginId: string
  nativeId: string
}>()

const router = useRouter()
const { t, currentLang } = useI18n()
const { setSession } = usePlayerSession()
const { uiPreferences, savePreferences } = useUiPreferences()
const { lastEnrichedDetail } = useVesselEvents()

const detail = ref<MediaDetail | null>(null)
const selectedSeason = shallowRef<number>(1)
const loading = shallowRef(false)

// Fix Match Modal state
const showFixMatchModal = shallowRef(false)
const fixProvider = shallowRef<'anilist' | 'tmdb_tv' | 'tmdb_movie' | 'imdb'>('tmdb_tv')
const fixIdInput = shallowRef('')
const fixDisableEnrichment = shallowRef(false)
const savingFixMatch = shallowRef(false)

// CloudStream-style source resolver modal state
const resolverEpisode = ref<Episode | null>(null)
const resolvingStreams = shallowRef(false)
const resolverError = shallowRef<string>('')
const streamChoices = ref<StreamCandidate[]>([])
const episodeSubtitles = ref<SubtitleTrack[]>([])

function cleanSynopsis(raw?: string): string {
  if (!raw) return ''
  return raw
    .replace(/<br\s*\/?>/gi, '\n')
    .replace(/<\/?[^>]+(>|$)/g, '')
    .replace(/\n{3,}/g, '\n\n')
    .trim()
}

const formattedSynopsis = computed(() => cleanSynopsis(detail.value?.synopsis))

const seasonNumbers = computed(() => {
  if (!detail.value) return []
  const set = new Set<number>(detail.value.episodes.map((e) => e.season))
  return Array.from(set).sort((a, b) => a - b)
})

const filteredEpisodes = computed(() => {
  if (!detail.value) return []
  if (seasonNumbers.value.length <= 1) return detail.value.episodes
  return detail.value.episodes.filter((ep) => ep.season === selectedSeason.value)
})

const firstEpisode = computed(() => filteredEpisodes.value[0] ?? detail.value?.episodes[0] ?? null)
const episodeLayout = computed(() => uiPreferences.value.details.episodeLayout)

watch(lastEnrichedDetail, (enriched) => {
  if (
    enriched &&
    enriched.pluginId === props.pluginId &&
    enriched.nativeId === props.nativeId
  ) {
    detail.value = enriched.detail
  }
})

async function setEpisodeLayout(layout: EpisodeListLayout): Promise<void> {
  const next = {
    ...uiPreferences.value,
    details: { ...uiPreferences.value.details, episodeLayout: layout },
  }
  try {
    await savePreferences(next)
  } catch (err) {
    pushToast('error', String(err))
  }
}

async function loadDetails(): Promise<void> {
  loading.value = true
  try {
    const res = await vesselApi.getDetailsWithEnrichment(
      props.pluginId,
      props.nativeId,
      currentLang.value,
    )
    detail.value = res
    if (res.episodes.length) {
      selectedSeason.value = res.episodes[0].season
    }
  } catch (err) {
    pushToast('error', String(err))
  } finally {
    loading.value = false
  }
}

async function saveLibrary(status: LibraryStatus): Promise<void> {
  if (!detail.value) return
  const now = new Date().toISOString()
  try {
    await vesselApi.upsertLibrary({
      media: { pluginId: props.pluginId, nativeId: props.nativeId },
      mediaType: detail.value.type,
      isAnime: detail.value.isAnime,
      title: detail.value.title,
      posterUrl: detail.value.poster,
      year: detail.value.year,
      status,
      addedAt: now,
      updatedAt: now,
    })
    pushToast('info', `Saved to ${status}`)
  } catch (err) {
    pushToast('error', String(err))
  }
}

function openFixMatchModal(): void {
  const ids = detail.value?.externalIds
  if (ids?.tmdb) {
    fixProvider.value = detail.value?.type === 'movie' ? 'tmdb_movie' : 'tmdb_tv'
    fixIdInput.value = String(ids.tmdb)
  } else if (ids?.anilist) {
    fixProvider.value = 'anilist'
    fixIdInput.value = String(ids.anilist)
  } else if (ids?.imdb) {
    fixProvider.value = 'imdb'
    fixIdInput.value = ids.imdb
  } else {
    fixIdInput.value = ''
  }
  fixDisableEnrichment.value = false
  showFixMatchModal.value = true
}

function buildCanonicalId(): string | null {
  const raw = fixIdInput.value.trim()
  if (!raw) return null
  if (raw.includes(':')) return raw
  switch (fixProvider.value) {
    case 'anilist':
      return `anilist:${raw}`
    case 'tmdb_tv':
      return `tmdb:tv:${raw}`
    case 'tmdb_movie':
      return `tmdb:movie:${raw}`
    case 'imdb':
      return `imdb:${raw}`
  }
}

async function applyFixMatch(): Promise<void> {
  savingFixMatch.value = true
  try {
    const canonical = fixDisableEnrichment.value ? null : buildCanonicalId()
    await vesselApi.setMetadataOverride(
      props.pluginId,
      props.nativeId,
      canonical,
      fixDisableEnrichment.value,
    )
    showFixMatchModal.value = false
    await loadDetails()
    pushToast('info', fixDisableEnrichment.value ? 'Disabled external metadata' : 'Updated metadata match')
  } catch (err) {
    pushToast('error', String(err))
  } finally {
    savingFixMatch.value = false
  }
}

/** Opens the CloudStream-style source resolver sheet before launching any player. */
async function openSourceResolver(ep: Episode): Promise<void> {
  resolverEpisode.value = ep
  resolvingStreams.value = true
  resolverError.value = ''
  streamChoices.value = []
  episodeSubtitles.value = []

  try {
    const res = await vesselApi.getStreams(props.pluginId, ep.streamId)
    if (resolverEpisode.value?.streamId !== ep.streamId) return
    streamChoices.value = res.streams
    episodeSubtitles.value = res.subtitles
    if (!res.streams.length) {
      resolverError.value = `No playable streams found from '${props.pluginId}' for Episode ${ep.episode}.`
    }
  } catch (err) {
    if (resolverEpisode.value?.streamId !== ep.streamId) return
    resolverError.value = String(err)
  } finally {
    if (resolverEpisode.value?.streamId === ep.streamId) {
      resolvingStreams.value = false
    }
  }
}

function closeSourceResolver(): void {
  resolverEpisode.value = null
  resolvingStreams.value = false
  resolverError.value = ''
  streamChoices.value = []
  episodeSubtitles.value = []
}

async function startStream(ep: Episode, candidate: StreamCandidate): Promise<void> {
  if (!detail.value) return
  const candidateIndex = Math.max(
    0,
    streamChoices.value.findIndex((item) => item.url === candidate.url && item.label === candidate.label),
  )
  setSession({
    pluginId: props.pluginId,
    nativeId: props.nativeId,
    detail: detail.value,
    episode: ep,
    streams: streamChoices.value.length ? [...streamChoices.value] : [candidate],
    subtitles: [...episodeSubtitles.value],
    activeStreamIndex: candidateIndex,
  })
  closeSourceResolver()
  await router.push({
    name: 'player',
    params: {
      pluginId: props.pluginId,
      nativeId: props.nativeId,
    },
    query: {
      streamId: ep.streamId,
      streamIndex: String(candidateIndex),
    },
  })
}

async function downloadCandidate(ep: Episode, candidate: StreamCandidate): Promise<void> {
  if (!detail.value) return
  try {
    await vesselApi.enqueueDownload({
      pluginId: props.pluginId,
      nativeId: props.nativeId,
      title: `${detail.value.title} — S${ep.season}E${ep.episode}`,
      season: ep.season,
      episode: ep.episode,
      kind: candidate.kind === 'hls' ? 'hls' : 'direct',
      sourceUrl: candidate.url,
      headers: candidate.headers,
      subtitles: [...candidate.subtitles, ...episodeSubtitles.value],
    })
    pushToast('info', `Queued download: S${ep.season}E${ep.episode}`)
  } catch (err) {
    pushToast('error', String(err))
  }
}

onMounted(() => {
  void loadDetails()
})
</script>

<template>
  <section class="nuvio-details">
    <div v-if="loading" class="hero-skeleton-wrap">
      <div class="v-skeleton hero-skeleton" />
    </div>

    <template v-else-if="detail">
      <!-- Deep Cinematic Full-Bleed Nuvio Hero Stage -->
      <header class="detail-hero">
        <img
          v-if="uiPreferences.details.showBackdrop && (detail.backdrop || detail.poster)"
          :src="detail.backdrop || detail.poster"
          :alt="detail.title"
          width="1440"
          height="680"
          fetchpriority="high"
          class="hero-backdrop"
        />
        <div
          class="hero-scrim"
          aria-hidden="true"
          :style="{
            '--scrim-dim': `${Math.max(58, uiPreferences.details.backdropDimPercent) / 100}`,
          }"
        />

        <div class="hero-nav-row">
          <button type="button" class="v-btn v-btn-sm" @click="router.back()">
            <ArrowLeft :size="14" aria-hidden="true" /> Back
          </button>
        </div>

        <div class="hero-stage">
          <img
            v-if="detail.poster"
            :src="detail.poster"
            :alt="detail.title"
            width="216"
            height="324"
            class="hero-poster"
          />

          <div class="hero-meta-col">
            <div class="meta-line">
              <span class="source-tag">{{ pluginId }}</span>
              <span v-if="detail.year">{{ detail.year }}</span>
              <span v-if="detail.rating !== undefined" class="rating-text">
                <Star :size="13" fill="currentColor" aria-hidden="true" />
                {{ detail.rating.toFixed(1) }}
              </span>
              <span v-if="detail.status">{{ detail.status }}</span>
              <template v-if="uiPreferences.details.showExternalIdBadges">
                <span v-if="detail.externalIds.anilist">AniList #{{ detail.externalIds.anilist }}</span>
                <span v-if="detail.externalIds.tmdb">TMDB #{{ detail.externalIds.tmdb }}</span>
                <span v-if="detail.externalIds.imdb">IMDb {{ detail.externalIds.imdb }}</span>
              </template>
            </div>

            <div class="title-block">
              <h1 class="detail-title">{{ detail.title }}</h1>
              <p
                v-if="detail.originalTitle && detail.originalTitle !== detail.title"
                class="orig-title v-muted"
              >
                {{ detail.originalTitle }}
              </p>
            </div>

            <div v-if="detail.genres.length" class="genres-line">
              <span v-for="genre in detail.genres.slice(0, 6)" :key="genre" class="v-badge">
                {{ genre }}
              </span>
            </div>

            <p v-if="formattedSynopsis" class="synopsis">{{ formattedSynopsis }}</p>

            <div class="cta-row">
              <button
                v-if="firstEpisode"
                type="button"
                class="v-btn v-btn-primary"
                @click="openSourceResolver(firstEpisode)"
              >
                <Play :size="16" fill="currentColor" aria-hidden="true" />
                {{ t('details.watch') }} S{{ firstEpisode.season }}:E{{ firstEpisode.episode }}
              </button>

              <div class="library-picker">
                <BookmarkPlus :size="15" class="picker-icon" aria-hidden="true" />
                <select
                  aria-label="Library status"
                  class="v-select status-select"
                  @change="saveLibrary(($event.target as HTMLSelectElement).value as LibraryStatus)"
                >
                  <option value="" disabled selected>{{ t('details.library_status') }}</option>
                  <option value="watching">{{ t('library.watching') }}</option>
                  <option value="plan_to_watch">{{ t('library.plan_to_watch') }}</option>
                  <option value="completed">{{ t('library.completed') }}</option>
                  <option value="on_hold">{{ t('library.on_hold') }}</option>
                  <option value="dropped">{{ t('library.dropped') }}</option>
                </select>
              </div>

              <button
                v-if="uiPreferences.details.showFixMatch"
                type="button"
                class="v-btn"
                @click="openFixMatchModal"
              >
                <Wrench :size="14" aria-hidden="true" /> {{ t('details.fix_match') }}
              </button>
            </div>
          </div>
        </div>
      </header>

      <!-- Nuvio Episodes Section with Layout Switcher -->
      <div class="episodes-stage">
        <div class="episodes-head">
          <div class="ep-head-title">
            <h2>{{ t('details.episodes') }}</h2>
            <span class="v-badge">{{ filteredEpisodes.length }}</span>
          </div>

          <div class="ep-head-controls">
            <div v-if="seasonNumbers.length > 1" class="season-tabs">
              <button
                v-for="s in seasonNumbers"
                :key="s"
                type="button"
                class="v-btn v-btn-sm"
                :class="{ 'v-btn-primary': selectedSeason === s }"
                @click="selectedSeason = s"
              >
                {{ t('details.season') }} {{ s }}
              </button>
            </div>

            <div class="layout-switcher" role="group" aria-label="Episode layout mode">
              <button
                type="button"
                class="layout-btn"
                :class="{ 'is-active': episodeLayout === 'grid_cards' }"
                aria-label="Grid cards layout"
                title="Grid Cards"
                @click="setEpisodeLayout('grid_cards')"
              >
                <LayoutGrid :size="15" aria-hidden="true" />
              </button>
              <button
                type="button"
                class="layout-btn"
                :class="{ 'is-active': episodeLayout === 'detailed_list' }"
                aria-label="Detailed list layout"
                title="Detailed List"
                @click="setEpisodeLayout('detailed_list')"
              >
                <List :size="15" aria-hidden="true" />
              </button>
              <button
                type="button"
                class="layout-btn"
                :class="{ 'is-active': episodeLayout === 'compact_rows' }"
                aria-label="Compact rows layout"
                title="Compact Rows"
                @click="setEpisodeLayout('compact_rows')"
              >
                <AlignJustify :size="15" aria-hidden="true" />
              </button>
            </div>
          </div>
        </div>

        <div v-if="!filteredEpisodes.length" class="v-card v-muted">
          {{ t('details.no_episodes') }}
        </div>

        <div v-else class="episodes-container" :class="`mode-${episodeLayout}`">
          <article
            v-for="ep in filteredEpisodes"
            :key="ep.streamId"
            class="nuvio-ep-item"
          >
            <button
              v-if="episodeLayout !== 'compact_rows'"
              type="button"
              class="ep-thumb-btn"
              :aria-label="`Find sources for Episode ${ep.episode}`"
              @click="openSourceResolver(ep)"
            >
              <img
                v-if="!uiPreferences.details.hideEpisodeThumbnails && (ep.thumbnail || detail.backdrop || detail.poster)"
                :src="ep.thumbnail || detail.backdrop || detail.poster"
                :alt="`Episode ${ep.episode}`"
                width="320"
                height="180"
                loading="lazy"
                class="ep-thumb-img"
              />
              <div class="ep-thumb-scrim" aria-hidden="true">
                <span class="ep-play-circle">
                  <Play :size="16" fill="currentColor" />
                </span>
              </div>
              <span class="ep-corner-tag">S{{ ep.season }}·E{{ ep.episode }}</span>
            </button>

            <button type="button" class="ep-content-btn" @click="openSourceResolver(ep)">
              <span class="ep-title">
                <span v-if="episodeLayout === 'compact_rows'" class="compact-num">
                  E{{ String(ep.episode).padStart(2, '0') }}
                </span>
                <template v-else>{{ ep.episode }}. </template>
                {{ ep.title ?? `Episode ${ep.episode}` }}
              </span>
              <span
                v-if="ep.overview && episodeLayout !== 'compact_rows' && !uiPreferences.details.hideEpisodeOverviews"
                class="ep-overview v-muted"
              >
                {{ ep.overview }}
              </span>
            </button>

            <div class="ep-controls">
              <button
                type="button"
                class="v-btn v-btn-sm"
                :aria-label="`Find sources for Episode ${ep.episode}`"
                @click="openSourceResolver(ep)"
              >
                <Server :size="14" aria-hidden="true" /> {{ t('details.sources') }}
              </button>
            </div>
          </article>
        </div>
      </div>

      <!-- Fix Match Modal -->
      <Teleport to="body">
        <Transition name="modal-pop">
          <div
            v-if="showFixMatchModal"
            class="stream-backdrop"
            role="dialog"
            aria-modal="true"
            :aria-label="t('details.fix_match')"
          >
            <div class="v-card stream-modal">
              <div class="picker-head">
                <div class="picker-title-group">
                  <h3 class="picker-title">{{ t('details.fix_match') }}</h3>
                  <span class="v-muted picker-sub">
                    {{ t('details.fix_match_desc') }}
                  </span>
                </div>
                <button
                  type="button"
                  class="v-btn v-btn-icon"
                  aria-label="Close Fix Match dialog"
                  @click="showFixMatchModal = false"
                >
                  <X :size="16" aria-hidden="true" />
                </button>
              </div>

              <div class="fix-form">
                <div class="fix-row">
                  <select v-model="fixProvider" aria-label="Metadata source" class="v-select fix-provider">
                    <option value="tmdb_tv">TMDB TV</option>
                    <option value="tmdb_movie">TMDB Movie</option>
                    <option value="anilist">AniList</option>
                    <option value="imdb">IMDb</option>
                  </select>
                  <input
                    v-model="fixIdInput"
                    name="canonicalId"
                    autocomplete="off"
                    aria-label="External metadata ID"
                    class="v-input"
                    placeholder="e.g. 1399, 918, tt0944947…"
                  />
                </div>

                <label class="v-switch">
                  <input v-model="fixDisableEnrichment" type="checkbox" class="v-switch-input" />
                  <span class="v-switch-track" aria-hidden="true" />
                  <span>{{ t('details.fix_match_disable') }}</span>
                </label>
              </div>

              <div class="fix-actions">
                <button type="button" class="v-btn" @click="showFixMatchModal = false">
                  {{ t('common.cancel') }}
                </button>
                <button
                  type="button"
                  class="v-btn v-btn-primary"
                  :disabled="savingFixMatch"
                  @click="applyFixMatch"
                >
                  {{ savingFixMatch ? t('common.loading') : t('details.fix_match_apply') }}
                </button>
              </div>
            </div>
          </div>
        </Transition>
      </Teleport>

      <!-- CloudStream-style Source Resolver & Picker Modal -->
      <Teleport to="body">
        <Transition name="modal-pop">
          <div
            v-if="resolverEpisode"
            class="stream-backdrop"
            role="dialog"
            aria-modal="true"
            aria-label="Stream source resolver"
          >
            <div class="v-card stream-modal">
              <div class="picker-head">
                <div class="picker-title-group">
                  <h3 class="picker-title">
                    S{{ resolverEpisode.season }}:E{{ resolverEpisode.episode }} —
                    {{ resolverEpisode.title ?? `Episode ${resolverEpisode.episode}` }}
                  </h3>
                  <span class="v-muted picker-sub">
                    Provider: <strong>{{ pluginId }}</strong>
                  </span>
                </div>
                <button
                  type="button"
                  class="v-btn v-btn-icon"
                  aria-label="Close source resolver"
                  @click="closeSourceResolver"
                >
                  <X :size="16" aria-hidden="true" />
                </button>
              </div>

              <div v-if="resolvingStreams" class="resolver-state">
                <Loader2 :size="28" class="spin accent-icon" aria-hidden="true" />
                <strong>{{ t('details.searching_streams') }}</strong>
                <span class="v-muted">{{ pluginId }}</span>
              </div>

              <div v-else-if="resolverError || !streamChoices.length" class="resolver-state">
                <AlertTriangle :size="28" class="warn-icon" aria-hidden="true" />
                <strong>{{ t('details.no_streams') }}</strong>
                <p class="v-muted resolver-err-msg">
                  {{ resolverError || t('details.no_streams') }}
                </p>
                <div class="resolver-err-actions">
                  <button
                    type="button"
                    class="v-btn v-btn-sm"
                    @click="openSourceResolver(resolverEpisode)"
                  >
                    <RefreshCw :size="14" aria-hidden="true" /> {{ t('common.retry') }}
                  </button>
                  <button type="button" class="v-btn v-btn-sm" @click="closeSourceResolver">
                    {{ t('common.cancel') }}
                  </button>
                </div>
              </div>

              <div v-else class="stream-list">
                <div
                  v-for="(cand, idx) in streamChoices"
                  :key="idx"
                  class="cs-source-row"
                >
                  <button
                    type="button"
                    class="cs-source-play"
                    @click="startStream(resolverEpisode, cand)"
                  >
                    <span class="cs-play-badge" aria-hidden="true">
                      <Play :size="14" fill="currentColor" />
                    </span>
                    <div class="cs-source-info">
                      <span class="stream-label">{{ cand.label }}</span>
                      <div class="cs-source-tags">
                        <span class="v-badge v-badge-accent">{{ cand.kind.toUpperCase() }}</span>
                        <span v-if="cand.quality" class="v-badge">{{ cand.quality }}</span>
                        <span v-if="cand.audioType" class="v-badge v-badge-warn">
                          {{ cand.audioType.toUpperCase() }}
                        </span>
                        <span
                          v-if="cand.subtitles.length || episodeSubtitles.length"
                          class="v-badge v-badge-success"
                        >
                          <Subtitles :size="11" aria-hidden="true" />
                          {{ cand.subtitles.length + episodeSubtitles.length }}
                        </span>
                      </div>
                    </div>
                  </button>

                  <button
                    type="button"
                    class="v-btn v-btn-icon"
                    :aria-label="`Download ${cand.label}`"
                    title="Download source"
                    @click="downloadCandidate(resolverEpisode, cand)"
                  >
                    <Download :size="15" aria-hidden="true" />
                  </button>
                </div>
              </div>
            </div>
          </div>
        </Transition>
      </Teleport>
    </template>
  </section>
</template>

<style scoped>
.nuvio-details {
  display: flex;
  flex-direction: column;
  gap: 1.5rem;
}

.hero-skeleton-wrap {
  padding: 2rem 2.75rem;
}

.hero-skeleton {
  height: 440px;
}

.detail-hero {
  position: relative;
  min-height: 480px;
  padding: 1.75rem 2.75rem 2.75rem;
  display: flex;
  flex-direction: column;
  justify-content: space-between;
  overflow: hidden;
}

.hero-backdrop {
  position: absolute;
  inset: 0;
  width: 100%;
  height: 100%;
  object-fit: cover;
  object-position: center 20%;
  transform: scale(1.05);
  pointer-events: none;
}

.hero-scrim {
  position: absolute;
  inset: 0;
  background:
    radial-gradient(circle at 80% 25%, color-mix(in srgb, var(--vessel-accent) 10%, transparent), transparent 45%),
    linear-gradient(
      90deg,
      var(--vessel-bg-base) 0%,
      color-mix(in srgb, var(--vessel-bg-base) 88%, transparent) 45%,
      color-mix(in srgb, var(--vessel-bg-base) 36%, transparent) 100%
    ),
    linear-gradient(
      180deg,
      color-mix(in srgb, var(--vessel-bg-base) 30%, transparent) 0%,
      color-mix(in srgb, var(--vessel-bg-base) 78%, transparent) 64%,
      var(--vessel-bg-base) 100%
    );
  pointer-events: none;
}

.hero-nav-row {
  position: relative;
  z-index: 2;
  margin-bottom: 1.5rem;
}

.hero-stage {
  position: relative;
  z-index: 2;
  display: flex;
  gap: 2.25rem;
  align-items: flex-end;
  flex-wrap: wrap;
}

.hero-poster {
  width: 216px;
  aspect-ratio: 2 / 3;
  object-fit: cover;
  border-radius: var(--vessel-radius);
  box-shadow:
    0 0 0 1px rgba(255, 255, 255, 0.16),
    0 28px 60px rgba(0, 0, 0, 0.78);
  flex-shrink: 0;
}

.hero-meta-col {
  flex: 1;
  min-width: 280px;
  display: flex;
  flex-direction: column;
  gap: 0.95rem;
}

.meta-line {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 0.8rem;
  font-size: 0.86rem;
  font-weight: 600;
  color: rgba(244, 246, 252, 0.84);
  font-variant-numeric: tabular-nums;
}

.source-tag {
  padding: 0.15rem 0.6rem;
  border-radius: 6px;
  background: rgba(203, 166, 247, 0.22);
  color: var(--vessel-accent);
  text-transform: uppercase;
  font-size: 0.72rem;
  letter-spacing: 0.06em;
}

.rating-text {
  display: inline-flex;
  align-items: center;
  gap: 0.26rem;
  color: var(--vessel-warning);
}

.title-block {
  display: flex;
  flex-direction: column;
  gap: 0.22rem;
}

.detail-title {
  margin: 0;
  font-size: 2.85rem;
  font-weight: 850;
  letter-spacing: -0.035em;
  line-height: 1.06;
  text-wrap: balance;
}

.orig-title {
  margin: 0;
  font-size: 0.96rem;
}

.genres-line {
  display: flex;
  flex-wrap: wrap;
  gap: 0.45rem;
}

.synopsis {
  margin: 0;
  color: rgba(244, 246, 252, 0.88);
  line-height: 1.65;
  max-width: 74ch;
  white-space: pre-line;
  display: -webkit-box;
  -webkit-line-clamp: 4;
  -webkit-box-orient: vertical;
  overflow: hidden;
}

.cta-row {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 0.75rem;
  margin-top: 0.35rem;
}

.library-picker {
  position: relative;
  display: flex;
  align-items: center;
}

.picker-icon {
  position: absolute;
  left: 0.9rem;
  color: var(--vessel-text-muted);
  pointer-events: none;
  z-index: 1;
}

.status-select {
  height: 40px;
  width: 205px;
  padding-left: 2.35rem;
  border-radius: 999px;
}

.episodes-stage {
  padding: 0 2.75rem 3rem;
  display: flex;
  flex-direction: column;
  gap: 1.1rem;
}

.episodes-head {
  display: flex;
  justify-content: space-between;
  align-items: center;
  flex-wrap: wrap;
  gap: 0.75rem;
}

.ep-head-title,
.ep-head-controls {
  display: flex;
  align-items: center;
  gap: 0.65rem;
  flex-wrap: wrap;
}

.ep-head-title h2 {
  margin: 0;
  font-size: 1.35rem;
  font-weight: 700;
}

.season-tabs {
  display: flex;
  gap: 0.45rem;
  flex-wrap: wrap;
}

.layout-switcher {
  display: inline-flex;
  align-items: center;
  padding: 0.2rem;
  border-radius: 999px;
  background: var(--vessel-surface-input);
  border: 1px solid var(--vessel-border);
  gap: 0.15rem;
}

.layout-btn {
  width: 30px;
  height: 28px;
  border-radius: 999px;
  border: none;
  background: transparent;
  color: var(--vessel-text-muted);
  display: flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;
  transition: background-color 0.16s ease, color 0.16s ease;
}

.layout-btn:hover {
  color: var(--vessel-text-primary);
}

.layout-btn.is-active {
  background: rgba(203, 166, 247, 0.22);
  color: var(--vessel-accent);
}

/* Mode 1: Detailed List (Nuvio default horizontal row) */
.episodes-container {
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
}

/* Mode 2: Grid Cards */
.episodes-container.mode-grid_cards {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(290px, 1fr));
  gap: 1rem;
}

.episodes-container.mode-grid_cards .nuvio-ep-item {
  flex-direction: column;
  align-items: stretch;
  padding: 0.85rem;
}

.episodes-container.mode-grid_cards .ep-thumb-btn {
  width: 100%;
}

.episodes-container.mode-grid_cards .ep-controls {
  justify-content: flex-end;
}

/* Mode 3: Compact Rows */
.episodes-container.mode-compact_rows .nuvio-ep-item {
  padding: 0.6rem 0.95rem;
}

.compact-num {
  color: var(--vessel-accent);
  font-weight: 700;
  margin-right: 0.45rem;
  font-variant-numeric: tabular-nums;
}

.nuvio-ep-item {
  display: flex;
  align-items: center;
  gap: 1.15rem;
  padding: 0.75rem;
  border-radius: calc(var(--vessel-radius) + 2px);
  background: var(--vessel-surface-card);
  border: 1px solid var(--vessel-border);
  transition:
    background-color 0.22s ease,
    border-color 0.22s ease,
    transform 0.22s cubic-bezier(0.22, 1, 0.36, 1);
}

.nuvio-ep-item:hover {
  background: var(--vessel-surface-elevated);
  border-color: var(--vessel-border-strong);
  transform: translateY(-1px);
}

.ep-thumb-btn {
  position: relative;
  width: 176px;
  aspect-ratio: 16 / 9;
  border-radius: 10px;
  overflow: hidden;
  border: none;
  padding: 0;
  background: var(--vessel-bg-elevated);
  flex-shrink: 0;
  cursor: pointer;
}

.ep-thumb-img {
  width: 100%;
  height: 100%;
  object-fit: cover;
  display: block;
  transition: transform 0.28s ease;
}

.ep-thumb-btn:hover .ep-thumb-img {
  transform: scale(1.06);
}

.ep-thumb-scrim {
  position: absolute;
  inset: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  background: rgba(10, 11, 16, 0.38);
  transition: background-color 0.2s ease;
}

.ep-thumb-btn:hover .ep-thumb-scrim {
  background: rgba(10, 11, 16, 0.56);
}

.ep-play-circle {
  width: 36px;
  height: 36px;
  border-radius: 999px;
  display: flex;
  align-items: center;
  justify-content: center;
  background: rgba(255, 255, 255, 0.92);
  color: #0a0b10;
  box-shadow: 0 6px 16px rgba(0, 0, 0, 0.45);
}

.ep-corner-tag {
  position: absolute;
  left: 0.45rem;
  bottom: 0.4rem;
  padding: 0.12rem 0.45rem;
  border-radius: 6px;
  background: rgba(10, 11, 16, 0.78);
  color: #ffffff;
  font-size: 0.72rem;
  font-weight: 700;
  font-variant-numeric: tabular-nums;
}

.ep-content-btn {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 0.3rem;
  border: none;
  background: transparent;
  padding: 0;
  text-align: left;
  cursor: pointer;
}

.ep-title {
  font-size: 1.02rem;
  font-weight: 650;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.ep-overview {
  font-size: 0.86rem;
  line-height: 1.45;
  display: -webkit-box;
  -webkit-line-clamp: 3;
  -webkit-box-orient: vertical;
  overflow: hidden;
}

.ep-controls {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  flex-shrink: 0;
}

.stream-backdrop {
  position: fixed;
  inset: 0;
  z-index: 65;
  display: flex;
  align-items: center;
  justify-content: center;
  background: rgba(10, 11, 16, 0.78);
  backdrop-filter: blur(14px);
  -webkit-backdrop-filter: blur(14px);
  padding: 1.25rem;
  overscroll-behavior: contain;
}

.stream-modal {
  width: min(580px, 100%);
  display: flex;
  flex-direction: column;
  gap: 1.05rem;
  background: var(--vessel-surface-overlay);
  border-color: var(--vessel-border-strong);
}

.picker-head {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  gap: 0.75rem;
}

.picker-title-group {
  display: flex;
  flex-direction: column;
  gap: 0.2rem;
}

.picker-title {
  margin: 0;
  font-size: 1.15rem;
}

.picker-sub {
  font-size: 0.84rem;
}

.fix-form {
  display: flex;
  flex-direction: column;
  gap: 0.9rem;
}

.fix-row {
  display: flex;
  gap: 0.6rem;
}

.fix-provider {
  width: 160px;
  flex-shrink: 0;
}

.fix-actions {
  display: flex;
  justify-content: flex-end;
  gap: 0.55rem;
}

.resolver-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  text-align: center;
  gap: 0.55rem;
  padding: 1.75rem 1rem;
}

.resolver-err-msg {
  margin: 0;
  max-width: 44ch;
  font-size: 0.88rem;
}

.resolver-err-actions {
  display: flex;
  gap: 0.5rem;
  margin-top: 0.4rem;
}

.accent-icon {
  color: var(--vessel-accent);
}

.warn-icon {
  color: var(--vessel-warning);
}

.stream-list {
  display: flex;
  flex-direction: column;
  gap: 0.55rem;
  max-height: 380px;
  overflow-y: auto;
}

.cs-source-row {
  display: flex;
  align-items: center;
  gap: 0.55rem;
  padding: 0.55rem 0.65rem;
  border-radius: var(--vessel-radius);
  border: 1px solid var(--vessel-border);
  background: var(--vessel-surface-card);
  transition: border-color 0.18s ease, background-color 0.18s ease;
}

.cs-source-row:hover {
  border-color: color-mix(in srgb, var(--vessel-accent) 55%, transparent);
  background: var(--vessel-surface-elevated);
}

.cs-source-play {
  flex: 1;
  min-width: 0;
  display: flex;
  align-items: center;
  gap: 0.8rem;
  border: none;
  background: transparent;
  padding: 0.25rem;
  text-align: left;
  cursor: pointer;
}

.cs-play-badge {
  width: 34px;
  height: 34px;
  border-radius: 999px;
  display: flex;
  align-items: center;
  justify-content: center;
  background: #f4f6fc;
  color: #0a0b10;
  flex-shrink: 0;
}

.cs-source-info {
  display: flex;
  flex-direction: column;
  gap: 0.3rem;
  min-width: 0;
}

.stream-label {
  font-weight: 600;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.cs-source-tags {
  display: flex;
  flex-wrap: wrap;
  gap: 0.35rem;
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
