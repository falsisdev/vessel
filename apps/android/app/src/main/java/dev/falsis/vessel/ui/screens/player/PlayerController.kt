package dev.falsis.vessel.ui.screens.player

import kotlinx.coroutines.flow.StateFlow

/** Active Android playback backend selected via `Auto` or user preference. */
enum class PlayerBackendEngine {
    EXOPLAYER,
    MPVKIT,
}

/** Normalized audio or embedded subtitle track exposed by any Android player backend. */
data class PlayerTrackItem(
    val id: String,
    val label: String,
    val meta: String,
    val selected: Boolean,
)

/** Unified live playback state observed by the Compose Player HUD. */
data class PlayerPlaybackState(
    val isPlaying: Boolean = false,
    val isBuffering: Boolean = true,
    val positionMs: Long = 0L,
    val bufferedPositionMs: Long = 0L,
    val durationMs: Long = 0L,
    val error: String? = null,
    val unrecognizedFormat: Boolean = false,
)

/** Unified live stream telemetry used by the Player Stats HUD overlay. */
data class PlayerStatsSnapshot(
    val videoBitrateBps: Long? = null,
    val audioBitrateBps: Long? = null,
    val resolution: String? = null,
    val videoCodec: String? = null,
    val audioCodec: String? = null,
    val bufferSeconds: Double? = null,
)

/**
 * Unified playback controller implemented by both [ExoPlayerController] (Media3 + FFmpeg audio)
 * and [MpvKitController] (libmpv + libass for styled `.ass`/`.ssa` & MKV playback).
 */
interface VesselPlayerController {
    val engine: PlayerBackendEngine
    val state: StateFlow<PlayerPlaybackState>
    val audioTracks: StateFlow<List<PlayerTrackItem>>
    val subtitleTracks: StateFlow<List<PlayerTrackItem>>
    val stats: StateFlow<PlayerStatsSnapshot>

    fun load(
        url: String,
        headers: Map<String, String>,
        startPositionMs: Long,
        isLive: Boolean,
    )

    fun play()
    fun pause()
    fun seekTo(positionMs: Long)
    fun setSpeed(speed: Float)
    fun setVolume(volume: Float)
    fun selectAudioTrack(id: String)
    fun selectSubtitleTrack(id: String?)
    fun addExternalSubtitle(urlOrPath: String)
    fun setSubtitleDelayMs(delayMs: Long)
    fun setResizeMode(mode: Int)
    fun release()
}
