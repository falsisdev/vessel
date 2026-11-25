import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import type {
  AppUpdateInfo,
  BackupImportMode,
  ContinueWatchingEntry,
  DownloadKind,
  EngineEvent,
  EngineInfo,
  EpgNowNext,
  ExternalIds,
  HomeShelfResult,
  IptvKind,
  LibraryItem,
  LibraryStatus,
  LocalPlaybackBundle,
  MediaDetail,
  MediaType,
  PlaybackProgress,
  PluginListEntry,
  ResolvedSource,
  SkipSegment,
  StoredDownload,
  StoredIptvChannel,
  StoredIptvProfile,
  StreamCandidate,
  StreamResult,
  SubtitleTrack,
  ThemeSpec,
  UiPreferences,
  UpdateDownloadProgress,
  UpdatePreferences,
} from '@/types/vessel'

export function isTauriRuntime(): boolean {
  return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window
}

async function call<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  if (!isTauriRuntime()) {
    throw new Error(`[VesselDesktopError]: Tauri runtime unavailable for '${command}'`)
  }
  return invoke<T>(command, args)
}

export async function listenVesselEvents(
  handler: (event: EngineEvent) => void,
): Promise<UnlistenFn> {
  if (!isTauriRuntime()) {
    return () => {}
  }
  return listen<EngineEvent>('vessel://event', (msg) => handler(msg.payload))
}

export const vesselApi = {
  getEngineInfo: () => call<EngineInfo>('get_engine_info'),
  setAlwaysProxy: (enabled: boolean) => call<void>('set_always_proxy', { enabled }),
  setMetadataEnabled: (enabled: boolean) => call<void>('set_metadata_enabled', { enabled }),
  setTmdbApiKey: (apiKey: string) => call<void>('set_tmdb_api_key', { apiKey }),
  setSimklClientId: (clientId: string) => call<void>('set_simkl_client_id', { clientId }),
  setDownloadDirectory: (path?: string | null) =>
    call<string>('set_download_directory', { path: path ?? null }),
  pickDownloadDirectory: () => call<string | null>('pick_download_directory'),

  getUiPreferences: () => call<UiPreferences>('get_ui_preferences'),
  setUiPreferences: (prefs: UiPreferences) => call<void>('set_ui_preferences', { prefs }),

  listThemes: () => call<ThemeSpec[]>('list_themes'),
  importThemeJson: (json: string) => call<ThemeSpec>('import_theme_json', { json }),
  listCustomLocales: () => call<string[]>('list_custom_locales'),
  loadCustomLocale: (lang: string) =>
    call<Record<string, unknown> | null>('load_custom_locale', { lang }),

  listPlugins: () => call<PluginListEntry[]>('list_plugins'),
  resolveSource: (rawInput: string) => call<ResolvedSource>('resolve_source', { rawInput }),
  installSource: (rawInput: string, repoId?: string) =>
    call<{ pluginId: string; badge: string }>('install_source', { rawInput, repoId }),
  setPluginEnabled: (pluginId: string, enabled: boolean) =>
    call<void>('set_plugin_enabled', { pluginId, enabled }),
  uninstallPlugin: (pluginId: string) => call<void>('uninstall_plugin', { pluginId }),
  grantPluginDomain: (pluginId: string, domain: string, permanent: boolean) =>
    call<void>('grant_plugin_domain', { pluginId, domain, permanent }),
  getPluginSettings: (pluginId: string) =>
    call<Record<string, unknown>>('get_plugin_settings', { pluginId }),
  setPluginSettings: (pluginId: string, values: Record<string, unknown>) =>
    call<void>('set_plugin_settings', { pluginId, values }),

  getHomeFeeds: (pluginIds?: string[] | null) =>
    call<HomeShelfResult[]>('get_home_feeds', {
      pluginIds: pluginIds === null ? undefined : pluginIds,
    }),
  startHomeFeeds: (pluginIds?: string[] | null) =>
    call<string>('start_home_feeds', {
      pluginIds: pluginIds === null ? undefined : pluginIds,
    }),
  searchAll: (
    query: string,
    filters?: Record<string, string>,
    page = 1,
    pluginIds?: string[],
  ) => call<string>('search_all', { query, filters, page, pluginIds }),
  cancelSearch: () => call<void>('cancel_search'),
  getDetailsWithEnrichment: (pluginId: string, nativeId: string, preferredLang = 'en') =>
    call<MediaDetail>('get_details_with_enrichment', { pluginId, nativeId, preferredLang }),
  getStreams: (pluginId: string, episodeId: string) =>
    call<StreamResult>('get_streams', { pluginId, episodeId }),
  getStreamsWithLinked: (payload: {
    pluginId: string
    nativeId: string
    episodeId: string
    season: number
    episode: number
    absoluteEpisode?: number
    isMovie: boolean
  }) => call<StreamResult>('get_streams_with_linked', payload),
  searchPluginSync: (pluginId: string, query: string, page = 1) =>
    call<{ items: import('@/types/vessel').MediaSummary[]; hasMore: boolean }>('search_plugin_sync', {
      pluginId,
      query,
      page,
    }),
  searchMetadata: (
    query: string,
    provider: import('@/types/vessel').MetadataProviderFilter = 'all',
    preferredLang = 'en',
  ) =>
    call<import('@/types/vessel').MetadataSearchResponse>('search_metadata', {
      query,
      provider,
      preferredLang,
    }),
  getLinkedSources: (pluginId: string, nativeId: string) =>
    call<import('@/types/vessel').LinkedMediaSource[]>('get_linked_sources', {
      pluginId,
      nativeId,
    }),
  setLinkedSources: (
    pluginId: string,
    nativeId: string,
    sources: import('@/types/vessel').LinkedMediaSource[],
  ) =>
    call<import('@/types/vessel').LinkedMediaSource[]>('set_linked_sources', {
      pluginId,
      nativeId,
      sources,
    }),
  setMetadataOverride: (
    pluginId: string,
    nativeId: string,
    canonicalId: string | null,
    isDisabled: boolean,
  ) => call<void>('set_metadata_override', { pluginId, nativeId, canonicalId, isDisabled }),

  listLibrary: (status?: LibraryStatus) => call<LibraryItem[]>('list_library', { status }),
  upsertLibrary: (item: LibraryItem) => call<void>('upsert_library', { item }),
  deleteLibraryItem: (pluginId: string, nativeId: string) =>
    call<void>('delete_library_item', { pluginId, nativeId }),

  preparePlaybackUrl: (candidate: StreamCandidate) =>
    call<string>('prepare_playback_url', { candidate }),
  clearRuntimeCaches: () => call<void>('clear_runtime_caches'),
  fetchSubtitleText: (url: string, headers?: Record<string, string>) =>
    call<string>('fetch_subtitle_text', { url, headers }),
  playCandidate: (payload: {
    pluginId: string
    nativeId: string
    canonicalId?: string
    season: number
    episode: number
    absoluteEpisode?: number
    mediaType: MediaType
    candidate: StreamCandidate
    extraSubtitles?: SubtitleTrack[]
    startPositionMs?: number
    title?: string
  }) => call<string>('play_candidate', { payload }),
  playerPlay: () => call<void>('player_play'),
  playerPause: () => call<void>('player_pause'),
  playerSeek: (positionMs: number) => call<void>('player_seek', { positionMs }),
  playerStop: () => call<void>('player_stop'),
  addSubtitle: (track: SubtitleTrack) => call<void>('add_subtitle', { track }),
  setSubtitleDelay: (delayMs: number) => call<void>('set_subtitle_delay', { delayMs }),

  recordProgress: (progress: PlaybackProgress) => call<void>('record_progress', { progress }),
  getWatchProgress: (pluginId: string, nativeId: string, season: number, episode: number) =>
    call<PlaybackProgress | null>('get_watch_progress', { pluginId, nativeId, season, episode }),
  listMediaWatchProgress: (pluginId: string, nativeId: string) =>
    call<PlaybackProgress[]>('list_media_watch_progress', { pluginId, nativeId }),
  listAllWatchProgress: (limit = 200) =>
    call<PlaybackProgress[]>('list_all_watch_progress', { limit }),
  listContinueWatching: (limit = 16) =>
    call<ContinueWatchingEntry[]>('list_continue_watching', { limit }),
  deleteWatchProgress: (pluginId: string, nativeId: string) =>
    call<void>('delete_watch_progress', { pluginId, nativeId }),
  getSkipSegments: (payload: {
    pluginId?: string
    nativeId?: string
    mediaType: MediaType
    isAnime?: boolean
    externalIds?: ExternalIds
    season?: number
    episode?: number
    absoluteEpisode?: number
    durationMs?: number
    pluginSegments?: SkipSegment[]
  }) => call<SkipSegment[]>('get_skip_segments', { payload }),

  listIptvProfiles: () => call<StoredIptvProfile[]>('list_iptv_profiles'),
  upsertIptvProfile: (payload: {
    id: string
    name: string
    kind: IptvKind
    serverUrl: string
    username?: string
    password?: string
    epgUrl?: string
    userAgent?: string
  }) => call<StoredIptvProfile>('upsert_iptv_profile', { payload }),
  deleteIptvProfile: (profileId: string) => call<void>('delete_iptv_profile', { profileId }),
  syncIptvProfile: (profileId: string) => call<number>('sync_iptv_profile', { profileId }),
  syncIptvEpg: (profileId: string) => call<number>('sync_iptv_epg', { profileId }),
  searchIptvChannels: (params: {
    profileId?: string
    query?: string
    groupTitle?: string
    favoritesOnly?: boolean
    limit?: number
  }) => call<StoredIptvChannel[]>('search_iptv_channels', params),
  listIptvGroups: (profileId: string) => call<string[]>('list_iptv_groups', { profileId }),
  setIptvFavorite: (channelId: number, isFavorite: boolean) =>
    call<void>('set_iptv_favorite', { channelId, isFavorite }),
  getEpgNowNext: (profileId: string, channelTvgId: string, nowTs: number) =>
    call<EpgNowNext>('get_epg_now_next', { profileId, channelTvgId, nowTs }),
  playIptvChannel: (channel: StoredIptvChannel, profileUserAgent?: string) =>
    call<string>('play_iptv_channel', { channel, profileUserAgent }),

  listDownloads: () => call<StoredDownload[]>('list_downloads'),
  enqueueDownload: (payload: {
    pluginId: string
    nativeId: string
    title: string
    season?: number
    episode?: number
    streamLabel?: string
    kind: DownloadKind
    sourceUrl: string
    headers?: Record<string, string>
    subtitles?: SubtitleTrack[]
  }) => call<StoredDownload>('enqueue_download', { payload }),
  reorderDownloads: (orderedIds: string[]) =>
    call<void>('reorder_downloads', { orderedIds }),
  prepareLocalDownloadPlayback: (downloadId: string) =>
    call<LocalPlaybackBundle>('prepare_local_download_playback', { downloadId }),
  pauseDownload: (downloadId: string) => call<void>('pause_download', { downloadId }),
  resumeDownload: (downloadId: string) => call<StoredDownload>('resume_download', { downloadId }),
  deleteDownload: (downloadId: string) => call<void>('delete_download', { downloadId }),

  exportBackupJson: (includeSecrets: boolean) =>
    call<string>('export_backup_json', { includeSecrets }),
  importBackupJson: (json: string, mode: BackupImportMode) =>
    call<void>('import_backup_json', { json, mode }),

  getUpdatePreferences: () => call<UpdatePreferences>('get_update_preferences'),
  setAutoCheckUpdates: (enabled: boolean) => call<void>('set_auto_check_updates', { enabled }),
  skipAppUpdateVersion: (version?: string | null) =>
    call<void>('skip_app_update_version', { version: version ?? null }),
  checkAppUpdate: () => call<AppUpdateInfo>('check_app_update'),
  downloadAndOpenUpdate: (downloadUrl: string, fileName: string) =>
    call<string>('download_and_open_update', { downloadUrl, fileName }),
  openExternalUrl: (url: string) =>
    isTauriRuntime()
      ? call<void>('open_external_url', { url })
      : Promise.resolve(void window.open(url, '_blank', 'noopener,noreferrer')),
}

export async function listenUpdateProgress(
  handler: (progress: UpdateDownloadProgress) => void,
): Promise<UnlistenFn> {
  if (!isTauriRuntime()) {
    return () => {}
  }
  return listen<UpdateDownloadProgress>('vessel://update-progress', (msg) => handler(msg.payload))
}
