<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, ref, shallowRef, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import Hls from 'hls.js'
import mpegts from 'mpegts.js'
import {
  ArrowLeft,
  Captions,
  Check,
  ExternalLink,
  FastForward,
  Layers,
  ListVideo,
  Lock,
  Maximize,
  Minimize,
  Pause,
  Play,
  RotateCcw,
  RotateCw,
  Settings,
  SkipBack,
  SkipForward,
  Unlock,
  Volume2,
  VolumeX,
  X,
} from 'lucide-vue-next'
import { vesselApi } from '@/api/vessel'
import { useI18n } from '@/composables/useI18n'
import { usePlayerSession } from '@/composables/usePlayerSession'
import { useUiPreferences } from '@/composables/useUiPreferences'
import { useVesselEvents } from '@/composables/useVesselEvents'
import { parseSubtitles, type SubtitleCue } from '@/utils/subtitleParser'
import type { Episode, StreamCandidate, SubtitleTrack } from '@/types/vessel'

const props = defineProps<{
  pluginId: string
  nativeId: string
}>()

const route = useRoute()
const router = useRouter()
const { t } = useI18n()
const { pushToast } = useVesselEvents()
const { uiPreferences, savePreferences } = useUiPreferences()
const { activeSession, setSession, updateEpisodeSession, setActiveStreamIndex } = usePlayerSession()

type DrawerTab = 'episodes' | 'sources' | 'subtitles' | 'settings' | null
type FitMode = 'contain' | 'cover' | 'fill'

const playerStageRef = ref<HTMLElement | null>(null)
const videoRef = ref<HTMLVideoElement | null>(null)
const progressBarRef = ref<HTMLElement | null>(null)

const loadingInitial = ref(false)
const switchingEpisode = ref(false)
const playbackError = ref('')

const isPlaying = ref(false)
const isBuffering = ref(false)
const isMuted = ref(false)
const volume = ref(1)
const currentTime = ref(0)
const duration = ref(0)
const bufferedEnd = ref(0)
const playbackRate = ref(1)
const fitMode = ref<FitMode>('contain')
const isFullscreen = ref(false)

const isLocked = ref(false)
const controlsVisible = ref(true)
const activeDrawer = ref<DrawerTab>(null)
const hoverTime = ref<number | null>(null)
const hoverPercent = ref(0)

const activeSubtitleIndex = ref<number>(-1)
const subtitleDelayMs = ref(0)
const activeCues = shallowRef<SubtitleCue[]>([])
const loadingSubtitle = shallowRef(false)
const subtitleError = shallowRef('')
const pendingSeekSeconds = ref<number | null>(null)
const playableSrc = ref<string>('')
const gestureLabel = ref<string>('')

let hideTimer: number | null = null
let progressTimer: number | null = null
let subSaveTimer: number | null = null
let gestureTimer: number | null = null
let touchSeekTimer: number | null = null
let touchSeekDelta = 0
let lastPinchAt = 0
let hlsInstance: Hls | null = null
let mpegtsPlayer: mpegts.Player | null = null
let forceHlsJsNextTry = false
let userExplicitPause = false

function destroyMediaEngines(): void {
  if (hlsInstance) {
    hlsInstance.destroy()
    hlsInstance = null
  }
  if (mpegtsPlayer) {
    try {
      mpegtsPlayer.pause()
      mpegtsPlayer.unload()
      mpegtsPlayer.detachMediaElement()
      mpegtsPlayer.destroy()
    } catch {
      // Ignore cleanup errors on torn-down MSE instances
    }
    mpegtsPlayer = null
  }
}

function scheduleSaveSubtitles(): void {
  if (subSaveTimer !== null) window.clearTimeout(subSaveTimer)
  subSaveTimer = window.setTimeout(() => {
    void savePreferences(uiPreferences.value).catch(() => {})
  }, 350)
}

const subtitleOverlayStyle = computed(() => {
  const baseBottom = Math.max(12, Math.min(280, uiPreferences.value.subtitles.bottomOffsetPx ?? 56))
  const raised = controlsVisible.value && !isLocked.value ? baseBottom + 84 : baseBottom
  return {
    bottom: `${raised}px`,
  }
})

const subtitleLineStyle = computed(() => {
  const subs = uiPreferences.value.subtitles
  const alpha = Math.max(0, Math.min(100, subs.backgroundOpacity ?? 72)) / 100
  const fontSize = `${Math.max(14, Math.min(56, subs.fontSizePx ?? 28))}px`
  const fontFamilyMap: Record<string, string> = {
    'system-ui': 'system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif',
    serif: 'Georgia, "Times New Roman", serif',
    monospace: '"JetBrains Mono", "SFMono-Regular", Menlo, monospace',
    rounded: '"Nunito", "SF Pro Rounded", system-ui, sans-serif',
  }
  const family = fontFamilyMap[subs.fontFamily] ?? (subs.fontFamily || fontFamilyMap['system-ui'])
  const outlineStyle = subs.outlineStyle ?? 'outline'
  const textShadow =
    outlineStyle === 'none'
      ? 'none'
      : outlineStyle === 'shadow'
        ? '0 2px 6px rgba(0,0,0,0.95), 0 4px 14px rgba(0,0,0,0.85)'
        : '0 0 3px #000, -1px -1px 0 #000, 1px -1px 0 #000, -1px 1px 0 #000, 1px 1px 0 #000, 0 2px 8px rgba(0,0,0,0.9)'
  return {
    fontSize,
    fontFamily: family,
    color: subs.textColor || '#ffffff',
    fontWeight: subs.bold ? '700' : '600',
    backgroundColor: outlineStyle === 'none' ? 'transparent' : `rgba(8, 9, 14, ${alpha})`,
    textShadow,
  }
})

async function persistWatchProgress(forceCompleted = false): Promise<void> {
  if (isLive.value) return
  const ep = currentEpisode.value
  const d = detail.value
  const el = videoRef.value
  if (!ep || !d || !el) return
  const posSec = Number.isFinite(el.currentTime) ? el.currentTime : currentTime.value
  const durSec = Number.isFinite(el.duration) ? el.duration : duration.value
  if (!forceCompleted && posSec < 2) return
  const positionMs = Math.max(0, Math.floor(posSec * 1000))
  const durationMs = Math.max(0, Math.floor((durSec || 0) * 1000))
  const completed =
    forceCompleted || (durationMs > 0 && positionMs / durationMs >= 0.92)
  const canonicalId =
    d.externalIds.tmdb
      ? `tmdb:${d.externalIds.tmdb}`
      : d.externalIds.imdb
        ? `imdb:${d.externalIds.imdb}`
        : d.externalIds.anilist
          ? `anilist:${d.externalIds.anilist}`
          : undefined
  try {
    await vesselApi.recordProgress({
      media: { pluginId: props.pluginId, nativeId: props.nativeId },
      streamId: ep.streamId,
      canonicalId,
      season: ep.season,
      episode: ep.episode,
      absoluteEpisode: ep.absoluteEpisode,
      title: d.title,
      posterUrl: d.poster,
      backdropUrl: d.backdrop,
      episodeTitle: ep.title,
      positionMs,
      durationMs,
      completed,
      lastWatchedAt: new Date().toISOString(),
    })
  } catch {
    // Non-fatal background persistence
  }
}

async function resolveDesiredStartSeconds(ep: Episode | null): Promise<number> {
  if (isLive.value || !ep) return 0
  if (session.value?.startPositionMs && session.value.startPositionMs > 2000) {
    return session.value.startPositionMs / 1000
  }
  try {
    const saved = await vesselApi.getWatchProgress(
      props.pluginId,
      props.nativeId,
      ep.season,
      ep.episode,
    )
    if (saved && !saved.completed && saved.positionMs > 2000) {
      return saved.positionMs / 1000
    }
  } catch {
    // Ignore if unavailable
  }
  return 0
}

function showGestureHud(label: string): void {
  gestureLabel.value = label
  if (gestureTimer !== null) window.clearTimeout(gestureTimer)
  gestureTimer = window.setTimeout(() => {
    gestureLabel.value = ''
  }, 950)
}

function handleTouchpadWheel(ev: WheelEvent): void {
  if (isLocked.value) return
  if ((ev.target as HTMLElement | null)?.closest('.cs-drawer')) return

  if (ev.ctrlKey) {
    ev.preventDefault()
    const now = Date.now()
    if (now - lastPinchAt > 450 && Math.abs(ev.deltaY) > 2) {
      lastPinchAt = now
      fitMode.value = fitMode.value === 'contain' ? 'cover' : 'contain'
      showGestureHud(fitMode.value === 'cover' ? 'Fill Screen' : 'Fit Screen')
    }
    return
  }

  const absX = Math.abs(ev.deltaX)
  const absY = Math.abs(ev.deltaY)
  if (absX > absY && absX > 2 && !isLive.value && duration.value > 0) {
    ev.preventDefault()
    touchSeekDelta += ev.deltaX * 0.14
    const rounded = Math.round(touchSeekDelta)
    if (rounded !== 0) {
      showGestureHud(rounded > 0 ? `+${rounded}s »` : `« ${rounded}s`)
    }
    if (touchSeekTimer !== null) window.clearTimeout(touchSeekTimer)
    touchSeekTimer = window.setTimeout(() => {
      const commit = Math.round(touchSeekDelta)
      touchSeekDelta = 0
      if (commit !== 0) {
        seekBy(commit)
      }
    }, 150)
    return
  }

  if (absY > absX && absY > 2) {
    ev.preventDefault()
    const nextVol = Math.min(1, Math.max(0, volume.value - ev.deltaY * 0.0018))
    setVolumeLevel(nextVol)
    showGestureHud(`Volume ${Math.round(nextVol * 100)}%`)
  }
}

const session = computed(() => activeSession.value)
const detail = computed(() => session.value?.detail ?? null)
const currentEpisode = computed(() => session.value?.episode ?? null)
const isLive = computed(() => {
  if (detail.value?.type === 'live') return true
  const sid = currentEpisode.value?.streamId?.toLowerCase() ?? ''
  const nid = props.nativeId.toLowerCase()
  return (
    props.pluginId === 'iptv' ||
    sid.startsWith('tv:') ||
    sid.startsWith('tv_') ||
    sid.startsWith('iptv_') ||
    sid.startsWith('vns::tv::') ||
    nid.startsWith('tv:') ||
    nid.startsWith('tv_') ||
    nid.startsWith('iptv_')
  )
})
const streams = computed(() => session.value?.streams ?? [])
const activeStream = computed<StreamCandidate | null>(() => {
  if (!session.value) return null
  return session.value.streams[session.value.activeStreamIndex] ?? session.value.streams[0] ?? null
})

const availableSubtitles = computed<SubtitleTrack[]>(() => {
  const fromStream = activeStream.value?.subtitles ?? []
  const fromEpisode = session.value?.subtitles ?? []
  return [...fromStream, ...fromEpisode]
})

const activeSubtitleLines = computed<string[]>(() => {
  if (isLive.value || activeSubtitleIndex.value < 0 || !activeCues.value.length) return []
  const targetMs = currentTime.value * 1000 - subtitleDelayMs.value
  const cues = activeCues.value
  // Binary search first cue whose endMs >= targetMs, then collect active cues around that index
  let lo = 0
  let hi = cues.length - 1
  let pos = cues.length
  while (lo <= hi) {
    const mid = (lo + hi) >> 1
    if (cues[mid].endMs >= targetMs) {
      pos = mid
      hi = mid - 1
    } else {
      lo = mid + 1
    }
  }
  const out: string[] = []
  for (let i = pos; i < cues.length; i++) {
    const cue = cues[i]
    if (cue.startMs > targetMs) break
    if (targetMs >= cue.startMs && targetMs <= cue.endMs) {
      out.push(...cue.lines)
    }
  }
  return out
})

watch(
  availableSubtitles,
  (tracks) => {
    if (!tracks.length) {
      activeSubtitleIndex.value = -1
      return
    }
    if (activeSubtitleIndex.value >= tracks.length) {
      activeSubtitleIndex.value = -1
    }
    if (activeSubtitleIndex.value === -1) {
      const defIdx = tracks.findIndex((t) => t.isDefault)
      if (defIdx >= 0) {
        activeSubtitleIndex.value = defIdx
      }
    }
  },
  { immediate: true },
)

watch(
  [activeSubtitleIndex, availableSubtitles],
  async ([idx, tracks], _prev, onCleanup) => {
    let cancelled = false
    onCleanup(() => {
      cancelled = true
    })

    subtitleError.value = ''
    if (isLive.value || idx < 0 || !tracks[idx]) {
      activeCues.value = []
      loadingSubtitle.value = false
      return
    }

    const track = tracks[idx]
    if (track.source.kind === 'inline_content') {
      activeCues.value = parseSubtitles(track.source.content, track.format)
      loadingSubtitle.value = false
      return
    }

    loadingSubtitle.value = true
    try {
      const raw = await vesselApi.fetchSubtitleText(track.source.url, track.source.headers)
      if (cancelled) return
      const cues = parseSubtitles(raw, track.format)
      activeCues.value = cues
      if (!cues.length) {
        subtitleError.value = 'Subtitle file contained no readable timed cues.'
      }
    } catch (err) {
      if (!cancelled) {
        activeCues.value = []
        subtitleError.value = String(err)
      }
    } finally {
      if (!cancelled) {
        loadingSubtitle.value = false
      }
    }
  },
  { immediate: true },
)

const allEpisodes = computed<Episode[]>(() => detail.value?.episodes ?? [])
const currentEpisodeIndex = computed(() => {
  const ep = currentEpisode.value
  if (!ep) return -1
  return allEpisodes.value.findIndex((item) => item.streamId === ep.streamId)
})
const prevEpisode = computed(() =>
  currentEpisodeIndex.value > 0 ? allEpisodes.value[currentEpisodeIndex.value - 1] : null,
)
const nextEpisode = computed(() =>
  currentEpisodeIndex.value >= 0 && currentEpisodeIndex.value < allEpisodes.value.length - 1
    ? allEpisodes.value[currentEpisodeIndex.value + 1]
    : null,
)

const progressPercent = computed(() => {
  if (!duration.value || duration.value <= 0) return 0
  return Math.min(100, Math.max(0, (currentTime.value / duration.value) * 100))
})

const bufferedPercent = computed(() => {
  if (!duration.value || duration.value <= 0) return 0
  return Math.min(100, Math.max(0, (bufferedEnd.value / duration.value) * 100))
})

const fitLabel = computed(() => {
  if (fitMode.value === 'cover') return 'Fill'
  if (fitMode.value === 'fill') return 'Stretch'
  return 'Fit'
})

function formatClock(sec: number): string {
  if (!Number.isFinite(sec) || sec < 0) return '0:00'
  const total = Math.floor(sec)
  const hrs = Math.floor(total / 3600)
  const mins = Math.floor((total % 3600) / 60)
  const secs = total % 60
  if (hrs > 0) {
    return `${hrs}:${String(mins).padStart(2, '0')}:${String(secs).padStart(2, '0')}`
  }
  return `${mins}:${String(secs).padStart(2, '0')}`
}

function scheduleHideControls(): void {
  if (hideTimer !== null) {
    window.clearTimeout(hideTimer)
    hideTimer = null
  }
  controlsVisible.value = true
  if (isPlaying.value && !activeDrawer.value && !isLocked.value) {
    hideTimer = window.setTimeout(() => {
      controlsVisible.value = false
    }, 3200)
  }
}

function handleStagePointerMove(): void {
  scheduleHideControls()
}

function togglePlay(): void {
  const el = videoRef.value
  if (!el || isLocked.value) return
  if (el.paused) {
    userExplicitPause = false
    void el.play().catch((err) => {
      playbackError.value = String(err)
    })
  } else {
    userExplicitPause = true
    el.pause()
  }
  scheduleHideControls()
}

function seekBy(deltaSec: number): void {
  const el = videoRef.value
  if (!el || isLocked.value || isLive.value) return
  const target = Math.max(0, Math.min(el.duration || Infinity, el.currentTime + deltaSec))
  el.currentTime = target
  currentTime.value = target
  scheduleHideControls()
}

function seekFromBar(ev: MouseEvent): void {
  const bar = progressBarRef.value
  const el = videoRef.value
  if (!bar || !el || !duration.value || isLocked.value) return
  const rect = bar.getBoundingClientRect()
  const ratio = Math.min(1, Math.max(0, (ev.clientX - rect.left) / rect.width))
  const target = ratio * duration.value
  el.currentTime = target
  currentTime.value = target
  scheduleHideControls()
}

function updateHoverTime(ev: MouseEvent): void {
  const bar = progressBarRef.value
  if (!bar || !duration.value) {
    hoverTime.value = null
    return
  }
  const rect = bar.getBoundingClientRect()
  const ratio = Math.min(1, Math.max(0, (ev.clientX - rect.left) / rect.width))
  hoverPercent.value = ratio * 100
  hoverTime.value = ratio * duration.value
}

function toggleMute(): void {
  const el = videoRef.value
  if (!el) return
  el.muted = !el.muted
  isMuted.value = el.muted
  scheduleHideControls()
}

function setVolumeLevel(val: number): void {
  const el = videoRef.value
  if (!el) return
  const clamped = Math.min(1, Math.max(0, val))
  el.volume = clamped
  el.muted = clamped === 0
  volume.value = clamped
  isMuted.value = clamped === 0
  scheduleHideControls()
}

function setSpeed(rate: number): void {
  const el = videoRef.value
  playbackRate.value = rate
  if (el) el.playbackRate = rate
  scheduleHideControls()
}

function cycleFitMode(): void {
  const order: FitMode[] = ['contain', 'cover', 'fill']
  const idx = order.indexOf(fitMode.value)
  fitMode.value = order[(idx + 1) % order.length]
  scheduleHideControls()
}

function toggleDrawer(tab: DrawerTab): void {
  if (isLocked.value) return
  activeDrawer.value = activeDrawer.value === tab ? null : tab
  scheduleHideControls()
}

function toggleLock(): void {
  isLocked.value = !isLocked.value
  if (isLocked.value) {
    activeDrawer.value = null
    controlsVisible.value = true
  } else {
    scheduleHideControls()
  }
}

async function toggleFullscreen(): Promise<void> {
  const stage = playerStageRef.value
  if (!stage) return
  try {
    if (!document.fullscreenElement) {
      await stage.requestFullscreen()
      isFullscreen.value = true
    } else {
      await document.exitFullscreen()
      isFullscreen.value = false
    }
  } catch {
    // WebView may block fullscreen API in some contexts; ignore gracefully
  }
  scheduleHideControls()
}

function switchSource(index: number): void {
  const el = videoRef.value
  if (el && Number.isFinite(el.currentTime) && el.currentTime > 1) {
    pendingSeekSeconds.value = el.currentTime
  }
  playbackError.value = ''
  setActiveStreamIndex(index)
  activeDrawer.value = null
  scheduleHideControls()
}

async function switchEpisode(ep: Episode): Promise<void> {
  if (!detail.value || switchingEpisode.value) return
  await persistWatchProgress()
  switchingEpisode.value = true
  playbackError.value = ''
  try {
    const directCandidate = activeSession.value?.channelStreams?.[ep.streamId]
    if (directCandidate) {
      pendingSeekSeconds.value = 0
      setSession({
        ...activeSession.value!,
        detail: {
          ...detail.value,
          title: ep.title ?? detail.value.title,
        },
        episode: ep,
        streams: [directCandidate],
        subtitles: [],
        activeStreamIndex: 0,
      })
      activeDrawer.value = null
      return
    }
    const res = await vesselApi.getStreams(props.pluginId, ep.streamId)
    if (!res.streams.length) {
      pushToast(
        'warn',
        isLive.value
          ? `No playable streams found for ${ep.title ?? `Channel ${ep.episode}`}.`
          : `No playable streams found for Episode ${ep.episode}.`,
      )
      return
    }
    pendingSeekSeconds.value = 0
    updateEpisodeSession(ep, res.streams, res.subtitles, 0)
    activeDrawer.value = null
  } catch (err) {
    pushToast('error', String(err))
  } finally {
    switchingEpisode.value = false
    scheduleHideControls()
  }
}

async function openInExternalMpv(): Promise<void> {
  if (!detail.value || !currentEpisode.value || !activeStream.value) return
  try {
    await vesselApi.playCandidate({
      pluginId: props.pluginId,
      nativeId: props.nativeId,
      season: currentEpisode.value.season,
      episode: currentEpisode.value.episode,
      absoluteEpisode: currentEpisode.value.absoluteEpisode,
      mediaType: detail.value.type,
      candidate: activeStream.value,
      extraSubtitles: isLive.value ? [] : availableSubtitles.value,
      startPositionMs: isLive.value ? 0 : Math.floor(currentTime.value * 1000),
      title: isLive.value
        ? (currentEpisode.value.title ?? detail.value.title)
        : `${detail.value.title} — S${currentEpisode.value.season}:E${currentEpisode.value.episode}`,
    })
    videoRef.value?.pause()
    pushToast('info', 'Handed off playback to external mpv')
  } catch (err) {
    pushToast('warn', String(err))
  }
}

function goBack(): void {
  void persistWatchProgress()
  if (document.fullscreenElement) {
    void document.exitFullscreen().catch(() => {})
  }
  if (window.history.length > 1) {
    router.back()
  } else if (props.pluginId === 'iptv') {
    void router.push({ name: 'iptv' })
  } else {
    void router.push({
      name: 'details',
      params: { pluginId: props.pluginId, nativeId: props.nativeId },
    })
  }
}

function onLoadedMetadata(): void {
  const el = videoRef.value
  if (!el) return
  duration.value = Number.isFinite(el.duration) ? el.duration : 0
  el.playbackRate = isLive.value ? 1 : playbackRate.value
  el.volume = volume.value
  el.muted = isMuted.value
  if (
    pendingSeekSeconds.value !== null &&
    pendingSeekSeconds.value > 2 &&
    !hlsInstance &&
    (!el.duration || pendingSeekSeconds.value < el.duration - 2)
  ) {
    const target = pendingSeekSeconds.value
    pendingSeekSeconds.value = null
    el.currentTime = target
    currentTime.value = target
    return
  }
  if (!userExplicitPause) {
    void el.play().catch(() => {})
  }
  isBuffering.value = false
}

function onSeeked(): void {
  const el = videoRef.value
  if (!el) return
  isBuffering.value = false
  if (!userExplicitPause && el.paused) {
    void el.play().catch(() => {})
  }
}

function onCanPlay(): void {
  const el = videoRef.value
  if (!el) return
  if (pendingSeekSeconds.value !== null && pendingSeekSeconds.value > 2 && !hlsInstance) {
    const target = pendingSeekSeconds.value
    pendingSeekSeconds.value = null
    el.currentTime = target
    currentTime.value = target
    return
  }
  isBuffering.value = false
  if (!userExplicitPause && el.paused) {
    void el.play().catch(() => {})
  }
}

function onTimeUpdate(): void {
  const el = videoRef.value
  if (!el) return
  currentTime.value = el.currentTime
  if (el.buffered.length > 0) {
    bufferedEnd.value = el.buffered.end(el.buffered.length - 1)
  }
}

function onVideoError(): void {
  const el = videoRef.value
  const src = playableSrc.value
  const stream = activeStream.value
  if (el && src && stream) {
    const isHls =
      stream.kind === 'hls' || src.toLowerCase().includes('.m3u8')
    if (isHls && !forceHlsJsNextTry && Hls.isSupported() && !hlsInstance) {
      forceHlsJsNextTry = true
      playbackError.value = ''
      isBuffering.value = true
      void attachMediaSource(src, stream, true)
      return
    }
    // Auto-try next stream candidate (e.g. Xtream HLS <-> MPEG-TS) when primary stream fails
    const curIdx = session.value?.activeStreamIndex ?? 0
    if (curIdx + 1 < streams.value.length) {
      switchSource(curIdx + 1)
      return
    }
  }
  const code = el?.error?.code
  playbackError.value =
    code === 4
      ? t('player.error_unsupported')
      : t('player.error_failed')
  isBuffering.value = false
  isPlaying.value = false
  controlsVisible.value = true
}

function retryStream(): void {
  const stream = activeStream.value
  const src = playableSrc.value
  if (!stream || !src) return
  playbackError.value = ''
  isBuffering.value = true
  void attachMediaSource(src, stream, forceHlsJsNextTry)
}

function handleKeydown(ev: KeyboardEvent): void {
  if (ev.target instanceof HTMLInputElement || ev.target instanceof HTMLTextAreaElement) return
  if (ev.key === 'Escape') {
    if (activeDrawer.value) {
      activeDrawer.value = null
    } else if (isLocked.value) {
      isLocked.value = false
    } else if (document.fullscreenElement) {
      void document.exitFullscreen().catch(() => {})
    } else {
      goBack()
    }
    return
  }
  if (isLocked.value) return
  switch (ev.key.toLowerCase()) {
    case ' ':
    case 'k':
      ev.preventDefault()
      togglePlay()
      break
    case 'arrowleft':
    case 'j':
      ev.preventDefault()
      seekBy(-10)
      break
    case 'arrowright':
    case 'l':
      ev.preventDefault()
      seekBy(10)
      break
    case 'arrowup':
      ev.preventDefault()
      setVolumeLevel(volume.value + 0.05)
      break
    case 'arrowdown':
      ev.preventDefault()
      setVolumeLevel(volume.value - 0.05)
      break
    case 'm':
      ev.preventDefault()
      toggleMute()
      break
    case 'f':
      ev.preventDefault()
      void toggleFullscreen()
      break
  }
}

function onFullscreenChange(): void {
  isFullscreen.value = Boolean(document.fullscreenElement)
}

async function ensureSessionLoaded(): Promise<void> {
  if (activeSession.value && activeSession.value.pluginId === props.pluginId && activeSession.value.nativeId === props.nativeId) {
    return
  }
  if (props.pluginId === 'iptv') {
    playbackError.value = 'Return to Live TV to pick a channel.'
    return
  }
  loadingInitial.value = true
  playbackError.value = ''
  try {
    const loadedDetail = await vesselApi.getDetailsWithEnrichment(props.pluginId, props.nativeId, 'en')
    const queryStreamId = typeof route.query.streamId === 'string' ? route.query.streamId : ''
    const ep =
      loadedDetail.episodes.find((item) => item.streamId === queryStreamId) ??
      loadedDetail.episodes[0]
    if (!ep) {
      playbackError.value = 'No playable episodes available for this title.'
      return
    }
    const res = await vesselApi.getStreams(props.pluginId, ep.streamId)
    if (!res.streams.length) {
      playbackError.value = `No playable streams found for Episode ${ep.episode}.`
      return
    }
    const idx = Number(route.query.streamIndex ?? 0)
    setSession({
      pluginId: props.pluginId,
      nativeId: props.nativeId,
      detail: loadedDetail,
      episode: ep,
      streams: res.streams,
      subtitles: res.subtitles,
      activeStreamIndex: Number.isFinite(idx) && idx >= 0 && idx < res.streams.length ? idx : 0,
    })
  } catch (err) {
    playbackError.value = String(err)
  } finally {
    loadingInitial.value = false
  }
}

async function attachMediaSource(
  src: string,
  stream: StreamCandidate,
  preferHlsJs = false,
  startSec = 0,
): Promise<void> {
  await nextTick()
  const el = videoRef.value
  if (!el || !src) return

  userExplicitPause = false
  destroyMediaEngines()
  el.pause()

  const lowerSrc = src.toLowerCase()
  const looksMpegTs =
    stream.kind === 'mpeg_ts' ||
    (lowerSrc.includes('.ts') && !lowerSrc.includes('.m3u8'))
  const looksHls = stream.kind === 'hls' || lowerSrc.includes('.m3u8')

  if (looksMpegTs && mpegts.isSupported()) {
    try {
      const player = mpegts.createPlayer(
        {
          type: 'mpegts',
          isLive: isLive.value,
          url: src,
        },
        {
          enableWorker: true,
          liveBufferLatencyChasing: isLive.value,
        },
      )
      mpegtsPlayer = player
      player.on(mpegts.Events.ERROR, () => {
        const curIdx = session.value?.activeStreamIndex ?? 0
        if (curIdx + 1 < streams.value.length) {
          switchSource(curIdx + 1)
        } else {
          playbackError.value = t('player.error_failed')
          isBuffering.value = false
          isPlaying.value = false
        }
      })
      player.attachMediaElement(el)
      player.load()
      void el.play().catch(() => {})
      return
    } catch {
      destroyMediaEngines()
    }
  }

  const canPlayNativeHls = el.canPlayType('application/vnd.apple.mpegurl') !== ''
  if (looksHls && Hls.isSupported() && (preferHlsJs || !canPlayNativeHls)) {
    const hls = new Hls({
      enableWorker: true,
      lowLatencyMode: isLive.value,
      startPosition: startSec > 2 ? startSec : -1,
    })
    hlsInstance = hls
    hls.on(Hls.Events.MANIFEST_PARSED, () => {
      void el.play().catch(() => {})
    })
    hls.on(Hls.Events.ERROR, (_ev, data) => {
      if (data.fatal) {
        if (data.type === Hls.ErrorTypes.NETWORK_ERROR) {
          hls.startLoad()
        } else if (data.type === Hls.ErrorTypes.MEDIA_ERROR) {
          hls.recoverMediaError()
        } else {
          destroyMediaEngines()
          const curIdx = session.value?.activeStreamIndex ?? 0
          if (curIdx + 1 < streams.value.length) {
            switchSource(curIdx + 1)
          } else {
            playbackError.value = t('player.error_failed')
            isBuffering.value = false
            isPlaying.value = false
          }
        }
      }
    })
    hls.loadSource(src)
    hls.attachMedia(el)
    return
  }

  pendingSeekSeconds.value = startSec > 2 ? startSec : null
  el.src = src
}

let attachSeq = 0

watch(
  () => activeStream.value,
  async (stream) => {
    const seq = ++attachSeq
    destroyMediaEngines()
    forceHlsJsNextTry = false
    playbackError.value = ''
    isBuffering.value = true
    currentTime.value = 0
    duration.value = 0
    playableSrc.value = ''
    if (!stream) return
    const [resolved, startSec] = await Promise.all([
      vesselApi.preparePlaybackUrl(stream).catch(() => stream.url),
      resolveDesiredStartSeconds(currentEpisode.value),
    ])
    if (seq !== attachSeq) return
    playableSrc.value = resolved
    await attachMediaSource(resolved, stream, false, startSec)
  },
  { immediate: true },
)

onMounted(async () => {
  window.addEventListener('keydown', handleKeydown)
  document.addEventListener('fullscreenchange', onFullscreenChange)
  await ensureSessionLoaded()
  if (playableSrc.value && activeStream.value && !hlsInstance && !mpegtsPlayer) {
    const startSec = await resolveDesiredStartSeconds(currentEpisode.value)
    await attachMediaSource(playableSrc.value, activeStream.value, forceHlsJsNextTry, startSec)
  }
  progressTimer = window.setInterval(() => {
    if (isPlaying.value) {
      void persistWatchProgress()
    }
  }, 5000)
  scheduleHideControls()
})

onUnmounted(() => {
  void persistWatchProgress()
  destroyMediaEngines()
  window.removeEventListener('keydown', handleKeydown)
  document.removeEventListener('fullscreenchange', onFullscreenChange)
  if (hideTimer !== null) window.clearTimeout(hideTimer)
  if (progressTimer !== null) window.clearInterval(progressTimer)
  if (subSaveTimer !== null) window.clearTimeout(subSaveTimer)
  if (gestureTimer !== null) window.clearTimeout(gestureTimer)
  if (touchSeekTimer !== null) window.clearTimeout(touchSeekTimer)
  if (document.fullscreenElement) {
    void document.exitFullscreen().catch(() => {})
  }
})
</script>

<template>
  <div
    ref="playerStageRef"
    class="cs-player"
    :class="{ 'hud-hidden': !controlsVisible && isPlaying && !activeDrawer && !isLocked }"
    @mousemove="handleStagePointerMove"
    @mousedown="scheduleHideControls"
    @wheel="handleTouchpadWheel"
  >
    <!-- Video Surface -->
    <video
      v-show="playableSrc"
      ref="videoRef"
      class="cs-video"
      :class="`fit-${fitMode}`"
      playsinline
      @click="togglePlay"
      @dblclick="toggleFullscreen"
      @play="userExplicitPause = false; isPlaying = true; isBuffering = false; scheduleHideControls()"
      @pause="isPlaying = false; scheduleHideControls(); void persistWatchProgress()"
      @ended="isPlaying = false; void persistWatchProgress(true)"
      @waiting="isBuffering = true"
      @canplay="onCanPlay"
      @seeked="onSeeked"
      @loadedmetadata="onLoadedMetadata"
      @timeupdate="onTimeUpdate"
      @error="onVideoError"
    />

    <!-- Touchpad Gesture Feedback Pill -->
    <Transition name="fade-fast">
      <div v-if="gestureLabel" class="cs-gesture-pill" aria-live="polite">
        {{ gestureLabel }}
      </div>
    </Transition>

    <!-- In-App Rendered Subtitle Overlay (SRT / VTT / ASS / Inline + Sync Delay) -->
    <div
      v-if="activeSubtitleLines.length && !isLive"
      class="cs-subtitle-overlay"
      :style="subtitleOverlayStyle"
      aria-live="off"
    >
      <span
        v-for="(line, i) in activeSubtitleLines"
        :key="i"
        class="cs-subtitle-line"
        :style="subtitleLineStyle"
      >
        {{ line }}
      </span>
    </div>

    <!-- Top & Bottom Cinematic Vignettes -->
    <div v-if="controlsVisible && !isLocked" class="cs-vignette-top" />
    <div v-if="controlsVisible && !isLocked" class="cs-vignette-bottom" />

    <!-- Buffering / Initial Loading Overlay -->
    <div v-if="loadingInitial || switchingEpisode || (isBuffering && !playbackError)" class="cs-center-state">
      <div class="cs-spinner" />
      <span class="cs-state-label">
        {{
          loadingInitial
            ? 'Loading title & resolving streams…'
            : switchingEpisode
              ? 'Loading next episode…'
              : 'Buffering stream…'
        }}
      </span>
    </div>

    <!-- Error Overlay -->
    <div v-if="playbackError" class="cs-error-overlay">
      <div class="v-card cs-error-card">
        <h3>Playback Issue</h3>
        <p>{{ playbackError }}</p>
        <div class="cs-error-actions">
          <button type="button" class="v-btn v-btn-primary" @click="retryStream">Retry</button>
          <button
            v-if="streams.length > 1"
            type="button"
            class="v-btn v-btn-secondary"
            @click="activeDrawer = 'sources'"
          >
            Switch Source ({{ streams.length }})
          </button>
          <button type="button" class="v-btn v-btn-ghost" @click="goBack">Back to Details</button>
        </div>
      </div>
    </div>

    <!-- Locked Screen Floating Unlock Pill -->
    <Transition name="fade-fast">
      <button
        v-if="isLocked"
        type="button"
        class="cs-unlock-pill"
        @click="toggleLock"
      >
        <Unlock :size="15" aria-hidden="true" />
        <span>Screen Locked — Tap to Unlock</span>
      </button>
    </Transition>

    <!-- HUD Overlay (Hidden automatically when playing & idle) -->
    <Transition name="fade-fast">
      <div v-if="controlsVisible && !isLocked" class="cs-hud">
        <!-- TOP BAR -->
        <header class="cs-top-bar">
          <div class="cs-top-left">
            <button type="button" class="cs-icon-btn" title="Back" @click="goBack">
              <ArrowLeft :size="20" aria-hidden="true" />
            </button>
            <div class="cs-title-meta">
              <div class="cs-show-title-row">
                <span v-if="isLive" class="cs-live-badge">
                  <span class="cs-live-dot" />
                  LIVE
                </span>
                <div class="cs-show-title">{{ detail?.title ?? 'Vessel Player' }}</div>
              </div>
              <div class="cs-ep-sub">
                <template v-if="isLive">
                  <span v-if="currentEpisode?.overview">{{ currentEpisode.overview }}</span>
                  <span v-else-if="detail?.genres.length">{{ detail.genres.join(' · ') }}</span>
                </template>
                <span v-else-if="currentEpisode">
                  S{{ currentEpisode.season }} · E{{ currentEpisode.episode }}
                  <template v-if="currentEpisode.title"> — {{ currentEpisode.title }}</template>
                </span>
                <span v-if="activeStream" class="cs-source-pill">
                  {{ activeStream.label }} · {{ activeStream.quality ?? activeStream.kind.toUpperCase() }}
                </span>
              </div>
            </div>
          </div>

          <div class="cs-top-actions">
            <button type="button" class="cs-pill-btn" title="Lock screen" @click="toggleLock">
              <Lock :size="15" aria-hidden="true" />
              <span>Lock</span>
            </button>
            <button
              v-if="!isLive || allEpisodes.length > 1"
              type="button"
              class="cs-pill-btn"
              :class="{ active: activeDrawer === 'episodes' }"
              @click="toggleDrawer('episodes')"
            >
              <ListVideo :size="15" aria-hidden="true" />
              <span>{{ isLive ? `Channels (${allEpisodes.length})` : 'Episodes' }}</span>
            </button>
            <button
              type="button"
              class="cs-pill-btn"
              :class="{ active: activeDrawer === 'sources' }"
              @click="toggleDrawer('sources')"
            >
              <Layers :size="15" aria-hidden="true" />
              <span>Sources ({{ streams.length }})</span>
            </button>
            <button
              v-if="!isLive"
              type="button"
              class="cs-pill-btn"
              :class="{ active: activeDrawer === 'subtitles' }"
              @click="toggleDrawer('subtitles')"
            >
              <Captions :size="15" aria-hidden="true" />
              <span>Subtitles</span>
            </button>
            <button
              type="button"
              class="cs-pill-btn"
              :class="{ active: activeDrawer === 'settings' }"
              @click="toggleDrawer('settings')"
            >
              <Settings :size="15" aria-hidden="true" />
              <span>Settings</span>
            </button>
            <button
              type="button"
              class="cs-pill-btn"
              title="Open in external mpv (if installed)"
              @click="openInExternalMpv"
            >
              <ExternalLink :size="15" aria-hidden="true" />
              <span>MPV</span>
            </button>
          </div>
        </header>

        <!-- CENTER TRANSPORT CONTROLS -->
        <div v-if="!playbackError" class="cs-center-controls">
          <button
            v-if="!isLive || allEpisodes.length > 1"
            type="button"
            class="cs-transport-btn"
            :disabled="!prevEpisode"
            :title="isLive ? 'Previous Channel' : 'Previous Episode'"
            @click="prevEpisode && switchEpisode(prevEpisode)"
          >
            <SkipBack :size="22" aria-hidden="true" />
          </button>

          <button
            v-if="!isLive"
            type="button"
            class="cs-transport-btn"
            title="Rewind 10s (J)"
            @click="seekBy(-10)"
          >
            <RotateCcw :size="24" aria-hidden="true" />
            <span class="cs-seek-tag">10</span>
          </button>

          <button type="button" class="cs-play-fab" title="Play / Pause (Space)" @click="togglePlay">
            <Pause v-if="isPlaying" :size="30" aria-hidden="true" />
            <Play v-else :size="30" class="cs-play-icon-offset" aria-hidden="true" />
          </button>

          <button
            v-if="!isLive"
            type="button"
            class="cs-transport-btn"
            title="Forward 10s (L)"
            @click="seekBy(10)"
          >
            <RotateCw :size="24" aria-hidden="true" />
            <span class="cs-seek-tag">10</span>
          </button>

          <button
            v-if="!isLive || allEpisodes.length > 1"
            type="button"
            class="cs-transport-btn"
            :disabled="!nextEpisode"
            :title="isLive ? 'Next Channel' : 'Next Episode'"
            @click="nextEpisode && switchEpisode(nextEpisode)"
          >
            <SkipForward :size="22" aria-hidden="true" />
          </button>
        </div>

        <!-- BOTTOM BAR -->
        <footer class="cs-bottom-bar">
          <!-- Seek Scrubber (VOD only) -->
          <div v-if="!isLive" class="cs-scrubber-row">
            <span class="cs-time">{{ formatClock(currentTime) }}</span>

            <div
              ref="progressBarRef"
              class="cs-progress-track"
              @click="seekFromBar"
              @mousemove="updateHoverTime"
              @mouseleave="hoverTime = null"
            >
              <div class="cs-progress-buffer" :style="{ width: `${bufferedPercent}%` }" />
              <div class="cs-progress-fill" :style="{ width: `${progressPercent}%` }">
                <span class="cs-progress-thumb" />
              </div>
              <div
                v-if="hoverTime !== null"
                class="cs-hover-tooltip"
                :style="{ left: `${hoverPercent}%` }"
              >
                {{ formatClock(hoverTime) }}
              </div>
            </div>

            <span class="cs-time">{{ formatClock(duration) }}</span>
          </div>

          <!-- Live Broadcast Indicator Bar -->
          <div v-else class="cs-live-bar">
            <span class="cs-live-badge">
              <span class="cs-live-dot" />
              LIVE BROADCAST
            </span>
            <div class="cs-live-rail" />
            <span v-if="activeStream?.quality" class="cs-live-meta">{{ activeStream.quality }}</span>
          </div>

          <!-- Bottom Actions -->
          <div class="cs-bottom-actions">
            <div class="cs-bottom-left">
              <button type="button" class="cs-icon-btn" @click="togglePlay">
                <Pause v-if="isPlaying" :size="18" aria-hidden="true" />
                <Play v-else :size="18" aria-hidden="true" />
              </button>

              <div class="cs-volume-group">
                <button type="button" class="cs-icon-btn" title="Mute (M)" @click="toggleMute">
                  <VolumeX v-if="isMuted || volume === 0" :size="18" aria-hidden="true" />
                  <Volume2 v-else :size="18" aria-hidden="true" />
                </button>
                <input
                  type="range"
                  min="0"
                  max="1"
                  step="0.02"
                  :value="isMuted ? 0 : volume"
                  class="cs-volume-slider"
                  aria-label="Volume"
                  @input="setVolumeLevel(Number(($event.target as HTMLInputElement).value))"
                />
              </div>

              <button
                v-if="!isLive"
                type="button"
                class="cs-skip-intro-btn"
                title="Skip Intro (+85s)"
                @click="seekBy(85)"
              >
                <FastForward :size="14" aria-hidden="true" />
                <span>+85s Intro</span>
              </button>
            </div>

            <div class="cs-bottom-right">
              <button
                v-if="!isLive"
                type="button"
                class="cs-mini-pill"
                @click="toggleDrawer('settings')"
              >
                {{ playbackRate }}x
              </button>
              <button type="button" class="cs-mini-pill" title="Resize / Aspect Ratio" @click="cycleFitMode">
                {{ fitLabel }}
              </button>
              <button
                v-if="nextEpisode && !isLive"
                type="button"
                class="cs-mini-pill cs-next-ep"
                @click="switchEpisode(nextEpisode)"
              >
                <span>Next E{{ nextEpisode.episode }}</span>
                <SkipForward :size="13" aria-hidden="true" />
              </button>
              <button type="button" class="cs-icon-btn" title="Fullscreen (F)" @click="toggleFullscreen">
                <Minimize v-if="isFullscreen" :size="18" aria-hidden="true" />
                <Maximize v-else :size="18" aria-hidden="true" />
              </button>
            </div>
          </div>
        </footer>
      </div>
    </Transition>

    <!-- SLIDE-OVER RIGHT DRAWER (Episodes or Channels / Sources / Subtitles / Settings) -->
    <Transition name="drawer-slide">
      <aside v-if="activeDrawer && !isLocked" class="cs-drawer" @click.stop>
        <div class="cs-drawer-head">
          <h3>
            {{
              activeDrawer === 'episodes'
                ? isLive
                  ? `Channels (${allEpisodes.length})`
                  : `Episodes (${allEpisodes.length})`
                : activeDrawer === 'sources'
                  ? `Sources (${streams.length})`
                  : activeDrawer === 'subtitles'
                    ? 'Subtitles & Sync'
                    : 'Player Settings'
            }}
          </h3>
          <button type="button" class="cs-icon-btn" @click="activeDrawer = null">
            <X :size="18" aria-hidden="true" />
          </button>
        </div>

        <!-- EPISODES OR CHANNELS LIST -->
        <div v-if="activeDrawer === 'episodes'" class="cs-drawer-body">
          <button
            v-for="ep in allEpisodes"
            :key="ep.streamId"
            type="button"
            class="cs-drawer-item"
            :class="{ active: ep.streamId === currentEpisode?.streamId }"
            @click="switchEpisode(ep)"
          >
            <div class="cs-drawer-item-row">
              <img
                v-if="ep.thumbnail || (!isLive && (detail?.backdrop || detail?.poster))"
                :src="ep.thumbnail || detail?.backdrop || detail?.poster"
                alt=""
                :class="isLive ? 'cs-channel-thumb' : 'cs-ep-thumb'"
                loading="lazy"
              />
              <div class="cs-item-main">
                <span class="cs-item-code">{{
                  isLive ? `CH ${ep.episode}` : `S${ep.season} · E${ep.episode}`
                }}</span>
                <span class="cs-item-title">{{
                  ep.title?.trim() || (isLive ? `Channel ${ep.episode}` : `Episode ${ep.episode}`)
                }}</span>
                <span v-if="ep.overview" class="cs-item-sub cs-ep-desc">{{ ep.overview }}</span>
              </div>
            </div>
            <Check
              v-if="ep.streamId === currentEpisode?.streamId"
              :size="16"
              class="cs-active-check"
              aria-hidden="true"
            />
          </button>
        </div>

        <!-- SOURCES / SERVERS LIST -->
        <div v-else-if="activeDrawer === 'sources'" class="cs-drawer-body">
          <button
            v-for="(stream, idx) in streams"
            :key="`${stream.url}-${idx}`"
            type="button"
            class="cs-drawer-item"
            :class="{ active: stream.url === activeStream?.url }"
            @click="switchSource(idx)"
          >
            <div class="cs-item-main">
              <span class="cs-item-title">{{ stream.label }}</span>
              <div class="cs-source-tags">
                <span class="cs-tag">{{ stream.kind.toUpperCase() }}</span>
                <span v-if="stream.quality" class="cs-tag accent">{{ stream.quality }}</span>
                <span v-if="stream.language" class="cs-tag">{{ stream.language }}</span>
              </div>
            </div>
            <Check
              v-if="stream.url === activeStream?.url"
              :size="16"
              class="cs-active-check"
              aria-hidden="true"
            />
          </button>
        </div>

        <!-- SUBTITLES & APPEARANCE (VOD only) -->
        <div v-else-if="activeDrawer === 'subtitles' && !isLive" class="cs-drawer-body">
          <div class="cs-setting-block">
            <span class="cs-setting-label">{{ t('player.subtitles_delay') }}</span>
            <div class="cs-delay-row">
              <button type="button" class="cs-mini-pill" @click="subtitleDelayMs -= 500">-500ms</button>
              <span class="cs-delay-val">{{ subtitleDelayMs }} ms</span>
              <button type="button" class="cs-mini-pill" @click="subtitleDelayMs += 500">+500ms</button>
            </div>
          </div>

          <div class="cs-setting-block">
            <span class="cs-setting-label">{{ t('subtitles.font_size') }} ({{ uiPreferences.subtitles.fontSizePx }}px)</span>
            <input
              v-model.number="uiPreferences.subtitles.fontSizePx"
              type="range"
              min="16"
              max="52"
              step="2"
              class="cs-range"
              @change="scheduleSaveSubtitles"
            />
          </div>

          <div class="cs-setting-block">
            <span class="cs-setting-label">{{ t('subtitles.bottom_offset') }} ({{ uiPreferences.subtitles.bottomOffsetPx }}px)</span>
            <input
              v-model.number="uiPreferences.subtitles.bottomOffsetPx"
              type="range"
              min="16"
              max="220"
              step="4"
              class="cs-range"
              @change="scheduleSaveSubtitles"
            />
          </div>

          <div class="cs-setting-block">
            <span class="cs-setting-label">{{ t('subtitles.bg_opacity') }} ({{ uiPreferences.subtitles.backgroundOpacity }}%)</span>
            <input
              v-model.number="uiPreferences.subtitles.backgroundOpacity"
              type="range"
              min="0"
              max="100"
              step="5"
              class="cs-range"
              @change="scheduleSaveSubtitles"
            />
          </div>

          <div class="cs-setting-block">
            <span class="cs-setting-label">{{ t('subtitles.font_family') }}</span>
            <div class="cs-chip-grid">
              <button
                v-for="f in ([
                  { id: 'system-ui', label: 'Sans' },
                  { id: 'rounded', label: 'Rounded' },
                  { id: 'serif', label: 'Serif' },
                  { id: 'monospace', label: 'Mono' },
                ] as const)"
                :key="f.id"
                type="button"
                class="cs-chip-btn"
                :class="{ active: uiPreferences.subtitles.fontFamily === f.id }"
                @click="uiPreferences.subtitles.fontFamily = f.id; scheduleSaveSubtitles()"
              >
                {{ f.label }}
              </button>
            </div>
          </div>

          <div class="cs-setting-block">
            <span class="cs-setting-label">{{ t('subtitles.outline_style') }}</span>
            <div class="cs-chip-grid">
              <button
                v-for="st in ([
                  { id: 'outline', label: t('subtitles.style_outline') },
                  { id: 'shadow', label: t('subtitles.style_shadow') },
                  { id: 'box', label: t('subtitles.style_box') },
                  { id: 'none', label: t('subtitles.style_none') },
                ] as const)"
                :key="st.id"
                type="button"
                class="cs-chip-btn"
                :class="{ active: uiPreferences.subtitles.outlineStyle === st.id }"
                @click="uiPreferences.subtitles.outlineStyle = st.id; scheduleSaveSubtitles()"
              >
                {{ st.label }}
              </button>
            </div>
          </div>

          <div class="cs-setting-block">
            <span class="cs-setting-label">{{ t('subtitles.text_color') }}</span>
            <div class="cs-color-row">
              <button
                v-for="clr in ['#ffffff', '#fde047', '#67e8f9', '#f9a8d4', '#a7f3d0']"
                :key="clr"
                type="button"
                class="cs-color-dot"
                :style="{ background: clr }"
                :class="{ active: uiPreferences.subtitles.textColor.toLowerCase() === clr }"
                :aria-label="clr"
                @click="uiPreferences.subtitles.textColor = clr; scheduleSaveSubtitles()"
              />
              <button
                type="button"
                class="cs-mini-pill"
                :class="{ active: uiPreferences.subtitles.bold }"
                @click="uiPreferences.subtitles.bold = !uiPreferences.subtitles.bold; scheduleSaveSubtitles()"
              >
                <strong>B</strong>
              </button>
            </div>
          </div>

          <button
            type="button"
            class="cs-drawer-item"
            :class="{ active: activeSubtitleIndex === -1 }"
            @click="activeSubtitleIndex = -1"
          >
            <span class="cs-item-title">{{ t('player.subtitles_off') }}</span>
            <Check v-if="activeSubtitleIndex === -1" :size="16" class="cs-active-check" />
          </button>

          <button
            v-for="(sub, idx) in availableSubtitles"
            :key="`${sub.label}-${idx}`"
            type="button"
            class="cs-drawer-item"
            :class="{ active: activeSubtitleIndex === idx }"
            @click="activeSubtitleIndex = idx"
          >
            <div class="cs-item-main">
              <span class="cs-item-title">{{ sub.label }}</span>
              <span class="cs-item-sub">{{ sub.lang.toUpperCase() }} · {{ sub.format.toUpperCase() }}</span>
            </div>
            <Check v-if="activeSubtitleIndex === idx" :size="16" class="cs-active-check" />
          </button>

          <p v-if="loadingSubtitle" class="cs-empty-note">Loading subtitle track…</p>
          <p v-else-if="subtitleError" class="cs-empty-note cs-sub-error">{{ subtitleError }}</p>
          <p v-else-if="activeSubtitleIndex >= 0 && activeCues.length" class="cs-empty-note">
            Loaded {{ activeCues.length }} subtitle cues.
          </p>
          <p v-if="!availableSubtitles.length" class="cs-empty-note">
            No subtitle tracks were provided by this source.
          </p>
        </div>

        <!-- SETTINGS (Speed & Aspect Ratio) -->
        <div v-else-if="activeDrawer === 'settings'" class="cs-drawer-body">
          <div v-if="!isLive" class="cs-setting-block">
            <span class="cs-setting-label">Playback Speed</span>
            <div class="cs-chip-grid">
              <button
                v-for="rate in [0.5, 0.75, 1, 1.25, 1.5, 2]"
                :key="rate"
                type="button"
                class="cs-chip-btn"
                :class="{ active: playbackRate === rate }"
                @click="setSpeed(rate)"
              >
                {{ rate }}x
              </button>
            </div>
          </div>

          <div class="cs-setting-block">
            <span class="cs-setting-label">Video Fit / Aspect</span>
            <div class="cs-chip-grid">
              <button
                v-for="mode in ([
                  { id: 'contain', label: 'Fit' },
                  { id: 'cover', label: 'Fill' },
                  { id: 'fill', label: 'Stretch' },
                ] as const)"
                :key="mode.id"
                type="button"
                class="cs-chip-btn"
                :class="{ active: fitMode === mode.id }"
                @click="fitMode = mode.id"
              >
                {{ mode.label }}
              </button>
            </div>
          </div>
        </div>
      </aside>
    </Transition>
  </div>
</template>

<style scoped>
.cs-player {
  position: fixed;
  inset: 0;
  z-index: 120;
  background: #000;
  color: #f4f4f8;
  overflow: hidden;
  user-select: none;
}

.cs-player.hud-hidden {
  cursor: none;
}

.cs-video {
  width: 100%;
  height: 100%;
  display: block;
  background: #000;
}

.cs-video.fit-contain {
  object-fit: contain;
}

.cs-video.fit-cover {
  object-fit: cover;
}

.cs-video.fit-fill {
  object-fit: fill;
}

.cs-vignette-top {
  position: absolute;
  inset: 0 0 auto 0;
  height: 160px;
  background: linear-gradient(180deg, rgba(0, 0, 0, 0.82) 0%, transparent 100%);
  pointer-events: none;
}

.cs-vignette-bottom {
  position: absolute;
  inset: auto 0 0 0;
  height: 200px;
  background: linear-gradient(0deg, rgba(0, 0, 0, 0.88) 0%, transparent 100%);
  pointer-events: none;
}

.cs-hud {
  position: absolute;
  inset: 0;
  display: flex;
  flex-direction: column;
  justify-content: space-between;
  padding: 1.15rem 1.5rem 1.25rem;
  pointer-events: none;
}

.cs-top-bar,
.cs-center-controls,
.cs-bottom-bar {
  pointer-events: auto;
}

.cs-top-bar {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 1rem;
}

.cs-top-left {
  display: flex;
  align-items: center;
  gap: 0.9rem;
  min-width: 0;
}

.cs-title-meta {
  display: flex;
  flex-direction: column;
  gap: 0.22rem;
  min-width: 0;
}

.cs-show-title {
  font-size: 1.08rem;
  font-weight: 700;
  color: #fff;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.cs-ep-sub {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 0.55rem;
  font-size: 0.82rem;
  color: rgba(244, 244, 248, 0.78);
}

.cs-source-pill {
  padding: 0.14rem 0.55rem;
  border-radius: 999px;
  font-size: 0.72rem;
  font-weight: 600;
  background: rgba(167, 139, 250, 0.2);
  border: 1px solid rgba(167, 139, 250, 0.4);
  color: #ddd6fe;
}

.cs-top-actions {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  justify-content: flex-end;
  gap: 0.45rem;
}

.cs-icon-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 38px;
  height: 38px;
  border-radius: 999px;
  border: 1px solid rgba(255, 255, 255, 0.12);
  background: rgba(18, 18, 28, 0.65);
  backdrop-filter: blur(10px);
  color: #fff;
  cursor: pointer;
  transition: background 0.16s ease, transform 0.14s ease;
}

.cs-icon-btn:hover {
  background: rgba(255, 255, 255, 0.16);
}

.cs-pill-btn {
  display: inline-flex;
  align-items: center;
  gap: 0.4rem;
  height: 36px;
  padding: 0 0.85rem;
  border-radius: 999px;
  border: 1px solid rgba(255, 255, 255, 0.14);
  background: rgba(18, 18, 28, 0.68);
  backdrop-filter: blur(10px);
  color: #f4f4f8;
  font-size: 0.8rem;
  font-weight: 600;
  cursor: pointer;
  transition: background 0.16s ease, border-color 0.16s ease;
}

.cs-pill-btn:hover,
.cs-pill-btn.active {
  background: rgba(167, 139, 250, 0.26);
  border-color: rgba(167, 139, 250, 0.65);
}

.cs-center-controls {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 1.5rem;
  margin: auto 0;
}

.cs-transport-btn {
  position: relative;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 50px;
  height: 50px;
  border-radius: 999px;
  border: 1px solid rgba(255, 255, 255, 0.14);
  background: rgba(12, 12, 20, 0.6);
  backdrop-filter: blur(10px);
  color: #fff;
  cursor: pointer;
  transition: transform 0.15s ease, background 0.15s ease;
}

.cs-transport-btn:hover:not(:disabled) {
  transform: scale(1.06);
  background: rgba(255, 255, 255, 0.18);
}

.cs-transport-btn:disabled {
  opacity: 0.35;
  cursor: not-allowed;
}

.cs-seek-tag {
  position: absolute;
  font-size: 0.62rem;
  font-weight: 700;
  bottom: 6px;
}

.cs-play-fab {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 72px;
  height: 72px;
  border-radius: 999px;
  border: none;
  background: var(--vessel-accent, #a78bfa);
  color: #0b0b12;
  cursor: pointer;
  box-shadow: 0 12px 32px rgba(0, 0, 0, 0.55);
  transition: transform 0.15s ease, filter 0.15s ease;
}

.cs-play-fab:hover {
  transform: scale(1.07);
  filter: brightness(1.06);
}

.cs-play-icon-offset {
  margin-left: 3px;
}

.cs-bottom-bar {
  display: flex;
  flex-direction: column;
  gap: 0.7rem;
}

.cs-scrubber-row {
  display: flex;
  align-items: center;
  gap: 0.8rem;
}

.cs-time {
  min-width: 48px;
  font-size: 0.8rem;
  font-variant-numeric: tabular-nums;
  color: rgba(255, 255, 255, 0.88);
}

.cs-progress-track {
  position: relative;
  flex: 1;
  height: 6px;
  border-radius: 999px;
  background: rgba(255, 255, 255, 0.22);
  cursor: pointer;
  transition: height 0.12s ease;
}

.cs-progress-track:hover {
  height: 9px;
}

.cs-progress-buffer {
  position: absolute;
  inset: 0 auto 0 0;
  border-radius: 999px;
  background: rgba(255, 255, 255, 0.32);
}

.cs-progress-fill {
  position: relative;
  height: 100%;
  border-radius: 999px;
  background: var(--vessel-accent, #a78bfa);
}

.cs-progress-thumb {
  position: absolute;
  right: -7px;
  top: 50%;
  transform: translateY(-50%);
  width: 14px;
  height: 14px;
  border-radius: 999px;
  background: #fff;
  box-shadow: 0 0 8px rgba(0, 0, 0, 0.6);
}

.cs-hover-tooltip {
  position: absolute;
  bottom: 16px;
  transform: translateX(-50%);
  padding: 0.2rem 0.5rem;
  border-radius: 6px;
  background: rgba(15, 15, 24, 0.92);
  border: 1px solid rgba(255, 255, 255, 0.15);
  font-size: 0.74rem;
  font-variant-numeric: tabular-nums;
  pointer-events: none;
}

.cs-bottom-actions {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 1rem;
}

.cs-bottom-left,
.cs-bottom-right {
  display: flex;
  align-items: center;
  gap: 0.55rem;
}

.cs-volume-group {
  display: flex;
  align-items: center;
  gap: 0.4rem;
}

.cs-volume-slider {
  width: 84px;
  accent-color: var(--vessel-accent, #a78bfa);
  cursor: pointer;
}

.cs-skip-intro-btn,
.cs-mini-pill {
  display: inline-flex;
  align-items: center;
  gap: 0.35rem;
  height: 32px;
  padding: 0 0.72rem;
  border-radius: 999px;
  border: 1px solid rgba(255, 255, 255, 0.15);
  background: rgba(18, 18, 28, 0.68);
  color: #f4f4f8;
  font-size: 0.78rem;
  font-weight: 600;
  cursor: pointer;
}

.cs-skip-intro-btn:hover,
.cs-mini-pill:hover {
  background: rgba(255, 255, 255, 0.16);
}

.cs-next-ep {
  background: rgba(167, 139, 250, 0.22);
  border-color: rgba(167, 139, 250, 0.45);
}

/* Right Drawer */
.cs-drawer {
  position: absolute;
  top: 1rem;
  right: 1rem;
  bottom: 1rem;
  width: min(370px, calc(100vw - 2rem));
  display: flex;
  flex-direction: column;
  border-radius: 18px;
  background: rgba(16, 16, 26, 0.94);
  backdrop-filter: blur(18px);
  border: 1px solid rgba(255, 255, 255, 0.12);
  box-shadow: 0 24px 60px rgba(0, 0, 0, 0.75);
  z-index: 15;
  overflow: hidden;
}

.cs-drawer-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 1rem 1.1rem;
  border-bottom: 1px solid rgba(255, 255, 255, 0.1);
}

.cs-drawer-head h3 {
  margin: 0;
  font-size: 1rem;
  font-weight: 700;
}

.cs-drawer-body {
  flex: 1;
  overflow-y: auto;
  padding: 0.85rem;
  display: flex;
  flex-direction: column;
  gap: 0.55rem;
}

.cs-drawer-item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.75rem;
  width: 100%;
  text-align: left;
  padding: 0.75rem 0.85rem;
  border-radius: 12px;
  border: 1px solid rgba(255, 255, 255, 0.08);
  background: rgba(255, 255, 255, 0.04);
  color: #f4f4f8;
  cursor: pointer;
  transition: background 0.14s ease, border-color 0.14s ease;
}

.cs-drawer-item:hover {
  background: rgba(255, 255, 255, 0.09);
}

.cs-drawer-item.active {
  background: rgba(167, 139, 250, 0.18);
  border-color: rgba(167, 139, 250, 0.55);
}

.cs-item-main {
  display: flex;
  flex-direction: column;
  gap: 0.22rem;
  min-width: 0;
}

.cs-item-code {
  font-size: 0.74rem;
  font-weight: 700;
  color: #c4b5fd;
}

.cs-item-title {
  font-size: 0.88rem;
  font-weight: 600;
}

.cs-item-sub {
  font-size: 0.76rem;
  color: rgba(244, 244, 248, 0.65);
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
}

.cs-source-tags {
  display: flex;
  flex-wrap: wrap;
  gap: 0.35rem;
  margin-top: 0.15rem;
}

.cs-tag {
  font-size: 0.68rem;
  font-weight: 700;
  padding: 0.12rem 0.45rem;
  border-radius: 6px;
  background: rgba(255, 255, 255, 0.1);
}

.cs-tag.accent {
  background: rgba(167, 139, 250, 0.26);
  color: #e9d5ff;
}

.cs-active-check {
  color: #c4b5fd;
  flex-shrink: 0;
}

.cs-setting-block {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
  padding: 0.7rem 0.75rem;
  border-radius: 12px;
  background: rgba(255, 255, 255, 0.04);
  border: 1px solid rgba(255, 255, 255, 0.08);
}

.cs-setting-label {
  font-size: 0.78rem;
  font-weight: 700;
  color: rgba(244, 244, 248, 0.78);
}

.cs-chip-grid {
  display: flex;
  flex-wrap: wrap;
  gap: 0.4rem;
}

.cs-chip-btn {
  padding: 0.38rem 0.75rem;
  border-radius: 999px;
  border: 1px solid rgba(255, 255, 255, 0.14);
  background: rgba(255, 255, 255, 0.06);
  color: #fff;
  font-size: 0.78rem;
  font-weight: 600;
  cursor: pointer;
}

.cs-chip-btn.active {
  background: var(--vessel-accent, #a78bfa);
  color: #0b0b12;
  border-color: transparent;
}

.cs-delay-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.cs-delay-val {
  font-size: 0.84rem;
  font-weight: 700;
  font-variant-numeric: tabular-nums;
}

.cs-empty-note {
  font-size: 0.82rem;
  color: rgba(244, 244, 248, 0.6);
  padding: 0.5rem;
}

.cs-center-state,
.cs-error-overlay {
  position: absolute;
  inset: 0;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 0.85rem;
  pointer-events: none;
}

.cs-error-overlay {
  background: rgba(5, 5, 10, 0.85);
  pointer-events: auto;
  z-index: 10;
}

.cs-error-card {
  max-width: 440px;
  padding: 1.5rem;
  display: flex;
  flex-direction: column;
  gap: 0.85rem;
}

.cs-error-card h3 {
  margin: 0;
}

.cs-error-card p {
  margin: 0;
  font-size: 0.88rem;
  color: var(--vessel-text-secondary);
}

.cs-error-actions {
  display: flex;
  flex-wrap: wrap;
  gap: 0.6rem;
}

.cs-spinner {
  width: 42px;
  height: 42px;
  border-radius: 999px;
  border: 3px solid rgba(255, 255, 255, 0.18);
  border-top-color: var(--vessel-accent, #a78bfa);
  animation: spin 0.8s linear infinite;
}

.cs-state-label {
  font-size: 0.85rem;
  color: rgba(255, 255, 255, 0.85);
}

.cs-unlock-pill {
  position: absolute;
  top: 1.2rem;
  right: 1.5rem;
  z-index: 20;
  display: inline-flex;
  align-items: center;
  gap: 0.45rem;
  padding: 0.5rem 0.95rem;
  border-radius: 999px;
  border: 1px solid rgba(167, 139, 250, 0.55);
  background: rgba(16, 16, 26, 0.88);
  color: #fff;
  font-size: 0.8rem;
  font-weight: 600;
  cursor: pointer;
}

@keyframes spin {
  to {
    transform: rotate(360deg);
  }
}

.fade-fast-enter-active,
.fade-fast-leave-active {
  transition: opacity 0.18s ease;
}
.fade-fast-enter-from,
.fade-fast-leave-to {
  opacity: 0;
}

.drawer-slide-enter-active,
.drawer-slide-leave-active {
  transition: transform 0.22s ease, opacity 0.22s ease;
}
.drawer-slide-enter-from,
.drawer-slide-leave-to {
  transform: translateX(24px);
  opacity: 0;
}

.cs-show-title-row {
  display: flex;
  align-items: center;
  gap: 0.55rem;
}

.cs-live-badge {
  display: inline-flex;
  align-items: center;
  gap: 0.35rem;
  padding: 0.16rem 0.55rem;
  border-radius: 999px;
  background: rgba(239, 68, 68, 0.22);
  border: 1px solid rgba(248, 113, 113, 0.55);
  color: #fecaca;
  font-size: 0.68rem;
  font-weight: 700;
  letter-spacing: 0.08em;
  flex-shrink: 0;
}

.cs-live-dot {
  width: 7px;
  height: 7px;
  border-radius: 999px;
  background: #ef4444;
  box-shadow: 0 0 8px rgba(239, 68, 68, 0.9);
  animation: livePulse 1.4s ease-in-out infinite;
}

.cs-live-bar {
  display: flex;
  align-items: center;
  gap: 0.75rem;
}

.cs-live-rail {
  flex: 1;
  height: 4px;
  border-radius: 999px;
  background: linear-gradient(90deg, rgba(239, 68, 68, 0.85), rgba(167, 139, 250, 0.5));
}

.cs-live-meta {
  font-size: 0.75rem;
  font-weight: 600;
  color: rgba(255, 255, 255, 0.82);
}

.cs-subtitle-overlay {
  position: absolute;
  left: 50%;
  bottom: 2.4rem;
  transform: translateX(-50%);
  z-index: 6;
  max-width: min(82%, 820px);
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 0.25rem;
  text-align: center;
  pointer-events: none;
  transition: bottom 0.2s ease;
}

.cs-subtitle-overlay.controls-raised {
  bottom: 7.4rem;
}

.cs-subtitle-line {
  display: inline-block;
  padding: 0.22rem 0.65rem;
  border-radius: 6px;
  background: rgba(8, 9, 14, 0.72);
  color: #ffffff;
  font-size: clamp(1rem, 2.1vw, 1.38rem);
  font-weight: 600;
  line-height: 1.35;
  text-shadow:
    0 1px 3px rgba(0, 0, 0, 0.95),
    0 2px 10px rgba(0, 0, 0, 0.85);
}

.cs-sub-error {
  color: #fda4af;
}

.cs-drawer-item-row {
  display: flex;
  align-items: center;
  gap: 0.65rem;
  min-width: 0;
  flex: 1;
}

.cs-channel-thumb {
  width: 32px;
  height: 32px;
  border-radius: 7px;
  object-fit: contain;
  background: rgba(10, 11, 18, 0.75);
  border: 1px solid rgba(255, 255, 255, 0.1);
  padding: 2px;
  flex-shrink: 0;
}

.cs-ep-thumb {
  width: 88px;
  aspect-ratio: 16 / 9;
  border-radius: 7px;
  object-fit: cover;
  background: rgba(10, 11, 18, 0.75);
  border: 1px solid rgba(255, 255, 255, 0.12);
  flex-shrink: 0;
}

.cs-ep-desc {
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
  white-space: normal;
  line-height: 1.35;
}

.cs-gesture-pill {
  position: absolute;
  top: 4.5rem;
  left: 50%;
  transform: translateX(-50%);
  z-index: 25;
  padding: 0.45rem 1.05rem;
  border-radius: 999px;
  background: rgba(10, 11, 18, 0.84);
  border: 1px solid rgba(255, 255, 255, 0.18);
  color: #ffffff;
  font-size: 0.88rem;
  font-weight: 700;
  pointer-events: none;
  backdrop-filter: blur(12px);
}

@keyframes livePulse {
  0%,
  100% {
    opacity: 1;
    transform: scale(1);
  }
  50% {
    opacity: 0.45;
    transform: scale(0.8);
  }
}

.cs-range {
  width: 100%;
  accent-color: var(--vessel-accent, #cba6f7);
  cursor: pointer;
}

.cs-color-row {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  flex-wrap: wrap;
}

.cs-color-dot {
  width: 24px;
  height: 24px;
  border-radius: 999px;
  border: 2px solid rgba(255, 255, 255, 0.28);
  cursor: pointer;
  transition: transform 0.16s ease, border-color 0.16s ease;
}

.cs-color-dot.active {
  transform: scale(1.16);
  border-color: var(--vessel-accent, #cba6f7);
  box-shadow: 0 0 0 2px rgba(203, 166, 247, 0.35);
}
</style>
