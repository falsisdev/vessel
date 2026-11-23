export type MediaType = 'movie' | 'series' | 'live'
export type ContentRating = 'sfw' | 'nsfw'
export type LibraryStatus = 'watching' | 'plan_to_watch' | 'completed' | 'on_hold' | 'dropped'
export type DownloadStatus = 'queued' | 'downloading' | 'paused' | 'completed' | 'failed'
export type DownloadKind = 'direct' | 'hls'
export type StreamKind = 'hls' | 'dash' | 'mpeg_ts' | 'mp4' | 'mkv' | 'webm' | 'auto' | 'embed'
export type StreamFlag = 'subtitled' | 'dubbed' | 'multi_audio' | 'hdr' | 'direct' | 'requires_proxy'
export type AudioType = 'sub' | 'dub' | 'original'
export type SubtitleFormat = 'srt' | 'ass' | 'ssa' | 'vtt'
export type PluginCapability = 'vod' | 'search' | 'live_tv' | 'subtitles' | 'extractor'
export type SettingFieldType = 'text' | 'secret' | 'toggle' | 'select'
export type IptvKind = 'xtream' | 'm3u'
export type BackupImportMode = 'smart_merge' | 'reset_and_import'

export type NavLayout = 'expanded_sidebar' | 'collapsed_rail' | 'top_navbar' | 'hidden'
export type NavModule = 'home' | 'search' | 'library' | 'live_tv' | 'downloads' | 'extensions'
export type UiDensity = 'compact' | 'comfortable' | 'spacious'
export type CornerStyle = 'sharp' | 'rounded' | 'pill'
export type HomePreset = 'default' | 'minimal_focus' | 'anime_first' | 'iptv_hub'
export type HeroBannerMode = 'full' | 'compact' | 'hidden'
export type CardAspectRatio = 'poster_2x3' | 'wide_16x9' | 'square'
export type EpisodeListLayout = 'grid_cards' | 'detailed_list' | 'compact_rows'
export type PlayClickBehavior = 'quick_start_best' | 'always_ask_source'

export interface ExternalIds {
  anilist?: number
  mal?: number
  tmdb?: number
  imdb?: string
  tvdb?: number
  kitsu?: number
}

export interface MediaSummary {
  nativeId: string
  pluginId?: string
  type: MediaType
  isAnime: boolean
  title: string
  year?: number
  poster?: string
  rating?: number
}

export interface Episode {
  streamId: string
  season: number
  episode: number
  absoluteEpisode?: number
  title?: string
  overview?: string
  thumbnail?: string
  airDate?: string
  tags: string[]
}

export interface SeasonSummary {
  id: string
  season: number
  title?: string
  episodeCount?: number
  poster?: string
}

export interface MediaDetail {
  nativeId: string
  pluginId?: string
  type: MediaType
  isAnime: boolean
  title: string
  originalTitle?: string
  year?: number
  synopsis?: string
  poster?: string
  backdrop?: string
  genres: string[]
  rating?: number
  status?: string
  contentRating?: ContentRating
  externalIds: ExternalIds
  seasons?: SeasonSummary[]
  episodes: Episode[]
}

export type SubtitleSource =
  | { kind: 'url'; url: string; headers?: Record<string, string> }
  | { kind: 'inline_content'; content: string }

export interface SubtitleTrack {
  id?: string
  source: SubtitleSource
  lang: string
  label: string
  format: SubtitleFormat
  isDefault?: boolean
}

export interface StreamCandidate {
  url: string
  kind: StreamKind
  quality?: string
  label: string
  language?: string
  audioType?: AudioType
  priority?: number
  isDefault?: boolean
  flags: StreamFlag[]
  headers: Record<string, string>
  needsExtractor?: boolean
  subtitles: SubtitleTrack[]
}

export interface StreamResult {
  streams: StreamCandidate[]
  subtitles: SubtitleTrack[]
}

export interface HomeSection {
  id: string
  title: string
  layout: 'poster' | 'landscape'
  items: MediaSummary[]
  nextCursor?: string
}

export interface HomeShelfResult {
  pluginId: string
  pluginName: string
  sections: HomeSection[]
  error?: string
}

export interface MediaRef {
  pluginId: string
  nativeId: string
}

export interface PlaybackProgress {
  media: MediaRef
  streamId: string
  canonicalId?: string
  season: number
  episode: number
  absoluteEpisode?: number
  positionMs: number
  durationMs: number
  completed: boolean
  lastWatchedAt: string
}

export interface ContinueWatchingEntry {
  media: MediaRef
  title: string
  poster?: string
  backdrop?: string
  mediaType: MediaType
  streamId: string
  canonicalId?: string
  season: number
  episode: number
  absoluteEpisode?: number
  episodeTitle?: string
  positionMs: number
  durationMs: number
  completed: boolean
  lastWatchedAt: string
}

export interface PluginSettingField {
  key: string
  label: string
  type: SettingFieldType
  default?: boolean | string
  required?: boolean
  placeholder?: string
  options: Array<{ label: string; value: string }>
}

export interface PluginManifest {
  schemaVersion: number
  id: string
  name: string
  version: string
  author?: string
  description?: string
  entry: string
  icon?: string
  capabilities: PluginCapability[]
  contentTypes: string[]
  languages: string[]
  domains: string[]
  allowAnyHttps?: boolean
  isAdult?: boolean
  settingsSchema: PluginSettingField[]
}

export interface PluginListEntry {
  manifest: PluginManifest
  repoId?: string
  sourceType: string
  localPath: string
  sha256?: string
  enabled: boolean
  allowDynamicDomains: boolean
  allowedDomains: string[]
}

export interface RepoPluginEntry {
  id: string
  name: string
  version: string
  description?: string
  capabilities: PluginCapability[]
  downloadUrl?: string
  sourcePath?: string
  sha256?: string
}

export interface RepoIndex {
  schemaVersion: number
  id: string
  name: string
  description?: string
  author?: string
  website?: string
  pluginLists: string[]
  plugins: RepoPluginEntry[]
}

export type ResolvedSource =
  | {
      kind: 'repository'
      resolvedUrl: string
      badge: 'official' | 'community'
      repo: RepoIndex
    }
  | {
      kind: 'package'
      resolvedUrl: string
      badge: 'official' | 'community'
      manifest: PluginManifest
      sha256: string
    }

export interface MediaRef {
  pluginId: string
  nativeId: string
}

export interface LibraryItem {
  media: MediaRef
  canonicalId?: string
  mediaType: MediaType
  isAnime: boolean
  title: string
  posterUrl?: string
  year?: number
  status: LibraryStatus
  addedAt: string
  updatedAt: string
}

export interface StoredIptvProfile {
  id: string
  name: string
  kind: IptvKind
  serverUrl: string
  username?: string
  secretRef?: string
  epgUrl?: string
  userAgent?: string
  lastSyncedAt?: string
}

export interface StoredIptvChannel {
  id: number
  profileId: string
  streamId: string
  name: string
  groupTitle: string
  logoUrl?: string
  streamUrl: string
  tvgId?: string
  isFavorite: boolean
}

export interface EpgProgram {
  channelTvgId: string
  title: string
  description?: string
  startTs: number
  endTs: number
}

export interface EpgNowNext {
  now?: EpgProgram
  next?: EpgProgram
}

export interface StoredDownload {
  id: string
  pluginId: string
  nativeId: string
  title: string
  season?: number
  episode?: number
  kind: DownloadKind
  sourceUrl: string
  localDir: string
  localMediaPath?: string
  status: DownloadStatus
  bytesDownloaded: number
  totalBytes?: number
  segmentsDone: number
  segmentsTotal: number
  errorMsg?: string
  createdAt: string
  updatedAt: string
}

export interface ThemeColors {
  bgBase: string
  bgSurface: string
  bgElevated: string
  textPrimary: string
  textMuted: string
  accent: string
  accentSecondary: string
  border: string
  success: string
  warning: string
  danger: string
}

export interface ThemeSpec {
  schemaVersion: number
  id: string
  name: string
  author: string
  isDark: boolean
  colors: ThemeColors
}

export interface UiPreferences {
  shell: {
    layout: NavLayout
    visibleModules: NavModule[]
    density: UiDensity
    fontScalePercent: number
    cornerStyle: CornerStyle
    reducedMotion: boolean
  }
  home: {
    preset: HomePreset
    heroBanner: HeroBannerMode
    showContinueWatching: boolean
    continueWatchingLimit: number
    showRecentIptv: boolean
    hiddenSections: string[]
  }
  cards: {
    aspectRatio: CardAspectRatio
    columns?: number
    showPluginBadge: boolean
    showRating: boolean
    showYear: boolean
    showContentType: boolean
    showWatchedCheck: boolean
    showProgressBar: boolean
    showHoverActions: boolean
    blurNsfwPosters: boolean
  }
  details: {
    showBackdrop: boolean
    backdropDimPercent: number
    showExternalIdBadges: boolean
    showFixMatch: boolean
    showSwitchSource: boolean
    episodeLayout: EpisodeListLayout
    hideEpisodeThumbnails: boolean
    hideEpisodeOverviews: boolean
  }
  playback: {
    clickBehavior: PlayClickBehavior
    showLiveControlBar: boolean
    autoNextDelaySecs: number
    watchedThresholdPercent: number
  }
  advanced: {
    customCssEnabled: boolean
    customCss: string
  }
}

export type CardPreferences = UiPreferences['cards']

export interface EngineInfo {
  rootDir: string
  downloadsDir: string
  pluginsDir: string
  themesDir: string
  localesDir: string
  backupsDir: string
  proxyBaseUrl: string
  alwaysProxy: boolean
  metadataEnabled: boolean
}

export interface PlayerStateSnapshot {
  isPlaying: boolean
  isPaused: boolean
  isBuffering: boolean
  positionMs: number
  durationMs: number
  currentStreamUrl?: string
  subtitleDelayMs: number
}

export interface DownloadProgressSnapshot {
  downloadId: string
  status: DownloadStatus
  bytesDownloaded: number
  totalBytes?: number
  segmentsDone: number
  segmentsTotal: number
}

export type EngineEvent =
  | {
      type: 'search_partial_result'
      searchId: string
      pluginId: string
      items: MediaSummary[]
    }
  | { type: 'search_completed'; searchId: string }
  | {
      type: 'metadata_enriched'
      pluginId: string
      nativeId: string
      detail: MediaDetail
    }
  | ({ type: 'player_state_changed' } & PlayerStateSnapshot)
  | {
      type: 'domain_permission_requested'
      requestId: string
      pluginId: string
      domain: string
    }
  | ({ type: 'download_progress' } & DownloadProgressSnapshot)
  | {
      type: 'plugin_update_available'
      pluginId: string
      currentVersion: string
      latestVersion: string
    }
  | {
      type: 'plugin_log'
      pluginId: string
      level: 'debug' | 'info' | 'warn' | 'error'
      message: string
    }
