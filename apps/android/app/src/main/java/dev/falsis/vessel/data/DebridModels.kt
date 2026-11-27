package dev.falsis.vessel.data

import kotlinx.serialization.Serializable

@Serializable
data class TorrentMeta(
    val infoHash: String? = null,
    val fileIndex: Int? = null,
    val filename: String? = null,
    val season: Int? = null,
    val episode: Int? = null,
    val seeders: Int? = null,
    val sizeBytes: Long? = null,
    val trackerSource: String? = null,
    val codecs: List<String> = emptyList(),
    val hdr: String? = null,
    val trackers: List<String> = emptyList(),
)

@Serializable
data class DebridCacheInfo(
    val target: String,
    val label: String,
    val state: String, // "cached" | "uncached" | "unknown"
)

@Serializable
data class DebridAccountStatus(
    val target: String,
    val label: String,
    val isBuiltin: Boolean = true,
    val supportsBulkCacheCheck: Boolean = false,
    val enabled: Boolean = false,
    val authenticated: Boolean = false,
    val username: String? = null,
    val email: String? = null,
    val isPremium: Boolean = false,
    val expiresAt: String? = null,
    val pointsOrQuota: String? = null,
    val error: String? = null,
)

@Serializable
data class DebridCloudItem(
    val id: String,
    val target: String,
    val providerLabel: String,
    val filename: String,
    val infoHash: String? = null,
    val sizeBytes: Long? = null,
    val downloadedBytes: Long? = null,
    val status: String, // "queued" | "downloading" | "uploading" | "ready" | "error"
    val progressPct: Float = 0f,
    val speedBps: Long? = null,
    val seeders: Int? = null,
    val etaSeconds: Long? = null,
    val links: List<String> = emptyList(),
    val addedAt: String? = null,
)

@Serializable
data class DebridPreferences(
    val priorityOrder: List<String> = emptyList(),
    val onlyCached: Boolean = false,
    val autoFallback: Boolean = true,
)

@Serializable
data class DebridOverview(
    val preferences: DebridPreferences = DebridPreferences(),
    val accounts: List<DebridAccountStatus> = emptyList(),
    val disabledStreamAddons: List<String> = emptyList(),
)

@Serializable
data class RealDebridDeviceAuthStart(
    val deviceCode: String,
    val userCode: String,
    val verificationUrl: String,
    val directVerificationUrl: String? = null,
    val intervalSeconds: Long = 5L,
    val expiresInSeconds: Long = 600L,
)

@Serializable
data class RealDebridDeviceAuthPoll(
    val authorized: Boolean = false,
    val pending: Boolean = false,
    val username: String? = null,
    val error: String? = null,
)

@Serializable
data class ResolveDebridRequest(
    val url: String,
    val torrent: TorrentMeta? = null,
    val preferredTarget: String? = null,
    val allowUncachedQueue: Boolean = false,
    val season: Int? = null,
    val episode: Int? = null,
)

@Serializable
data class DebridResolution(
    val status: String, // "ready" | "needs_confirmation" | "no_provider_configured"
    val target: String? = null,
    val providerLabel: String? = null,
    val url: String? = null,
    val headers: Map<String, String> = emptyMap(),
    val filename: String? = null,
    val sizeBytes: Long? = null,
    val expiresAt: String? = null,
    val infoHash: String? = null,
    val cloudStatus: String? = null,
    val progressPct: Float? = null,
    val speedBps: Long? = null,
    val seeders: Int? = null,
    val torrentId: String? = null,
    val message: String? = null,
)

@Serializable
data class ResolvedHttpLink(
    val url: String,
    val headers: Map<String, String> = emptyMap(),
    val filename: String? = null,
    val sizeBytes: Long? = null,
    val expiresAt: String? = null,
)

@Serializable
data class PlaybackStats(
    val networkSpeedBps: Long = 0L,
    val totalBytesTransferred: Long = 0L,
    val bufferSeconds: Double? = null,
    val cacheSpeedBps: Long? = null,
    val videoBitrateBps: Long? = null,
    val audioBitrateBps: Long? = null,
    val width: Int? = null,
    val height: Int? = null,
    val fps: Double? = null,
    val videoCodec: String? = null,
    val audioCodec: String? = null,
    val fileSizeBytes: Long? = null,
    val isDebridStream: Boolean = false,
)
