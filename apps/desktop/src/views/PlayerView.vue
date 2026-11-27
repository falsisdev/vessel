<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, ref, shallowRef, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import Hls from 'hls.js'
import mpegts from 'mpegts.js'
import {
  Activity,
  ArrowLeft,
  AudioLines,
  Captions,
  Check,
  ExternalLink,
  FastForward,
  Info,
  Layers,
  ListVideo,
  Loader2,
  Lock,
  Maximize,
  Minimize,
  Pause,
  Play,
  RotateCcw,
  RotateCw,
  Search,
  Settings,
  SkipBack,
  SkipForward,
  Unlock,
  Volume2,
  VolumeX,
  X,
} from 'lucide-vue-next'
import { vesselApi } from '@/api/vessel'
import { formatBytes } from '@/composables/useAppUpdater'
import { useI18n } from '@/composables/useI18n'
import { usePlayerSession } from '@/composables/usePlayerSession'
import { useUiPreferences } from '@/composables/useUiPreferences'
import { useVesselEvents } from '@/composables/useVesselEvents'
import { parseSubtitles, type SubtitleCue } from '@/utils/subtitleParser'
import { isLiveMediaIdentifier } from '@/utils/liveTv'
import type {
  EmbeddedPlayerState,
  EmbeddedTrackInfo,
  Episode,
  PlaybackStats,
  PluginListEntry,
  SkipSegment,
  StreamCandidate,
  SubtitleProviderError,
  SubtitleSearchHit,
  SubtitleTrack,
} from '@/types/vessel'

const props = defineProps<{
  pluginId: string
  nativeId: string
}>()

const route = useRoute()
const router = useRouter()
const { t, currentLang } = useI18n()
const { pushToast } = useVesselEvents()
const { uiPreferences, savePreferences } = useUiPreferences()
const { activeSession, setSession, updateEpisodeSession, setActiveStreamIndex } = usePlayerSession()

type DrawerTab = 'episodes' | 'sources' | 'audio' | 'subtitles' | 'settings' | null
type FitMode = 'contain' | 'cover' | 'fill'

interface AudioTrackOption {
  id: number
  label: string
  lang?: string
  meta?: string
  active: boolean
  source: 'hls' | 'native' | 'embedded'
}

interface MuxedSubtitleOption {
  id: number
  label: string
  lang?: string
  active: boolean
  source: 'hls' | 'native'
}

interface NativeAudioTrack {
  id?: string
  kind?: string
  label?: string
  language?: string
  enabled: boolean
}

interface NativeAudioTrackList extends EventTarget {
  length: number
  [index: number]: NativeAudioTrack
}

const playerStageRef = ref<HTMLElement | null>(null)
const videoRef = ref<HTMLVideoElement | null>(null)
const progressBarRef = ref<HTMLElement | null>(null)

const loadingInitial = ref(false)
const switchingEpisode = ref(false)
const playbackError = ref('')
const externalPlayerActive = ref(false)
const embeddedMpvCapable = ref(false)
const isEmbeddedMpv = ref(false)
const playerEngineMode = ref<'auto' | 'embedded_mpv' | 'html5'>('auto')
const embeddedAudioTracks = ref<EmbeddedTrackInfo[]>([])
const embeddedSubtitleTracks = ref<EmbeddedTrackInfo[]>([])
const activeEmbeddedSubId = ref<number | null>(null)

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
const subtitleTranscriptOpen = ref(false)
const subtitleTranscriptQuery = ref('')
const transcriptListRef = ref<HTMLElement | null>(null)
const activeCues = shallowRef<SubtitleCue[]>([])
const loadingSubtitle = shallowRef(false)
const subtitleError = shallowRef('')
const extraExternalSubtitles = ref<SubtitleTrack[]>([])
const extSubOpen = ref(false)
const extSubQueryText = ref('')
const extSubLangMode = ref<'preferred' | 'tr' | 'en' | 'all'>('preferred')
const extSubSourceFilter = ref<string>('all')
const extSubSearching = ref(false)
const extSubResolvingKey = ref('')
const extSubHits = ref<SubtitleSearchHit[]>([])
const extSubErrors = ref<SubtitleProviderError[]>([])
const extSubSearchedOnce = ref(false)
const pendingSeekSeconds = ref<number | null>(null)
const playableSrc = ref<string>('')
const gestureLabel = ref<string>('')
const holdSpeedBoostActive = ref(false)
const audioTracks = ref<AudioTrackOption[]>([])
const muxedWebSubtitleTracks = ref<MuxedSubtitleOption[]>([])
const activeMuxedWebSubKey = ref<string | null>(null)
const skipSegments = ref<SkipSegment[]>([])
const dismissedSkipKeys = ref<Set<string>>(new Set())
const loadedPlugins = ref<PluginListEntry[]>([])

let skipFetchSeq = 0
let lastFetchedSkipDurationMs = 0

let hideTimer: number | null = null
let progressTimer: number | null = null
let subSaveTimer: number | null = null
let gestureTimer: number | null = null
let holdSpeedTimer: number | null = null
let holdSpeedOriginX = 0
let holdSpeedOriginY = 0
let suppressNextStageClick = false
let touchSeekTimer: number | null = null
let touchSeekDelta = 0
let lastPinchAt = 0
let hlsInstance: Hls | null = null
let mpegtsPlayer: mpegts.Player | null = null
let forceNativeHlsNextTry = false
let lastTargetStartSec = 0
let isResettingMedia = false
let userExplicitPause = false
let triedAutoMpv = false
let loadWatchdogTimer: number | null = null
let unlistenEmbedded: UnlistenFn | null = null
let boundAudioTrackList: NativeAudioTrackList | null = null
let boundTextTrackList: TextTrackList | null = null
let lastEmbeddedTracksSig = ''

function trackListSignature(tracks: EmbeddedTrackInfo[]): string {
  return tracks.map((t) => `${t.id}:${t.selected ? 1 : 0}:${t.title ?? ''}:${t.lang ?? ''}`).join('|')
}

function setEmbeddedSurfaceClass(active: boolean): void {
  document.documentElement.classList.toggle('embedded-mpv-active', active)
}

async function stopEmbeddedMpv(destroy = false): Promise<void> {
  const wasActive = isEmbeddedMpv.value
  isEmbeddedMpv.value = false
  setEmbeddedSurfaceClass(false)
  embeddedAudioTracks.value = []
  embeddedSubtitleTracks.value = []
  activeEmbeddedSubId.value = null
  if (wasActive || destroy) {
    await vesselApi.embeddedMpvStop(destroy).catch(() => {})
  }
}

let embeddedCapsPromise: Promise<boolean> | null = null

async function ensureEmbeddedCapabilities(): Promise<boolean> {
  if (embeddedMpvCapable.value) return true
  if (!embeddedCapsPromise) {
    embeddedCapsPromise = vesselApi
      .embeddedMpvCapabilities()
      .then((caps) => {
        embeddedMpvCapable.value = caps.available
        return caps.available
      })
      .catch(() => false)
  }
  return embeddedCapsPromise
}

async function launchEmbeddedMpv(stream: StreamCandidate, startSec = 0): Promise<boolean> {
  const capable = await ensureEmbeddedCapabilities()
  if (!capable) return false
  destroyMediaEngines()
  playbackError.value = ''
  externalPlayerActive.value = false
  isBuffering.value = true
  isEmbeddedMpv.value = true
  setEmbeddedSurfaceClass(true)
  try {
    await vesselApi.embeddedMpvLoad({
      candidate: stream,
      startPositionMs: isLive.value ? 0 : Math.floor(Math.max(0, startSec) * 1000),
    })
    void vesselApi.embeddedMpvSetVolume(isMuted.value ? 0 : volume.value).catch(() => {})
    void vesselApi.embeddedMpvSetSpeed(isLive.value ? 1 : playbackRate.value).catch(() => {})
    void vesselApi.embeddedMpvSetFitMode(fitMode.value).catch(() => {})
    return true
  } catch {
    isEmbeddedMpv.value = false
    setEmbeddedSurfaceClass(false)
    return false
  }
}

function clearLoadWatchdog(): void {
  if (loadWatchdogTimer !== null) {
    window.clearTimeout(loadWatchdogTimer)
    loadWatchdogTimer = null
  }
}

function isLikelyUnsupportedInWebView(
  el: HTMLVideoElement,
  stream: StreamCandidate,
  src: string,
): boolean {
  const pathWithoutQuery = src.split(/[?#]/)[0]?.toLowerCase() ?? ''
  const meta = `${stream.label ?? ''} ${stream.quality ?? ''}`.toLowerCase()
  const isMatroska =
    stream.kind === 'mkv' ||
    pathWithoutQuery.endsWith('.mkv') ||
    /\bmkv\b/.test(meta)
  if (isMatroska && el.canPlayType('video/x-matroska') === '') {
    return true
  }
  if (/\.(?:avi|flv|wmv)$/.test(pathWithoutQuery)) {
    return true
  }
  if (/\b(?:truehd|dts(?:-hd)?|ac-?3|e-?ac-?3|ddp)\b/.test(meta)) {
    return true
  }
  if (
    /\b(?:h\.?265|hevc)\b/.test(meta) &&
    el.canPlayType('video/mp4; codecs="hvc1.1.6.L93.B0"') === ''
  ) {
    return true
  }
  return false
}

const handleNativeAudioTrackChange = (): void => {
  syncAudioTracks()
}

const handleNativeTextTrackChange = (): void => {
  syncMuxedWebSubtitles()
}

function unbindNativeAudioListeners(): void {
  if (boundAudioTrackList) {
    boundAudioTrackList.removeEventListener('addtrack', handleNativeAudioTrackChange)
    boundAudioTrackList.removeEventListener('removetrack', handleNativeAudioTrackChange)
    boundAudioTrackList.removeEventListener('change', handleNativeAudioTrackChange)
    boundAudioTrackList = null
  }
  if (boundTextTrackList) {
    boundTextTrackList.removeEventListener('addtrack', handleNativeTextTrackChange)
    boundTextTrackList.removeEventListener('removetrack', handleNativeTextTrackChange)
    boundTextTrackList.removeEventListener('change', handleNativeTextTrackChange)
    boundTextTrackList = null
  }
}

function bindNativeAudioListeners(el: HTMLVideoElement): void {
  unbindNativeAudioListeners()
  const list = (el as HTMLVideoElement & { audioTracks?: NativeAudioTrackList }).audioTracks
  if (list && typeof list.addEventListener === 'function') {
    boundAudioTrackList = list
    list.addEventListener('addtrack', handleNativeAudioTrackChange)
    list.addEventListener('removetrack', handleNativeAudioTrackChange)
    list.addEventListener('change', handleNativeAudioTrackChange)
  }
  const textList = el.textTracks
  if (textList && typeof textList.addEventListener === 'function') {
    boundTextTrackList = textList
    textList.addEventListener('addtrack', handleNativeTextTrackChange)
    textList.addEventListener('removetrack', handleNativeTextTrackChange)
    textList.addEventListener('change', handleNativeTextTrackChange)
  }
}

function syncMuxedWebSubtitles(): void {
  if (isEmbeddedMpv.value) {
    muxedWebSubtitleTracks.value = []
    activeMuxedWebSubKey.value = null
    return
  }

  if (hlsInstance) {
    const raw = hlsInstance.subtitleTracks ?? []
    const currentId = hlsInstance.subtitleTrack
    muxedWebSubtitleTracks.value = raw.map((trk, idx) => {
      const id = trk.id ?? idx
      const lang = trk.lang?.trim() || undefined
      const active = id === currentId && currentId >= 0
      return {
        id,
        label: trk.name?.trim() || lang?.toUpperCase() || `Subtitle ${idx + 1}`,
        lang,
        active,
        source: 'hls',
      }
    })
    const activeTrk = muxedWebSubtitleTracks.value.find((t) => t.active)
    activeMuxedWebSubKey.value = activeTrk ? `hls:${activeTrk.id}` : null
    return
  }

  const textList = videoRef.value?.textTracks
  if (textList && textList.length > 0) {
    const items: MuxedSubtitleOption[] = []
    for (let i = 0; i < textList.length; i += 1) {
      const trk = textList[i]
      if (!trk || (trk.kind !== 'subtitles' && trk.kind !== 'captions')) continue
      const lang = trk.language?.trim() || undefined
      items.push({
        id: i,
        label: trk.label?.trim() || lang?.toUpperCase() || `Subtitle ${items.length + 1}`,
        lang,
        active: trk.mode === 'showing',
        source: 'native',
      })
    }
    muxedWebSubtitleTracks.value = items
    const activeTrk = items.find((t) => t.active)
    activeMuxedWebSubKey.value = activeTrk ? `native:${activeTrk.id}` : null
    return
  }

  muxedWebSubtitleTracks.value = []
  activeMuxedWebSubKey.value = null
}

function selectMuxedWebSubtitle(option: MuxedSubtitleOption | null): void {
  if (option !== null) {
    activeSubtitleIndex.value = -1
    selectEmbeddedSubtitle(null)
  }
  if (hlsInstance) {
    if (!option || option.source !== 'hls') {
      hlsInstance.subtitleDisplay = false
      hlsInstance.subtitleTrack = -1
    } else {
      hlsInstance.subtitleDisplay = true
      hlsInstance.subtitleTrack = option.id
    }
    syncMuxedWebSubtitles()
    return
  }

  const textList = videoRef.value?.textTracks
  if (textList) {
    for (let i = 0; i < textList.length; i += 1) {
      const trk = textList[i]
      if (!trk) continue
      trk.mode = option && option.source === 'native' && i === option.id ? 'showing' : 'disabled'
    }
  }
  syncMuxedWebSubtitles()
}

function syncAudioTracks(): void {
  if (isEmbeddedMpv.value) {
    audioTracks.value = embeddedAudioTracks.value.map((trk, idx) => {
      const lang = trk.lang?.trim() || undefined
      const metaParts = [lang?.toUpperCase(), trk.channels, trk.codec?.toUpperCase()].filter(
        (v): v is string => Boolean(v),
      )
      return {
        id: trk.id,
        label: trk.title?.trim() || lang?.toUpperCase() || `Track ${idx + 1}`,
        lang,
        meta: metaParts.join(' · ') || undefined,
        active: trk.selected,
        source: 'embedded',
      }
    })
    return
  }

  if (hlsInstance) {
    const raw = hlsInstance.audioTracks ?? []
    const currentId = hlsInstance.audioTrack
    audioTracks.value = raw.map((trk, idx) => {
      const lang = trk.lang?.trim() || undefined
      const channels =
        typeof trk.attrs?.CHANNELS === 'string' && trk.attrs.CHANNELS.trim()
          ? `${trk.attrs.CHANNELS.trim()}ch`
          : undefined
      const metaParts = [lang?.toUpperCase(), channels, trk.audioCodec?.trim()].filter(
        (v): v is string => Boolean(v),
      )
      return {
        id: trk.id ?? idx,
        label: trk.name?.trim() || lang?.toUpperCase() || `Track ${idx + 1}`,
        lang,
        meta: metaParts.join(' · ') || undefined,
        active: (trk.id ?? idx) === currentId,
        source: 'hls',
      }
    })
    return
  }

  const el = videoRef.value as (HTMLVideoElement & { audioTracks?: NativeAudioTrackList }) | null
  const nativeList = el?.audioTracks
  if (nativeList && nativeList.length > 0) {
    const items: AudioTrackOption[] = []
    for (let i = 0; i < nativeList.length; i += 1) {
      const trk = nativeList[i]
      if (!trk) continue
      const lang = trk.language?.trim() || undefined
      const kind = trk.kind?.trim() && trk.kind !== 'main' ? trk.kind.trim() : undefined
      const metaParts = [lang?.toUpperCase(), kind].filter((v): v is string => Boolean(v))
      items.push({
        id: i,
        label: trk.label?.trim() || lang?.toUpperCase() || `Track ${i + 1}`,
        lang,
        meta: metaParts.join(' · ') || undefined,
        active: trk.enabled,
        source: 'native',
      })
    }
    audioTracks.value = items
    return
  }

  audioTracks.value = []
}

function switchAudioTrack(option: AudioTrackOption): void {
  if (option.source === 'embedded') {
    void vesselApi.embeddedMpvSelectAudioTrack(option.id).catch(() => {})
    embeddedAudioTracks.value = embeddedAudioTracks.value.map((t) => ({
      ...t,
      selected: t.id === option.id,
    }))
    syncAudioTracks()
    showGestureHud(`${t('player.audio')}: ${option.label}`)
    scheduleHideControls()
    return
  }

  if (option.source === 'hls' && hlsInstance) {
    hlsInstance.audioTrack = option.id
    syncAudioTracks()
    showGestureHud(`${t('player.audio')}: ${option.label}`)
    scheduleHideControls()
    return
  }

  if (option.source === 'native') {
    const el = videoRef.value as (HTMLVideoElement & { audioTracks?: NativeAudioTrackList }) | null
    const nativeList = el?.audioTracks
    if (!nativeList) return
    for (let i = 0; i < nativeList.length; i += 1) {
      const trk = nativeList[i]
      if (trk) trk.enabled = i === option.id
    }
    syncAudioTracks()
    showGestureHud(`${t('player.audio')}: ${option.label}`)
    scheduleHideControls()
  }
}

function destroyMediaEngines(): void {
  clearLoadWatchdog()
  unbindNativeAudioListeners()
  audioTracks.value = []
  muxedWebSubtitleTracks.value = []
  activeMuxedWebSubKey.value = null
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
  const el = videoRef.value
  if (el) {
    isResettingMedia = true
    try {
      el.pause()
      el.removeAttribute('src')
      el.load()
    } catch {
      // Ignore element cleanup errors
    } finally {
      window.setTimeout(() => {
        isResettingMedia = false
      }, 0)
    }
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
  if (!ep || !d || (!el && !isEmbeddedMpv.value)) return
  const posSec = isEmbeddedMpv.value
    ? currentTime.value
    : el && Number.isFinite(el.currentTime)
      ? el.currentTime
      : currentTime.value
  const durSec = isEmbeddedMpv.value
    ? duration.value
    : el && Number.isFinite(el.duration)
      ? el.duration
      : duration.value
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

  const scrollSign = uiPreferences.value.playback.naturalTouchpadScroll ? -1 : 1
  const dx = ev.deltaX * scrollSign
  const dy = ev.deltaY * scrollSign

  const absX = Math.abs(dx)
  const absY = Math.abs(dy)
  if (absX > absY && absX > 2 && !isLive.value && duration.value > 0) {
    ev.preventDefault()
    touchSeekDelta += dx * 0.14
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
    const nextVol = Math.min(1, Math.max(0, volume.value - dy * 0.0018))
    setVolumeLevel(nextVol)
    showGestureHud(`Volume ${Math.round(nextVol * 100)}%`)
  }
}

const session = computed(() => activeSession.value)
const detail = computed(() => session.value?.detail ?? null)
const currentEpisode = computed(() => session.value?.episode ?? null)
const isLive = computed(() => {
  const activePlugin = loadedPlugins.value.find((p) => p.manifest.id === props.pluginId) ?? null
  return isLiveMediaIdentifier(
    detail.value?.type,
    props.pluginId,
    currentEpisode.value?.streamId,
    props.nativeId,
    activePlugin,
  )
})
watch(isLive, (live) => {
  if (live && (activeDrawer.value === 'audio' || activeDrawer.value === 'subtitles')) {
    activeDrawer.value = null
  }
})
const streams = computed(() => session.value?.streams ?? [])
const activeStream = computed<StreamCandidate | null>(() => {
  if (!session.value) return null
  return session.value.streams[session.value.activeStreamIndex] ?? session.value.streams[0] ?? null
})

const episodesListRef = ref<HTMLElement | null>(null)

watch(
  activeDrawer,
  async (nextDrawer) => {
    if (nextDrawer !== 'subtitles') {
      subtitleTranscriptOpen.value = false
      subtitleTranscriptQuery.value = ''
    }
    if (nextDrawer === 'episodes') {
      await nextTick()
      const activeEl = episodesListRef.value?.querySelector('.cs-drawer-item.active')
      if (activeEl instanceof HTMLElement) {
        activeEl.scrollIntoView({ block: 'center', behavior: 'auto' })
        activeEl.focus()
      }
    }
  },
)

const filteredTranscriptCues = computed<SubtitleCue[]>(() => {
  const q = subtitleTranscriptQuery.value.trim().toLowerCase()
  if (!q) return activeCues.value
  return activeCues.value.filter((cue) => cue.lines.some((line) => line.toLowerCase().includes(q)))
})

const nearestTranscriptIndex = computed<number>(() => {
  const list = filteredTranscriptCues.value
  if (!list.length) return -1
  const targetMs = currentTime.value * 1000 - subtitleDelayMs.value
  const activeIdx = list.findIndex((cue) => targetMs >= cue.startMs && targetMs <= cue.endMs)
  if (activeIdx >= 0) return activeIdx
  const upcoming = list.findIndex((cue) => cue.startMs >= targetMs)
  return upcoming >= 0 ? upcoming : list.length - 1
})

async function openSubtitleTranscript(): Promise<void> {
  if (!activeCues.value.length) return
  subtitleTranscriptOpen.value = true
  await nextTick()
  const activeEl = transcriptListRef.value?.querySelector('.cs-transcript-item.active')
  if (activeEl instanceof HTMLElement) {
    activeEl.scrollIntoView({ block: 'center', behavior: 'auto' })
  }
}

function applyTranscriptSync(cue: SubtitleCue): void {
  const nextDelay = Math.round(currentTime.value * 1000 - cue.startMs)
  subtitleDelayMs.value = nextDelay
  subtitleTranscriptOpen.value = false
  subtitleTranscriptQuery.value = ''
  showGestureHud(t('player.sync_transcript_applied', { delay: nextDelay }))
  scheduleHideControls()
}

function closeDrawerOrTranscript(): void {
  if (activeDrawer.value === 'subtitles' && subtitleTranscriptOpen.value) {
    subtitleTranscriptOpen.value = false
    subtitleTranscriptQuery.value = ''
  } else {
    activeDrawer.value = null
  }
}

const availableSubtitles = computed<SubtitleTrack[]>(() => {
  const fromStream = activeStream.value?.subtitles ?? []
  const fromEpisode = session.value?.subtitles ?? []
  return [...fromStream, ...fromEpisode, ...extraExternalSubtitles.value]
})

const defaultPreferredSubLangs = computed<string[]>(() => {
  const appLang = (currentLang.value || 'en').toLowerCase().split('-')[0]
  return appLang === 'en' ? ['en'] : [appLang, 'en']
})

const extSubSources = computed<string[]>(() => {
  const set = new Set<string>()
  for (const hit of extSubHits.value) {
    set.add(hit.candidate.sourceName || hit.pluginName)
  }
  return Array.from(set)
})

const filteredExtSubHits = computed<SubtitleSearchHit[]>(() => {
  if (extSubSourceFilter.value === 'all') return extSubHits.value
  return extSubHits.value.filter(
    (h) => (h.candidate.sourceName || h.pluginName) === extSubSourceFilter.value,
  )
})

watch(
  () => currentEpisode.value?.streamId,
  () => {
    extraExternalSubtitles.value = []
    extSubHits.value = []
    extSubErrors.value = []
    extSubSearchedOnce.value = false
    extSubOpen.value = false
    extSubQueryText.value = ''
    extSubSourceFilter.value = 'all'
  },
)

function buildExtSubLanguages(): string[] {
  if (extSubLangMode.value === 'all') return []
  if (extSubLangMode.value === 'tr') return ['tr']
  if (extSubLangMode.value === 'en') return ['en']
  return defaultPreferredSubLangs.value
}

async function runExternalSubtitleSearch(): Promise<void> {
  const d = detail.value
  if (!d || extSubSearching.value) return
  const rawInput = extSubQueryText.value.trim()
  const imdbMatch = /^tt\d+$/i.test(rawInput) ? rawInput.toLowerCase() : null
  const searchTitle = imdbMatch
    ? d.originalTitle || d.title
    : rawInput || d.originalTitle || d.title
  const matchesOriginalDetailTitle =
    !rawInput ||
    rawInput.toLowerCase() === d.title.trim().toLowerCase() ||
    (d.originalTitle ? rawInput.toLowerCase() === d.originalTitle.trim().toLowerCase() : false)
  const effectiveExternalIds = imdbMatch
    ? { ...d.externalIds, imdb: imdbMatch }
    : matchesOriginalDetailTitle
      ? d.externalIds
      : undefined

  extSubSearching.value = true
  extSubSearchedOnce.value = true
  extSubErrors.value = []
  extSubSourceFilter.value = 'all'
  try {
    const res = await vesselApi.searchSubtitles({
      title: searchTitle,
      originalTitle: imdbMatch ? d.originalTitle : rawInput || d.originalTitle,
      year: matchesOriginalDetailTitle || imdbMatch ? d.year : undefined,
      mediaType: d.type,
      season: d.type === 'series' ? currentEpisode.value?.season : undefined,
      episode: d.type === 'series' ? currentEpisode.value?.episode : undefined,
      externalIds: effectiveExternalIds,
      preferredLanguages: buildExtSubLanguages(),
    })
    extSubHits.value = res.hits
    extSubErrors.value = res.errors
  } catch (err) {
    extSubHits.value = []
    extSubErrors.value = [{ pluginId: 'subtitles', pluginName: 'Subtitles', error: String(err) }]
  } finally {
    extSubSearching.value = false
  }
}

function toggleExternalSubtitleSearch(): void {
  extSubOpen.value = !extSubOpen.value
  if (extSubOpen.value) {
    if (!extSubQueryText.value.trim()) {
      extSubQueryText.value = detail.value?.originalTitle || detail.value?.title || ''
    }
    if (!extSubSearchedOnce.value && !extSubSearching.value) {
      void runExternalSubtitleSearch()
    }
  }
}

async function selectExternalSubtitleHit(hit: SubtitleSearchHit): Promise<void> {
  const key = `${hit.pluginId}:${hit.candidate.id}`
  if (extSubResolvingKey.value) return
  extSubResolvingKey.value = key
  try {
    const srcTag = hit.candidate.sourceName || hit.pluginName
    const finalLabel = hit.candidate.label.includes(srcTag)
      ? hit.candidate.label
      : `[${srcTag}] ${hit.candidate.label}`
    const track = await vesselApi.resolveSubtitle(
      hit.pluginId,
      hit.candidate.id,
      hit.candidate.lang,
      finalLabel,
    )
    const existingIdx = extraExternalSubtitles.value.findIndex((t) => t.id === track.id)
    if (existingIdx >= 0) {
      extraExternalSubtitles.value.splice(existingIdx, 1, track)
    } else {
      extraExternalSubtitles.value = [...extraExternalSubtitles.value, track]
    }
    await nextTick()
    const fullIdx = availableSubtitles.value.findIndex((t) => t.id === track.id)
    if (fullIdx >= 0) {
      selectEmbeddedSubtitle(null)
      activeSubtitleIndex.value = fullIdx
    }
    showGestureHud(t('player.ext_sub_loaded'))
  } catch (err) {
    pushToast('error', String(err))
  } finally {
    extSubResolvingKey.value = ''
  }
}

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
const prevEpisode = computed(() => {
  const list = allEpisodes.value
  const idx = currentEpisodeIndex.value
  if (isLive.value && list.length > 1 && idx >= 0) {
    return idx > 0 ? list[idx - 1] : list[list.length - 1]
  }
  return idx > 0 ? list[idx - 1] : null
})
const nextEpisode = computed(() => {
  const list = allEpisodes.value
  const idx = currentEpisodeIndex.value
  if (isLive.value && list.length > 1 && idx >= 0) {
    return idx < list.length - 1 ? list[idx + 1] : list[0]
  }
  return idx >= 0 && idx < list.length - 1 ? list[idx + 1] : null
})

function skipSegKey(seg: SkipSegment): string {
  return `${seg.type}:${seg.startMs}:${seg.endMs ?? 'end'}`
}

async function refreshSkipSegments(knownDurationSec?: number): Promise<void> {
  const d = detail.value
  const ep = currentEpisode.value
  if (!d || !ep || isLive.value) {
    skipSegments.value = []
    return
  }
  const durMs =
    knownDurationSec && Number.isFinite(knownDurationSec) && knownDurationSec > 10
      ? Math.floor(knownDurationSec * 1000)
      : duration.value > 10
        ? Math.floor(duration.value * 1000)
        : undefined
  const seq = ++skipFetchSeq
  lastFetchedSkipDurationMs = durMs ?? 0

  const pluginSegments: SkipSegment[] = [
    ...(activeStream.value?.skipSegments ?? []),
    ...(session.value?.skipSegments ?? []),
  ]

  try {
    const resolved = await vesselApi.getSkipSegments({
      pluginId: props.pluginId,
      nativeId: props.nativeId,
      mediaType: d.type,
      isAnime: d.isAnime,
      externalIds: d.externalIds,
      season: ep.season,
      episode: ep.episode,
      absoluteEpisode: ep.absoluteEpisode,
      durationMs: durMs,
      pluginSegments,
    })
    if (seq !== skipFetchSeq) return
    skipSegments.value = resolved
  } catch {
    if (seq === skipFetchSeq && pluginSegments.length) {
      skipSegments.value = pluginSegments
    }
  }
}

watch(
  [() => currentEpisode.value?.streamId, () => activeStream.value?.url],
  () => {
    dismissedSkipKeys.value = new Set()
    skipSegments.value = []
    lastFetchedSkipDurationMs = 0
    void refreshSkipSegments()
  },
  { immediate: true },
)

const validSkipSegments = computed<SkipSegment[]>(() => {
  if (isLive.value) return []
  const durMs = duration.value > 10 ? duration.value * 1000 : 0
  return skipSegments.value.filter((seg) => {
    if (durMs > 0 && seg.startMs >= durMs - 1500) return false
    const effectiveEnd = seg.endMs !== undefined ? (durMs > 0 ? Math.min(seg.endMs, durMs) : seg.endMs) : durMs
    if (effectiveEnd > 0) {
      const span = effectiveEnd - seg.startMs
      if (span < 3000) return false
      if (durMs > 0 && span > durMs * 0.85) return false
    }
    return true
  })
})

const activeSkipSegment = computed<SkipSegment | null>(() => {
  if (isLive.value || playbackError.value || !validSkipSegments.value.length) return null
  const nowMs = currentTime.value * 1000
  const durMs = duration.value > 10 ? duration.value * 1000 : 0
  for (const seg of validSkipSegments.value) {
    if (dismissedSkipKeys.value.has(skipSegKey(seg))) continue
    const endMs = seg.endMs ?? (durMs > 0 ? durMs : Number.MAX_SAFE_INTEGER)
    if (nowMs >= seg.startMs && nowMs < endMs - 400) {
      return seg
    }
  }
  return null
})

function skipButtonLabel(seg: SkipSegment): string {
  const durMs = duration.value > 10 ? duration.value * 1000 : 0
  const nearEnd =
    seg.endMs === undefined || (durMs > 0 && seg.endMs >= durMs - 4000)
  switch (seg.type) {
    case 'recap':
      return t('player.skip_recap')
    case 'intro':
      return t('player.skip_intro')
    case 'credits':
      return nextEpisode.value && nearEnd ? t('player.next_episode') : t('player.skip_credits')
    case 'preview':
      return nextEpisode.value ? t('player.next_episode') : t('player.skip_preview')
  }
}

function dismissSkipSegment(seg: SkipSegment): void {
  const next = new Set(dismissedSkipKeys.value)
  next.add(skipSegKey(seg))
  dismissedSkipKeys.value = next
}

function executeSkipSegment(seg: SkipSegment): void {
  const el = videoRef.value
  dismissSkipSegment(seg)
  const durMs = duration.value > 10 ? duration.value * 1000 : 0
  const nearEnd =
    seg.endMs === undefined || (durMs > 0 && seg.endMs >= durMs - 4000)

  if ((seg.type === 'credits' || seg.type === 'preview') && nearEnd && nextEpisode.value) {
    void switchEpisode(nextEpisode.value)
    return
  }

  if ((el || isEmbeddedMpv.value) && !isLive.value) {
    const targetSec =
      seg.endMs !== undefined
        ? Math.min(duration.value > 2 ? duration.value - 0.5 : seg.endMs / 1000, seg.endMs / 1000)
        : Math.max(0, (duration.value || currentTime.value) - 0.5)
    if (isEmbeddedMpv.value) {
      void vesselApi.embeddedMpvSeek(Math.floor(targetSec * 1000)).catch(() => {})
    } else if (el) {
      el.currentTime = targetSec
    }
    currentTime.value = targetSec
    showGestureHud(skipButtonLabel(seg))
    scheduleHideControls()
  }
}

function timelineSegmentStyle(seg: SkipSegment): Record<string, string> {
  if (!duration.value || duration.value <= 0) return { display: 'none' }
  const durMs = duration.value * 1000
  const left = Math.min(100, Math.max(0, (seg.startMs / durMs) * 100))
  const endMs = Math.min(durMs, seg.endMs ?? durMs)
  const width = Math.max(0.6, ((endMs - seg.startMs) / durMs) * 100)
  return {
    left: `${left}%`,
    width: `${width}%`,
  }
}

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
  if (isLocked.value) return
  if (isEmbeddedMpv.value) {
    if (isPlaying.value) {
      userExplicitPause = true
      isPlaying.value = false
      void vesselApi.embeddedMpvPause().catch(() => {})
    } else {
      userExplicitPause = false
      isPlaying.value = true
      void vesselApi.embeddedMpvPlay().catch(() => {})
    }
    scheduleHideControls()
    return
  }
  const el = videoRef.value
  if (!el) return
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
  if (isLocked.value || isLive.value) return
  if (isEmbeddedMpv.value) {
    const maxSec = duration.value > 0 ? duration.value : Infinity
    const target = Math.max(0, Math.min(maxSec, currentTime.value + deltaSec))
    currentTime.value = target
    void vesselApi.embeddedMpvSeek(Math.floor(target * 1000)).catch(() => {})
    scheduleHideControls()
    return
  }
  const el = videoRef.value
  if (!el) return
  const target = Math.max(0, Math.min(el.duration || Infinity, el.currentTime + deltaSec))
  el.currentTime = target
  currentTime.value = target
  scheduleHideControls()
}

function seekFromBar(ev: MouseEvent): void {
  const bar = progressBarRef.value
  const el = videoRef.value
  if (!bar || (!el && !isEmbeddedMpv.value) || !duration.value || isLocked.value) return
  const rect = bar.getBoundingClientRect()
  const ratio = Math.min(1, Math.max(0, (ev.clientX - rect.left) / rect.width))
  const target = ratio * duration.value
  if (isEmbeddedMpv.value) {
    void vesselApi.embeddedMpvSeek(Math.floor(target * 1000)).catch(() => {})
  } else if (el) {
    el.currentTime = target
  }
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
  if (isEmbeddedMpv.value) {
    isMuted.value = !isMuted.value
    void vesselApi.embeddedMpvSetVolume(isMuted.value ? 0 : volume.value).catch(() => {})
    scheduleHideControls()
    return
  }
  const el = videoRef.value
  if (!el) return
  el.muted = !el.muted
  isMuted.value = el.muted
  scheduleHideControls()
}

function setVolumeLevel(val: number): void {
  const clamped = Math.min(1, Math.max(0, val))
  volume.value = clamped
  isMuted.value = clamped === 0
  if (isEmbeddedMpv.value) {
    void vesselApi.embeddedMpvSetVolume(clamped).catch(() => {})
    scheduleHideControls()
    return
  }
  const el = videoRef.value
  if (!el) return
  el.volume = clamped
  el.muted = clamped === 0
  scheduleHideControls()
}

function applyRuntimeSpeed(rate: number): void {
  if (isEmbeddedMpv.value) {
    void vesselApi.embeddedMpvSetSpeed(rate).catch(() => {})
  } else if (videoRef.value) {
    videoRef.value.playbackRate = rate
  }
}

function setSpeed(rate: number): void {
  playbackRate.value = rate
  applyRuntimeSpeed(rate)
  scheduleHideControls()
}

function startHoldSpeedBoost(ev: PointerEvent): void {
  if (ev.pointerType === 'mouse' && ev.button !== 0) return
  if (isLocked.value || isLive.value || activeDrawer.value) return
  if (holdSpeedTimer !== null) {
    window.clearTimeout(holdSpeedTimer)
    holdSpeedTimer = null
  }
  holdSpeedOriginX = ev.clientX
  holdSpeedOriginY = ev.clientY
  holdSpeedTimer = window.setTimeout(() => {
    holdSpeedTimer = null
    if (isLocked.value || isLive.value || activeDrawer.value) return
    holdSpeedBoostActive.value = true
    suppressNextStageClick = true
    applyRuntimeSpeed(2)
  }, 260)
}

function moveHoldSpeedBoost(ev: PointerEvent): void {
  if (holdSpeedTimer === null) return
  if (Math.hypot(ev.clientX - holdSpeedOriginX, ev.clientY - holdSpeedOriginY) > 12) {
    window.clearTimeout(holdSpeedTimer)
    holdSpeedTimer = null
  }
}

function endHoldSpeedBoost(): void {
  if (holdSpeedTimer !== null) {
    window.clearTimeout(holdSpeedTimer)
    holdSpeedTimer = null
  }
  if (!holdSpeedBoostActive.value) return
  holdSpeedBoostActive.value = false
  applyRuntimeSpeed(isLive.value ? 1 : playbackRate.value)
  window.setTimeout(() => {
    suppressNextStageClick = false
  }, 80)
}

function handleStageClick(): void {
  if (suppressNextStageClick) {
    suppressNextStageClick = false
    return
  }
  togglePlay()
}

function cycleFitMode(): void {
  const order: FitMode[] = ['contain', 'cover', 'fill']
  const idx = order.indexOf(fitMode.value)
  fitMode.value = order[(idx + 1) % order.length]
  if (isEmbeddedMpv.value) {
    void vesselApi.embeddedMpvSetFitMode(fitMode.value).catch(() => {})
  }
  scheduleHideControls()
}

function applyFitMode(mode: FitMode): void {
  fitMode.value = mode
  if (isEmbeddedMpv.value) {
    void vesselApi.embeddedMpvSetFitMode(mode).catch(() => {})
  }
  scheduleHideControls()
}

function selectEmbeddedSubtitle(trackId: number | null): void {
  if (trackId !== null) {
    activeSubtitleIndex.value = -1
    selectMuxedWebSubtitle(null)
  }
  activeEmbeddedSubId.value = trackId
  void vesselApi.embeddedMpvSelectSubtitleTrack(trackId).catch(() => {})
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
      if (isLive.value) {
        showGestureHud(ep.title ?? detail.value.title)
      }
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
    if (isLive.value && activeSession.value && detail.value) {
      setSession({
        ...activeSession.value,
        detail: {
          ...detail.value,
          title: ep.title ?? detail.value.title,
          poster: ep.thumbnail ?? detail.value.poster,
          backdrop: ep.thumbnail ?? detail.value.backdrop,
        },
        episode: ep,
        streams: res.streams,
        subtitles: res.subtitles,
        skipSegments: res.skipSegments,
        activeStreamIndex: 0,
        startPositionMs: 0,
      })
      showGestureHud(ep.title ?? detail.value.title)
    } else {
      updateEpisodeSession(ep, res.streams, res.subtitles, 0, res.skipSegments)
    }
    activeDrawer.value = null
  } catch (err) {
    pushToast('error', String(err))
  } finally {
    switchingEpisode.value = false
    scheduleHideControls()
  }
}

async function openInExternalMpv(silentOnFailure = false): Promise<boolean> {
  if (!detail.value || !currentEpisode.value || !activeStream.value) return false
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
    destroyMediaEngines()
    isBuffering.value = false
    isPlaying.value = false
    playbackError.value = ''
    externalPlayerActive.value = true
    controlsVisible.value = true
    pushToast('info', 'Handed off playback to external mpv')
    return true
  } catch (err) {
    if (!silentOnFailure) {
      pushToast('warn', String(err))
    }
    return false
  }
}

function goBack(): void {
  void persistWatchProgress()
  if (document.fullscreenElement) {
    void document.exitFullscreen().catch(() => {})
  }
  if (window.history.length > 1) {
    router.back()
  } else if (isLive.value || props.pluginId === 'iptv') {
    void router.push({ name: 'iptv' })
  } else {
    void router.push({
      name: 'details',
      params: { pluginId: props.pluginId, nativeId: props.nativeId },
    })
  }
}

function goToDetails(): void {
  void persistWatchProgress()
  if (document.fullscreenElement) {
    void document.exitFullscreen().catch(() => {})
  }
  if (isLive.value || props.pluginId === 'iptv') {
    void router.push({ name: 'iptv' })
    return
  }
  void router.push({
    name: 'details',
    params: { pluginId: props.pluginId, nativeId: props.nativeId },
  })
}

function onLoadedMetadata(): void {
  const el = videoRef.value
  if (!el) return
  clearLoadWatchdog()
  playbackError.value = ''
  duration.value = Number.isFinite(el.duration) ? el.duration : 0
  if (
    el.duration > 10 &&
    (lastFetchedSkipDurationMs === 0 ||
      Math.abs(el.duration * 1000 - lastFetchedSkipDurationMs) > 30_000)
  ) {
    void refreshSkipSegments(el.duration)
  }
  el.playbackRate = isLive.value ? 1 : playbackRate.value
  el.volume = volume.value
  el.muted = isMuted.value
  syncAudioTracks()
  syncMuxedWebSubtitles()
  if (
    pendingSeekSeconds.value !== null &&
    pendingSeekSeconds.value > 2 &&
    !hlsInstance &&
    (!el.duration || pendingSeekSeconds.value < el.duration - 2)
  ) {
    const target = pendingSeekSeconds.value
    pendingSeekSeconds.value = null
    try {
      el.currentTime = target
      currentTime.value = target
    } catch {
      // Ignore transient seek before buffer ready
    }
  }
  if (!userExplicitPause) {
    void el.play().catch(() => {})
  }
  isBuffering.value = false
}

function onSeeked(): void {
  const el = videoRef.value
  if (!el) return
  playbackError.value = ''
  isBuffering.value = false
  if (!userExplicitPause && el.paused) {
    void el.play().catch(() => {})
  }
}

function onCanPlay(): void {
  const el = videoRef.value
  if (!el) return
  clearLoadWatchdog()
  playbackError.value = ''
  syncAudioTracks()
  if (pendingSeekSeconds.value !== null && pendingSeekSeconds.value > 2 && !hlsInstance) {
    const target = pendingSeekSeconds.value
    pendingSeekSeconds.value = null
    try {
      el.currentTime = target
      currentTime.value = target
    } catch {
      // Ignore transient seek error
    }
  }
  isBuffering.value = false
  if (!userExplicitPause && el.paused) {
    void el.play().catch(() => {})
  }
}

function onPlaying(): void {
  clearLoadWatchdog()
  playbackError.value = ''
  isBuffering.value = false
  isPlaying.value = true
}

function onTimeUpdate(): void {
  const el = videoRef.value
  if (!el) return
  if (playbackError.value && !el.error && el.readyState >= 2) {
    playbackError.value = ''
  }
  currentTime.value = el.currentTime
  if (el.buffered.length > 0) {
    bufferedEnd.value = el.buffered.end(el.buffered.length - 1)
  }
}

function onVideoError(): void {
  const el = videoRef.value
  if (isResettingMedia || !playableSrc.value) return
  const code = el?.error?.code
  // Ignore MEDIA_ERR_ABORTED (code 1) and cases where media is actually healthy & playing
  if (!code || code === 1) return
  if (el && !el.paused && el.readyState >= 2 && el.currentTime > 0) {
    playbackError.value = ''
    return
  }
  clearLoadWatchdog()

  const src = playableSrc.value
  const stream = activeStream.value
  if (el && src && stream) {
    const isHls =
      stream.kind === 'hls' || src.toLowerCase().includes('.m3u8')
    const canPlayNativeHls = el.canPlayType('application/vnd.apple.mpegurl') !== ''
    if (isHls && !forceNativeHlsNextTry && canPlayNativeHls && hlsInstance) {
      forceNativeHlsNextTry = true
      playbackError.value = ''
      isBuffering.value = true
      void attachMediaSource(src, stream, true, lastTargetStartSec)
      return
    }
  }
  if (code === 4 && !triedAutoMpv) {
    triedAutoMpv = true
    isBuffering.value = false
    isPlaying.value = false
    controlsVisible.value = true
    if (stream && embeddedMpvCapable.value && playerEngineMode.value !== 'html5') {
      void launchEmbeddedMpv(stream, lastTargetStartSec).then((launched) => {
        if (launched) return
        void openInExternalMpv(true).then((ext) => {
          if (ext) return
          const curIdx = session.value?.activeStreamIndex ?? 0
          if (curIdx + 1 < streams.value.length) {
            switchSource(curIdx + 1)
            return
          }
          playbackError.value = t('player.error_unsupported')
        })
      })
      return
    }
    void openInExternalMpv(true).then((launched) => {
      if (launched) return
      const curIdx = session.value?.activeStreamIndex ?? 0
      if (curIdx + 1 < streams.value.length) {
        switchSource(curIdx + 1)
        return
      }
      playbackError.value = t('player.error_unsupported')
    })
    return
  }
  const curIdx = session.value?.activeStreamIndex ?? 0
  if (curIdx + 1 < streams.value.length) {
    switchSource(curIdx + 1)
    return
  }
  playbackError.value =
    code === 4
      ? t('player.error_unsupported')
      : t('player.error_failed')
  isBuffering.value = false
  isPlaying.value = false
  controlsVisible.value = true
}

function retryStream(forceInApp = false): void {
  const stream = activeStream.value
  const src = playableSrc.value
  if (!stream || !src) return
  playbackError.value = ''
  externalPlayerActive.value = false
  isBuffering.value = true
  void attachMediaSource(src, stream, forceNativeHlsNextTry, lastTargetStartSec, forceInApp)
}

function switchEngineMode(mode: 'auto' | 'embedded_mpv' | 'html5'): void {
  const resumePos = currentTime.value > 2 ? currentTime.value : lastTargetStartSec
  playerEngineMode.value = mode
  const stream = activeStream.value
  const src = playableSrc.value
  if (!stream || !src) return
  playbackError.value = ''
  externalPlayerActive.value = false
  isBuffering.value = true
  void attachMediaSource(src, stream, false, resumePos, mode === 'html5')
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
    case 'i':
      ev.preventDefault()
      toggleStatsHud()
      break
    case 'pageup':
    case ']':
      if (isLive.value && nextEpisode.value) {
        ev.preventDefault()
        void switchEpisode(nextEpisode.value)
      }
      break
    case 'pagedown':
    case '[':
      if (isLive.value && prevEpisode.value) {
        ev.preventDefault()
        void switchEpisode(prevEpisode.value)
      }
      break
    case 'c':
      if (isLive.value && allEpisodes.value.length > 1) {
        ev.preventDefault()
        toggleDrawer('episodes')
      }
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
    const isVirtualMeta = props.pluginId === 'vessel.metadata' || props.nativeId.startsWith('virtual::')
    const loadedDetail = isVirtualMeta
      ? await vesselApi.getVirtualMetadataDetail(
          props.nativeId.replace(/^virtual::/, ''),
          undefined,
          undefined,
          currentLang.value,
        )
      : await vesselApi.getDetailsWithEnrichment(props.pluginId, props.nativeId, 'en')
    const queryStreamId = typeof route.query.streamId === 'string' ? route.query.streamId : ''
    const ep =
      loadedDetail.episodes.find((item) => item.streamId === queryStreamId) ??
      loadedDetail.episodes[0]
    if (!ep) {
      playbackError.value = 'No playable episodes available for this title.'
      return
    }
    const res = await vesselApi.getStreamsWithLinked({
      pluginId: props.pluginId,
      nativeId: props.nativeId,
      episodeId: ep.streamId,
      season: ep.season,
      episode: ep.episode,
      absoluteEpisode: ep.absoluteEpisode,
      isMovie: loadedDetail.type === 'movie',
      externalIds: loadedDetail.externalIds,
    })
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
  preferNativeFallback = false,
  startSec = 0,
  skipExternalAuto = false,
): Promise<void> {
  await nextTick()
  const el = videoRef.value
  if (!el || !src) return

  userExplicitPause = false
  triedAutoMpv = skipExternalAuto
  externalPlayerActive.value = false
  lastTargetStartSec = startSec
  destroyMediaEngines()
  isResettingMedia = false

  const lowerSrc = src.toLowerCase()
  const lowerOriginal = stream.url.toLowerCase()
  const looksMpegTs =
    stream.kind === 'mpeg_ts' ||
    ((lowerSrc.includes('.ts') || lowerOriginal.includes('.ts')) &&
      !lowerSrc.includes('.m3u8') &&
      !lowerOriginal.includes('.m3u8'))
  const looksHls =
    stream.kind === 'hls' || lowerSrc.includes('.m3u8') || lowerOriginal.includes('.m3u8')

  if (
    !preferNativeFallback &&
    !skipExternalAuto &&
    playerEngineMode.value !== 'html5' &&
    (playerEngineMode.value === 'embedded_mpv' ||
      (!looksHls && !looksMpegTs && isLikelyUnsupportedInWebView(el, stream, stream.url)))
  ) {
    if (await ensureEmbeddedCapabilities()) {
      triedAutoMpv = true
      const launched = await launchEmbeddedMpv(stream, startSec)
      if (launched) return
    }
    if (playerEngineMode.value === 'auto') {
      triedAutoMpv = true
      const launched = await openInExternalMpv(true)
      if (launched) return
    }
  }
  await stopEmbeddedMpv(false)

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
      isResettingMedia = false
    }
  }

  const canPlayNativeHls = el.canPlayType('application/vnd.apple.mpegurl') !== ''
  // Prefer hls.js first when supported so startPosition resumes directly on target segments without stalling
  if (looksHls && Hls.isSupported() && !preferNativeFallback) {
    let networkRetries = 0
    let mediaRetries = 0
    const hls = new Hls({
      enableWorker: false,
      lowLatencyMode: isLive.value,
      startPosition: startSec > 2 ? startSec : -1,
      fragLoadingTimeOut: 20000,
      manifestLoadingTimeOut: 15000,
      levelLoadingTimeOut: 15000,
      fragLoadingMaxRetry: 4,
      manifestLoadingMaxRetry: 3,
    })
    hlsInstance = hls
    hls.on(Hls.Events.MANIFEST_PARSED, () => {
      playbackError.value = ''
      isBuffering.value = false
      syncAudioTracks()
      syncMuxedWebSubtitles()
      if (!userExplicitPause) {
        void el.play().catch(() => {})
      }
    })
    hls.on(Hls.Events.AUDIO_TRACKS_UPDATED, syncAudioTracks)
    hls.on(Hls.Events.AUDIO_TRACK_SWITCHED, syncAudioTracks)
    hls.on(Hls.Events.SUBTITLE_TRACKS_UPDATED, syncMuxedWebSubtitles)
    hls.on(Hls.Events.SUBTITLE_TRACK_SWITCH, syncMuxedWebSubtitles)
    hls.on(Hls.Events.ERROR, (_ev, data) => {
      if (!data.fatal) return
      if (hlsInstance !== hls) return
      if (data.type === Hls.ErrorTypes.NETWORK_ERROR && networkRetries < 3) {
        networkRetries += 1
        hls.startLoad()
        return
      }
      if (data.type === Hls.ErrorTypes.MEDIA_ERROR && mediaRetries < 2) {
        mediaRetries += 1
        hls.recoverMediaError()
        return
      }
      if (canPlayNativeHls && !preferNativeFallback) {
        forceNativeHlsNextTry = true
        void attachMediaSource(src, stream, true, startSec)
        return
      }
      if (!triedAutoMpv && playerEngineMode.value !== 'html5') {
        triedAutoMpv = true
        const resumePos = currentTime.value > 2 ? currentTime.value : startSec
        destroyMediaEngines()
        if (embeddedMpvCapable.value) {
          void launchEmbeddedMpv(stream, resumePos).then((launched) => {
            if (launched) return
            void openInExternalMpv(true).then((ext) => {
              if (ext) return
              const curIdx = session.value?.activeStreamIndex ?? 0
              if (curIdx + 1 < streams.value.length) {
                switchSource(curIdx + 1)
              } else {
                playbackError.value = t('player.error_failed')
                isBuffering.value = false
                isPlaying.value = false
              }
            })
          })
          return
        }
        void openInExternalMpv(true).then((ext) => {
          if (ext) return
          const curIdx = session.value?.activeStreamIndex ?? 0
          if (curIdx + 1 < streams.value.length) {
            switchSource(curIdx + 1)
          } else {
            playbackError.value = t('player.error_failed')
            isBuffering.value = false
            isPlaying.value = false
          }
        })
        return
      }
      destroyMediaEngines()
      const curIdx = session.value?.activeStreamIndex ?? 0
      if (curIdx + 1 < streams.value.length) {
        switchSource(curIdx + 1)
      } else {
        playbackError.value = t('player.error_failed')
        isBuffering.value = false
        isPlaying.value = false
      }
    })
    hls.loadSource(src)
    hls.attachMedia(el)
    return
  }

  bindNativeAudioListeners(el)
  pendingSeekSeconds.value = startSec > 2 ? startSec : null
  el.src = src

  const watchdogSeq = attachSeq
  loadWatchdogTimer = window.setTimeout(() => {
    loadWatchdogTimer = null
    if (watchdogSeq !== attachSeq) return
    if (playbackError.value || externalPlayerActive.value) return
    if (!isBuffering.value || el.readyState >= 2) return
    if (!triedAutoMpv) {
      triedAutoMpv = true
      if (embeddedMpvCapable.value && playerEngineMode.value !== 'html5') {
        void launchEmbeddedMpv(stream, startSec).then((launched) => {
          if (launched || watchdogSeq !== attachSeq) return
          void openInExternalMpv(true).then((ext) => {
            if (ext || watchdogSeq !== attachSeq) return
            isBuffering.value = false
            isPlaying.value = false
            controlsVisible.value = true
            playbackError.value = t('player.error_failed')
          })
        })
        return
      }
      void openInExternalMpv(true).then((launched) => {
        if (launched || watchdogSeq !== attachSeq) return
        isBuffering.value = false
        isPlaying.value = false
        controlsVisible.value = true
        playbackError.value = t('player.error_failed')
      })
      return
    }
    isBuffering.value = false
    isPlaying.value = false
    controlsVisible.value = true
    playbackError.value = t('player.error_failed')
  }, 12_000)
}

let attachSeq = 0

watch(
  () => activeStream.value,
  async (stream) => {
    const seq = ++attachSeq
    destroyMediaEngines()
    forceNativeHlsNextTry = false
    lastTargetStartSec = 0
    externalPlayerActive.value = false
    playbackError.value = ''
    isBuffering.value = true
    currentTime.value = 0
    duration.value = 0
    playableSrc.value = ''
    if (!stream) return
    let effectiveStream = stream
    if (stream.kind === 'torrent' || stream.url.trim().toLowerCase().startsWith('magnet:?')) {
      try {
        const debridRes = await vesselApi.resolveTorrentCandidate({
          url: stream.url,
          torrent: stream.torrent,
          preferredTarget: stream.debridCache?.target,
          allowUncachedQueue: false,
          season: currentEpisode.value?.season,
          episode: currentEpisode.value?.episode,
        })
        if (seq !== attachSeq) return
        if (debridRes.status !== 'ready') {
          isBuffering.value = false
          playbackError.value =
            debridRes.status === 'no_provider_configured'
              ? debridRes.message
              : `${debridRes.providerLabel}: Torrent not cached (${Math.round(debridRes.progressPct)}%). Queue it in Details or Settings > Debrid.`
          return
        }
        effectiveStream = {
          ...stream,
          url: debridRes.url,
          headers: debridRes.headers ?? {},
        }
      } catch (err) {
        if (seq !== attachSeq) return
        isBuffering.value = false
        playbackError.value = String(err)
        return
      }
    }
    const isLocalOffline =
      effectiveStream.url.startsWith('http://127.0.0.1:') && effectiveStream.url.includes('/local/')
    const [resolved, startSec] = await Promise.all([
      isLocalOffline
        ? Promise.resolve(effectiveStream.url)
        : vesselApi.preparePlaybackUrl(effectiveStream).catch(() => effectiveStream.url),
      resolveDesiredStartSeconds(currentEpisode.value),
    ])
    if (seq !== attachSeq) return
    playableSrc.value = resolved
    await attachMediaSource(resolved, effectiveStream, false, startSec)
  },
  { immediate: true },
)

onMounted(async () => {
  window.addEventListener('keydown', handleKeydown)
  window.addEventListener('pointerup', endHoldSpeedBoost)
  window.addEventListener('pointercancel', endHoldSpeedBoost)
  window.addEventListener('blur', endHoldSpeedBoost)
  document.addEventListener('fullscreenchange', onFullscreenChange)
  void vesselApi
    .embeddedMpvCapabilities()
    .then((caps) => {
      embeddedMpvCapable.value = caps.available
    })
    .catch(() => {})
  try {
    unlistenEmbedded = await listen<EmbeddedPlayerState>('vessel://embedded-player-state', (ev) => {
      if (!isEmbeddedMpv.value) return
      const st = ev.payload
      isBuffering.value = st.isBuffering
      isPlaying.value = st.isPlaying
      if (st.durationMs > 0) {
        duration.value = st.durationMs / 1000
      }
      if (st.positionMs >= 0) {
        currentTime.value = st.positionMs / 1000
      }
      const nextSig = `${trackListSignature(st.audioTracks)}##${trackListSignature(st.subtitleTracks)}`
      if (nextSig !== lastEmbeddedTracksSig) {
        lastEmbeddedTracksSig = nextSig
        embeddedAudioTracks.value = st.audioTracks
        embeddedSubtitleTracks.value = st.subtitleTracks
        const activeSub = st.subtitleTracks.find((t) => t.selected)
        activeEmbeddedSubId.value = activeSub ? activeSub.id : null
        syncAudioTracks()
      }
      if (st.error) {
        playbackError.value = st.error
        isBuffering.value = false
        isPlaying.value = false
        controlsVisible.value = true
      } else if (st.eofReached && !isLive.value) {
        isPlaying.value = false
        void persistWatchProgress(true)
      }
    })
  } catch {
    // Non-Tauri browser preview fallback
  }
  void vesselApi.listPlugins().then((list) => {
    loadedPlugins.value = list
  }).catch(() => {})
  await ensureSessionLoaded()
  progressTimer = window.setInterval(() => {
    if (isPlaying.value) {
      void persistWatchProgress()
    }
  }, 5000)
  scheduleHideControls()
})

const showStatsHud = shallowRef(false)
const statsSnapshot = ref<PlaybackStats | null>(null)
let statsTimer: number | null = null

async function refreshStatsSnapshot(): Promise<void> {
  try {
    const s = await vesselApi.getPlaybackStats()
    const el = videoRef.value
    if (el && !isEmbeddedMpv.value) {
      if (!s.width && el.videoWidth > 0) s.width = el.videoWidth
      if (!s.height && el.videoHeight > 0) s.height = el.videoHeight
      if (s.bufferSeconds === undefined && el.buffered.length > 0) {
        const end = el.buffered.end(el.buffered.length - 1)
        s.bufferSeconds = Math.max(0, end - el.currentTime)
      }
    }
    statsSnapshot.value = s
  } catch {
    // Non-fatal
  }
}

function toggleStatsHud(): void {
  showStatsHud.value = !showStatsHud.value
  if (statsTimer !== null) {
    window.clearInterval(statsTimer)
    statsTimer = null
  }
  if (showStatsHud.value) {
    void refreshStatsSnapshot()
    statsTimer = window.setInterval(() => {
      void refreshStatsSnapshot()
    }, 1000)
  }
}

onUnmounted(() => {
  void persistWatchProgress()
  unlistenEmbedded?.()
  unlistenEmbedded = null
  void stopEmbeddedMpv(true)
  destroyMediaEngines()
  window.removeEventListener('keydown', handleKeydown)
  window.removeEventListener('pointerup', endHoldSpeedBoost)
  window.removeEventListener('pointercancel', endHoldSpeedBoost)
  window.removeEventListener('blur', endHoldSpeedBoost)
  document.removeEventListener('fullscreenchange', onFullscreenChange)
  if (hideTimer !== null) window.clearTimeout(hideTimer)
  if (progressTimer !== null) window.clearInterval(progressTimer)
  if (statsTimer !== null) window.clearInterval(statsTimer)
  if (subSaveTimer !== null) window.clearTimeout(subSaveTimer)
  if (gestureTimer !== null) window.clearTimeout(gestureTimer)
  if (holdSpeedTimer !== null) window.clearTimeout(holdSpeedTimer)
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
    :class="{
      'embedded-active': isEmbeddedMpv,
      'hud-hidden': !controlsVisible && isPlaying && !activeDrawer && !isLocked,
    }"
    @mousemove="handleStagePointerMove"
    @mousedown="scheduleHideControls"
    @wheel="handleTouchpadWheel"
  >
    <!-- Embedded libmpv Hit-Test Surface (transparent over native GL/DX layer) -->
    <div
      v-if="isEmbeddedMpv"
      class="cs-embedded-stage-hit"
      @pointerdown="startHoldSpeedBoost"
      @pointermove="moveHoldSpeedBoost"
      @click="handleStageClick"
      @dblclick="toggleFullscreen"
    />

    <!-- Video Surface -->
    <video
      v-show="playableSrc && !isEmbeddedMpv"
      ref="videoRef"
      class="cs-video"
      :class="`fit-${fitMode}`"
      preload="auto"
      playsinline
      @pointerdown="startHoldSpeedBoost"
      @pointermove="moveHoldSpeedBoost"
      @click="handleStageClick"
      @dblclick="toggleFullscreen"
      @play="userExplicitPause = false; playbackError = ''; isPlaying = true; isBuffering = false; scheduleHideControls()"
      @playing="onPlaying"
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
      <div v-if="holdSpeedBoostActive || gestureLabel" class="cs-gesture-pill" aria-live="polite">
        {{ holdSpeedBoostActive ? '2x »' : gestureLabel }}
      </div>
    </Transition>

    <!-- Live Stream & Debrid Stats HUD -->
    <Transition name="fade-fast">
      <div v-if="showStatsHud && statsSnapshot" class="cs-stats-hud" @click.stop>
        <div class="cs-stats-head">
          <strong>{{ t('stats.title') }}</strong>
          <button type="button" class="cs-stats-close" @click="toggleStatsHud">
            <X :size="13" />
          </button>
        </div>
        <div class="cs-stats-grid">
          <span class="v-muted">{{ t('stats.source') }}</span>
          <span>{{ statsSnapshot.isDebridStream ? 'Debrid HTTP' : 'Direct / Proxy' }}</span>
          <span class="v-muted">{{ t('stats.speed') }}</span>
          <span>{{ statsSnapshot.networkSpeedBps > 0 ? `${formatBytes(statsSnapshot.networkSpeedBps)}/s` : '—' }}</span>
          <span class="v-muted">{{ t('stats.transferred') }}</span>
          <span>{{ statsSnapshot.totalBytesTransferred > 0 ? formatBytes(statsSnapshot.totalBytesTransferred) : '—' }}</span>
          <span class="v-muted">{{ t('stats.buffer') }}</span>
          <span>{{ statsSnapshot.bufferSeconds !== undefined ? `${statsSnapshot.bufferSeconds.toFixed(1)}s` : '—' }}</span>
          <span v-if="statsSnapshot.width && statsSnapshot.height" class="v-muted">{{ t('stats.resolution') }}</span>
          <span v-if="statsSnapshot.width && statsSnapshot.height">
            {{ statsSnapshot.width }}×{{ statsSnapshot.height }}<template v-if="statsSnapshot.fps"> @ {{ statsSnapshot.fps.toFixed(1) }}fps</template>
          </span>
          <span v-if="statsSnapshot.videoCodec || statsSnapshot.videoBitrateBps" class="v-muted">{{ t('stats.video') }}</span>
          <span v-if="statsSnapshot.videoCodec || statsSnapshot.videoBitrateBps">
            {{ statsSnapshot.videoCodec || '—' }}<template v-if="statsSnapshot.videoBitrateBps"> · {{ (statsSnapshot.videoBitrateBps / 1_000_000).toFixed(2) }} Mbps</template>
          </span>
          <span v-if="statsSnapshot.audioCodec || statsSnapshot.audioBitrateBps" class="v-muted">{{ t('stats.audio') }}</span>
          <span v-if="statsSnapshot.audioCodec || statsSnapshot.audioBitrateBps">
            {{ statsSnapshot.audioCodec || '—' }}<template v-if="statsSnapshot.audioBitrateBps"> · {{ Math.round(statsSnapshot.audioBitrateBps / 1000) }} kbps</template>
          </span>
          <span v-if="statsSnapshot.fileSizeBytes" class="v-muted">{{ t('stats.size') }}</span>
          <span v-if="statsSnapshot.fileSizeBytes">{{ formatBytes(statsSnapshot.fileSizeBytes) }}</span>
        </div>
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
    <div
      v-if="loadingInitial || switchingEpisode || (isBuffering && !playbackError && !externalPlayerActive)"
      class="cs-center-state"
    >
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

    <!-- External MPV Active Overlay -->
    <div v-else-if="externalPlayerActive && !playbackError" class="cs-error-overlay">
      <div class="v-card cs-error-card">
        <h3>Playing in External MPV</h3>
        <p>Playback was handed off to mpv for full container &amp; codec support.</p>
        <div class="cs-error-actions">
          <button type="button" class="v-btn v-btn-primary" @click="() => void openInExternalMpv()">
            Open in MPV
          </button>
          <button type="button" class="v-btn v-btn-secondary" @click="() => retryStream(true)">
            Play in App
          </button>
          <button
            v-if="streams.length > 1"
            type="button"
            class="v-btn v-btn-secondary"
            @click="activeDrawer = 'sources'"
          >
            Switch Source ({{ streams.length }})
          </button>
          <button v-if="!isLive" type="button" class="v-btn v-btn-ghost" @click="goToDetails">
            {{ t('player.go_to_details') }}
          </button>
        </div>
      </div>
    </div>

    <!-- Error Overlay -->
    <div v-if="playbackError" class="cs-error-overlay">
      <div class="v-card cs-error-card">
        <h3>Playback Issue</h3>
        <p>{{ playbackError }}</p>
        <div class="cs-error-actions">
          <button type="button" class="v-btn v-btn-primary" @click="() => retryStream(false)">Retry</button>
          <button type="button" class="v-btn v-btn-secondary" @click="() => void openInExternalMpv()">
            Open in MPV
          </button>
          <button
            v-if="streams.length > 1"
            type="button"
            class="v-btn v-btn-secondary"
            @click="activeDrawer = 'sources'"
          >
            Switch Source ({{ streams.length }})
          </button>
          <button v-if="!isLive" type="button" class="v-btn v-btn-ghost" @click="goToDetails">
            {{ t('player.go_to_details') }}
          </button>
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
                <button
                  v-if="!isLive"
                  type="button"
                  class="cs-show-title cs-title-link"
                  :title="t('player.go_to_details')"
                  @click="goToDetails"
                >
                  {{ detail?.title ?? 'Vessel Player' }}
                </button>
                <div v-else class="cs-show-title">{{ detail?.title ?? 'Vessel Player' }}</div>
              </div>
              <div class="cs-ep-sub">
                <template v-if="isLive">
                  <span v-if="currentEpisode?.overview">{{ currentEpisode.overview }}</span>
                  <span v-else-if="detail?.genres.length">{{ detail.genres.join(' · ') }}</span>
                </template>
                <span v-else-if="currentEpisode && detail?.type === 'movie'">
                  <template v-if="currentEpisode.title && currentEpisode.title !== detail?.title">
                    {{ currentEpisode.title }}
                  </template>
                </span>
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
            <button
              v-if="!isLive"
              type="button"
              class="cs-pill-btn"
              :title="t('player.go_to_details')"
              @click="goToDetails"
            >
              <Info :size="15" aria-hidden="true" />
              <span>{{ t('player.go_to_details') }}</span>
            </button>
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
              :class="{ active: activeDrawer === 'audio' }"
              @click="toggleDrawer('audio')"
            >
              <AudioLines :size="15" aria-hidden="true" />
              <span>
                {{ t('player.audio') }}<template v-if="audioTracks.length > 1"> ({{ audioTracks.length }})</template>
              </span>
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
              :class="{ active: showStatsHud }"
              :title="t('stats.title')"
              @click="toggleStatsHud"
            >
              <Activity :size="15" aria-hidden="true" />
              <span>Stats</span>
            </button>
            <button
              type="button"
              class="cs-pill-btn"
              title="Open in external mpv (if installed)"
              @click="() => void openInExternalMpv()"
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
              <div
                v-for="seg in validSkipSegments"
                :key="`seg-${seg.type}-${seg.startMs}`"
                class="cs-seg-marker"
                :class="`seg-${seg.type}`"
                :style="timelineSegmentStyle(seg)"
              />
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

    <!-- Floating Verified Skip Segment Button (Recap / Intro / Credits / Preview) -->
    <Transition name="modal-pop">
      <div
        v-if="activeSkipSegment && !isLocked && !activeDrawer"
        class="cs-skip-floating"
        :class="{ 'is-raised': controlsVisible }"
        @click.stop
      >
        <button
          type="button"
          class="cs-skip-floating-btn"
          @click="executeSkipSegment(activeSkipSegment)"
        >
          <FastForward :size="15" aria-hidden="true" />
          <span>{{ skipButtonLabel(activeSkipSegment) }}</span>
        </button>
        <button
          type="button"
          class="cs-skip-dismiss"
          :title="t('common.close')"
          :aria-label="t('common.close')"
          @click="dismissSkipSegment(activeSkipSegment)"
        >
          <X :size="13" aria-hidden="true" />
        </button>
      </div>
    </Transition>

    <!-- SLIDE-OVER RIGHT DRAWER (Episodes or Channels / Sources / Subtitles / Settings) -->
    <Transition name="drawer-slide">
      <aside
        v-if="activeDrawer && !isLocked"
        class="cs-drawer"
        :class="{ 'is-live-mini-channels': isLive && activeDrawer === 'episodes' }"
        @click.stop
      >
        <div class="cs-drawer-head">
          <h3>
            {{
              activeDrawer === 'episodes'
                ? isLive
                  ? `Channels (${allEpisodes.length})`
                  : `Episodes (${allEpisodes.length})`
                : activeDrawer === 'sources'
                  ? `Sources (${streams.length})`
                  : activeDrawer === 'audio'
                    ? `${t('player.audio_tracks')}${audioTracks.length ? ` (${audioTracks.length})` : ''}`
                    : activeDrawer === 'subtitles'
                      ? subtitleTranscriptOpen
                        ? t('player.sync_transcript')
                        : t('player.subtitles_sync')
                      : 'Player Settings'
            }}
          </h3>
          <button type="button" class="cs-icon-btn" @click="closeDrawerOrTranscript">
            <X :size="18" aria-hidden="true" />
          </button>
        </div>

        <!-- EPISODES OR CHANNELS LIST -->
        <div v-if="activeDrawer === 'episodes'" ref="episodesListRef" class="cs-drawer-body">
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
                <span v-if="stream.pluginName || stream.pluginId" class="cs-tag accent">
                  {{ stream.pluginName || stream.pluginId }}
                </span>
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

        <!-- AUDIO TRACKS LIST (VOD only) -->
        <div v-else-if="activeDrawer === 'audio' && !isLive" class="cs-drawer-body">
          <button
            v-for="trk in audioTracks"
            :key="`${trk.source}-${trk.id}`"
            type="button"
            class="cs-drawer-item"
            :class="{ active: trk.active }"
            @click="switchAudioTrack(trk)"
          >
            <div class="cs-item-main">
              <span class="cs-item-title">{{ trk.label }}</span>
              <div v-if="trk.meta" class="cs-source-tags">
                <span class="cs-tag">{{ trk.meta }}</span>
              </div>
            </div>
            <Check
              v-if="trk.active"
              :size="16"
              class="cs-active-check"
              aria-hidden="true"
            />
          </button>

          <p v-if="!audioTracks.length" class="cs-empty-note">
            {{ t('player.audio_default_only') }}
          </p>
          <p v-else-if="audioTracks.length === 1" class="cs-empty-note">
            {{ t('player.audio_single_note') }}
          </p>
        </div>

        <!-- SUBTITLES & APPEARANCE (VOD only) -->
        <div
          v-else-if="activeDrawer === 'subtitles' && !isLive && subtitleTranscriptOpen && activeCues.length"
          class="cs-drawer-body cs-transcript-panel"
        >
          <p class="cs-transcript-hint">{{ t('player.sync_transcript_hint') }}</p>
          <div class="cs-transcript-meta">
            <span class="cs-tag accent">{{ formatClock(currentTime) }} · {{ subtitleDelayMs }} ms</span>
            <div class="cs-transcript-meta-actions">
              <button
                v-if="subtitleDelayMs !== 0"
                type="button"
                class="cs-mini-pill"
                @click="subtitleDelayMs = 0"
              >
                {{ t('player.sync_reset') }}
              </button>
              <button
                type="button"
                class="cs-mini-pill"
                @click="subtitleTranscriptOpen = false; subtitleTranscriptQuery = ''"
              >
                Back
              </button>
            </div>
          </div>
          <input
            v-model="subtitleTranscriptQuery"
            type="search"
            class="cs-transcript-search"
            :placeholder="t('player.sync_transcript_search')"
          />
          <div ref="transcriptListRef" class="cs-transcript-list">
            <button
              v-for="(cue, idx) in filteredTranscriptCues"
              :key="`${cue.startMs}-${idx}`"
              type="button"
              class="cs-drawer-item cs-transcript-item"
              :class="{ active: idx === nearestTranscriptIndex }"
              @click="applyTranscriptSync(cue)"
            >
              <span class="cs-tag" :class="{ accent: idx === nearestTranscriptIndex }">
                {{ formatClock(cue.startMs / 1000) }}
              </span>
              <span class="cs-transcript-text">{{ cue.lines.join(' ') }}</span>
            </button>
          </div>
        </div>

        <div v-else-if="activeDrawer === 'subtitles' && !isLive" class="cs-drawer-body">
          <div class="cs-setting-block">
            <span class="cs-setting-label">{{ t('player.subtitles_delay') }}</span>
            <div class="cs-delay-row">
              <button type="button" class="cs-mini-pill" @click="subtitleDelayMs -= 500">-500ms</button>
              <span class="cs-delay-val">{{ subtitleDelayMs }} ms</span>
              <button type="button" class="cs-mini-pill" @click="subtitleDelayMs += 500">+500ms</button>
            </div>
            <div v-if="activeCues.length" class="cs-transcript-actions">
              <button type="button" class="cs-mini-pill" @click="void openSubtitleTranscript()">
                <ListVideo :size="14" aria-hidden="true" />
                <span>{{ t('player.sync_transcript') }}</span>
              </button>
              <button
                v-if="subtitleDelayMs !== 0"
                type="button"
                class="cs-mini-pill"
                @click="subtitleDelayMs = 0"
              >
                {{ t('player.sync_reset') }}
              </button>
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
            :class="{ active: activeSubtitleIndex === -1 && activeEmbeddedSubId === null && activeMuxedWebSubKey === null }"
            @click="activeSubtitleIndex = -1; selectEmbeddedSubtitle(null); selectMuxedWebSubtitle(null)"
          >
            <span class="cs-item-title">{{ t('player.subtitles_off') }}</span>
            <Check
              v-if="activeSubtitleIndex === -1 && activeEmbeddedSubId === null && activeMuxedWebSubKey === null"
              :size="16"
              class="cs-active-check"
            />
          </button>

          <button
            v-for="(sub, idx) in availableSubtitles"
            :key="`${sub.label}-${idx}`"
            type="button"
            class="cs-drawer-item"
            :class="{ active: activeSubtitleIndex === idx }"
            @click="selectEmbeddedSubtitle(null); selectMuxedWebSubtitle(null); activeSubtitleIndex = idx"
          >
            <div class="cs-item-main">
              <span class="cs-item-title">{{ sub.label }}</span>
              <span class="cs-item-sub">{{ sub.lang.toUpperCase() }} · {{ sub.format.toUpperCase() }}</span>
            </div>
            <Check v-if="activeSubtitleIndex === idx" :size="16" class="cs-active-check" />
          </button>

          <button
            v-for="wtrk in muxedWebSubtitleTracks"
            :key="`${wtrk.source}-sub-${wtrk.id}`"
            type="button"
            class="cs-drawer-item"
            :class="{ active: activeSubtitleIndex === -1 && activeMuxedWebSubKey === `${wtrk.source}:${wtrk.id}` }"
            @click="selectMuxedWebSubtitle(wtrk)"
          >
            <div class="cs-item-main">
              <span class="cs-item-title">{{ wtrk.label }}</span>
              <span class="cs-item-sub">
                {{ [wtrk.lang?.toUpperCase(), wtrk.source.toUpperCase(), 'Muxed'].filter(Boolean).join(' · ') }}
              </span>
            </div>
            <Check
              v-if="activeSubtitleIndex === -1 && activeMuxedWebSubKey === `${wtrk.source}:${wtrk.id}`"
              :size="16"
              class="cs-active-check"
            />
          </button>

          <button
            v-for="etrk in embeddedSubtitleTracks"
            :key="`emb-sub-${etrk.id}`"
            type="button"
            class="cs-drawer-item"
            :class="{ active: activeSubtitleIndex === -1 && activeEmbeddedSubId === etrk.id }"
            @click="selectEmbeddedSubtitle(etrk.id)"
          >
            <div class="cs-item-main">
              <span class="cs-item-title">{{ etrk.title || etrk.lang?.toUpperCase() || `Embedded #${etrk.id}` }}</span>
              <span class="cs-item-sub">
                {{ [etrk.lang?.toUpperCase(), etrk.codec?.toUpperCase(), 'Embedded'].filter(Boolean).join(' · ') }}
              </span>
            </div>
            <Check
              v-if="activeSubtitleIndex === -1 && activeEmbeddedSubId === etrk.id"
              :size="16"
              class="cs-active-check"
            />
          </button>

          <p v-if="loadingSubtitle" class="cs-empty-note">Loading subtitle track…</p>
          <p v-else-if="subtitleError" class="cs-empty-note cs-sub-error">{{ subtitleError }}</p>
          <p v-else-if="activeSubtitleIndex >= 0 && activeCues.length" class="cs-empty-note">
            Loaded {{ activeCues.length }} subtitle cues.
          </p>
          <p v-if="!availableSubtitles.length && !embeddedSubtitleTracks.length && !muxedWebSubtitleTracks.length" class="cs-empty-note">
            No subtitle tracks were provided by this source.
          </p>

          <div class="cs-setting-block cs-ext-sub-block">
            <button
              type="button"
              class="cs-drawer-item cs-ext-sub-trigger"
              :class="{ active: extSubOpen }"
              @click="toggleExternalSubtitleSearch"
            >
              <div class="cs-drawer-item-row">
                <Search :size="15" aria-hidden="true" />
                <span class="cs-item-title">{{ t('player.ext_sub_search') }}</span>
              </div>
              <Loader2 v-if="extSubSearching" :size="15" class="spin" />
            </button>

            <div v-if="extSubOpen" class="cs-ext-sub-panel">
              <form class="cs-ext-sub-form" @submit.prevent="void runExternalSubtitleSearch()">
                <input
                  v-model="extSubQueryText"
                  type="text"
                  class="cs-transcript-search"
                  :placeholder="t('player.ext_sub_placeholder')"
                />
                <button
                  type="submit"
                  class="cs-mini-pill cs-ext-sub-btn"
                  :disabled="extSubSearching"
                >
                  <Loader2 v-if="extSubSearching" :size="13" class="spin" />
                  <Search v-else :size="13" />
                </button>
              </form>

              <div class="cs-chip-grid">
                <button
                  v-for="opt in ([
                    {
                      id: 'preferred',
                      label: t('player.ext_sub_lang_pref', {
                        langs: defaultPreferredSubLangs.map((l) => l.toUpperCase()).join(', '),
                      }),
                    },
                    { id: 'tr', label: 'TR' },
                    { id: 'en', label: 'EN' },
                    { id: 'all', label: t('player.ext_sub_lang_all') },
                  ] as const)"
                  :key="opt.id"
                  type="button"
                  class="cs-chip-btn"
                  :class="{ active: extSubLangMode === opt.id }"
                  @click="extSubLangMode = opt.id; void runExternalSubtitleSearch()"
                >
                  {{ opt.label }}
                </button>
              </div>

              <div v-if="extSubSources.length > 1" class="cs-chip-grid">
                <button
                  type="button"
                  class="cs-chip-btn"
                  :class="{ active: extSubSourceFilter === 'all' }"
                  @click="extSubSourceFilter = 'all'"
                >
                  All ({{ extSubHits.length }})
                </button>
                <button
                  v-for="src in extSubSources"
                  :key="src"
                  type="button"
                  class="cs-chip-btn"
                  :class="{ active: extSubSourceFilter === src }"
                  @click="extSubSourceFilter = src"
                >
                  {{ src }}
                </button>
              </div>

              <p v-if="extSubErrors.length" class="cs-empty-note cs-sub-error">
                {{ extSubErrors.map((e) => `${e.pluginName}: ${e.error}`).join(' · ') }}
              </p>

              <p v-if="extSubSearching" class="cs-empty-note">{{ t('player.ext_sub_searching') }}</p>
              <p v-else-if="extSubSearchedOnce && !filteredExtSubHits.length" class="cs-empty-note">
                {{ t('player.ext_sub_empty') }}
              </p>

              <div v-if="filteredExtSubHits.length" class="cs-ext-sub-list">
                <button
                  v-for="hit in filteredExtSubHits"
                  :key="`${hit.pluginId}:${hit.candidate.id}`"
                  type="button"
                  class="cs-drawer-item"
                  :disabled="extSubResolvingKey.length > 0"
                  @click="void selectExternalSubtitleHit(hit)"
                >
                  <div class="cs-item-main">
                    <span class="cs-item-title">{{ hit.candidate.label }}</span>
                    <span class="cs-item-sub">
                      {{
                        [
                          hit.candidate.sourceName || hit.pluginName,
                          hit.candidate.lang.toUpperCase(),
                          hit.candidate.format.toUpperCase(),
                          hit.candidate.hearingImpaired ? 'HI' : null,
                          hit.candidate.downloads ? `${hit.candidate.downloads} dl` : null,
                        ]
                          .filter(Boolean)
                          .join(' · ')
                      }}
                    </span>
                  </div>
                  <Loader2
                    v-if="extSubResolvingKey === `${hit.pluginId}:${hit.candidate.id}`"
                    :size="15"
                    class="spin"
                  />
                </button>
              </div>
            </div>
          </div>
        </div>

        <!-- SETTINGS (Speed, Aspect Ratio & Playback Engine) -->
        <div v-else-if="activeDrawer === 'settings'" class="cs-drawer-body">
          <div v-if="embeddedMpvCapable" class="cs-setting-block">
            <span class="cs-setting-label">Playback Engine</span>
            <div class="cs-chip-grid">
              <button
                v-for="eng in ([
                  { id: 'auto', label: 'Auto' },
                  { id: 'embedded_mpv', label: 'Embedded MPV' },
                  { id: 'html5', label: 'WebView' },
                ] as const)"
                :key="eng.id"
                type="button"
                class="cs-chip-btn"
                :class="{ active: playerEngineMode === eng.id }"
                @click="switchEngineMode(eng.id)"
              >
                {{ eng.label }}
              </button>
            </div>
          </div>

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
                @click="applyFitMode(mode.id)"
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

.cs-player.embedded-active {
  background: transparent !important;
}

.cs-embedded-stage-hit {
  position: absolute;
  inset: 0;
  background: transparent;
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
  z-index: 2;
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

.cs-title-link {
  padding: 0;
  border: none;
  background: transparent;
  color: inherit;
  text-align: left;
  cursor: pointer;
}

.cs-title-link:hover {
  color: var(--vessel-accent, #cba6f7);
  text-decoration: underline;
}

.cs-seg-marker {
  position: absolute;
  top: 0;
  bottom: 0;
  z-index: 1;
  border-radius: 999px;
  pointer-events: none;
  opacity: 0.72;
}

.cs-seg-marker.seg-intro {
  background: #cba6f7;
}

.cs-seg-marker.seg-recap {
  background: #f9e2af;
}

.cs-seg-marker.seg-credits {
  background: #89b4fa;
}

.cs-seg-marker.seg-preview {
  background: #a6e3a1;
}

.cs-skip-floating {
  position: absolute;
  right: 1.75rem;
  bottom: 2rem;
  z-index: 14;
  display: inline-flex;
  align-items: center;
  border-radius: 999px;
  background: rgba(14, 15, 24, 0.9);
  backdrop-filter: blur(12px);
  border: 1px solid rgba(255, 255, 255, 0.28);
  box-shadow: 0 12px 32px rgba(0, 0, 0, 0.55);
  overflow: hidden;
  transition: bottom 0.22s ease, transform 0.2s ease, border-color 0.2s ease;
}

.cs-skip-floating.is-raised {
  bottom: 7.4rem;
}

.cs-skip-floating:hover {
  transform: translateY(-2px);
  border-color: var(--vessel-accent, #cba6f7);
}

.cs-skip-floating-btn {
  display: inline-flex;
  align-items: center;
  gap: 0.45rem;
  padding: 0.58rem 0.95rem;
  border: none;
  background: transparent;
  color: #ffffff;
  font-size: 0.88rem;
  font-weight: 700;
  cursor: pointer;
}

.cs-skip-floating-btn:hover {
  background: rgba(203, 166, 247, 0.18);
}

.cs-skip-dismiss {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  padding: 0.58rem 0.65rem 0.58rem 0.45rem;
  border: none;
  border-left: 1px solid rgba(255, 255, 255, 0.14);
  background: transparent;
  color: rgba(255, 255, 255, 0.72);
  cursor: pointer;
}

.cs-skip-dismiss:hover {
  color: #ffffff;
  background: rgba(255, 255, 255, 0.1);
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

.cs-drawer.is-live-mini-channels {
  right: auto;
  left: 1rem;
  top: 3.75rem;
  bottom: 3.75rem;
  width: min(275px, calc(100vw - 2rem));
  background: rgba(12, 13, 22, 0.8);
}

.cs-drawer.is-live-mini-channels .cs-drawer-item {
  padding: 0.5rem 0.65rem;
}

.cs-drawer-item:focus-visible {
  outline: 2px solid var(--vessel-accent, #a78bfa);
  outline-offset: -2px;
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

.cs-transcript-panel {
  overflow: hidden;
}

.cs-transcript-hint {
  margin: 0;
  font-size: 0.76rem;
  color: rgba(244, 244, 248, 0.72);
  line-height: 1.35;
}

.cs-transcript-meta {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.5rem;
}

.cs-transcript-meta-actions,
.cs-transcript-actions {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 0.4rem;
}

.cs-transcript-actions {
  margin-top: 0.35rem;
}

.cs-transcript-search {
  width: 100%;
  padding: 0.5rem 0.7rem;
  border-radius: 10px;
  border: 1px solid rgba(255, 255, 255, 0.14);
  background: rgba(255, 255, 255, 0.06);
  color: #f4f4f8;
  font-size: 0.82rem;
  outline: none;
}

.cs-transcript-search:focus {
  border-color: var(--vessel-accent, #cba6f7);
}

.cs-transcript-list {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  display: flex;
  flex-direction: column;
  gap: 0.45rem;
}

.cs-transcript-item {
  align-items: flex-start;
  justify-content: flex-start;
}

.cs-transcript-text {
  flex: 1;
  font-size: 0.82rem;
  line-height: 1.35;
  color: #f4f4f8;
  text-align: left;
}

.cs-ext-sub-block {
  margin-top: 0.35rem;
  padding-top: 0.65rem;
  border-top: 1px solid rgba(255, 255, 255, 0.1);
}

.cs-ext-sub-trigger {
  justify-content: space-between;
}

.cs-ext-sub-panel {
  display: flex;
  flex-direction: column;
  gap: 0.55rem;
  margin-top: 0.35rem;
}

.cs-ext-sub-form {
  display: flex;
  align-items: center;
  gap: 0.4rem;
}

.cs-ext-sub-btn {
  flex-shrink: 0;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  height: 32px;
  min-width: 34px;
}

.cs-ext-sub-list {
  display: flex;
  flex-direction: column;
  gap: 0.35rem;
  max-height: 260px;
  overflow-y: auto;
}

.cs-stats-hud {
  position: absolute;
  top: 4.25rem;
  left: 1.25rem;
  z-index: 25;
  min-width: 245px;
  max-width: 310px;
  padding: 0.7rem 0.85rem;
  border-radius: 12px;
  background: rgba(12, 14, 22, 0.82);
  backdrop-filter: blur(12px);
  border: 1px solid rgba(255, 255, 255, 0.14);
  color: #f4f4f8;
  font-size: 0.78rem;
  box-shadow: 0 10px 28px rgba(0, 0, 0, 0.45);
}

.cs-stats-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 0.45rem;
  font-size: 0.82rem;
}

.cs-stats-close {
  border: none;
  background: transparent;
  color: rgba(255, 255, 255, 0.7);
  cursor: pointer;
  padding: 2px;
}

.cs-stats-grid {
  display: grid;
  grid-template-columns: auto 1fr;
  gap: 0.28rem 0.75rem;
  align-items: center;
}
</style>
