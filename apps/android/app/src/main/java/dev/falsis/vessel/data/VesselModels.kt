package dev.falsis.vessel.data

import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable
import kotlinx.serialization.json.JsonElement

@Serializable
data class MediaRef(
    val pluginId: String,
    val nativeId: String,
)

@Serializable
data class ExternalIds(
    val anilist: Long? = null,
    val mal: Long? = null,
    val tmdb: Long? = null,
    val imdb: String? = null,
    val tvdb: Long? = null,
    val kitsu: Long? = null,
)

@Serializable
data class MediaSummary(
    val nativeId: String,
    val pluginId: String? = null,
    val type: String = "series",
    val isAnime: Boolean = false,
    val title: String,
    val year: Int? = null,
    val poster: String? = null,
    val rating: Float? = null,
)

@Serializable
data class SeasonSummary(
    val id: String,
    val season: Int,
    val title: String? = null,
    val episodeCount: Int? = null,
    val poster: String? = null,
)

@Serializable
data class Episode(
    val streamId: String,
    val season: Int,
    val episode: Int,
    val absoluteEpisode: Int? = null,
    val title: String? = null,
    val overview: String? = null,
    val thumbnail: String? = null,
    val airDate: String? = null,
    val tags: List<String> = emptyList(),
)

@Serializable
data class MediaDetail(
    val nativeId: String,
    val pluginId: String? = null,
    val type: String = "series",
    val isAnime: Boolean = false,
    val title: String,
    val originalTitle: String? = null,
    val year: Int? = null,
    val synopsis: String? = null,
    val poster: String? = null,
    val backdrop: String? = null,
    val genres: List<String> = emptyList(),
    val rating: Float? = null,
    val status: String? = null,
    val contentRating: String = "sfw",
    val externalIds: ExternalIds = ExternalIds(),
    val seasons: List<SeasonSummary>? = null,
    val episodes: List<Episode> = emptyList(),
)

@Serializable
data class SubtitleSource(
    val kind: String = "url",
    val url: String? = null,
    val headers: Map<String, String> = emptyMap(),
    val content: String? = null,
)

@Serializable
data class SubtitleTrack(
    val id: String? = null,
    val source: SubtitleSource,
    val lang: String,
    val label: String,
    val format: String = "vtt",
    val isDefault: Boolean = false,
)

@Serializable
data class SkipSegment(
    val type: String,
    val startMs: Long,
    val endMs: Long? = null,
    val source: String? = null,
    val confidence: Int? = null,
)

@Serializable
data class StreamCandidate(
    val url: String,
    val kind: String = "auto",
    val quality: String? = null,
    val label: String,
    val language: String? = null,
    val audioType: String? = null,
    val priority: Int? = null,
    val isDefault: Boolean = false,
    val flags: List<String> = emptyList(),
    val headers: Map<String, String> = emptyMap(),
    val needsExtractor: Boolean = false,
    val subtitles: List<SubtitleTrack> = emptyList(),
    val skipSegments: List<SkipSegment> = emptyList(),
)

@Serializable
data class StreamResult(
    val streams: List<StreamCandidate> = emptyList(),
    val subtitles: List<SubtitleTrack> = emptyList(),
)

@Serializable
data class HomeSection(
    val id: String,
    val title: String,
    val layout: String = "poster",
    val items: List<MediaSummary> = emptyList(),
)

@Serializable
data class HomeShelfResult(
    val pluginId: String,
    val pluginName: String,
    val sections: List<HomeSection> = emptyList(),
    val error: String? = null,
)

@Serializable
data class PlaybackProgress(
    val media: MediaRef,
    val streamId: String,
    val canonicalId: String? = null,
    val season: Int,
    val episode: Int,
    val absoluteEpisode: Int? = null,
    val title: String? = null,
    val posterUrl: String? = null,
    val backdropUrl: String? = null,
    val episodeTitle: String? = null,
    val positionMs: Long,
    val durationMs: Long,
    val completed: Boolean = false,
    val lastWatchedAt: String = "",
)

@Serializable
data class ContinueWatchingEntry(
    val progress: PlaybackProgress,
    val mediaType: String = "series",
    val isNextEpisode: Boolean = false,
)

@Serializable
data class LibraryItem(
    val media: MediaRef,
    val canonicalId: String? = null,
    val mediaType: String = "series",
    val isAnime: Boolean = false,
    val title: String,
    val posterUrl: String? = null,
    val status: String = "watching",
    val addedAt: String = "",
    val updatedAt: String = "",
)

@Serializable
data class StoredDownload(
    val id: String,
    val pluginId: String,
    val nativeId: String,
    val title: String,
    val season: Int? = null,
    val episode: Int? = null,
    val streamLabel: String? = null,
    val kind: String = "hls",
    val sourceUrl: String,
    val localPath: String,
    val status: String = "queued",
    val bytesDownloaded: Long = 0,
    val totalBytes: Long? = null,
    val segmentsDone: Int = 0,
    val segmentsTotal: Int = 0,
    val queueOrder: Int = 0,
    val errorMessage: String? = null,
    val createdAt: String = "",
    val updatedAt: String = "",
)

@Serializable
data class LocalPlaybackBundle(
    val downloadId: String,
    val url: String,
    val kind: String = "hls",
    val subtitles: List<SubtitleTrack> = emptyList(),
)

@Serializable
data class PluginSettingField(
    val key: String,
    val label: String,
    val type: String,
    val required: Boolean = false,
    val placeholder: String? = null,
    val default: JsonElement? = null,
)

@Serializable
data class PluginManifest(
    val schemaVersion: Int = 1,
    val id: String,
    val name: String,
    val version: String,
    val author: String? = null,
    val description: String? = null,
    val entry: String = "index.ts",
    val icon: String? = null,
    val capabilities: List<String> = emptyList(),
    val contentTypes: List<String> = emptyList(),
    val languages: List<String> = emptyList(),
    val domains: List<String> = emptyList(),
    val settingsSchema: List<PluginSettingField> = emptyList(),
)

@Serializable
data class PluginListEntry(
    val manifest: PluginManifest,
    val repoId: String? = null,
    val sourceType: String = "local",
    val localPath: String = "",
    val sha256: String? = null,
    val enabled: Boolean = true,
    val allowDynamicDomains: Boolean = false,
    val allowedDomains: List<String> = emptyList(),
)

@Serializable
data class RepoPluginEntry(
    val id: String,
    val name: String,
    val version: String,
    val description: String? = null,
    val capabilities: List<String> = emptyList(),
    val contentTypes: List<String> = emptyList(),
    val languages: List<String> = emptyList(),
    val iconUrl: String? = null,
    val downloadUrl: String? = null,
    val sourcePath: String? = null,
)

@Serializable
data class RepoIndex(
    val schemaVersion: Int = 1,
    val id: String,
    val name: String,
    val description: String? = null,
    val author: String? = null,
    val iconUrl: String? = null,
    val plugins: List<RepoPluginEntry> = emptyList(),
)

@Serializable
data class ResolvedSource(
    val kind: String,
    val resolvedUrl: String,
    val badge: String = "community",
    val repo: RepoIndex? = null,
    val manifest: PluginManifest? = null,
    val sha256: String? = null,
)

@Serializable
data class InstallResult(
    val pluginId: String,
    val badge: String,
)

@Serializable
data class ThemeColors(
    val bgBase: String = "#1e1e2e",
    val bgSurface: String = "#181825",
    val bgElevated: String = "#313244",
    val textPrimary: String = "#cdd6f4",
    val textMuted: String = "#a6adc8",
    val accent: String = "#cba6f7",
    val accentSecondary: String = "#89b4fa",
    val border: String = "#45475a",
    val success: String = "#a6e3a1",
    val warning: String = "#f9e2af",
    val danger: String = "#f38ba8",
)

@Serializable
data class ThemeSpec(
    val schemaVersion: Int = 1,
    val id: String = "catppuccin-mocha",
    val name: String = "Catppuccin Mocha",
    val author: String = "Vessel",
    val isDark: Boolean = true,
    val colors: ThemeColors = ThemeColors(),
)

@Serializable
data class HomePreferences(
    val heroBanner: String = "full",
    val showContinueWatching: Boolean = true,
    val continueWatchingLimit: Int = 12,
    val selectedPlugins: List<String>? = null,
)

@Serializable
data class PlaybackPreferences(
    val clickBehavior: String = "quick_start_best",
    val autoNextDelaySecs: Int = 10,
    val watchedThresholdPercent: Int = 90,
)

@Serializable
data class SubtitlePreferences(
    val fontSizePx: Int = 28,
    val textColor: String = "#ffffff",
    val backgroundOpacity: Int = 72,
    val outlineStyle: String = "outline",
    val bottomOffsetPx: Int = 56,
    val bold: Boolean = false,
)

@Serializable
data class UiPreferences(
    val home: HomePreferences = HomePreferences(),
    val playback: PlaybackPreferences = PlaybackPreferences(),
    val subtitles: SubtitlePreferences = SubtitlePreferences(),
)

@Serializable
data class EngineInfo(
    val rootDir: String,
    val downloadsDir: String,
    val pluginsDir: String,
    val themesDir: String,
    val localesDir: String,
    val backupsDir: String,
    val proxyBaseUrl: String,
    val alwaysProxy: Boolean = false,
    val metadataEnabled: Boolean = true,
)

@Serializable
data class StoredIptvProfile(
    val id: String,
    val name: String,
    val kind: String = "m3u",
    val serverUrl: String,
    val username: String? = null,
    val epgUrl: String? = null,
    val userAgent: String? = null,
    val lastSyncedAt: String? = null,
)

@Serializable
data class StoredIptvChannel(
    val id: Long,
    val profileId: String,
    val tvgId: String? = null,
    val name: String,
    val groupTitle: String? = null,
    val logoUrl: String? = null,
    val streamUrl: String,
    val userAgent: String? = null,
    val isFavorite: Boolean = false,
)

@Serializable
data class EpgProgram(
    val title: String,
    val description: String? = null,
    val startTs: Long,
    val endTs: Long,
)

@Serializable
data class EpgNowNext(
    val now: EpgProgram? = null,
    val next: EpgProgram? = null,
)

@Serializable
data class DomainPermissionPrompt(
    val requestId: String = "",
    val pluginId: String,
    val domain: String,
    val url: String? = null,
)

@Serializable
data class SearchResultEvent(
    val searchId: String,
    val pluginId: String = "",
    val items: List<MediaSummary> = emptyList(),
    val hasMore: Boolean = false,
    val error: String? = null,
    val done: Boolean = false,
)

@Serializable
data class DownloadProgressSnapshot(
    val downloadId: String,
    val status: String,
    val bytesDownloaded: Long = 0,
    val totalBytes: Long? = null,
    val segmentsDone: Int = 0,
    val segmentsTotal: Int = 0,
    val errorMessage: String? = null,
)

@Serializable
data class GetSkipSegmentsPayload(
    val pluginId: String? = null,
    val nativeId: String? = null,
    val mediaType: String = "series",
    val isAnime: Boolean = false,
    val externalIds: ExternalIds = ExternalIds(),
    val season: Int = 1,
    val episode: Int = 1,
    val absoluteEpisode: Int? = null,
    val durationMs: Long? = null,
    val pluginSegments: List<SkipSegment> = emptyList(),
)

@Serializable
data class PlayCandidatePayload(
    val pluginId: String,
    val nativeId: String,
    val canonicalId: String? = null,
    val season: Int = 1,
    val episode: Int = 1,
    val absoluteEpisode: Int? = null,
    val mediaType: String = "series",
    val candidate: StreamCandidate,
    val extraSubtitles: List<SubtitleTrack> = emptyList(),
    val startPositionMs: Long? = null,
    val title: String? = null,
)

@Serializable
data class EnqueueDownloadPayload(
    val pluginId: String,
    val nativeId: String,
    val title: String,
    val season: Int? = null,
    val episode: Int? = null,
    val streamLabel: String? = null,
    val kind: String = "hls",
    val sourceUrl: String,
    val headers: Map<String, String> = emptyMap(),
    val subtitles: List<SubtitleTrack> = emptyList(),
)

@Serializable
data class UpsertIptvProfilePayload(
    val id: String,
    val name: String,
    val kind: String = "m3u",
    val serverUrl: String,
    val username: String? = null,
    val password: String? = null,
    val epgUrl: String? = null,
    val userAgent: String? = null,
)
