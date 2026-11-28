package dev.falsis.vessel.ui.screens.player

import android.content.Context
import android.os.Handler
import android.os.Looper
import androidx.annotation.OptIn
import androidx.media3.common.AudioAttributes
import androidx.media3.common.C
import androidx.media3.common.MediaItem
import androidx.media3.common.MimeTypes
import androidx.media3.common.ParserException
import androidx.media3.common.PlaybackException
import androidx.media3.common.Player
import androidx.media3.common.TrackGroup
import androidx.media3.common.TrackSelectionOverride
import androidx.media3.common.Tracks
import androidx.media3.common.util.UnstableApi
import androidx.media3.datasource.DefaultHttpDataSource
import androidx.media3.exoplayer.DefaultRenderersFactory
import androidx.media3.exoplayer.ExoPlayer
import androidx.media3.exoplayer.source.DefaultMediaSourceFactory
import androidx.media3.exoplayer.source.UnrecognizedInputFormatException
import androidx.media3.extractor.DefaultExtractorsFactory
import androidx.media3.extractor.ts.DefaultTsPayloadReaderFactory
import androidx.media3.ui.PlayerView
import java.util.Locale
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow

private const val EXO_DEFAULT_USER_AGENT =
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36"

private data class ExoTrackRef(
    val group: TrackGroup,
    val trackIndex: Int,
)

@OptIn(UnstableApi::class)
class ExoPlayerController(context: Context) : VesselPlayerController {
    override val engine: PlayerBackendEngine = PlayerBackendEngine.EXOPLAYER

    private val _state = MutableStateFlow(PlayerPlaybackState())
    override val state: StateFlow<PlayerPlaybackState> = _state.asStateFlow()

    private val _audioTracks = MutableStateFlow<List<PlayerTrackItem>>(emptyList())
    override val audioTracks: StateFlow<List<PlayerTrackItem>> = _audioTracks.asStateFlow()

    private val _subtitleTracks = MutableStateFlow<List<PlayerTrackItem>>(emptyList())
    override val subtitleTracks: StateFlow<List<PlayerTrackItem>> = _subtitleTracks.asStateFlow()

    private val _stats = MutableStateFlow(PlayerStatsSnapshot())
    override val stats: StateFlow<PlayerStatsSnapshot> = _stats.asStateFlow()

    private val trackRefMap = mutableMapOf<String, ExoTrackRef>()
    private var attachedView: PlayerView? = null
    private val mainHandler = Handler(Looper.getMainLooper())
    private var released = false

    private val httpFactory = DefaultHttpDataSource.Factory()
        .setAllowCrossProtocolRedirects(true)
        .setConnectTimeoutMs(15_000)
        .setReadTimeoutMs(25_000)
        .setUserAgent(EXO_DEFAULT_USER_AGENT)

    val player: ExoPlayer = run {
        val extractorsFactory = DefaultExtractorsFactory()
            .setTsExtractorFlags(
                DefaultTsPayloadReaderFactory.FLAG_ALLOW_NON_IDR_KEYFRAMES or
                    DefaultTsPayloadReaderFactory.FLAG_DETECT_ACCESS_UNITS,
            )
            .setTsExtractorTimestampSearchBytes(4000 * 188)
        val renderersFactory = DefaultRenderersFactory(context)
            .setExtensionRendererMode(DefaultRenderersFactory.EXTENSION_RENDERER_MODE_ON)
        ExoPlayer.Builder(context, renderersFactory)
            .setMediaSourceFactory(DefaultMediaSourceFactory(httpFactory, extractorsFactory))
            .setAudioAttributes(
                AudioAttributes.Builder()
                    .setUsage(C.USAGE_MEDIA)
                    .setContentType(C.AUDIO_CONTENT_TYPE_MOVIE)
                    .build(),
                true,
            )
            .setHandleAudioBecomingNoisy(true)
            .setWakeMode(C.WAKE_MODE_NETWORK)
            .build()
            .apply { playWhenReady = true }
    }

    private val pollRunnable = object : Runnable {
        override fun run() {
            if (released) return
            refreshPositionAndStats()
            mainHandler.postDelayed(this, 250L)
        }
    }

    private val listener = object : Player.Listener {
        override fun onPlaybackStateChanged(playbackState: Int) {
            val cur = _state.value
            val dur = player.duration.coerceAtLeast(0L)
            _state.value = cur.copy(
                isBuffering = playbackState == Player.STATE_BUFFERING,
                durationMs = if (dur > 0L) dur else cur.durationMs,
                error = if (playbackState == Player.STATE_READY) null else cur.error,
                unrecognizedFormat = if (playbackState == Player.STATE_READY) false else cur.unrecognizedFormat,
            )
            refreshPositionAndStats()
        }

        override fun onIsPlayingChanged(playing: Boolean) {
            val cur = _state.value
            _state.value = cur.copy(
                isPlaying = playing,
                isBuffering = if (playing) false else cur.isBuffering,
                error = if (playing) null else cur.error,
                unrecognizedFormat = if (playing) false else cur.unrecognizedFormat,
            )
        }

        override fun onTracksChanged(tracks: Tracks) {
            syncTracks(tracks)
        }

        override fun onPlayerError(error: PlaybackException) {
            if (error.errorCode == PlaybackException.ERROR_CODE_BEHIND_LIVE_WINDOW) {
                _state.value = _state.value.copy(
                    isBuffering = true,
                    error = null,
                    unrecognizedFormat = false,
                )
                player.seekToDefaultPosition()
                player.prepare()
                player.playWhenReady = true
                return
            }

            val unrecognized =
                error.errorCode == PlaybackException.ERROR_CODE_PARSING_CONTAINER_UNSUPPORTED ||
                    error.errorCode == PlaybackException.ERROR_CODE_PARSING_CONTAINER_MALFORMED ||
                    generateSequence<Throwable>(error) { it.cause }
                        .any { it is UnrecognizedInputFormatException || it is ParserException }

            _state.value = _state.value.copy(
                isPlaying = false,
                isBuffering = false,
                error = error.localizedMessage ?: error.message ?: "Playback error",
                unrecognizedFormat = unrecognized,
            )
        }
    }

    init {
        player.addListener(listener)
        mainHandler.postDelayed(pollRunnable, 250L)
    }

    fun attachPlayerView(view: PlayerView) {
        attachedView = view
        if (view.player !== player) {
            view.player = player
        }
    }

    fun refreshPositionAndStats() {
        val pos = player.currentPosition.coerceAtLeast(0L)
        val buf = player.bufferedPosition.coerceAtLeast(0L)
        val dur = player.duration.coerceAtLeast(0L)
        val cur = _state.value
        _state.value = cur.copy(
            positionMs = pos,
            bufferedPositionMs = buf,
            durationMs = if (dur > 0L) dur else cur.durationMs,
        )
        val vf = player.videoFormat
        val af = player.audioFormat
        val bufSec = ((buf - pos).coerceAtLeast(0L)) / 1000.0
        val res = if (vf != null && vf.width > 0 && vf.height > 0) "${vf.width}x${vf.height}" else null
        _stats.value = PlayerStatsSnapshot(
            videoBitrateBps = vf?.bitrate?.takeIf { it > 0 }?.toLong(),
            audioBitrateBps = af?.bitrate?.takeIf { it > 0 }?.toLong(),
            resolution = res,
            videoCodec = vf?.codecs ?: vf?.sampleMimeType,
            audioCodec = af?.codecs ?: af?.sampleMimeType,
            bufferSeconds = bufSec,
        )
    }

    fun loadWithOptions(
        url: String,
        headers: Map<String, String>,
        startPositionMs: Long,
        isLive: Boolean,
        forceHlsMime: Boolean = false,
        forceDashMime: Boolean = false,
        disableEmbeddedText: Boolean = false,
    ) {
        _state.value = _state.value.copy(
            isBuffering = true,
            error = null,
            unrecognizedFormat = false,
        )

        if (headers.isNotEmpty()) {
            val ua = headers.entries.firstOrNull { it.key.equals("user-agent", ignoreCase = true) }?.value
                ?: EXO_DEFAULT_USER_AGENT
            val filtered = headers.filterKeys { !it.equals("user-agent", ignoreCase = true) }
            httpFactory.setUserAgent(ua)
            httpFactory.setDefaultRequestProperties(filtered)
        } else {
            httpFactory.setUserAgent(EXO_DEFAULT_USER_AGENT)
            httpFactory.setDefaultRequestProperties(emptyMap())
        }

        val lower = url.lowercase(Locale.US)
        val mimeType = when {
            forceHlsMime || lower.contains(".m3u8") -> MimeTypes.APPLICATION_M3U8
            forceDashMime || lower.contains(".mpd") -> MimeTypes.APPLICATION_MPD
            else -> null
        }

        val itemBuilder = MediaItem.Builder().setUri(url)
        if (mimeType != null) {
            itemBuilder.setMimeType(mimeType)
        }

        player.trackSelectionParameters = player.trackSelectionParameters
            .buildUpon()
            .setTrackTypeDisabled(C.TRACK_TYPE_TEXT, disableEmbeddedText)
            .build()

        if (!isLive && startPositionMs > 2000L) {
            player.setMediaItem(itemBuilder.build(), startPositionMs)
        } else {
            player.setMediaItem(itemBuilder.build())
        }
        player.prepare()
        player.playWhenReady = true
    }

    override fun load(
        url: String,
        headers: Map<String, String>,
        startPositionMs: Long,
        isLive: Boolean,
    ) {
        loadWithOptions(
            url = url,
            headers = headers,
            startPositionMs = startPositionMs,
            isLive = isLive,
        )
    }

    override fun play() {
        player.play()
    }

    override fun pause() {
        player.pause()
    }

    override fun seekTo(positionMs: Long) {
        player.seekTo(positionMs.coerceAtLeast(0L))
    }

    override fun setSpeed(speed: Float) {
        player.playbackParameters = player.playbackParameters.withSpeed(speed.coerceIn(0.25f, 4.0f))
    }

    override fun setVolume(volume: Float) {
        player.volume = volume.coerceIn(0f, 1f)
    }

    override fun selectAudioTrack(id: String) {
        val ref = trackRefMap[id] ?: return
        player.trackSelectionParameters = player.trackSelectionParameters
            .buildUpon()
            .setOverrideForType(TrackSelectionOverride(ref.group, ref.trackIndex))
            .build()
    }

    override fun selectSubtitleTrack(id: String?) {
        if (id == null) {
            player.trackSelectionParameters = player.trackSelectionParameters
                .buildUpon()
                .setTrackTypeDisabled(C.TRACK_TYPE_TEXT, true)
                .build()
            return
        }
        val ref = trackRefMap[id] ?: return
        player.trackSelectionParameters = player.trackSelectionParameters
            .buildUpon()
            .setTrackTypeDisabled(C.TRACK_TYPE_TEXT, false)
            .setOverrideForType(TrackSelectionOverride(ref.group, ref.trackIndex))
            .build()
    }

    override fun addExternalSubtitle(urlOrPath: String) {
        // External subtitles on ExoPlayer are rendered by PlayerScreen's custom Compose SubtitleOverlay
        // so sync delay and styling apply uniformly; disable embedded tracks when active.
        selectSubtitleTrack(null)
    }

    override fun setSubtitleDelayMs(delayMs: Long) {
        // Handled by PlayerScreen's Compose SubtitleOverlay timing on ExoPlayer.
    }

    override fun setResizeMode(mode: Int) {
        attachedView?.resizeMode = mode
    }

    private fun syncTracks(tracks: Tracks) {
        val nextAudio = mutableListOf<PlayerTrackItem>()
        val nextSubs = mutableListOf<PlayerTrackItem>()
        val nextRefs = mutableMapOf<String, ExoTrackRef>()
        var audioOrdinal = 1
        var subOrdinal = 1

        for (group in tracks.groups) {
            if (group.type == C.TRACK_TYPE_AUDIO) {
                for (i in 0 until group.length) {
                    if (!group.isTrackSupported(i)) continue
                    val fmt = group.getTrackFormat(i)
                    val lang = fmt.language?.takeIf { it != "und" }?.uppercase(Locale.US)
                    val title = fmt.label?.takeIf { it.isNotBlank() }
                        ?: lang?.let { "Audio $audioOrdinal ($it)" }
                        ?: "Audio $audioOrdinal"
                    val metaParts = buildList {
                        if (lang != null) add(lang)
                        if (fmt.channelCount > 0) add("${fmt.channelCount}ch")
                        if (fmt.bitrate > 0) add("${fmt.bitrate / 1000}kbps")
                    }
                    val id = "a-${nextAudio.size}-${fmt.id ?: i}"
                    nextRefs[id] = ExoTrackRef(group.mediaTrackGroup, i)
                    nextAudio += PlayerTrackItem(
                        id = id,
                        label = title,
                        meta = metaParts.joinToString(" · "),
                        selected = group.isTrackSelected(i),
                    )
                    audioOrdinal++
                }
            } else if (group.type == C.TRACK_TYPE_TEXT) {
                for (i in 0 until group.length) {
                    if (!group.isTrackSupported(i)) continue
                    val fmt = group.getTrackFormat(i)
                    val lang = fmt.language?.takeIf { it != "und" }?.uppercase(Locale.US)
                    val title = fmt.label?.takeIf { it.isNotBlank() }
                        ?: lang?.let { "Subtitle $subOrdinal ($it)" }
                        ?: "Subtitle $subOrdinal"
                    val id = "s-${nextSubs.size}-${fmt.id ?: i}"
                    nextRefs[id] = ExoTrackRef(group.mediaTrackGroup, i)
                    nextSubs += PlayerTrackItem(
                        id = id,
                        label = title,
                        meta = listOfNotNull(
                            lang,
                            fmt.sampleMimeType?.substringAfterLast('/')?.uppercase(Locale.US),
                            "Embedded",
                        ).joinToString(" · "),
                        selected = group.isTrackSelected(i),
                    )
                    subOrdinal++
                }
            }
        }

        trackRefMap.clear()
        trackRefMap.putAll(nextRefs)
        _audioTracks.value = nextAudio
        _subtitleTracks.value = nextSubs
    }

    override fun release() {
        if (released) return
        released = true
        mainHandler.removeCallbacks(pollRunnable)
        player.removeListener(listener)
        attachedView?.player = null
        attachedView = null
        player.release()
    }
}
