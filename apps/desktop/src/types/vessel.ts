export type MediaType = 'movie' | 'series' | 'live'
export type ContentRating = 'sfw' | 'nsfw'
export type LibraryStatus = 'watching' | 'plan_to_watch' | 'completed' | 'on_hold' | 'dropped'
export type DownloadStatus = 'queued' | 'downloading' | 'paused' | 'completed' | 'failed'
export type DownloadKind = 'direct' | 'hls'
export type StreamKind = 'hls' | 'dash' | 'mpeg_ts' | 'mp4' | 'mkv' | 'webm' | 'auto' | 'embed' | 'torrent'
export type StreamFlag = 'subtitled' | 'dubbed' | 'multi_audio' | 'hdr' | 'direct' | 'requires_proxy'
export type AudioType = 'sub' | 'dub' | 'original'
export type SubtitleFormat = 'srt' | 'ass' | 'ssa' | 'vtt'
export type SkipSegmentType = 'intro' | 'recap' | 'credits' | 'preview'
export type PluginCapability = 'vod' | 'search' | 'live_tv' | 'subtitles' | 'extractor' | 'stream' | 'debrid'
export type SettingFieldType = 'text' | 'secret' | 'toggle' | 'select'
export type IptvKind = 'xtream' | 'm3u'
export type BackupImportMode = 'smart_merge' | 'reset_and_import'

export type NavLayout =
  | 'expanded_sidebar'
  | 'pinned_sidebar'
  | 'floating_bottom'
  | 'collapsed_rail'
  | 'top_navbar'
  | 'hidden'
export type SubtitleOutlineStyle = 'outline' | 'shadow' | 'box' | 'none'
export type NavModule = 'home' | 'search' | 'library' | 'live_tv' | 'downloads' | 'extensions'
export type UiDensity = 'compact' | 'comfortable' | 'spacious'
export type CornerStyle = 'sharp' | 'rounded' | 'pill'
export type HomePreset = 'default' | 'minimal_focus' | 'anime_first' | 'iptv_hub'
export type HeroBannerMode = 'full' | 'compact' | 'hidden'
export type CardAspectRatio =
  | 'poster_2x3'
  | 'portrait_3x4'
  | 'portrait_4x5'
  | 'square'
  | 'classic_4x3'
  | 'wide_16x9'
  | 'cinema_21x9'
export type EpisodeListLayout = 'grid_cards' | 'detailed_list' | 'compact_rows'
export type PlayClickBehavior = 'quick_start_best' | 'always_ask_source'

export interface ExternalIds {
  anilist?: number
  mal?: number
  tmdb?: number
  imdb?: string
  tvdb?: number
  kitsu?: number
  simkl?: number
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

export interface SubtitleQuery {
  title: string
  originalTitle?: string
  year?: number
  mediaType: MediaType
  season?: number
  episode?: number
  externalIds?: ExternalIds
  preferredLanguages?: string[]
}

export interface SubtitleCandidate {
  id: string
  lang: string
  label: string
  format: SubtitleFormat
  releaseName?: string
  sourceName?: string
  downloads?: number
  hearingImpaired?: boolean
}

export interface SubtitleSearchHit {
  pluginId: string
  pluginName: string
  candidate: SubtitleCandidate
}

export interface SubtitleProviderError {
  pluginId: string
  pluginName: string
  error: string
}

export interface SubtitleSearchOutcome {
  hits: SubtitleSearchHit[]
  errors: SubtitleProviderError[]
}

export interface SubtitlePluginInfo {
  pluginId: string
  name: string
}

export interface SkipSegment {
  type: SkipSegmentType
  startMs: number
  endMs?: number
  source?: string
  confidence?: number
}

export type DebridProviderKind = 'real_debrid' | 'tor_box' | 'all_debrid' | 'premiumize' | 'debrid_link'
export type DebridCacheState = 'cached' | 'uncached' | 'unknown'
export type DebridCloudStatus = 'queued' | 'downloading' | 'uploading' | 'ready' | 'error'

export interface DebridCacheInfo {
  target: string
  label: string
  state: DebridCacheState
}

export interface TorrentMeta {
  infoHash?: string
  fileIndex?: number
  filename?: string
  season?: number
  episode?: number
  seeders?: number
  sizeBytes?: number
  trackerSource?: string
  codecs?: string[]
  hdr?: string
  trackers?: string[]
}

export interface DebridAccountStatus {
  target: string
  label: string
  isBuiltin: boolean
  supportsBulkCacheCheck: boolean
  enabled: boolean
  authenticated: boolean
  username?: string
  email?: string
  isPremium: boolean
  expiresAt?: string
  pointsOrQuota?: string
  error?: string
}

export interface DebridCloudItem {
  id: string
  target: string
  providerLabel: string
  filename: string
  infoHash?: string
  sizeBytes?: number
  downloadedBytes?: number
  status: DebridCloudStatus
  progressPct: number
  speedBps?: number
  seeders?: number
  etaSeconds?: number
  links: string[]
  addedAt?: string
}

export interface DebridPreferences {
  priorityOrder: string[]
  onlyCached: boolean
  autoFallback: boolean
}

export interface DebridOverview {
  preferences: DebridPreferences
  accounts: DebridAccountStatus[]
  disabledStreamAddons: string[]
}

export interface RealDebridDeviceAuthStart {
  deviceCode: string
  userCode: string
  verificationUrl: string
  directVerificationUrl?: string
  intervalSeconds: number
  expiresInSeconds: number
}

export interface RealDebridDeviceAuthPoll {
  authorized: boolean
  pending: boolean
  username?: string
  error?: string
}

export type DebridResolution =
  | {
      status: 'ready'
      target: string
      providerLabel: string
      url: string
      headers: Record<string, string>
      filename?: string
      sizeBytes?: number
      expiresAt?: string
    }
  | {
      status: 'needs_confirmation'
      target: string
      providerLabel: string
      infoHash: string
      cloudStatus: DebridCloudStatus
      progressPct: number
      speedBps?: number
      seeders?: number
      torrentId?: string
    }
  | {
      status: 'no_provider_configured'
      message: string
    }

export interface PlaybackStats {
  networkSpeedBps: number
  totalBytesTransferred: number
  bufferSeconds?: number
  cacheSpeedBps?: number
  videoBitrateBps?: number
  audioBitrateBps?: number
  width?: number
  height?: number
  fps?: number
  videoCodec?: string
  audioCodec?: string
  fileSizeBytes?: number
  isDebridStream: boolean
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
  skipSegments?: SkipSegment[]
  pluginId?: string
  pluginName?: string
  torrent?: TorrentMeta
  debridCache?: DebridCacheInfo
}

export type MetadataProviderFilter =
  | 'all'
  | 'tmdb'
  | 'imdb'
  | 'simkl'
  | 'anilist'
  | 'mal'
  | 'kitsu'

export interface MetadataSearchHit {
  canonicalId: string
  provider: string
  title: string
  originalTitle?: string
  year?: number
  mediaType?: string
  poster?: string
  synopsis?: string
  externalIds: ExternalIds
}

export interface MetadataSearchResponse {
  hits: MetadataSearchHit[]
  notes: string[]
}

export interface LinkedMediaSource {
  pluginId: string
  pluginName?: string
  nativeId: string
  title: string
  poster?: string
}

export interface SearchPage {
  items: MediaSummary[]
  hasMore: boolean
}

export interface StreamResult {
  streams: StreamCandidate[]
  subtitles: SubtitleTrack[]
  skipSegments?: SkipSegment[]
  pluginId?: string
  pluginName?: string
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
  title?: string
  posterUrl?: string
  backdropUrl?: string
  episodeTitle?: string
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
  configureUrl?: string
}

export interface PluginListEntry {
  manifest: PluginManifest
  repoId?: string
  sourceType: string
  sourceUrl?: string
  localPath: string
  sha256?: string
  enabled: boolean
  allowDynamicDomains: boolean
  allowedDomains: string[]
}

export interface PluginUpdateInfo {
  pluginId: string
  currentVersion: string
  latestVersion?: string
  updateAvailable: boolean
  sourceUrl?: string
  error?: string
}

export interface UpdatePluginResult {
  pluginId: string
  badge: string
  version: string
}

export interface BatchUpdateResult {
  checked: number
  updated: UpdatePluginResult[]
  failed: Array<{ pluginId: string; error: string }>
}

export interface RepoPluginEntry {
  id: string
  name: string
  version: string
  description?: string
  capabilities: PluginCapability[]
  iconUrl?: string
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
  iconUrl?: string
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
  streamLabel?: string
  queueOrder: number
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

export interface LocalPlaybackBundle {
  downloadId: string
  url: string
  kind: StreamKind
  subtitles: SubtitleTrack[]
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
    mobileNavLayout?: 'standard' | 'floating_bottom'
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
    selectedPlugins?: string[] | null
    heroPlugins?: string[] | null
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
    naturalTouchpadScroll: boolean
  }
  subtitles: {
    fontSizePx: number
    fontFamily: string
    textColor: string
    backgroundOpacity: number
    outlineStyle: SubtitleOutlineStyle
    bottomOffsetPx: number
    bold: boolean
  }
  advanced: {
    customCssEnabled: boolean
    customCss: string
  }
  liveTv: {
    pluginOrder: string[]
    sectionOrderByPlugin: Record<string, string[]>
    favoriteChannels: FavoriteLiveChannel[]
    viewMode: LiveTvViewMode
    density: UiDensity
    columns?: number
    showLogos: boolean
    showCategories: boolean
  }
  searchHistory?: string[]
}

export type LiveTvViewMode = 'grid' | 'compact_list' | 'showcase_tiles'

export interface FavoriteLiveChannel {
  pluginId: string
  nativeId: string
  title: string
  poster?: string
  sectionTitle?: string
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
      type: 'home_feed_partial_result'
      requestId: string
      pluginId: string
      pluginName: string
      sections: HomeSection[]
      error?: string
    }
  | { type: 'home_feed_completed'; requestId: string }
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

export interface AppUpdateInfo {
  currentVersion: string
  latestVersion: string
  tagName: string
  updateAvailable: boolean
  releaseName?: string
  releaseNotes: string
  publishedAt?: string
  htmlUrl: string
  assetName?: string
  assetSizeBytes?: number
  downloadUrl?: string
  autoCheckEnabled: boolean
  skippedVersion?: string
}

export interface EmbeddedCapabilities {
  available: boolean
  libraryPath?: string
}

export interface EmbeddedTrackInfo {
  id: number
  trackType: string
  title?: string
  lang?: string
  codec?: string
  channels?: string
  selected: boolean
  external: boolean
}

export interface EmbeddedPlayerState {
  active: boolean
  isPlaying: boolean
  isPaused: boolean
  isBuffering: boolean
  positionMs: number
  durationMs: number
  volume: number
  speed: number
  eofReached: boolean
  audioTracks: EmbeddedTrackInfo[]
  subtitleTracks: EmbeddedTrackInfo[]
  error?: string
}

export interface UpdatePreferences {
  autoCheckEnabled: boolean
  skippedVersion?: string
}

export interface UpdateDownloadProgress {
  fileName: string
  downloadedBytes: number
  totalBytes?: number
  percent?: number
}
