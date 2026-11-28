package dev.falsis.vessel.ui.screens.player

import android.content.Context
import android.os.Handler
import android.os.Looper
import android.view.ViewGroup
import android.widget.FrameLayout
import androidx.annotation.OptIn
import androidx.media3.common.util.UnstableApi
import androidx.media3.ui.AspectRatioFrameLayout
import `is`.xyz.mpv.BaseMPVView
import `is`.xyz.mpv.MPV
import `is`.xyz.mpv.MPVNode
import java.io.File
import java.util.Locale
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow

private const val MPV_DEFAULT_USER_AGENT =
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36"

private class VesselMpvSurfaceView(
    context: Context,
    private val observer: MPV.EventObserver,
) : BaseMPVView(context, null) {
    var initialized = false
        private set

    override fun initOptions() {
        mpv.setOptionString("profile", "fast")
        mpv.setOptionString("vo", "gpu")
        mpv.setOptionString("gpu-context", "android")
        mpv.setOptionString("opengl-es", "yes")
        mpv.setOptionString("hwdec", "auto-safe")
        mpv.setOptionString("ao", "audiotrack,opensles")
        mpv.setOptionString("tls-verify", "no")
        mpv.setOptionString("ytdl", "no")
        mpv.setOptionString("keep-open", "always")
        mpv.setOptionString("save-position-on-quit", "no")
        mpv.setOptionString("sub-auto", "fuzzy")
        mpv.setOptionString("sub-ass-override", "scale")
        mpv.setOptionString("demuxer-max-bytes", "67108864")
        mpv.setOptionString("demuxer-max-back-bytes", "25165824")
        mpv.setOptionString("user-agent", MPV_DEFAULT_USER_AGENT)
    }

    override fun postInitOptions() {
        mpv.addObserver(observer)
    }

    override fun observeProperties() {
        mpv.observeProperty("pause", MPV.mpvFormat.MPV_FORMAT_FLAG)
        mpv.observeProperty("paused-for-cache", MPV.mpvFormat.MPV_FORMAT_FLAG)
        mpv.observeProperty("core-idle", MPV.mpvFormat.MPV_FORMAT_FLAG)
        mpv.observeProperty("time-pos", MPV.mpvFormat.MPV_FORMAT_DOUBLE)
        mpv.observeProperty("duration", MPV.mpvFormat.MPV_FORMAT_DOUBLE)
        mpv.observeProperty("demuxer-cache-duration", MPV.mpvFormat.MPV_FORMAT_DOUBLE)
        mpv.observeProperty("track-list", MPV.mpvFormat.MPV_FORMAT_NODE)
    }

    fun ensureInitialized(context: Context) {
        if (initialized) return
        val configDir = File(context.filesDir, "mpv-config").apply { mkdirs() }.absolutePath
        val cacheDir = File(context.cacheDir, "mpv-cache").apply { mkdirs() }.absolutePath
        initialize(configDir, cacheDir)
        initialized = true
    }
}

/**
 * Native `libmpv` + `libass` playback controller powered by `mpv-android-lib`.
 * Handles styled `.ass`/`.ssa` subtitles, MKV/Hi10p/PGS tracks, and hardware decoding on VOD.
 */
@OptIn(UnstableApi::class)
class MpvKitController(private val context: Context) : VesselPlayerController {
    override val engine: PlayerBackendEngine = PlayerBackendEngine.MPVKIT

    private val _state = MutableStateFlow(PlayerPlaybackState())
    override val state: StateFlow<PlayerPlaybackState> = _state.asStateFlow()

    private val _audioTracks = MutableStateFlow<List<PlayerTrackItem>>(emptyList())
    override val audioTracks: StateFlow<List<PlayerTrackItem>> = _audioTracks.asStateFlow()

    private val _subtitleTracks = MutableStateFlow<List<PlayerTrackItem>>(emptyList())
    override val subtitleTracks: StateFlow<List<PlayerTrackItem>> = _subtitleTracks.asStateFlow()

    private val _stats = MutableStateFlow(PlayerStatsSnapshot())
    override val stats: StateFlow<PlayerStatsSnapshot> = _stats.asStateFlow()

    private val mainHandler = Handler(Looper.getMainLooper())
    private var released = false
    private var surfaceView: VesselMpvSurfaceView? = null
    private var pendingUrl: String? = null
    private var pendingHeaders: Map<String, String> = emptyMap()
    private var pendingStartMs: Long = 0L
    private var pendingIsLive: Boolean = false
    private var pendingResizeMode: Int = AspectRatioFrameLayout.RESIZE_MODE_FIT
    private var pendingSpeed: Float = 1.0f
    private var pendingVolume: Float = 1.0f
    private var pausedForCache: Boolean = false
    private var pausedByUser: Boolean = false

    private val eventObserver = object : MPV.EventObserver {
        override fun eventProperty(property: String) {
            if (property == "track-list") refreshTracks()
        }

        override fun eventProperty(property: String, value: Long) = Unit

        override fun eventProperty(property: String, value: Boolean) {
            when (property) {
                "pause" -> {
                    pausedByUser = value
                    _state.value = _state.value.copy(
                        isPlaying = !value && !pausedForCache,
                    )
                }
                "paused-for-cache" -> {
                    pausedForCache = value
                    _state.value = _state.value.copy(
                        isBuffering = value,
                        isPlaying = !pausedByUser && !value,
                    )
                }
                "core-idle" -> {
                    if (!pausedByUser && value) {
                        _state.value = _state.value.copy(isBuffering = true)
                    }
                }
            }
        }

        override fun eventProperty(property: String, value: String) = Unit

        override fun eventProperty(property: String, value: Double) {
            val cur = _state.value
            when (property) {
                "time-pos" -> {
                    val posMs = (value * 1000.0).toLong().coerceAtLeast(0L)
                    _state.value = cur.copy(
                        positionMs = posMs,
                        isBuffering = false,
                    )
                }
                "duration" -> {
                    val durMs = (value * 1000.0).toLong().coerceAtLeast(0L)
                    if (durMs > 0L) {
                        _state.value = cur.copy(durationMs = durMs)
                    }
                }
                "demuxer-cache-duration" -> {
                    val curPos = _state.value.positionMs
                    val cachedMs = (value * 1000.0).toLong().coerceAtLeast(0L)
                    _state.value = _state.value.copy(bufferedPositionMs = curPos + cachedMs)
                }
            }
        }

        override fun eventProperty(property: String, value: MPVNode) {
            if (property == "track-list") {
                parseTrackListNode(value)
            }
        }

        override fun event(eventId: Int, data: MPVNode) {
            when (eventId) {
                MPV.mpvEvent.MPV_EVENT_START_FILE -> {
                    _state.value = _state.value.copy(
                        isBuffering = true,
                        error = null,
                        unrecognizedFormat = false,
                    )
                }
                MPV.mpvEvent.MPV_EVENT_FILE_LOADED,
                MPV.mpvEvent.MPV_EVENT_PLAYBACK_RESTART -> {
                    val mpv = surfaceView?.mpv
                    if (mpv != null) {
                        mpv.setPropertyBoolean("pause", false)
                        mpv.setPropertyDouble("speed", pendingSpeed.toDouble())
                        mpv.setPropertyDouble("volume", (pendingVolume * 100.0).coerceIn(0.0, 100.0))
                    }
                    _state.value = _state.value.copy(
                        isPlaying = !pausedByUser,
                        isBuffering = false,
                        error = null,
                        unrecognizedFormat = false,
                    )
                    refreshTracks()
                }
                MPV.mpvEvent.MPV_EVENT_END_FILE -> {
                    val reason = runCatching { data?.get("reason")?.asString() }.getOrNull()
                    if (reason == "error") {
                        val errMsg = runCatching { data?.get("file_error")?.asString() }.getOrNull()
                            ?: "libmpv playback error"
                        _state.value = _state.value.copy(
                            isPlaying = false,
                            isBuffering = false,
                            error = errMsg,
                            unrecognizedFormat = errMsg.contains("format", ignoreCase = true) ||
                                errMsg.contains("demuxer", ignoreCase = true),
                        )
                    }
                }
            }
        }
    }

    private val statsPollRunnable = object : Runnable {
        override fun run() {
            if (released) return
            pollStats()
            mainHandler.postDelayed(this, 750L)
        }
    }

    init {
        mainHandler.postDelayed(statsPollRunnable, 750L)
    }

    fun getOrCreateView(viewContext: Context): BaseMPVView {
        surfaceView?.let { existing ->
            return existing
        }
        val view = VesselMpvSurfaceView(viewContext, eventObserver).apply {
            layoutParams = FrameLayout.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT,
                ViewGroup.LayoutParams.MATCH_PARENT,
            )
            isFocusable = false
            isFocusableInTouchMode = false
            ensureInitialized(context.applicationContext)
        }
        surfaceView = view
        applyResizeModeInternal(pendingResizeMode)
        pendingUrl?.let { queuedUrl ->
            issueLoadCommand(queuedUrl, pendingHeaders, pendingStartMs, pendingIsLive)
        }
        return view
    }

    override fun load(
        url: String,
        headers: Map<String, String>,
        startPositionMs: Long,
        isLive: Boolean,
    ) {
        pendingUrl = url
        pendingHeaders = headers
        pendingStartMs = startPositionMs
        pendingIsLive = isLive
        pausedByUser = false
        _state.value = _state.value.copy(
            isBuffering = true,
            error = null,
            unrecognizedFormat = false,
        )
        if (surfaceView?.initialized == true) {
            issueLoadCommand(url, headers, startPositionMs, isLive)
        }
    }

    private fun issueLoadCommand(
        url: String,
        headers: Map<String, String>,
        startPositionMs: Long,
        isLive: Boolean,
    ) {
        val mpv = surfaceView?.mpv ?: return
        val ua = headers.entries.firstOrNull { it.key.equals("user-agent", ignoreCase = true) }?.value
            ?: MPV_DEFAULT_USER_AGENT
        mpv.setOptionString("user-agent", ua)
        val extraHeaders = headers
            .filterKeys { !it.equals("user-agent", ignoreCase = true) }
            .entries
            .joinToString(",") { "${it.key}: ${it.value}" }
        mpv.setOptionString("http-header-fields", extraHeaders)

        if (!isLive && startPositionMs > 2000L) {
            mpv.setOptionString("start", String.format(Locale.US, "%.2f", startPositionMs / 1000.0))
        } else {
            mpv.setOptionString("start", "none")
        }
        mpv.setPropertyBoolean("pause", false)
        mpv.command("loadfile", url, "replace")
    }

    override fun play() {
        pausedByUser = false
        surfaceView?.mpv?.setPropertyBoolean("pause", false)
        _state.value = _state.value.copy(isPlaying = true)
    }

    override fun pause() {
        pausedByUser = true
        surfaceView?.mpv?.setPropertyBoolean("pause", true)
        _state.value = _state.value.copy(isPlaying = false)
    }

    override fun seekTo(positionMs: Long) {
        val sec = (positionMs.coerceAtLeast(0L)) / 1000.0
        surfaceView?.mpv?.setPropertyDouble("time-pos", sec)
        _state.value = _state.value.copy(positionMs = positionMs.coerceAtLeast(0L))
    }

    override fun setSpeed(speed: Float) {
        pendingSpeed = speed.coerceIn(0.25f, 4.0f)
        surfaceView?.mpv?.setPropertyDouble("speed", pendingSpeed.toDouble())
    }

    override fun setVolume(volume: Float) {
        pendingVolume = volume.coerceIn(0f, 1f)
        surfaceView?.mpv?.setPropertyDouble("volume", (pendingVolume * 100.0))
    }

    override fun selectAudioTrack(id: String) {
        val numId = id.toIntOrNull() ?: return
        surfaceView?.mpv?.setPropertyInt("aid", numId)
        refreshTracks()
    }

    override fun selectSubtitleTrack(id: String?) {
        val mpv = surfaceView?.mpv ?: return
        if (id == null) {
            mpv.setPropertyString("sid", "no")
        } else {
            mpv.setPropertyString("sid", id)
        }
        refreshTracks()
    }

    override fun addExternalSubtitle(urlOrPath: String) {
        if (urlOrPath.isBlank()) return
        surfaceView?.mpv?.command("sub-add", urlOrPath, "select")
        refreshTracks()
    }

    override fun setSubtitleDelayMs(delayMs: Long) {
        surfaceView?.mpv?.setPropertyDouble("sub-delay", delayMs / 1000.0)
    }

    override fun setResizeMode(mode: Int) {
        pendingResizeMode = mode
        applyResizeModeInternal(mode)
    }

    private fun applyResizeModeInternal(mode: Int) {
        val mpv = surfaceView?.mpv ?: return
        when (mode) {
            AspectRatioFrameLayout.RESIZE_MODE_ZOOM -> {
                mpv.setPropertyString("keepaspect", "yes")
                mpv.setPropertyDouble("panscan", 1.0)
            }
            AspectRatioFrameLayout.RESIZE_MODE_FILL -> {
                mpv.setPropertyDouble("panscan", 0.0)
                mpv.setPropertyString("keepaspect", "no")
            }
            else -> {
                mpv.setPropertyDouble("panscan", 0.0)
                mpv.setPropertyString("keepaspect", "yes")
            }
        }
    }

    private fun refreshTracks() {
        val mpv = surfaceView?.mpv ?: return
        val node = runCatching { mpv.getPropertyNode("track-list") }.getOrNull() ?: return
        parseTrackListNode(node)
    }

    private fun parseTrackListNode(node: MPVNode) {
        val items = runCatching { node.asArray() }.getOrNull() ?: return
        val nextAudio = mutableListOf<PlayerTrackItem>()
        val nextSubs = mutableListOf<PlayerTrackItem>()
        var audioOrdinal = 1
        var subOrdinal = 1

        for (entry in items) {
            val type = runCatching { entry.get("type")?.asString() }.getOrNull() ?: continue
            val idLong = runCatching { entry.get("id")?.asInt() }.getOrNull() ?: continue
            val lang = runCatching { entry.get("lang")?.asString() }.getOrNull()
                ?.takeIf { it.isNotBlank() && it != "und" }?.uppercase(Locale.US)
            val titleRaw = runCatching { entry.get("title")?.asString() }.getOrNull()
                ?.takeIf { it.isNotBlank() }
            val codec = runCatching { entry.get("codec")?.asString() }.getOrNull()
                ?.takeIf { it.isNotBlank() }?.uppercase(Locale.US)
            val selected = runCatching { entry.get("selected")?.asBoolean() }.getOrNull() == true

            when (type) {
                "audio" -> {
                    val ch = runCatching { entry.get("demux-channel-count")?.asInt() }.getOrNull()
                    val title = titleRaw ?: lang?.let { "Audio $audioOrdinal ($it)" } ?: "Audio $audioOrdinal"
                    val meta = listOfNotNull(
                        lang,
                        ch?.let { "${it}ch" },
                        codec,
                    ).joinToString(" · ")
                    nextAudio += PlayerTrackItem(
                        id = idLong.toString(),
                        label = title,
                        meta = meta,
                        selected = selected,
                    )
                    audioOrdinal++
                }
                "sub" -> {
                    val ext = runCatching { entry.get("external")?.asBoolean() }.getOrNull() == true
                    val title = titleRaw ?: lang?.let { "Subtitle $subOrdinal ($it)" } ?: "Subtitle $subOrdinal"
                    val meta = listOfNotNull(
                        lang,
                        codec,
                        if (ext) "External (libass)" else "Embedded",
                    ).joinToString(" · ")
                    nextSubs += PlayerTrackItem(
                        id = idLong.toString(),
                        label = title,
                        meta = meta,
                        selected = selected,
                    )
                    subOrdinal++
                }
            }
        }

        _audioTracks.value = nextAudio
        _subtitleTracks.value = nextSubs
    }

    private fun pollStats() {
        val view = surfaceView ?: return
        if (!view.initialized) return
        val mpv = view.mpv
        val w = runCatching { mpv.getPropertyInt("width") }.getOrNull()?.takeIf { it > 0 }
        val h = runCatching { mpv.getPropertyInt("height") }.getOrNull()?.takeIf { it > 0 }
        val res = if (w != null && h != null) "${w}x${h}" else null
        val vBitrate = runCatching { mpv.getPropertyInt("video-bitrate") }.getOrNull()?.takeIf { it > 0 }?.toLong()
        val aBitrate = runCatching { mpv.getPropertyInt("audio-bitrate") }.getOrNull()?.takeIf { it > 0 }?.toLong()
        val vCodec = runCatching { mpv.getPropertyString("video-codec") }.getOrNull()?.takeIf { it.isNotBlank() }
        val aCodec = runCatching { mpv.getPropertyString("audio-codec") }.getOrNull()?.takeIf { it.isNotBlank() }
        val cacheDur = runCatching { mpv.getPropertyDouble("demuxer-cache-duration") }.getOrNull()?.takeIf { it >= 0.0 }
        _stats.value = PlayerStatsSnapshot(
            videoBitrateBps = vBitrate,
            audioBitrateBps = aBitrate,
            resolution = res,
            videoCodec = vCodec,
            audioCodec = aCodec,
            bufferSeconds = cacheDur,
        )
    }

    override fun release() {
        if (released) return
        released = true
        mainHandler.removeCallbacks(statsPollRunnable)
        surfaceView?.let { view ->
            runCatching { view.mpv.removeObserver(eventObserver) }
            runCatching { if (view.initialized) view.destroy() }
        }
        surfaceView = null
    }
}
