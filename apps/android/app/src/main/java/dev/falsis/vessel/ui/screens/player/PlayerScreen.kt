package dev.falsis.vessel.ui.screens

import android.app.Activity
import android.content.Context
import android.content.pm.ActivityInfo
import android.graphics.Typeface
import android.media.AudioManager
import android.os.Build
import android.view.ViewGroup
import android.view.WindowManager
import android.widget.FrameLayout
import androidx.activity.compose.BackHandler
import androidx.annotation.OptIn
import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.core.tween
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.scaleIn
import androidx.compose.animation.scaleOut
import androidx.compose.animation.slideInHorizontally
import androidx.compose.animation.slideOutHorizontally
import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.focusGroup
import androidx.compose.foundation.focusable
import androidx.compose.foundation.gestures.detectHorizontalDragGestures
import androidx.compose.foundation.gestures.detectTapGestures
import androidx.compose.foundation.gestures.detectVerticalDragGestures
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.systemBarsPadding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.lazy.itemsIndexed
import androidx.compose.foundation.lazy.rememberLazyListState
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.automirrored.filled.List
import androidx.compose.material.icons.automirrored.filled.VolumeOff
import androidx.compose.material.icons.automirrored.filled.VolumeUp
import androidx.compose.material.icons.filled.Audiotrack
import androidx.compose.material.icons.filled.Check
import androidx.compose.material.icons.filled.Close
import androidx.compose.material.icons.filled.ClosedCaption
import androidx.compose.material.icons.filled.FastForward
import androidx.compose.material.icons.filled.Forward10
import androidx.compose.material.icons.filled.Fullscreen
import androidx.compose.material.icons.filled.FullscreenExit
import androidx.compose.material.icons.filled.Info
import androidx.compose.material.icons.filled.Layers
import androidx.compose.material.icons.filled.Lock
import androidx.compose.material.icons.filled.LockOpen
import androidx.compose.material.icons.filled.Pause
import androidx.compose.material.icons.filled.PlayArrow
import androidx.compose.material.icons.filled.Replay10
import androidx.compose.material.icons.filled.ScreenRotation
import androidx.compose.material.icons.filled.Search
import androidx.compose.material.icons.filled.Settings
import androidx.compose.material.icons.filled.SkipNext
import androidx.compose.material.icons.filled.SkipPrevious
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.Slider
import androidx.compose.material3.SliderDefaults
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableLongStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.focus.FocusRequester
import androidx.compose.ui.focus.focusProperties
import androidx.compose.ui.focus.focusRequester
import androidx.compose.ui.focus.onFocusChanged
import androidx.compose.ui.geometry.CornerRadius
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.Shadow
import androidx.compose.ui.graphics.toArgb
import androidx.compose.ui.input.key.Key
import androidx.compose.ui.input.key.KeyEventType
import androidx.compose.ui.input.key.key
import androidx.compose.ui.input.key.onPreviewKeyEvent
import androidx.compose.ui.input.key.type
import androidx.compose.ui.input.pointer.PointerEventPass
import androidx.compose.ui.input.pointer.pointerInput
import dev.falsis.vessel.ui.components.VesselOutlinedTextField
import dev.falsis.vessel.ui.components.vesselDpadInputEscape
import dev.falsis.vessel.ui.components.vesselFocusBorder
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.platform.LocalView
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.compose.ui.viewinterop.AndroidView
import androidx.core.view.WindowCompat
import androidx.core.view.WindowInsetsCompat
import androidx.core.view.WindowInsetsControllerCompat
import androidx.media3.common.util.UnstableApi
import androidx.media3.ui.AspectRatioFrameLayout
import androidx.media3.ui.CaptionStyleCompat
import androidx.media3.ui.PlayerView
import coil3.compose.AsyncImage
import dev.falsis.vessel.data.Episode
import dev.falsis.vessel.data.GetSkipSegmentsPayload
import dev.falsis.vessel.data.MediaDetail
import dev.falsis.vessel.data.MediaRef
import dev.falsis.vessel.data.PlaybackProgress
import dev.falsis.vessel.data.PlaybackStats
import dev.falsis.vessel.data.ResolveDebridRequest
import dev.falsis.vessel.data.SkipSegment
import dev.falsis.vessel.data.StreamCandidate
import dev.falsis.vessel.data.StreamResult
import dev.falsis.vessel.data.SubtitleProviderError
import dev.falsis.vessel.data.SubtitleQueryPayload
import dev.falsis.vessel.data.SubtitleSearchHit
import dev.falsis.vessel.data.SubtitleTrack
import dev.falsis.vessel.data.UiPreferences
import dev.falsis.vessel.data.VesselRepository
import dev.falsis.vessel.ui.components.formatSizeBytes
import dev.falsis.vessel.ui.i18n.VesselI18n
import dev.falsis.vessel.ui.isLiveMediaIdentifier
import dev.falsis.vessel.ui.screens.player.ExoPlayerController
import dev.falsis.vessel.ui.screens.player.MpvKitController
import dev.falsis.vessel.ui.screens.player.PlayerBackendEngine
import dev.falsis.vessel.ui.screens.player.VesselPlayerController
import dev.falsis.vessel.ui.theme.LocalIsTvDevice
import dev.falsis.vessel.ui.theme.LocalVesselPalette
import dev.falsis.vessel.ui.theme.parseHexColor
import dev.falsis.vessel.ui.utils.SubtitleCue
import dev.falsis.vessel.ui.utils.findActiveSubtitleLines
import dev.falsis.vessel.ui.utils.parseSubtitles
import java.util.Locale
import kotlin.math.abs
import kotlin.math.roundToInt
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch

private const val DEFAULT_PLAYER_USER_AGENT =
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36"

private val DrawerSurfaceColor = Color(0xF010101A)
private val HudControlBg = Color(0xA612121C)
private val HudBorderColor = Color(0x24FFFFFF)

@kotlin.OptIn(ExperimentalLayoutApi::class)
@OptIn(UnstableApi::class)
@Composable
fun PlayerScreen(
    repo: VesselRepository,
    pluginId: String,
    nativeId: String,
    detail: MediaDetail,
    episode: Episode,
    streamResult: StreamResult,
    initialStreamIndex: Int,
    initialPositionMs: Long,
    channelStreams: Map<String, List<StreamCandidate>> = emptyMap(),
    onBack: () -> Unit,
    onGoToDetails: () -> Unit,
) {
    val context = LocalContext.current
    val view = LocalView.current
    val activity = context as? Activity
    val palette = LocalVesselPalette.current
    val isTv = LocalIsTvDevice.current
    val scope = rememberCoroutineScope()
    val prefs by repo.uiPreferences.collectAsState()
    val appLocale by repo.locale.collectAsState()
    val plugins by repo.plugins.collectAsState()
    val audioManager = remember(context) {
        context.getSystemService(Context.AUDIO_SERVICE) as? AudioManager
    }

    var activeDetail by remember(detail) { mutableStateOf(detail) }
    var currentEpisode by remember(episode) { mutableStateOf(episode) }
    var activeStreamResult by remember(streamResult) { mutableStateOf(streamResult) }
    var selectedIndex by remember(streamResult) {
        mutableIntStateOf(initialStreamIndex.coerceIn(0, (streamResult.streams.size - 1).coerceAtLeast(0)))
    }
    var currentStartPosMs by remember(episode) { mutableLongStateOf(initialPositionMs) }

    val activePlugin = remember(plugins, pluginId) { plugins.firstOrNull { it.manifest.id == pluginId } }
    val isLive = remember(activeDetail.type, pluginId, nativeId, currentEpisode.streamId, activePlugin) {
        isLiveMediaIdentifier(
            mediaType = activeDetail.type,
            pluginId = pluginId,
            streamId = currentEpisode.streamId,
            nativeId = nativeId,
            plugin = activePlugin,
        )
    }

    var isLocked by remember { mutableStateOf(false) }
    var isLandscape by remember { mutableStateOf(true) }
    var controlsVisible by remember { mutableStateOf(true) }
    var showStatsHud by remember { mutableStateOf(false) }
    var playbackStats by remember { mutableStateOf<PlaybackStats?>(null) }
    var interactionNonce by remember { mutableIntStateOf(0) }
    var resizeMode by remember { mutableIntStateOf(AspectRatioFrameLayout.RESIZE_MODE_FIT) }
    var activeDrawer by remember { mutableStateOf<PlayerDrawer?>(null) }
    var playbackSpeed by remember { mutableFloatStateOf(1.0f) }
    var longPressSpeedActive by remember { mutableStateOf(false) }
    var playerVolume by remember { mutableFloatStateOf(1.0f) }
    var isMuted by remember { mutableStateOf(false) }

    var isPlaying by remember { mutableStateOf(false) }
    var isBuffering by remember { mutableStateOf(true) }
    var playbackError by remember { mutableStateOf<String?>(null) }
    var switchingEpisode by remember { mutableStateOf(false) }
    var forceHlsRetry by remember { mutableStateOf(false) }
    var forceProxyRetry by remember { mutableStateOf(false) }

    LaunchedEffect(isLive) {
        if (isLive && (activeDrawer == PlayerDrawer.AUDIO || activeDrawer == PlayerDrawer.SUBTITLES)) {
            activeDrawer = null
        }
    }
    var lastLoadWasProxied by remember { mutableStateOf(false) }

    var embeddedSubsDisabled by remember { mutableStateOf(true) }

    // External parsed subtitles (full Desktop SubtitleParser + Sync Delay + Styling parity)
    var activeExternalSubIndex by remember(currentEpisode, selectedIndex) { mutableIntStateOf(-1) }
    var subtitleDelayMs by remember { mutableLongStateOf(0L) }
    var subtitleTranscriptOpen by remember { mutableStateOf(false) }
    var subtitleTranscriptQuery by remember { mutableStateOf("") }
    var externalCues by remember { mutableStateOf<List<SubtitleCue>>(emptyList()) }
    var loadingSubtitle by remember { mutableStateOf(false) }
    var subtitleError by remember { mutableStateOf<String?>(null) }
    var extraFetchedSubs by remember(currentEpisode) { mutableStateOf<List<SubtitleTrack>>(emptyList()) }
    var extSubOpen by remember(currentEpisode) { mutableStateOf(false) }
    var extSubQueryText by remember(currentEpisode) { mutableStateOf("") }
    var extSubLangMode by remember(currentEpisode) { mutableStateOf("preferred") }
    var extSubSourceFilter by remember(currentEpisode) { mutableStateOf("all") }
    var extSubSearching by remember { mutableStateOf(false) }
    var extSubResolvingKey by remember { mutableStateOf<String?>(null) }
    var extSubHits by remember(currentEpisode) { mutableStateOf<List<SubtitleSearchHit>>(emptyList()) }
    var extSubErrors by remember(currentEpisode) { mutableStateOf<List<SubtitleProviderError>>(emptyList()) }
    var extSubSearchedOnce by remember(currentEpisode) { mutableStateOf(false) }

    var skipSegments by remember(currentEpisode) { mutableStateOf<List<SkipSegment>>(emptyList()) }
    var skipFetchSeq by remember(currentEpisode) { mutableIntStateOf(0) }
    var lastSkipFetchDurationMs by remember(currentEpisode) { mutableLongStateOf(-1L) }
    var dismissedSkipKeys by remember(currentEpisode) { mutableStateOf<Set<String>>(emptySet()) }
    var dismissUpNext by remember(currentEpisode) { mutableStateOf(false) }

    var currentPosMs by remember { mutableLongStateOf(initialPositionMs) }
    var bufferedPosMs by remember { mutableLongStateOf(0L) }
    var totalDurMs by remember { mutableLongStateOf(0L) }
    var isScrubbing by remember { mutableStateOf(false) }
    var scrubPreviewMs by remember { mutableLongStateOf(0L) }

    var gestureHudLabel by remember { mutableStateOf<String?>(null) }
    var gestureHudNonce by remember { mutableIntStateOf(0) }

    val activeCandidate =
        activeStreamResult.streams.getOrNull(selectedIndex) ?: activeStreamResult.streams.firstOrNull()

    val availableExternalSubs = remember(activeCandidate, activeStreamResult, extraFetchedSubs) {
        (activeCandidate?.subtitles ?: emptyList()) + activeStreamResult.subtitles + extraFetchedSubs
    }

    val defaultPreferredSubLangs = remember(appLocale) {
        val base = appLocale.lowercase(Locale.US).substringBefore('-')
        if (base.isBlank() || base == "en") listOf("en") else listOf(base, "en")
    }

    fun runExternalSubSearch() {
        if (extSubSearching) return
        val rawInput = extSubQueryText.trim()
        val imdbMatch = Regex("^tt\\d+$", RegexOption.IGNORE_CASE).matches(rawInput)
        val searchTitle = if (imdbMatch) {
            activeDetail.originalTitle?.takeIf { it.isNotBlank() } ?: activeDetail.title
        } else {
            rawInput.ifBlank { activeDetail.originalTitle?.takeIf { it.isNotBlank() } ?: activeDetail.title }
        }
        val langs = when (extSubLangMode) {
            "all" -> emptyList()
            "tr" -> listOf("tr")
            "en" -> listOf("en")
            else -> defaultPreferredSubLangs
        }
        val matchesOriginalTitle = rawInput.isBlank() ||
            rawInput.equals(activeDetail.title.trim(), ignoreCase = true) ||
            (activeDetail.originalTitle?.trim()?.let { rawInput.equals(it, ignoreCase = true) } == true)
        val ids = when {
            imdbMatch -> activeDetail.externalIds.copy(imdb = rawInput.lowercase(Locale.US))
            matchesOriginalTitle -> activeDetail.externalIds
            else -> dev.falsis.vessel.data.ExternalIds()
        }
        extSubSearching = true
        extSubSearchedOnce = true
        extSubErrors = emptyList()
        extSubSourceFilter = "all"
        scope.launch {
            runCatching {
                repo.searchSubtitles(
                    SubtitleQueryPayload(
                        title = searchTitle,
                        originalTitle = if (imdbMatch) activeDetail.originalTitle else rawInput.ifBlank { activeDetail.originalTitle },
                        year = if (matchesOriginalTitle || imdbMatch) activeDetail.year else null,
                        mediaType = activeDetail.type,
                        season = if (activeDetail.type == "series") currentEpisode.season else null,
                        episode = if (activeDetail.type == "series") currentEpisode.episode else null,
                        externalIds = ids,
                        preferredLanguages = langs,
                    ),
                )
            }.onSuccess { outcome ->
                extSubHits = outcome.hits
                extSubErrors = outcome.errors
            }.onFailure { err ->
                extSubHits = emptyList()
                extSubErrors = listOf(
                    SubtitleProviderError("subtitles", "Subtitles", err.message ?: "Subtitle search failed"),
                )
            }
            extSubSearching = false
        }
    }

    val episodes = activeDetail.episodes
    val currentEpIdx = remember(episodes, currentEpisode) {
        episodes.indexOfFirst { it.streamId == currentEpisode.streamId }
    }
    val prevEpisode = remember(episodes, currentEpIdx, isLive) {
        when {
            isLive && episodes.size > 1 && currentEpIdx >= 0 ->
                if (currentEpIdx > 0) episodes[currentEpIdx - 1] else episodes.last()
            currentEpIdx > 0 -> episodes[currentEpIdx - 1]
            else -> null
        }
    }
    val nextEpisode = remember(episodes, currentEpIdx, isLive) {
        when {
            isLive && episodes.size > 1 && currentEpIdx >= 0 ->
                if (currentEpIdx + 1 < episodes.size) episodes[currentEpIdx + 1] else episodes.first()
            currentEpIdx >= 0 && currentEpIdx + 1 < episodes.size -> episodes[currentEpIdx + 1]
            else -> null
        }
    }

    val desiredEngine = remember(prefs.playback.androidPlayerEngine, isLive) {
        when (prefs.playback.androidPlayerEngine) {
            "exoplayer" -> PlayerBackendEngine.EXOPLAYER
            "mpvkit" -> PlayerBackendEngine.MPVKIT
            else -> if (isLive) PlayerBackendEngine.EXOPLAYER else PlayerBackendEngine.MPVKIT
        }
    }

    val controller: VesselPlayerController = remember(desiredEngine) {
        when (desiredEngine) {
            PlayerBackendEngine.MPVKIT -> MpvKitController(context.applicationContext)
            PlayerBackendEngine.EXOPLAYER -> ExoPlayerController(context.applicationContext)
        }
    }
    val controllerState by controller.state.collectAsState()
    val audioTracks by controller.audioTracks.collectAsState()
    val embeddedSubTracks by controller.subtitleTracks.collectAsState()

    LaunchedEffect(isLive, isLocked, activeDrawer) {
        if (longPressSpeedActive && (isLive || isLocked || activeDrawer != null)) {
            longPressSpeedActive = false
            controller.setSpeed(if (isLive) 1.0f else playbackSpeed)
        }
    }

    fun showHud(label: String) {
        gestureHudLabel = label
        gestureHudNonce++
    }

    LaunchedEffect(gestureHudNonce) {
        if (gestureHudLabel != null) {
            delay(950L)
            gestureHudLabel = null
        }
    }

    LaunchedEffect(controlsVisible, isPlaying, activeDrawer, isLocked, playbackError, interactionNonce) {
        if (controlsVisible && isPlaying && activeDrawer == null && !isLocked && playbackError == null) {
            delay(3200L)
            controlsVisible = false
        }
    }

    fun bumpControls() {
        controlsVisible = true
        interactionNonce++
    }

    LaunchedEffect(embeddedSubTracks, activeExternalSubIndex) {
        if (activeExternalSubIndex == -1 && controller is MpvKitController && embeddedSubTracks.any { it.selected }) {
            embeddedSubsDisabled = false
        }
    }

    suspend fun refreshSkipSegments(knownDurationMs: Long) {
        if (isLive) {
            skipSegments = emptyList()
            return
        }
        val seq = ++skipFetchSeq
        val validDur = knownDurationMs.takeIf { it > 10_000L }
        lastSkipFetchDurationMs = validDur ?: 0L
        val pluginSegs = (activeCandidate?.skipSegments ?: emptyList()) + activeStreamResult.skipSegments
        runCatching {
            repo.getSkipSegments(
                GetSkipSegmentsPayload(
                    pluginId = pluginId,
                    nativeId = nativeId,
                    mediaType = activeDetail.type,
                    isAnime = activeDetail.isAnime,
                    externalIds = activeDetail.externalIds,
                    season = currentEpisode.season,
                    episode = currentEpisode.episode,
                    absoluteEpisode = currentEpisode.absoluteEpisode,
                    durationMs = validDur,
                    pluginSegments = pluginSegs,
                ),
            )
        }.onSuccess { resolved ->
            if (seq == skipFetchSeq) {
                skipSegments = resolved
            }
        }.onFailure {
            if (seq == skipFetchSeq && skipSegments.isEmpty() && pluginSegs.isNotEmpty()) {
                skipSegments = pluginSegs
            }
        }
    }

    // Listen for background MetadataEnriched events & fallback refresh so Skip Intro/Recap/Credits load even if entered Player early
    LaunchedEffect(pluginId, nativeId, isLive) {
        if (isLive) return@LaunchedEffect
        repo.metadataEnriched.collect { ev ->
            if (ev.pluginId == pluginId && ev.nativeId == nativeId) {
                activeDetail = ev.detail
                refreshSkipSegments(totalDurMs)
            }
        }
    }

    LaunchedEffect(pluginId, nativeId, currentEpisode.streamId, isLive) {
        if (isLive) return@LaunchedEffect
        val ids = activeDetail.externalIds
        if (ids.tmdb == null && ids.imdb == null && ids.anilist == null && ids.mal == null) {
            delay(1800L)
            runCatching { repo.getDetails(pluginId, nativeId) }
                .onSuccess { fresh ->
                    val fIds = fresh.externalIds
                    if (fIds.tmdb != null || fIds.imdb != null || fIds.anilist != null || fIds.mal != null) {
                        activeDetail = fresh
                        refreshSkipSegments(totalDurMs)
                    }
                }
        }
    }

    // Auto-select default external subtitle when stream changes
    LaunchedEffect(availableExternalSubs) {
        if (availableExternalSubs.isEmpty()) {
            activeExternalSubIndex = -1
            externalCues = emptyList()
            return@LaunchedEffect
        }
        if (activeExternalSubIndex >= availableExternalSubs.size) {
            activeExternalSubIndex = -1
        }
        if (activeExternalSubIndex == -1 && embeddedSubsDisabled) {
            val defIdx = availableExternalSubs.indexOfFirst { it.isDefault }
            if (defIdx >= 0) {
                activeExternalSubIndex = defIdx
            }
        }
    }

    // Load & parse external subtitle cues whenever activeExternalSubIndex changes (supports Sync Delay & full styling)
    LaunchedEffect(activeExternalSubIndex, availableExternalSubs, isLive) {
        subtitleError = null
        if (isLive || activeExternalSubIndex < 0 || activeExternalSubIndex >= availableExternalSubs.size) {
            externalCues = emptyList()
            loadingSubtitle = false
            return@LaunchedEffect
        }
        // Disable embedded text rendering while custom external cue overlay is active
        controller.selectSubtitleTrack(null)
        embeddedSubsDisabled = true

        val track = availableExternalSubs[activeExternalSubIndex]
        if (track.source.kind == "inline_content") {
            val content = track.source.content.orEmpty()
            externalCues = parseSubtitles(content, track.format)
            loadingSubtitle = false
            return@LaunchedEffect
        }
        val rawUrl = track.source.url?.takeIf { it.isNotBlank() }
        if (rawUrl == null) {
            externalCues = emptyList()
            loadingSubtitle = false
            return@LaunchedEffect
        }
        loadingSubtitle = true
        runCatching { repo.fetchTextViaProxy(rawUrl, track.source.headers) }
            .onSuccess { rawText ->
                val parsed = parseSubtitles(rawText, track.format)
                externalCues = parsed
                if (parsed.isEmpty()) {
                    subtitleError = "Subtitle file contained no readable timed cues."
                }
            }
            .onFailure { err ->
                externalCues = emptyList()
                subtitleError = err.message ?: "Failed to load subtitle"
            }
        loadingSubtitle = false
    }

    suspend fun loadCandidate(
        cand: StreamCandidate,
        startPosMs: Long,
        forceHls: Boolean,
        forceProxy: Boolean,
    ) {
        playbackError = null
        isBuffering = true

        val isLocalLoopback =
            cand.url.startsWith("http://127.0.0.1:") || cand.url.startsWith("http://localhost:")
        val candidateForProxy = if (!isLocalLoopback && (forceProxy || cand.url.startsWith("http://", ignoreCase = true))) {
            cand.copy(flags = (cand.flags + "requires_proxy").distinct())
        } else {
            cand
        }
        val resolvedUrl = if (isLocalLoopback) {
            cand.url
        } else {
            runCatching { repo.preparePlaybackUrl(candidateForProxy) }.getOrElse { cand.url }
        }
        val proxied = !isLocalLoopback &&
            (resolvedUrl.startsWith("http://127.0.0.1:") || resolvedUrl.startsWith("http://localhost:"))
        lastLoadWasProxied = proxied || isLocalLoopback

        // Only attach upstream headers directly when bypassing the local proxy;
        // when proxied, the Rust loopback proxy injects them cleanly upstream.
        val effectiveHeaders = if (!lastLoadWasProxied) cand.headers else emptyMap()
        val lowerRaw = cand.url.lowercase(Locale.US)
        val lowerResolved = resolvedUrl.lowercase(Locale.US)
        val isHls = forceHls ||
            cand.kind.equals("hls", ignoreCase = true) ||
            lowerRaw.contains(".m3u8") ||
            lowerResolved.contains(".m3u8")
        val isDash = cand.kind.equals("dash", ignoreCase = true) ||
            lowerRaw.contains(".mpd") ||
            lowerResolved.contains(".mpd")

        if (controller is ExoPlayerController) {
            controller.loadWithOptions(
                url = resolvedUrl,
                headers = effectiveHeaders,
                startPositionMs = startPosMs,
                isLive = isLive,
                forceHlsMime = isHls,
                forceDashMime = isDash,
                disableEmbeddedText = activeExternalSubIndex >= 0 || embeddedSubsDisabled,
            )
        } else {
            controller.load(
                url = resolvedUrl,
                headers = effectiveHeaders,
                startPositionMs = startPosMs,
                isLive = isLive,
            )
        }
        controller.setSpeed(if (isLive) 1.0f else playbackSpeed)
        controller.setVolume(if (isMuted) 0f else playerVolume)
    }

    LaunchedEffect(showStatsHud, selectedIndex) {
        if (!showStatsHud) return@LaunchedEffect
        while (true) {
            val backend = runCatching { repo.getPlaybackStats() }.getOrDefault(PlaybackStats())
            val snap = controller.stats.value
            val resParts = snap.resolution?.split('x')
            val w = resParts?.getOrNull(0)?.toIntOrNull()
            val h = resParts?.getOrNull(1)?.toIntOrNull()
            val cand = activeCandidate
            playbackStats = backend.copy(
                bufferSeconds = snap.bufferSeconds ?: backend.bufferSeconds,
                width = w ?: backend.width,
                height = h ?: backend.height,
                videoCodec = snap.videoCodec ?: backend.videoCodec,
                audioCodec = snap.audioCodec ?: backend.audioCodec,
                videoBitrateBps = snap.videoBitrateBps ?: backend.videoBitrateBps,
                audioBitrateBps = snap.audioBitrateBps ?: backend.audioBitrateBps,
                fileSizeBytes = cand?.torrent?.sizeBytes ?: backend.fileSizeBytes,
                isDebridStream = backend.isDebridStream || cand?.torrent != null || cand?.debridCache != null,
            )
            delay(1000)
        }
    }

    fun playEpisode(target: Episode) {
        if (switchingEpisode) return
        val directChannelCands = channelStreams[target.streamId]
        if (!directChannelCands.isNullOrEmpty()) {
            currentEpisode = target
            activeDetail = activeDetail.copy(
                title = target.title?.takeIf { it.isNotBlank() } ?: activeDetail.title,
                poster = target.thumbnail ?: activeDetail.poster,
                backdrop = target.thumbnail ?: activeDetail.backdrop,
            )
            activeStreamResult = StreamResult(streams = directChannelCands)
            selectedIndex = if (isLive) {
                repo.getPreferredLiveStreamIndex(pluginId, activeDetail.nativeId, target.streamId, directChannelCands)
            } else {
                directChannelCands.indexOfFirst { it.isDefault }.takeIf { it >= 0 } ?: 0
            }
            currentStartPosMs = 0L
            forceHlsRetry = false
            forceProxyRetry = false
            dismissedSkipKeys = emptySet()
            dismissUpNext = false
            skipSegments = emptyList()
            lastSkipFetchDurationMs = -1L
            playbackError = null
            activeDrawer = null
            if (isLive) showHud(target.title?.takeIf { it.isNotBlank() } ?: activeDetail.title)
            bumpControls()
            return
        }

        switchingEpisode = true
        scope.launch {
            runCatching { repo.getStreams(pluginId, target.streamId) }
                .onSuccess { sr ->
                    if (sr.streams.isEmpty()) {
                        repo.notify(
                            if (isLive) "No playable streams found for ${target.title ?: "channel"}."
                            else VesselI18n.tr("details.no_streams"),
                        )
                    } else {
                        currentEpisode = target
                        if (isLive) {
                            activeDetail = activeDetail.copy(
                                title = target.title?.takeIf { it.isNotBlank() } ?: activeDetail.title,
                                poster = target.thumbnail ?: activeDetail.poster,
                                backdrop = target.thumbnail ?: activeDetail.backdrop,
                            )
                            showHud(target.title?.takeIf { it.isNotBlank() } ?: activeDetail.title)
                        }
                        activeStreamResult = sr
                        selectedIndex = if (isLive) {
                            repo.getPreferredLiveStreamIndex(pluginId, activeDetail.nativeId, target.streamId, sr.streams)
                        } else {
                            sr.streams.indexOfFirst { it.isDefault }.takeIf { it >= 0 } ?: 0
                        }
                        currentStartPosMs = 0L
                        forceHlsRetry = false
                        forceProxyRetry = false
                        dismissedSkipKeys = emptySet()
                        dismissUpNext = false
                        skipSegments = emptyList()
                        lastSkipFetchDurationMs = -1L
                        playbackError = null
                        activeDrawer = null
                    }
                }
                .onFailure {
                    repo.notifyError(it, "Failed to load episode")
                }
            switchingEpisode = false
        }
    }

    LaunchedEffect(controllerState) {
        isPlaying = controllerState.isPlaying
        isBuffering = controllerState.isBuffering
        val dur = controllerState.durationMs
        if (dur > 0L) {
            totalDurMs = dur
            if (dur > 10_000L && (lastSkipFetchDurationMs <= 0L || abs(dur - lastSkipFetchDurationMs) > 30_000L)) {
                scope.launch { refreshSkipSegments(dur) }
            }
        }
        if (!isScrubbing) {
            currentPosMs = controllerState.positionMs
        }
        bufferedPosMs = controllerState.bufferedPositionMs
    }

    LaunchedEffect(controllerState.error, controllerState.unrecognizedFormat, selectedIndex) {
        val err = controllerState.error ?: return@LaunchedEffect
        val cand = activeStreamResult.streams.getOrNull(selectedIndex)
            ?: activeStreamResult.streams.firstOrNull()

        if (controllerState.unrecognizedFormat && !forceHlsRetry && cand != null) {
            forceHlsRetry = true
            isBuffering = true
            val resumeMs = if (isLive) 0L else controllerState.positionMs.coerceAtLeast(0L)
            loadCandidate(cand, resumeMs, forceHls = true, forceProxy = forceProxyRetry)
            return@LaunchedEffect
        }

        if (!lastLoadWasProxied && !forceProxyRetry && cand != null) {
            forceProxyRetry = true
            forceHlsRetry = false
            isBuffering = true
            val resumeMs = if (isLive) 0L else controllerState.positionMs.coerceAtLeast(0L)
            loadCandidate(cand, resumeMs, forceHls = false, forceProxy = true)
            return@LaunchedEffect
        }

        if (selectedIndex + 1 < activeStreamResult.streams.size) {
            forceHlsRetry = false
            forceProxyRetry = false
            isBuffering = true
            currentStartPosMs = if (isLive) 0L else controllerState.positionMs.coerceAtLeast(0L)
            selectedIndex += 1
            return@LaunchedEffect
        }

        isBuffering = false
        isPlaying = false
        controlsVisible = true
        playbackError = err
    }

    LaunchedEffect(activeCandidate, currentEpisode, controller) {
        val cand = activeCandidate ?: return@LaunchedEffect
        forceHlsRetry = false
        forceProxyRetry = false
        lastSkipFetchDurationMs = -1L
        totalDurMs = 0L
        currentPosMs = if (isLive) 0L else currentStartPosMs
        bufferedPosMs = 0L
        loadCandidate(cand, currentStartPosMs, forceHls = false, forceProxy = false)
        refreshSkipSegments(0L)
    }

    suspend fun persistProgress(posMs: Long, durMs: Long) {
        if (isLive || posMs < 2000L) return
        val threshold = (prefs.playback.watchedThresholdPercent.coerceIn(50, 99)) / 100.0
        val completed = durMs > 0L && posMs >= (durMs * threshold).toLong()
        runCatching {
            repo.recordProgress(
                PlaybackProgress(
                    media = MediaRef(pluginId, nativeId),
                    streamId = currentEpisode.streamId,
                    season = currentEpisode.season,
                    episode = currentEpisode.episode,
                    absoluteEpisode = currentEpisode.absoluteEpisode,
                    title = activeDetail.title,
                    posterUrl = activeDetail.poster,
                    backdropUrl = activeDetail.backdrop,
                    episodeTitle = currentEpisode.title,
                    positionMs = posMs,
                    durationMs = durMs,
                    completed = completed,
                    lastWatchedAt = java.time.Instant.now().toString(),
                ),
            )
        }
    }

    LaunchedEffect(controller, currentEpisode) {
        var tick = 0
        while (true) {
            val st = controller.state.value
            val dur = st.durationMs.coerceAtLeast(0L)
            tick++
            if (tick % 10 == 0 && st.isPlaying) {
                persistProgress(st.positionMs.coerceAtLeast(0L), dur)
            }
            delay(250L)
        }
    }

    DisposableEffect(controller) {
        onDispose {
            val st = controller.state.value
            scope.launch { persistProgress(st.positionMs.coerceAtLeast(0L), st.durationMs.coerceAtLeast(0L)) }
            controller.release()
        }
    }

    DisposableEffect(Unit) {
        val win = activity?.window
        win?.addFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON)
        val insetsController = win?.let { WindowCompat.getInsetsController(it, view) }
        insetsController?.systemBarsBehavior =
            WindowInsetsControllerCompat.BEHAVIOR_SHOW_TRANSIENT_BARS_BY_SWIPE
        insetsController?.hide(WindowInsetsCompat.Type.systemBars())
        // Default to landscape orientation when entering Player
        activity?.requestedOrientation = ActivityInfo.SCREEN_ORIENTATION_SENSOR_LANDSCAPE

        onDispose {
            insetsController?.hide(WindowInsetsCompat.Type.systemBars())
            win?.clearFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON)
            activity?.requestedOrientation = ActivityInfo.SCREEN_ORIENTATION_UNSPECIFIED
        }
    }

    val validSkipSegments = remember(skipSegments, totalDurMs, isLive) {
        if (isLive) {
            emptyList()
        } else {
            val durMs = totalDurMs.takeIf { it > 10_000L } ?: 0L
            skipSegments.filter { seg ->
                if (durMs > 0L && seg.startMs >= durMs - 1500L) return@filter false
                val effectiveEnd = when {
                    seg.endMs != null && durMs > 0L -> minOf(seg.endMs, durMs)
                    seg.endMs != null -> seg.endMs
                    else -> durMs
                }
                if (effectiveEnd > 0L) {
                    val span = effectiveEnd - seg.startMs
                    if (span < 3000L) return@filter false
                    if (durMs > 0L && span > (durMs * 0.85).toLong()) return@filter false
                }
                true
            }
        }
    }

    val activeSkip = remember(validSkipSegments, currentPosMs, totalDurMs, dismissedSkipKeys, playbackError) {
        if (playbackError != null) {
            null
        } else {
            val durMs = totalDurMs.takeIf { it > 10_000L } ?: 0L
            validSkipSegments.firstOrNull { seg ->
                val key = "${seg.type}:${seg.startMs}:${seg.endMs ?: "end"}"
                if (key in dismissedSkipKeys) return@firstOrNull false
                val end = seg.endMs ?: if (durMs > 0L) durMs else Long.MAX_VALUE
                currentPosMs >= seg.startMs && currentPosMs < end - 400L
            }
        }
    }

    val showUpNext = remember(
        nextEpisode,
        totalDurMs,
        currentPosMs,
        dismissUpNext,
        prefs.playback.autoNextDelaySecs,
        isLive,
        activeSkip,
    ) {
        if (isLive || activeSkip != null || nextEpisode == null || dismissUpNext || totalDurMs <= 30_000L) {
            false
        } else {
            val triggerWindowMs = (prefs.playback.autoNextDelaySecs.coerceIn(5, 30)) * 1000L
            (totalDurMs - currentPosMs) in 0..triggerWindowMs
        }
    }

    val activeExternalLines = remember(externalCues, currentPosMs, subtitleDelayMs, activeExternalSubIndex, isLive) {
        if (isLive || activeExternalSubIndex < 0 || externalCues.isEmpty()) {
            emptyList()
        } else {
            findActiveSubtitleLines(externalCues, currentPosMs - subtitleDelayMs)
        }
    }

    val fitLabel = when (resizeMode) {
        AspectRatioFrameLayout.RESIZE_MODE_ZOOM -> VesselI18n.tr("player.fit_fill")
        AspectRatioFrameLayout.RESIZE_MODE_FILL -> VesselI18n.tr("player.fit_stretch")
        else -> VesselI18n.tr("player.fit_contain")
    }

    fun togglePlayPause() {
        if (isPlaying) controller.pause() else controller.play()
        bumpControls()
    }

    fun seekByMs(deltaMs: Long) {
        if (isLive) return
        val cap = totalDurMs.takeIf { it > 0L } ?: Long.MAX_VALUE
        val target = (controller.state.value.positionMs + deltaMs).coerceIn(0L, cap)
        controller.seekTo(target)
        currentPosMs = target
        showHud(if (deltaMs < 0) VesselI18n.tr("player.seek_back_10") else VesselI18n.tr("player.seek_fwd_10"))
    }

    val playerRootFocusRequester = remember { FocusRequester() }
    val playPauseFocusRequester = remember { FocusRequester() }
    val drawerFirstFocusRequester = remember { FocusRequester() }
    val drawerCloseFocusRequester = remember { FocusRequester() }
    val backBtnFocus = remember { FocusRequester() }
    val sourcesPillFocus = remember { FocusRequester() }
    val scrubberFocus = remember { FocusRequester() }
    val bottomPlayPauseFocus = remember { FocusRequester() }
    val errorRetryFocus = remember { FocusRequester() }
    val skipIntroFocusRequester = remember { FocusRequester() }
    val skipCloseFocusRequester = remember { FocusRequester() }
    var skipIntroHasFocus by remember { mutableStateOf(false) }
    var skipCloseHasFocus by remember { mutableStateOf(false) }

    LaunchedEffect(activeDrawer) {
        if (activeDrawer == null) {
            subtitleTranscriptOpen = false
            subtitleTranscriptQuery = ""
        }
    }

    LaunchedEffect(controlsVisible, isLocked, activeDrawer, playbackError, subtitleTranscriptOpen, activeSkip, isTv) {
        if (playbackError != null) {
            kotlinx.coroutines.delay(60)
            runCatching { errorRetryFocus.requestFocus() }
        } else if (activeDrawer != null && !isLocked) {
            kotlinx.coroutines.delay(80)
            runCatching { drawerFirstFocusRequester.requestFocus() }
                .onFailure { runCatching { drawerCloseFocusRequester.requestFocus() } }
        } else if (activeDrawer == null) {
            if (!controlsVisible && !isLocked && isTv && activeSkip != null) {
                kotlinx.coroutines.delay(80)
                runCatching { skipIntroFocusRequester.requestFocus() }
            } else if (!controlsVisible || isLocked) {
                runCatching { playerRootFocusRequester.requestFocus() }
            } else {
                kotlinx.coroutines.delay(50)
                runCatching { playPauseFocusRequester.requestFocus() }
            }
        }
    }

    BackHandler(enabled = isLocked || activeDrawer != null || controlsVisible) {
        when {
            isLocked -> {
                isLocked = false
                bumpControls()
            }
            activeDrawer == PlayerDrawer.SUBTITLES && subtitleTranscriptOpen -> {
                subtitleTranscriptOpen = false
                subtitleTranscriptQuery = ""
                bumpControls()
            }
            activeDrawer != null -> {
                activeDrawer = null
                bumpControls()
            }
            controlsVisible -> {
                controlsVisible = false
            }
        }
    }

    fun toggleOrientation() {
        isLandscape = !isLandscape
        activity?.requestedOrientation = if (isLandscape) {
            ActivityInfo.SCREEN_ORIENTATION_SENSOR_LANDSCAPE
        } else {
            ActivityInfo.SCREEN_ORIENTATION_SENSOR_PORTRAIT
        }
        bumpControls()
    }

    fun cycleFitMode() {
        val nextMode = when (resizeMode) {
            AspectRatioFrameLayout.RESIZE_MODE_FIT -> AspectRatioFrameLayout.RESIZE_MODE_ZOOM
            AspectRatioFrameLayout.RESIZE_MODE_ZOOM -> AspectRatioFrameLayout.RESIZE_MODE_FILL
            else -> AspectRatioFrameLayout.RESIZE_MODE_FIT
        }
        resizeMode = nextMode
        controller.setResizeMode(nextMode)
        val nextLabel = when (nextMode) {
            AspectRatioFrameLayout.RESIZE_MODE_ZOOM -> VesselI18n.tr("player.fit_fill")
            AspectRatioFrameLayout.RESIZE_MODE_FILL -> VesselI18n.tr("player.fit_stretch")
            else -> VesselI18n.tr("player.fit_contain")
        }
        showHud(nextLabel)
        bumpControls()
    }

    fun saveSubPrefs(transform: (UiPreferences) -> UiPreferences) {
        val next = transform(prefs)
        scope.launch { runCatching { repo.saveUiPreferences(next) } }
    }

    Box(
        modifier = Modifier
            .fillMaxSize()
            .background(Color.Black)
            .focusRequester(playerRootFocusRequester)
            .focusProperties { canFocus = activeDrawer == null }
            .focusable()
            .onPreviewKeyEvent { ev ->
                if (ev.type != KeyEventType.KeyDown) return@onPreviewKeyEvent false
                if (playbackError != null) return@onPreviewKeyEvent false
                if (isLocked) {
                    return@onPreviewKeyEvent when (ev.key) {
                        Key.DirectionCenter, Key.Enter, Key.NumPadEnter, Key.Back, Key.Escape -> {
                            isLocked = false
                            bumpControls()
                            true
                        }
                        else -> true
                    }
                }
                if (activeDrawer != null) {
                    if (ev.key == Key.Back || ev.key == Key.Escape) {
                        if (activeDrawer == PlayerDrawer.SUBTITLES && subtitleTranscriptOpen) {
                            subtitleTranscriptOpen = false
                            subtitleTranscriptQuery = ""
                        } else {
                            activeDrawer = null
                        }
                        bumpControls()
                        return@onPreviewKeyEvent true
                    }
                    if (isLive && activeDrawer == PlayerDrawer.EPISODES && ev.key == Key.DirectionRight) {
                        activeDrawer = null
                        return@onPreviewKeyEvent true
                    }
                    bumpControls()
                    return@onPreviewKeyEvent false
                }
                when (ev.nativeKeyEvent.keyCode) {
                    android.view.KeyEvent.KEYCODE_CHANNEL_UP,
                    android.view.KeyEvent.KEYCODE_PAGE_UP -> {
                        if (isLive && nextEpisode != null) {
                            playEpisode(nextEpisode)
                            return@onPreviewKeyEvent true
                        }
                    }
                    android.view.KeyEvent.KEYCODE_CHANNEL_DOWN,
                    android.view.KeyEvent.KEYCODE_PAGE_DOWN -> {
                        if (isLive && prevEpisode != null) {
                            playEpisode(prevEpisode)
                            return@onPreviewKeyEvent true
                        }
                    }
                }
                if (!controlsVisible) {
                    if (isTv && activeSkip != null && (skipIntroHasFocus || skipCloseHasFocus)) {
                        when (ev.key) {
                            Key.DirectionCenter, Key.Enter, Key.NumPadEnter -> return@onPreviewKeyEvent false
                            Key.DirectionRight -> if (skipIntroHasFocus) return@onPreviewKeyEvent false
                            Key.DirectionLeft -> if (skipCloseHasFocus) return@onPreviewKeyEvent false
                            else -> {}
                        }
                    }
                    return@onPreviewKeyEvent when (ev.key) {
                        Key.DirectionLeft -> {
                            if (isLive && episodes.size > 1) {
                                activeDrawer = PlayerDrawer.EPISODES
                                bumpControls()
                            } else if (!isLive) {
                                seekByMs(-10_000L)
                            } else {
                                bumpControls()
                            }
                            true
                        }
                        Key.MediaRewind -> {
                            if (!isLive) seekByMs(-10_000L) else bumpControls()
                            true
                        }
                        Key.DirectionRight, Key.MediaFastForward -> {
                            if (!isLive) seekByMs(10_000L) else bumpControls()
                            true
                        }
                        Key.Spacebar, Key.MediaPlayPause, Key.MediaPlay, Key.MediaPause -> {
                            togglePlayPause()
                            true
                        }
                        Key.Menu -> {
                            controlsVisible = true
                            activeDrawer = if (isLive && episodes.size > 1) PlayerDrawer.EPISODES else PlayerDrawer.SOURCES
                            true
                        }
                        Key.DirectionUp -> {
                            if (isLive && nextEpisode != null) {
                                playEpisode(nextEpisode)
                            } else {
                                bumpControls()
                            }
                            true
                        }
                        Key.DirectionDown -> {
                            if (isLive && prevEpisode != null) {
                                playEpisode(prevEpisode)
                            } else {
                                bumpControls()
                            }
                            true
                        }
                        Key.DirectionCenter, Key.Enter, Key.NumPadEnter -> {
                            bumpControls()
                            true
                        }
                        else -> false
                    }
                }
                when (ev.key) {
                    Key.Spacebar, Key.MediaPlayPause, Key.MediaPlay, Key.MediaPause -> {
                        togglePlayPause()
                        true
                    }
                    Key.MediaRewind -> {
                        seekByMs(-10_000L)
                        bumpControls()
                        true
                    }
                    Key.MediaFastForward -> {
                        seekByMs(10_000L)
                        bumpControls()
                        true
                    }
                    Key.Menu -> {
                        activeDrawer = PlayerDrawer.SOURCES
                        bumpControls()
                        true
                    }
                    Key.Back, Key.Escape -> {
                        controlsVisible = false
                        true
                    }
                    Key.DirectionUp, Key.DirectionDown, Key.DirectionLeft, Key.DirectionRight -> {
                        bumpControls()
                        false
                    }
                    else -> false
                }
            },
    ) {
        // Video Surface + Embedded SubtitleView (ExoPlayer or libmpv/libass)
        when (controller) {
            is ExoPlayerController -> {
                AndroidView(
                    factory = { ctx ->
                        PlayerView(ctx).apply {
                            controller.attachPlayerView(this)
                            useController = false
                            this.resizeMode = resizeMode
                            setShowBuffering(PlayerView.SHOW_BUFFERING_NEVER)
                            isFocusable = false
                            isFocusableInTouchMode = false
                            descendantFocusability = ViewGroup.FOCUS_BLOCK_DESCENDANTS
                            layoutParams = FrameLayout.LayoutParams(
                                ViewGroup.LayoutParams.MATCH_PARENT,
                                ViewGroup.LayoutParams.MATCH_PARENT,
                            )
                        }
                    },
                    update = { pv ->
                        controller.attachPlayerView(pv)
                        pv.useController = false
                        pv.resizeMode = resizeMode
                        pv.subtitleView?.let { sv ->
                            val sub = prefs.subtitles
                            val fg = parseHexColor(sub.textColor, Color.White).toArgb()
                            val bgAlpha = (sub.backgroundOpacity.coerceIn(0, 100) * 255) / 100
                            val bg = if (sub.outlineStyle == "none") {
                                android.graphics.Color.TRANSPARENT
                            } else {
                                android.graphics.Color.argb(bgAlpha, 8, 10, 16)
                            }
                            val edge = when (sub.outlineStyle) {
                                "shadow" -> CaptionStyleCompat.EDGE_TYPE_DROP_SHADOW
                                "none" -> CaptionStyleCompat.EDGE_TYPE_NONE
                                else -> CaptionStyleCompat.EDGE_TYPE_OUTLINE
                            }
                            sv.setStyle(
                                CaptionStyleCompat(
                                    fg,
                                    bg,
                                    android.graphics.Color.TRANSPARENT,
                                    edge,
                                    android.graphics.Color.BLACK,
                                    if (sub.bold) Typeface.DEFAULT_BOLD else Typeface.DEFAULT,
                                ),
                            )
                            sv.setFractionalTextSize((sub.fontSizePx / 520f).coerceIn(0.035f, 0.11f))
                            sv.setBottomPaddingFraction((sub.bottomOffsetPx / 720f).coerceIn(0.02f, 0.30f))
                        }
                    },
                    modifier = Modifier.fillMaxSize(),
                )
            }
            is MpvKitController -> {
                AndroidView(
                    factory = { ctx -> controller.getOrCreateView(ctx) },
                    update = { controller.setResizeMode(resizeMode) },
                    modifier = Modifier.fillMaxSize(),
                )
            }
        }

        // Touch Gesture Surface (Single tap toggle HUD, Double tap -10s/+10s, Long press 2x, Vertical swipe Brightness/Volume)
        Box(
            modifier = Modifier
                .fillMaxSize()
                .pointerInput(isLive, playbackSpeed) {
                    awaitPointerEventScope {
                        while (true) {
                            val ev = awaitPointerEvent(PointerEventPass.Final)
                            if (longPressSpeedActive && ev.changes.none { it.pressed }) {
                                longPressSpeedActive = false
                                controller.setSpeed(if (isLive) 1.0f else playbackSpeed)
                            }
                        }
                    }
                }
                .pointerInput(isLocked, isLive) {
                    detectTapGestures(
                        onTap = {
                            if (activeDrawer != null) {
                                activeDrawer = null
                                bumpControls()
                            } else if (!isLocked) {
                                if (controlsVisible) {
                                    controlsVisible = false
                                } else {
                                    bumpControls()
                                }
                            } else {
                                controlsVisible = true
                            }
                        },
                        onDoubleTap = { offset ->
                            if (isLocked || isLive) return@detectTapGestures
                            val width = size.width.toFloat().coerceAtLeast(1f)
                            val delta = if (offset.x < width * 0.45f) -10_000L else 10_000L
                            val target = (controller.state.value.positionMs + delta).coerceIn(
                                0L,
                                totalDurMs.takeIf { it > 0L } ?: Long.MAX_VALUE,
                            )
                            controller.seekTo(target)
                            currentPosMs = target
                            showHud(if (delta < 0) VesselI18n.tr("player.seek_back_10") else VesselI18n.tr("player.seek_fwd_10"))
                            bumpControls()
                        },
                        onLongPress = {
                            if (isLocked || isLive || activeDrawer != null) return@detectTapGestures
                            longPressSpeedActive = true
                            controller.setSpeed(2.0f)
                        },
                    )
                }
                .pointerInput(isLocked, isLive, activeDrawer, prevEpisode, nextEpisode) {
                    var totalDragX = 0f
                    detectHorizontalDragGestures(
                        onDragStart = { totalDragX = 0f },
                        onHorizontalDrag = { _, dragAmount ->
                            totalDragX += dragAmount
                        },
                        onDragEnd = {
                            if (!isLocked && isLive && activeDrawer == null) {
                                if (totalDragX < -90f && nextEpisode != null) {
                                    playEpisode(nextEpisode)
                                } else if (totalDragX > 90f && prevEpisode != null) {
                                    playEpisode(prevEpisode)
                                }
                            }
                            totalDragX = 0f
                        },
                        onDragCancel = { totalDragX = 0f },
                    )
                }
                .pointerInput(isLocked) {
                    var startX = 0f
                    detectVerticalDragGestures(
                        onDragStart = { startX = it.x },
                        onVerticalDrag = { _, dragAmount ->
                            if (isLocked || activeDrawer != null) return@detectVerticalDragGestures
                            val width = size.width.toFloat().coerceAtLeast(1f)
                            val height = size.height.toFloat().coerceAtLeast(1f)
                            val deltaRatio = (-dragAmount / height) * 1.4f
                            if (startX < width / 2f) {
                                val win = activity?.window
                                if (win != null) {
                                    val lp = win.attributes
                                    val cur = if (lp.screenBrightness < 0f) 0.5f else lp.screenBrightness
                                    val next = (cur + deltaRatio).coerceIn(0.05f, 1.0f)
                                    lp.screenBrightness = next
                                    win.attributes = lp
                                    showHud("${VesselI18n.tr("player.brightness", "Parlaklık")} ${(next * 100).roundToInt()}%")
                                }
                            } else if (audioManager != null) {
                                val maxVol = audioManager.getStreamMaxVolume(AudioManager.STREAM_MUSIC).coerceAtLeast(1)
                                val curVol = audioManager.getStreamVolume(AudioManager.STREAM_MUSIC)
                                val step = if (abs(deltaRatio) > 0.035f) (if (deltaRatio > 0) 1 else -1) else 0
                                if (step != 0) {
                                    val nextVol = (curVol + step).coerceIn(0, maxVol)
                                    audioManager.setStreamVolume(AudioManager.STREAM_MUSIC, nextVol, 0)
                                    isMuted = nextVol == 0
                                    showHud("${VesselI18n.tr("player.volume", "Ses")} ${(nextVol * 100) / maxVol}%")
                                }
                            }
                        },
                    )
                },
        )

        // Live Player Stats HUD Overlay
        if (showStatsHud && playbackStats != null) {
            val st = playbackStats!!
            Surface(
                shape = RoundedCornerShape(12.dp),
                color = Color(0xD80C0D14),
                border = BorderStroke(1.dp, Color.White.copy(alpha = 0.14f)),
                modifier = Modifier
                    .align(Alignment.TopStart)
                    .systemBarsPadding()
                    .padding(start = 16.dp, top = 74.dp),
            ) {
                Column(
                    modifier = Modifier.padding(12.dp).widthIn(min = 210.dp, max = 280.dp),
                    verticalArrangement = Arrangement.spacedBy(4.dp),
                ) {
                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        horizontalArrangement = Arrangement.SpaceBetween,
                        verticalAlignment = Alignment.CenterVertically,
                    ) {
                        Text(
                            text = VesselI18n.tr("stats.title", "Stream Stats"),
                            color = palette.accent,
                            style = MaterialTheme.typography.labelMedium,
                            fontWeight = FontWeight.Bold,
                        )
                        Icon(
                            Icons.Default.Close,
                            contentDescription = VesselI18n.tr("common.close"),
                            tint = Color.White.copy(alpha = 0.7f),
                            modifier = Modifier.size(14.dp).clickable { showStatsHud = false },
                        )
                    }
                    Text(
                        text = "${VesselI18n.tr("stats.speed", "Speed")}: ${formatSizeBytes(st.networkSpeedBps)}/s",
                        color = Color.White,
                        style = MaterialTheme.typography.labelSmall,
                        fontFamily = FontFamily.Monospace,
                    )
                    Text(
                        text = "${VesselI18n.tr("stats.transferred", "Transferred")}: ${formatSizeBytes(st.totalBytesTransferred)}",
                        color = Color.White.copy(alpha = 0.85f),
                        style = MaterialTheme.typography.labelSmall,
                        fontFamily = FontFamily.Monospace,
                    )
                    st.bufferSeconds?.let { buf ->
                        Text(
                            text = "${VesselI18n.tr("stats.buffer", "Buffer")}: ${String.format(Locale.US, "%.1fs", buf)}",
                            color = Color.White.copy(alpha = 0.85f),
                            style = MaterialTheme.typography.labelSmall,
                            fontFamily = FontFamily.Monospace,
                        )
                    }
                    if (st.width != null && st.height != null) {
                        val fpsLabel = st.fps?.let { String.format(Locale.US, " · %.0ffps", it) }.orEmpty()
                        Text(
                            text = "${VesselI18n.tr("stats.resolution", "Resolution")}: ${st.width}x${st.height}$fpsLabel",
                            color = Color.White.copy(alpha = 0.85f),
                            style = MaterialTheme.typography.labelSmall,
                            fontFamily = FontFamily.Monospace,
                        )
                    }
                    st.videoCodec?.let { vc ->
                        val br = st.videoBitrateBps?.let { " · ${it / 1000} kbps" }.orEmpty()
                        Text(
                            text = "${VesselI18n.tr("stats.video", "Video")}: $vc$br",
                            color = Color.White.copy(alpha = 0.8f),
                            style = MaterialTheme.typography.labelSmall,
                            fontFamily = FontFamily.Monospace,
                            maxLines = 1,
                            overflow = TextOverflow.Ellipsis,
                        )
                    }
                    st.audioCodec?.let { ac ->
                        val br = st.audioBitrateBps?.let { " · ${it / 1000} kbps" }.orEmpty()
                        Text(
                            text = "${VesselI18n.tr("stats.audio", "Audio")}: $ac$br",
                            color = Color.White.copy(alpha = 0.8f),
                            style = MaterialTheme.typography.labelSmall,
                            fontFamily = FontFamily.Monospace,
                            maxLines = 1,
                            overflow = TextOverflow.Ellipsis,
                        )
                    }
                    st.fileSizeBytes?.takeIf { it > 0L }?.let { sz ->
                        Text(
                            text = "${VesselI18n.tr("stats.size", "Size")}: ${formatSizeBytes(sz)}",
                            color = Color.White.copy(alpha = 0.8f),
                            style = MaterialTheme.typography.labelSmall,
                            fontFamily = FontFamily.Monospace,
                        )
                    }
                    Text(
                        text = "${VesselI18n.tr("stats.source", "Transport")}: ${if (st.isDebridStream) "Debrid" else if (lastLoadWasProxied) "Proxy" else "Direct"}",
                        color = palette.accent,
                        style = MaterialTheme.typography.labelSmall,
                        fontFamily = FontFamily.Monospace,
                    )
                }
            }
        }

        // In-App Styled Subtitle Overlay (Desktop PlayerView .cs-subtitle-overlay parity)
        if (activeExternalLines.isNotEmpty() && !isLive) {
            val sub = prefs.subtitles
            val baseBottomDp = (sub.bottomOffsetPx.coerceIn(12, 220) * 0.45f).dp
            val raisedBottomDp = if (controlsVisible && !isLocked) baseBottomDp + 96.dp else baseBottomDp
            val bgAlpha = (sub.backgroundOpacity.coerceIn(0, 100) / 100f)
            val lineBg = if (sub.outlineStyle == "none") Color.Transparent else Color(0xFF08090E).copy(alpha = bgAlpha)
            val fontFamily = when (sub.fontFamily) {
                "serif" -> FontFamily.Serif
                "monospace" -> FontFamily.Monospace
                else -> FontFamily.SansSerif
            }
            val textShadow = when (sub.outlineStyle) {
                "none" -> null
                "shadow" -> Shadow(color = Color.Black.copy(alpha = 0.95f), offset = Offset(0f, 3f), blurRadius = 8f)
                else -> Shadow(color = Color.Black, offset = Offset(1.5f, 1.5f), blurRadius = 4f)
            }
            Column(
                modifier = Modifier
                    .align(Alignment.BottomCenter)
                    .padding(horizontal = 24.dp)
                    .padding(bottom = raisedBottomDp),
                horizontalAlignment = Alignment.CenterHorizontally,
                verticalArrangement = Arrangement.spacedBy(4.dp),
            ) {
                activeExternalLines.forEach { line ->
                    Box(
                        modifier = Modifier
                            .clip(RoundedCornerShape(6.dp))
                            .background(lineBg)
                            .padding(horizontal = 10.dp, vertical = 4.dp),
                    ) {
                        Text(
                            text = line,
                            textAlign = TextAlign.Center,
                            style = TextStyle(
                                color = parseHexColor(sub.textColor, Color.White),
                                fontSize = (sub.fontSizePx.coerceIn(14, 52) * 0.68f).sp,
                                fontWeight = if (sub.bold) FontWeight.Bold else FontWeight.SemiBold,
                                fontFamily = fontFamily,
                                shadow = textShadow,
                            ),
                        )
                    }
                }
            }
        }

        // Top & Bottom Cinematic Vignettes
        AnimatedVisibility(
            visible = controlsVisible && !isLocked,
            enter = fadeIn(),
            exit = fadeOut(),
        ) {
            Box(modifier = Modifier.fillMaxSize()) {
                Box(
                    modifier = Modifier
                        .fillMaxWidth()
                        .height(160.dp)
                        .align(Alignment.TopCenter)
                        .background(
                            Brush.verticalGradient(
                                listOf(Color.Black.copy(alpha = 0.82f), Color.Transparent),
                            ),
                        ),
                )
                Box(
                    modifier = Modifier
                        .fillMaxWidth()
                        .height(200.dp)
                        .align(Alignment.BottomCenter)
                        .background(
                            Brush.verticalGradient(
                                listOf(Color.Transparent, Color.Black.copy(alpha = 0.88f)),
                            ),
                        ),
                )
            }
        }

        // Gesture Feedback Pill
        AnimatedVisibility(
            visible = longPressSpeedActive || gestureHudLabel != null,
            enter = fadeIn(),
            exit = fadeOut(),
            modifier = Modifier
                .align(Alignment.TopCenter)
                .systemBarsPadding()
                .padding(top = 72.dp),
        ) {
            Surface(
                shape = RoundedCornerShape(999.dp),
                color = Color(0xD60A0B12),
                border = BorderStroke(1.dp, Color.White.copy(alpha = 0.18f)),
            ) {
                Text(
                    text = if (longPressSpeedActive) "2x »" else gestureHudLabel.orEmpty(),
                    color = Color.White,
                    fontWeight = FontWeight.Bold,
                    style = MaterialTheme.typography.labelLarge,
                    modifier = Modifier.padding(horizontal = 16.dp, vertical = 8.dp),
                )
            }
        }

        // Center Buffering / Loading Indicator (shown standalone when HUD is hidden)
        if ((isBuffering || switchingEpisode) && playbackError == null && (!controlsVisible || isLocked)) {
            Column(
                modifier = Modifier.align(Alignment.Center),
                horizontalAlignment = Alignment.CenterHorizontally,
                verticalArrangement = Arrangement.spacedBy(10.dp),
            ) {
                CircularProgressIndicator(color = palette.accent, strokeWidth = 3.dp)
                Text(
                    text = if (switchingEpisode) {
                        VesselI18n.tr("player.loading_episode", "Sonraki bölüm yükleniyor…")
                    } else {
                        VesselI18n.tr("common.loading", "Tamponlanıyor…")
                    },
                    color = Color.White.copy(alpha = 0.88f),
                    style = MaterialTheme.typography.labelMedium,
                )
            }
        }

        // Desktop-Parity Error Overlay
        if (playbackError != null) {
            Box(
                modifier = Modifier
                    .fillMaxSize()
                    .background(Color(0xD905050A))
                    .clickable(
                        interactionSource = remember { MutableInteractionSource() },
                        indication = null,
                    ) {},
                contentAlignment = Alignment.Center,
            ) {
                Surface(
                    shape = RoundedCornerShape(18.dp),
                    color = palette.bgSurface,
                    border = BorderStroke(1.dp, palette.border.copy(alpha = 0.55f)),
                    modifier = Modifier
                        .widthIn(max = 440.dp)
                        .fillMaxWidth()
                        .padding(horizontal = 24.dp),
                ) {
                    Column(
                        modifier = Modifier.padding(22.dp),
                        verticalArrangement = Arrangement.spacedBy(12.dp),
                    ) {
                        Text(
                            text = VesselI18n.tr("player.error_title", "Oynatma Sorunu"),
                            style = MaterialTheme.typography.titleMedium,
                            fontWeight = FontWeight.Bold,
                            color = palette.textPrimary,
                        )
                        Text(
                            text = playbackError.orEmpty(),
                            style = MaterialTheme.typography.bodySmall,
                            color = palette.textMuted,
                        )
                        FlowRow(
                            horizontalArrangement = Arrangement.spacedBy(8.dp),
                            verticalArrangement = Arrangement.spacedBy(8.dp),
                        ) {
                            Button(
                                onClick = {
                                    val cand = activeCandidate ?: return@Button
                                    forceHlsRetry = false
                                    forceProxyRetry = false
                                    scope.launch {
                                        loadCandidate(
                                            cand = cand,
                                            startPosMs = if (isLive) 0L else currentPosMs,
                                            forceHls = false,
                                            forceProxy = false,
                                        )
                                    }
                                },
                                colors = ButtonDefaults.buttonColors(containerColor = palette.accent),
                                modifier = Modifier
                                    .focusRequester(errorRetryFocus)
                                    .vesselFocusBorder(RoundedCornerShape(999.dp)),
                            ) {
                                Text(VesselI18n.tr("common.retry", "Tekrar Dene"), color = Color(0xFF0B0B12))
                            }
                            if (activeStreamResult.streams.size > 1) {
                                OutlinedButton(
                                    onClick = { activeDrawer = PlayerDrawer.SOURCES },
                                    modifier = Modifier.vesselFocusBorder(RoundedCornerShape(999.dp)),
                                ) {
                                    Text("${VesselI18n.tr("details.sources", "Kaynaklar")} (${activeStreamResult.streams.size})")
                                }
                            }
                            if (!isLive) {
                                TextButton(
                                    onClick = onGoToDetails,
                                    modifier = Modifier.vesselFocusBorder(RoundedCornerShape(999.dp)),
                                ) {
                                    Text(VesselI18n.tr("player.go_to_details"))
                                }
                            }
                        }
                    }
                }
            }
        }

        // Locked Screen Floating Unlock Pill (Desktop .cs-unlock-pill parity)
        AnimatedVisibility(
            visible = isLocked,
            enter = fadeIn(),
            exit = fadeOut(),
            modifier = Modifier
                .align(Alignment.TopEnd)
                .systemBarsPadding()
                .padding(top = 14.dp, end = 16.dp),
        ) {
                Surface(
                shape = RoundedCornerShape(999.dp),
                color = Color(0xE010101A),
                border = BorderStroke(1.dp, palette.accent.copy(alpha = 0.55f)),
                modifier = Modifier
                    .clip(RoundedCornerShape(999.dp))
                    .vesselFocusBorder(RoundedCornerShape(999.dp))
                    .clickable {
                        isLocked = false
                        bumpControls()
                    },
            ) {
                Row(
                    verticalAlignment = Alignment.CenterVertically,
                    horizontalArrangement = Arrangement.spacedBy(7.dp),
                    modifier = Modifier.padding(horizontal = 14.dp, vertical = 9.dp),
                ) {
                    Icon(Icons.Default.Lock, contentDescription = null, tint = Color.White, modifier = Modifier.size(15.dp))
                    Text(
                        text = VesselI18n.tr("player.unlock_hint", "Ekran Kilitli — Açmak için dokun"),
                        color = Color.White,
                        style = MaterialTheme.typography.labelMedium,
                        fontWeight = FontWeight.SemiBold,
                    )
                }
            }
        }

        // Full Desktop-Parity HUD Overlay
        AnimatedVisibility(
            visible = controlsVisible && !isLocked,
            enter = fadeIn(),
            exit = fadeOut(),
            modifier = Modifier.fillMaxSize(),
        ) {
            BoxWithConstraints(
                modifier = Modifier
                    .fillMaxSize()
                    .focusProperties { canFocus = activeDrawer == null }
                    .systemBarsPadding()
                    .padding(horizontal = 14.dp, vertical = 10.dp),
            ) {
                val wideLayout = maxWidth >= 680.dp

                // TOP BAR (Desktop .cs-top-bar)
                Column(
                    modifier = Modifier
                        .fillMaxWidth()
                        .align(Alignment.TopCenter),
                    verticalArrangement = Arrangement.spacedBy(8.dp),
                ) {
                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        verticalAlignment = Alignment.CenterVertically,
                        horizontalArrangement = Arrangement.spacedBy(10.dp),
                    ) {
                        HudCircleIconButton(
                            onClick = onBack,
                            size = 34.dp,
                            modifier = Modifier
                                .focusRequester(backBtnFocus)
                                .focusProperties {
                                    down = if (!wideLayout || isTv) sourcesPillFocus else playPauseFocusRequester
                                },
                        ) {
                            Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = VesselI18n.tr("common.back"), tint = Color.White, modifier = Modifier.size(17.dp))
                        }

                        Column(
                            modifier = Modifier
                                .weight(1f)
                                .let { m ->
                                    if (!isLive && !isTv) {
                                        m.clip(RoundedCornerShape(8.dp))
                                            .vesselFocusBorder(RoundedCornerShape(8.dp))
                                            .clickable { onGoToDetails() }
                                    } else m
                                },
                            verticalArrangement = Arrangement.spacedBy(2.dp),
                        ) {
                            Row(
                                verticalAlignment = Alignment.CenterVertically,
                                horizontalArrangement = Arrangement.spacedBy(7.dp),
                            ) {
                                if (isLive) {
                                    Surface(
                                        shape = RoundedCornerShape(999.dp),
                                        color = Color(0x38EF4444),
                                        border = BorderStroke(1.dp, Color(0x8CF87171)),
                                    ) {
                                        Row(
                                            verticalAlignment = Alignment.CenterVertically,
                                            horizontalArrangement = Arrangement.spacedBy(5.dp),
                                            modifier = Modifier.padding(horizontal = 8.dp, vertical = 2.dp),
                                        ) {
                                            Box(
                                                modifier = Modifier
                                                    .size(7.dp)
                                                    .clip(CircleShape)
                                                    .background(Color(0xFFEF4444)),
                                            )
                                            Text(
                                                text = "LIVE",
                                                color = Color(0xFFFECACA),
                                                style = MaterialTheme.typography.labelSmall,
                                                fontWeight = FontWeight.Bold,
                                            )
                                        }
                                    }
                                }
                                Text(
                                    text = if (isLive) (currentEpisode.title ?: activeDetail.title) else activeDetail.title,
                                    color = Color.White,
                                    style = MaterialTheme.typography.titleMedium,
                                    fontWeight = FontWeight.Bold,
                                    maxLines = 1,
                                    overflow = TextOverflow.Ellipsis,
                                )
                            }

                            Row(
                                verticalAlignment = Alignment.CenterVertically,
                                horizontalArrangement = Arrangement.spacedBy(6.dp),
                            ) {
                                val epMeta = buildString {
                                    if (isLive) {
                                        currentEpisode.overview?.takeIf { it.isNotBlank() }?.let { append(it) }
                                            ?: activeDetail.genres.takeIf { it.isNotEmpty() }?.let { append(it.joinToString(" · ")) }
                                    } else if (activeDetail.type != "movie") {
                                        append("S${currentEpisode.season} · E${currentEpisode.episode}")
                                        currentEpisode.title?.takeIf { it.isNotBlank() }?.let { append(" — $it") }
                                    } else {
                                        currentEpisode.title?.takeIf { it.isNotBlank() && it != activeDetail.title }?.let { append(it) }
                                    }
                                }
                                if (epMeta.isNotBlank()) {
                                    Text(
                                        text = epMeta,
                                        color = Color.White.copy(alpha = 0.78f),
                                        style = MaterialTheme.typography.labelSmall,
                                        maxLines = 1,
                                        overflow = TextOverflow.Ellipsis,
                                        modifier = Modifier.weight(1f, fill = false),
                                    )
                                }
                                activeCandidate?.let { cand ->
                                    val q = cand.quality ?: cand.kind.uppercase(Locale.US)
                                    val srcLabel = cand.pluginName ?: cand.pluginId
                                    val chipText = listOfNotNull(
                                        srcLabel?.takeIf { it.isNotBlank() && it != pluginId },
                                        cand.label,
                                        q.takeIf { it.isNotBlank() },
                                    ).joinToString(" · ")
                                    Surface(
                                        shape = RoundedCornerShape(999.dp),
                                        color = palette.accent.copy(alpha = 0.20f),
                                        border = BorderStroke(1.dp, palette.accent.copy(alpha = 0.42f)),
                                    ) {
                                        Text(
                                            text = chipText,
                                            color = palette.accent.copy(alpha = 0.95f),
                                            style = MaterialTheme.typography.labelSmall,
                                            fontWeight = FontWeight.SemiBold,
                                            maxLines = 1,
                                            overflow = TextOverflow.Ellipsis,
                                            modifier = Modifier.padding(horizontal = 8.dp, vertical = 2.dp),
                                        )
                                    }
                                }
                            }
                        }

                        if (wideLayout && !isTv) {
                            TopActionPillsRow(
                                pluginId = pluginId,
                                isLive = isLive,
                                isTv = isTv,
                                episodesCount = episodes.size,
                                streamsCount = activeStreamResult.streams.size,
                                audioCount = audioTracks.size,
                                activeDrawer = activeDrawer,
                                showStatsHud = showStatsHud,
                                isLandscape = isLandscape,
                                sourcesPillFocus = sourcesPillFocus,
                                backBtnFocus = backBtnFocus,
                                centerFocus = playPauseFocusRequester,
                                onGoToDetails = onGoToDetails,
                                onLock = {
                                    isLocked = true
                                    activeDrawer = null
                                },
                                onToggleStats = {
                                    showStatsHud = !showStatsHud
                                    bumpControls()
                                },
                                onToggleDrawer = { d ->
                                    activeDrawer = if (activeDrawer == d) null else d
                                    bumpControls()
                                },
                                onToggleOrientation = ::toggleOrientation,
                                modifier = Modifier.horizontalScroll(rememberScrollState()),
                            )
                        }
                    }

                    if (!wideLayout || isTv) {
                        TopActionPillsRow(
                            pluginId = pluginId,
                            isLive = isLive,
                            isTv = isTv,
                            episodesCount = episodes.size,
                            streamsCount = activeStreamResult.streams.size,
                            audioCount = audioTracks.size,
                            activeDrawer = activeDrawer,
                            showStatsHud = showStatsHud,
                            isLandscape = isLandscape,
                            sourcesPillFocus = sourcesPillFocus,
                            backBtnFocus = backBtnFocus,
                            centerFocus = playPauseFocusRequester,
                            onGoToDetails = onGoToDetails,
                            onLock = {
                                isLocked = true
                                activeDrawer = null
                            },
                            onToggleStats = {
                                showStatsHud = !showStatsHud
                                bumpControls()
                            },
                            onToggleDrawer = { d ->
                                activeDrawer = if (activeDrawer == d) null else d
                                bumpControls()
                            },
                            onToggleOrientation = ::toggleOrientation,
                            modifier = Modifier
                                .fillMaxWidth()
                                .horizontalScroll(rememberScrollState()),
                        )
                    }
                }

                // CENTER TRANSPORT CONTROLS (Always mounted while HUD is visible so D-pad center focus never drops)
                if (playbackError == null) {
                    val downTargetFromCenter = if (!isLive) scrubberFocus else bottomPlayPauseFocus
                    Row(
                        modifier = Modifier.align(Alignment.Center),
                        verticalAlignment = Alignment.CenterVertically,
                        horizontalArrangement = Arrangement.spacedBy(if (wideLayout) 20.dp else 14.dp),
                    ) {
                        if (!isLive || episodes.size > 1) {
                            HudCircleIconButton(
                                enabled = prevEpisode != null,
                                onClick = { prevEpisode?.let(::playEpisode) },
                                size = 40.dp,
                                modifier = Modifier.focusProperties {
                                    up = sourcesPillFocus
                                    down = downTargetFromCenter
                                },
                            ) {
                                Icon(
                                    Icons.Default.SkipPrevious,
                                    contentDescription = "Previous",
                                    tint = if (prevEpisode != null) Color.White else Color.White.copy(alpha = 0.35f),
                                    modifier = Modifier.size(20.dp),
                                )
                            }
                        }
                        if (!isLive) {
                            HudCircleIconButton(
                                onClick = {
                                    val target = (controller.state.value.positionMs - 10_000L).coerceAtLeast(0L)
                                    controller.seekTo(target)
                                    currentPosMs = target
                                    bumpControls()
                                },
                                size = 40.dp,
                                modifier = Modifier.focusProperties {
                                    up = sourcesPillFocus
                                    down = downTargetFromCenter
                                },
                            ) {
                                Icon(Icons.Default.Replay10, contentDescription = "-10s", tint = Color.White, modifier = Modifier.size(20.dp))
                            }
                        }
                        Surface(
                            shape = CircleShape,
                            color = palette.accent,
                            shadowElevation = 8.dp,
                            modifier = Modifier
                                .size(54.dp)
                                .clip(CircleShape)
                                .focusRequester(playPauseFocusRequester)
                                .focusProperties {
                                    up = sourcesPillFocus
                                    down = downTargetFromCenter
                                }
                                .vesselFocusBorder(CircleShape, width = 3.dp)
                                .clickable { togglePlayPause() },
                        ) {
                            Box(contentAlignment = Alignment.Center) {
                                if (isBuffering || switchingEpisode) {
                                    CircularProgressIndicator(
                                        color = Color(0xFF0B0B12),
                                        strokeWidth = 2.5.dp,
                                        modifier = Modifier.size(24.dp),
                                    )
                                } else {
                                    Icon(
                                        imageVector = if (isPlaying) Icons.Default.Pause else Icons.Default.PlayArrow,
                                        contentDescription = if (isPlaying) "Pause" else "Play",
                                        tint = Color(0xFF0B0B12),
                                        modifier = Modifier.size(26.dp),
                                    )
                                }
                            }
                        }
                        if (!isLive) {
                            HudCircleIconButton(
                                onClick = {
                                    val cap = totalDurMs.takeIf { it > 0L } ?: Long.MAX_VALUE
                                    val target = (controller.state.value.positionMs + 10_000L).coerceAtMost(cap)
                                    controller.seekTo(target)
                                    currentPosMs = target
                                    bumpControls()
                                },
                                size = 40.dp,
                                modifier = Modifier.focusProperties {
                                    up = sourcesPillFocus
                                    down = downTargetFromCenter
                                },
                            ) {
                                Icon(Icons.Default.Forward10, contentDescription = "+10s", tint = Color.White, modifier = Modifier.size(20.dp))
                            }
                        }
                        if (!isLive || episodes.size > 1) {
                            HudCircleIconButton(
                                enabled = nextEpisode != null,
                                onClick = { nextEpisode?.let(::playEpisode) },
                                size = 40.dp,
                                modifier = Modifier.focusProperties {
                                    up = sourcesPillFocus
                                    down = downTargetFromCenter
                                },
                            ) {
                                Icon(
                                    Icons.Default.SkipNext,
                                    contentDescription = "Next",
                                    tint = if (nextEpisode != null) Color.White else Color.White.copy(alpha = 0.35f),
                                    modifier = Modifier.size(20.dp),
                                )
                            }
                        }
                    }
                }

                // BOTTOM BAR (Desktop .cs-bottom-bar)
                Column(
                    modifier = Modifier
                        .fillMaxWidth()
                        .align(Alignment.BottomCenter),
                    verticalArrangement = Arrangement.spacedBy(8.dp),
                ) {
                    if (!isLive) {
                        val effectiveDur = totalDurMs.coerceAtLeast(1L)
                        val displayPos = if (isScrubbing) scrubPreviewMs else currentPosMs.coerceIn(0L, effectiveDur)
                        Row(
                            modifier = Modifier.fillMaxWidth(),
                            verticalAlignment = Alignment.CenterVertically,
                            horizontalArrangement = Arrangement.spacedBy(10.dp),
                        ) {
                            Text(
                                text = formatClockMs(displayPos),
                                color = Color.White.copy(alpha = 0.9f),
                                style = MaterialTheme.typography.labelSmall,
                                modifier = Modifier.widthIn(min = 42.dp),
                            )
                            DesktopScrubberBar(
                                currentMs = displayPos,
                                bufferedMs = bufferedPosMs,
                                durationMs = totalDurMs,
                                skipSegments = validSkipSegments,
                                accentColor = palette.accent,
                                isScrubbing = isScrubbing,
                                onScrubChange = { pos ->
                                    isScrubbing = true
                                    scrubPreviewMs = pos
                                    bumpControls()
                                },
                                onScrubCommit = { pos ->
                                    controller.seekTo(pos)
                                    currentPosMs = pos
                                    isScrubbing = false
                                    bumpControls()
                                },
                                onTogglePlayPause = ::togglePlayPause,
                                scrubberModifier = Modifier
                                    .focusRequester(scrubberFocus)
                                    .focusProperties {
                                        up = playPauseFocusRequester
                                        down = bottomPlayPauseFocus
                                    },
                                modifier = Modifier.weight(1f),
                            )
                            Text(
                                text = formatClockMs(totalDurMs),
                                color = Color.White.copy(alpha = 0.9f),
                                style = MaterialTheme.typography.labelSmall,
                                textAlign = TextAlign.End,
                                modifier = Modifier.widthIn(min = 42.dp),
                            )
                        }
                    } else {
                        // Desktop .cs-live-bar parity
                        Row(
                            modifier = Modifier.fillMaxWidth().padding(vertical = 4.dp),
                            verticalAlignment = Alignment.CenterVertically,
                            horizontalArrangement = Arrangement.spacedBy(10.dp),
                        ) {
                            Surface(
                                shape = RoundedCornerShape(999.dp),
                                color = Color(0x38EF4444),
                                border = BorderStroke(1.dp, Color(0x8CF87171)),
                            ) {
                                Row(
                                    verticalAlignment = Alignment.CenterVertically,
                                    horizontalArrangement = Arrangement.spacedBy(5.dp),
                                    modifier = Modifier.padding(horizontal = 9.dp, vertical = 3.dp),
                                ) {
                                    Box(
                                        modifier = Modifier
                                            .size(7.dp)
                                            .clip(CircleShape)
                                            .background(Color(0xFFEF4444)),
                                    )
                                    Text(
                                        text = VesselI18n.tr("player.live_broadcast", "CANLI YAYIN"),
                                        color = Color(0xFFFECACA),
                                        style = MaterialTheme.typography.labelSmall,
                                        fontWeight = FontWeight.Bold,
                                    )
                                }
                            }
                            Box(
                                modifier = Modifier
                                    .weight(1f)
                                    .height(4.dp)
                                    .clip(RoundedCornerShape(999.dp))
                                    .background(
                                        Brush.horizontalGradient(
                                            listOf(Color(0xD9EF4444), palette.accent.copy(alpha = 0.65f)),
                                        ),
                                    ),
                            )
                            val liveMeta = activeCandidate?.quality ?: activeCandidate?.kind?.uppercase(Locale.US)
                            if (!liveMeta.isNullOrBlank()) {
                                Text(
                                    text = liveMeta,
                                    color = Color.White.copy(alpha = 0.82f),
                                    style = MaterialTheme.typography.labelSmall,
                                    fontWeight = FontWeight.SemiBold,
                                )
                            }
                        }
                    }

                    // Bottom Actions Row (Desktop .cs-bottom-actions)
                    val bottomUpTarget = if (!isLive) scrubberFocus else playPauseFocusRequester
                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        horizontalArrangement = Arrangement.SpaceBetween,
                        verticalAlignment = Alignment.CenterVertically,
                    ) {
                        Row(
                            verticalAlignment = Alignment.CenterVertically,
                            horizontalArrangement = Arrangement.spacedBy(8.dp),
                        ) {
                            HudCircleIconButton(
                                onClick = ::togglePlayPause,
                                size = 36.dp,
                                modifier = Modifier
                                    .focusRequester(bottomPlayPauseFocus)
                                    .focusProperties { up = bottomUpTarget },
                            ) {
                                Icon(
                                    if (isPlaying) Icons.Default.Pause else Icons.Default.PlayArrow,
                                    contentDescription = "Play/Pause",
                                    tint = Color.White,
                                    modifier = Modifier.size(18.dp),
                                )
                            }
                            HudCircleIconButton(
                                onClick = {
                                    isMuted = !isMuted
                                    controller.setVolume(if (isMuted) 0f else playerVolume)
                                    bumpControls()
                                },
                                size = 36.dp,
                                modifier = Modifier.focusProperties { up = bottomUpTarget },
                            ) {
                                Icon(
                                    if (isMuted || playerVolume == 0f) Icons.AutoMirrored.Filled.VolumeOff else Icons.AutoMirrored.Filled.VolumeUp,
                                    contentDescription = "Mute",
                                    tint = Color.White,
                                    modifier = Modifier.size(18.dp),
                                )
                            }
                            if (wideLayout && !isTv) {
                                Slider(
                                    value = if (isMuted) 0f else playerVolume,
                                    onValueChange = { v ->
                                        playerVolume = v.coerceIn(0f, 1f)
                                        isMuted = playerVolume == 0f
                                        controller.setVolume(playerVolume)
                                        bumpControls()
                                    },
                                    valueRange = 0f..1f,
                                    colors = SliderDefaults.colors(
                                        thumbColor = Color.White,
                                        activeTrackColor = palette.accent,
                                        inactiveTrackColor = Color.White.copy(alpha = 0.25f),
                                    ),
                                    modifier = Modifier.width(86.dp).height(24.dp),
                                )
                            }
                        }

                        Row(
                            verticalAlignment = Alignment.CenterVertically,
                            horizontalArrangement = Arrangement.spacedBy(8.dp),
                        ) {
                            if (!isLive) {
                                PlayerHudPill(
                                    icon = null,
                                    label = "${playbackSpeed}x",
                                    active = playbackSpeed != 1.0f,
                                    modifier = Modifier.focusProperties { up = bottomUpTarget },
                                    onClick = {
                                        activeDrawer = if (activeDrawer == PlayerDrawer.SETTINGS) null else PlayerDrawer.SETTINGS
                                        bumpControls()
                                    },
                                )
                            }
                            PlayerHudPill(
                                icon = null,
                                label = fitLabel,
                                active = resizeMode != AspectRatioFrameLayout.RESIZE_MODE_FIT,
                                modifier = Modifier.focusProperties { up = bottomUpTarget },
                                onClick = ::cycleFitMode,
                            )
                            if (!isLive && nextEpisode != null) {
                                PlayerHudPill(
                                    icon = { Icon(Icons.Default.SkipNext, contentDescription = null, tint = Color.White, modifier = Modifier.size(13.dp)) },
                                    label = VesselI18n.trf("player.next_ep_short", "ep" to nextEpisode.episode),
                                    active = true,
                                    modifier = Modifier.focusProperties { up = bottomUpTarget },
                                    onClick = { playEpisode(nextEpisode) },
                                )
                            }
                            if (!isTv) {
                                HudCircleIconButton(
                                    onClick = ::toggleOrientation,
                                    size = 32.dp,
                                    modifier = Modifier.focusProperties { up = bottomUpTarget },
                                ) {
                                    Icon(
                                        if (isLandscape) Icons.Default.FullscreenExit else Icons.Default.Fullscreen,
                                        contentDescription = VesselI18n.tr("common.landscape"),
                                        tint = Color.White,
                                        modifier = Modifier.size(16.dp),
                                    )
                                }
                            }
                        }
                    }
                }
            }
        }

        // Floating Verified Skip Segment Button (Desktop .cs-skip-floating parity)
        AnimatedVisibility(
            visible = !isLocked && activeSkip != null && activeDrawer == null,
            enter = fadeIn(tween(220)) + scaleIn(initialScale = 0.94f, animationSpec = tween(220)),
            exit = fadeOut(tween(180)) + scaleOut(targetScale = 0.96f, animationSpec = tween(180)),
            modifier = Modifier
                .align(Alignment.BottomEnd)
                .systemBarsPadding()
                .padding(end = 16.dp, bottom = if (controlsVisible) 104.dp else 20.dp),
        ) {
            activeSkip?.let { seg ->
                val durMs = totalDurMs.takeIf { it > 10_000L } ?: 0L
                val nearEnd = seg.endMs == null || (durMs > 0L && seg.endMs >= durMs - 4000L)
                val skipType = seg.type.lowercase(Locale.US)
                val jumpToNext = (skipType == "credits" || skipType == "preview") && nearEnd && nextEpisode != null
                val skipLabel = when {
                    jumpToNext -> VesselI18n.tr("player.next_episode")
                    skipType == "recap" -> VesselI18n.tr("player.skip_recap")
                    skipType == "credits" -> VesselI18n.tr("player.skip_credits")
                    skipType == "preview" -> VesselI18n.tr("player.skip_preview")
                    else -> VesselI18n.tr("player.skip_intro")
                }
                val segKey = "${seg.type}:${seg.startMs}:${seg.endMs ?: "end"}"

                Surface(
                    shape = RoundedCornerShape(999.dp),
                    color = Color(0xE60E0F18),
                    border = BorderStroke(1.dp, Color.White.copy(alpha = 0.28f)),
                    shadowElevation = 10.dp,
                ) {
                    Row(verticalAlignment = Alignment.CenterVertically) {
                        Row(
                            verticalAlignment = Alignment.CenterVertically,
                            horizontalArrangement = Arrangement.spacedBy(6.dp),
                            modifier = Modifier
                                .clip(RoundedCornerShape(topStart = 999.dp, bottomStart = 999.dp))
                                .focusRequester(skipIntroFocusRequester)
                                .onFocusChanged { skipIntroHasFocus = it.isFocused || it.hasFocus }
                                .focusProperties {
                                    right = skipCloseFocusRequester
                                    if (controlsVisible) {
                                        down = if (!isLive && nextEpisode != null) bottomPlayPauseFocus else if (!isLive) scrubberFocus else playPauseFocusRequester
                                    }
                                }
                                .vesselFocusBorder(RoundedCornerShape(topStart = 999.dp, bottomStart = 999.dp), width = 2.dp)
                                .clickable {
                                    dismissedSkipKeys = dismissedSkipKeys + segKey
                                    if (jumpToNext) {
                                        playEpisode(nextEpisode!!)
                                    } else {
                                        val targetMs = if (seg.endMs != null) {
                                            if (durMs > 2000L) minOf(durMs - 500L, seg.endMs) else seg.endMs
                                        } else {
                                            (durMs - 500L).coerceAtLeast(0L)
                                        }
                                        if (targetMs > 0L) {
                                            controller.seekTo(targetMs)
                                            currentPosMs = targetMs
                                            showHud(skipLabel)
                                        }
                                    }
                                }
                                .padding(start = 13.dp, end = 10.dp, top = 7.dp, bottom = 7.dp),
                        ) {
                            Icon(Icons.Default.FastForward, contentDescription = null, tint = Color.White, modifier = Modifier.size(15.dp))
                            Text(
                                text = skipLabel,
                                color = Color.White,
                                style = MaterialTheme.typography.labelMedium,
                                fontWeight = FontWeight.Bold,
                            )
                        }
                        Box(
                            modifier = Modifier
                                .width(1.dp)
                                .height(18.dp)
                                .background(Color.White.copy(alpha = 0.16f)),
                        )
                        Box(
                            modifier = Modifier
                                .clip(RoundedCornerShape(topEnd = 999.dp, bottomEnd = 999.dp))
                                .focusRequester(skipCloseFocusRequester)
                                .onFocusChanged { skipCloseHasFocus = it.isFocused || it.hasFocus }
                                .focusProperties {
                                    left = skipIntroFocusRequester
                                    if (controlsVisible) {
                                        down = if (!isLive && nextEpisode != null) bottomPlayPauseFocus else if (!isLive) scrubberFocus else playPauseFocusRequester
                                    }
                                }
                                .vesselFocusBorder(RoundedCornerShape(topEnd = 999.dp, bottomEnd = 999.dp), width = 2.dp)
                                .clickable { dismissedSkipKeys = dismissedSkipKeys + segKey }
                                .padding(start = 8.dp, end = 11.dp, top = 7.dp, bottom = 7.dp),
                            contentAlignment = Alignment.Center,
                        ) {
                            Icon(Icons.Default.Close, contentDescription = VesselI18n.tr("common.close"), tint = Color.White.copy(alpha = 0.78f), modifier = Modifier.size(13.dp))
                        }
                    }
                }
            }
        }

        // Up Next Episode Overlay
        AnimatedVisibility(
            visible = !isLocked && showUpNext && nextEpisode != null && activeDrawer == null,
            enter = fadeIn(),
            exit = fadeOut(),
            modifier = Modifier
                .align(Alignment.BottomEnd)
                .systemBarsPadding()
                .padding(end = 20.dp, bottom = if (controlsVisible) 128.dp else 28.dp),
        ) {
            nextEpisode?.let { nextEp ->
                Surface(
                    shape = RoundedCornerShape(16.dp),
                    color = Color(0xEA0E0F18),
                    border = BorderStroke(1.dp, palette.accent.copy(alpha = 0.45f)),
                    shadowElevation = 12.dp,
                ) {
                    Row(
                        verticalAlignment = Alignment.CenterVertically,
                        horizontalArrangement = Arrangement.spacedBy(10.dp),
                        modifier = Modifier.padding(12.dp),
                    ) {
                        Column(modifier = Modifier.widthIn(max = 175.dp)) {
                            Text(
                                VesselI18n.tr("home.up_next", "Sıradaki"),
                                style = MaterialTheme.typography.labelSmall,
                                color = palette.accent,
                                fontWeight = FontWeight.Bold,
                            )
                            Text(
                                text = "S${nextEp.season}·E${nextEp.episode} ${nextEp.title ?: ""}".trim(),
                                fontWeight = FontWeight.SemiBold,
                                maxLines = 1,
                                overflow = TextOverflow.Ellipsis,
                                color = Color.White,
                            )
                        }
                        Button(
                            onClick = { playEpisode(nextEp) },
                            colors = ButtonDefaults.buttonColors(containerColor = palette.accent),
                            modifier = Modifier.vesselFocusBorder(RoundedCornerShape(999.dp)),
                        ) {
                            Icon(Icons.Default.SkipNext, contentDescription = null, tint = Color(0xFF0B0B12), modifier = Modifier.size(16.dp))
                            Spacer(Modifier.width(4.dp))
                            Text(VesselI18n.tr("common.play", "Oynat"), color = Color(0xFF0B0B12), fontWeight = FontWeight.Bold)
                        }
                        TextButton(
                            onClick = { dismissUpNext = true },
                            modifier = Modifier.vesselFocusBorder(RoundedCornerShape(999.dp)),
                        ) {
                            Text("×", color = Color.White.copy(alpha = 0.8f))
                        }
                    }
                }
            }
        }

        // Compact Slide-Over Drawer (Mini Channel List on Start for Live TV, Right Drawer for others)
        val isMiniLiveChannelList = isLive && activeDrawer == PlayerDrawer.EPISODES
        AnimatedVisibility(
            visible = activeDrawer != null && !isLocked,
            enter = slideInHorizontally(
                tween(240),
                initialOffsetX = { if (isMiniLiveChannelList) -it else it },
            ) + fadeIn(tween(200)),
            exit = slideOutHorizontally(
                tween(200),
                targetOffsetX = { if (isMiniLiveChannelList) -it else it },
            ) + fadeOut(tween(180)),
            modifier = Modifier.fillMaxSize(),
        ) {
            BoxWithConstraints(
                modifier = Modifier
                    .fillMaxSize()
                    .pointerInput(Unit) {
                        detectTapGestures { activeDrawer = null }
                    },
            ) {
                val drawerWidth = when {
                    isMiniLiveChannelList && maxWidth > 600.dp -> minOf(256.dp, maxWidth * 0.34f)
                    isMiniLiveChannelList -> minOf(244.dp, maxWidth * 0.72f)
                    maxWidth > 600.dp -> minOf(300.dp, maxWidth * 0.44f)
                    else -> minOf(296.dp, maxWidth * 0.84f)
                }
                Surface(
                    shape = RoundedCornerShape(14.dp),
                    color = if (isMiniLiveChannelList) Color(0xC90B0C14) else DrawerSurfaceColor,
                    border = BorderStroke(1.dp, Color.White.copy(alpha = 0.14f)),
                    shadowElevation = 20.dp,
                    modifier = Modifier
                        .align(if (isMiniLiveChannelList) Alignment.CenterStart else Alignment.CenterEnd)
                        .systemBarsPadding()
                        .padding(8.dp)
                        .width(drawerWidth)
                        .fillMaxHeight(if (isMiniLiveChannelList) 0.84f else 1f)
                        .focusGroup()
                        .pointerInput(Unit) {
                            detectTapGestures { }
                        },
                ) {
                    Column(modifier = Modifier.fillMaxSize()) {
                        // Compact Drawer Header
                        Row(
                            modifier = Modifier
                                .fillMaxWidth()
                                .padding(horizontal = 13.dp, vertical = 10.dp),
                            verticalAlignment = Alignment.CenterVertically,
                            horizontalArrangement = Arrangement.SpaceBetween,
                        ) {
                            val drawerTitle = when (activeDrawer) {
                                PlayerDrawer.EPISODES ->
                                    if (isLive) "${VesselI18n.tr("iptv.channels")} (${episodes.size})"
                                    else "${VesselI18n.tr("details.episodes")} (${episodes.size})"
                                PlayerDrawer.SOURCES ->
                                    "${VesselI18n.tr("details.sources")} (${activeStreamResult.streams.size})"
                                PlayerDrawer.AUDIO ->
                                    "${VesselI18n.tr("player.audio_tracks")}${if (audioTracks.isNotEmpty()) " (${audioTracks.size})" else ""}"
                                PlayerDrawer.SUBTITLES ->
                                    if (subtitleTranscriptOpen) VesselI18n.tr("player.sync_transcript", "Transkriptten Eşitle")
                                    else VesselI18n.tr("player.subtitles_sync")
                                PlayerDrawer.SETTINGS ->
                                    VesselI18n.tr("nav.settings")
                                null -> ""
                            }
                            Text(
                                text = drawerTitle,
                                color = Color.White,
                                style = MaterialTheme.typography.titleSmall,
                                fontWeight = FontWeight.Bold,
                            )
                            HudCircleIconButton(
                                onClick = {
                                    if (activeDrawer == PlayerDrawer.SUBTITLES && subtitleTranscriptOpen) {
                                        subtitleTranscriptOpen = false
                                        subtitleTranscriptQuery = ""
                                    } else {
                                        activeDrawer = null
                                    }
                                },
                                size = 28.dp,
                                modifier = Modifier.focusRequester(drawerCloseFocusRequester),
                            ) {
                                Icon(Icons.Default.Close, contentDescription = VesselI18n.tr("common.close"), tint = Color.White, modifier = Modifier.size(14.dp))
                            }
                        }
                        HorizontalDivider(color = Color.White.copy(alpha = 0.10f))

                        // Drawer Content
                        when (activeDrawer) {
                            PlayerDrawer.EPISODES -> {
                                val initialFocusIdx = if (currentEpIdx >= 0) currentEpIdx else 0
                                val epListState = rememberLazyListState(
                                    initialFirstVisibleItemIndex = initialFocusIdx,
                                )
                                LaunchedEffect(activeDrawer, currentEpIdx) {
                                    if (activeDrawer == PlayerDrawer.EPISODES) {
                                        epListState.scrollToItem(initialFocusIdx)
                                        kotlinx.coroutines.delay(50)
                                        runCatching { drawerFirstFocusRequester.requestFocus() }
                                    }
                                }
                                LazyColumn(
                                    state = epListState,
                                    modifier = Modifier.fillMaxSize().padding(if (isLive) 8.dp else 12.dp),
                                    verticalArrangement = Arrangement.spacedBy(if (isLive) 5.dp else 8.dp),
                                ) {
                                    itemsIndexed(episodes, key = { idx, ep -> "${idx}:${ep.streamId}" }) { idx, ep ->
                                        val isCur = ep.streamId == currentEpisode.streamId
                                        val itemMod = Modifier
                                            .then(if (idx == initialFocusIdx) Modifier.focusRequester(drawerFirstFocusRequester) else Modifier)
                                            .then(if (idx == 0) Modifier.focusProperties { up = drawerCloseFocusRequester } else Modifier)
                                        DrawerItemCard(
                                            active = isCur,
                                            accentColor = palette.accent,
                                            modifier = itemMod,
                                            onClick = {
                                                activeDrawer = null
                                                if (!isCur) playEpisode(ep)
                                            },
                                        ) {
                                            Row(
                                                modifier = Modifier.fillMaxWidth(),
                                                verticalAlignment = Alignment.CenterVertically,
                                                horizontalArrangement = Arrangement.spacedBy(8.dp),
                                            ) {
                                                val thumb = ep.thumbnail ?: if (!isLive) (activeDetail.backdrop ?: activeDetail.poster) else null
                                                if (!thumb.isNullOrBlank()) {
                                                    AsyncImage(
                                                        model = thumb,
                                                        contentDescription = null,
                                                        contentScale = if (isLive) ContentScale.Fit else ContentScale.Crop,
                                                        modifier = Modifier
                                                            .width(if (isLive) 30.dp else 84.dp)
                                                            .height(if (isLive) 30.dp else 48.dp)
                                                            .clip(RoundedCornerShape(7.dp))
                                                            .background(Color(0xBF0A0B12)),
                                                    )
                                                }
                                                Column(modifier = Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(1.dp)) {
                                                    Text(
                                                        text = if (isLive) "CH ${ep.episode}" else "S${ep.season} · E${ep.episode}",
                                                        style = MaterialTheme.typography.labelSmall,
                                                        fontWeight = FontWeight.Bold,
                                                        color = palette.accent,
                                                    )
                                                    Text(
                                                        text = ep.title?.takeIf { it.isNotBlank() } ?: if (isLive) "Channel ${ep.episode}" else "Episode ${ep.episode}",
                                                        color = Color.White,
                                                        style = if (isLive) MaterialTheme.typography.bodySmall else MaterialTheme.typography.bodyMedium,
                                                        fontWeight = FontWeight.SemiBold,
                                                        maxLines = 1,
                                                        overflow = TextOverflow.Ellipsis,
                                                    )
                                                    if (!isLive) {
                                                        ep.overview?.takeIf { it.isNotBlank() }?.let { ov ->
                                                            Text(
                                                                text = ov,
                                                                style = MaterialTheme.typography.labelSmall,
                                                                color = Color.White.copy(alpha = 0.65f),
                                                                maxLines = 2,
                                                                overflow = TextOverflow.Ellipsis,
                                                            )
                                                        }
                                                    }
                                                }
                                                if (isCur) {
                                                    Icon(Icons.Default.Check, contentDescription = null, tint = palette.accent, modifier = Modifier.size(16.dp))
                                                }
                                            }
                                        }
                                    }
                                }
                            }

                            PlayerDrawer.SOURCES -> {
                                val srcListState = rememberLazyListState(
                                    initialFirstVisibleItemIndex = selectedIndex.coerceIn(0, (activeStreamResult.streams.size - 1).coerceAtLeast(0)),
                                )
                                val initialFocusIdx = selectedIndex.coerceIn(0, (activeStreamResult.streams.size - 1).coerceAtLeast(0))
                                LazyColumn(
                                    state = srcListState,
                                    modifier = Modifier.fillMaxSize().padding(12.dp),
                                    verticalArrangement = Arrangement.spacedBy(8.dp),
                                ) {
                                    itemsIndexed(activeStreamResult.streams) { idx, cand ->
                                        val isCur = idx == selectedIndex
                                        val itemMod = Modifier
                                            .then(if (idx == initialFocusIdx) Modifier.focusRequester(drawerFirstFocusRequester) else Modifier)
                                            .then(if (idx == 0) Modifier.focusProperties { up = drawerCloseFocusRequester } else Modifier)
                                        DrawerItemCard(
                                            active = isCur,
                                            accentColor = palette.accent,
                                            modifier = itemMod,
                                            onClick = {
                                                val isTorrent = cand.kind.equals("torrent", ignoreCase = true) ||
                                                    cand.torrent != null ||
                                                    cand.url.startsWith("magnet:", ignoreCase = true)
                                                val pos = if (isLive) 0L else controller.state.value.positionMs.coerceAtLeast(0L)
                                                if (!isTorrent) {
                                                    currentStartPosMs = pos
                                                    selectedIndex = idx
                                                    if (isLive) {
                                                        repo.savePreferredLiveStream(pluginId, activeDetail.nativeId, currentEpisode.streamId, cand, idx)
                                                    }
                                                    activeDrawer = null
                                                    bumpControls()
                                                } else {
                                                    scope.launch {
                                                        runCatching {
                                                            repo.resolveTorrentCandidate(
                                                                ResolveDebridRequest(
                                                                    url = cand.url,
                                                                    torrent = cand.torrent,
                                                                    preferredTarget = cand.debridCache?.takeIf { it.state == "cached" }?.target,
                                                                    allowUncachedQueue = false,
                                                                    season = if (activeDetail.type == "movie") null else currentEpisode.season,
                                                                    episode = if (activeDetail.type == "movie") null else currentEpisode.episode,
                                                                ),
                                                            )
                                                        }.onSuccess { res ->
                                                            if (res.status == "ready" && res.url != null) {
                                                                val updated = activeStreamResult.streams.mapIndexed { i, s ->
                                                                    if (i == idx) {
                                                                        s.copy(
                                                                            url = res.url,
                                                                            kind = "auto",
                                                                            headers = res.headers,
                                                                            flags = if (res.headers.isNotEmpty()) (s.flags + "requires_proxy").distinct() else s.flags,
                                                                        )
                                                                    } else s
                                                                }
                                                                activeStreamResult = activeStreamResult.copy(streams = updated)
                                                                currentStartPosMs = pos
                                                                selectedIndex = idx
                                                                activeDrawer = null
                                                                bumpControls()
                                                            } else {
                                                                repo.notify(res.message ?: "${res.providerLabel ?: "Debrid"}: ${res.status}")
                                                            }
                                                        }.onFailure {
                                                            repo.notifyError(it, "Failed to switch torrent source")
                                                        }
                                                    }
                                                }
                                            },
                                        ) {
                                            Row(
                                                modifier = Modifier.fillMaxWidth(),
                                                verticalAlignment = Alignment.CenterVertically,
                                                horizontalArrangement = Arrangement.spacedBy(8.dp),
                                            ) {
                                                Column(modifier = Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(5.dp)) {
                                                    Text(cand.label, color = Color.White, fontWeight = FontWeight.SemiBold)
                                                    FlowRow(
                                                        horizontalArrangement = Arrangement.spacedBy(6.dp),
                                                        verticalArrangement = Arrangement.spacedBy(4.dp),
                                                    ) {
                                                        (cand.pluginName ?: cand.pluginId)?.takeIf { it.isNotBlank() }?.let { src ->
                                                            DrawerTagBadge(src, accent = true, accentColor = palette.accent)
                                                        }
                                                        DrawerTagBadge(cand.kind.uppercase(Locale.US), accent = false, accentColor = palette.accent)
                                                        cand.quality?.takeIf { it.isNotBlank() }?.let {
                                                            DrawerTagBadge(it, accent = true, accentColor = palette.accent)
                                                        }
                                                        cand.debridCache?.let { c ->
                                                            val tag = when (c.state) {
                                                                "cached" -> "⚡ ${c.label}"
                                                                "uncached" -> "☁️ ${c.label}"
                                                                else -> c.label
                                                            }
                                                            DrawerTagBadge(tag, accent = c.state == "cached", accentColor = palette.accent)
                                                        }
                                                        cand.torrent?.sizeBytes?.takeIf { it > 0L }?.let { sz ->
                                                            DrawerTagBadge(formatSizeBytes(sz), accent = false, accentColor = palette.accent)
                                                        }
                                                        cand.torrent?.seeders?.let { s ->
                                                            DrawerTagBadge("👤 $s", accent = false, accentColor = palette.accent)
                                                        }
                                                        cand.language?.takeIf { it.isNotBlank() }?.let {
                                                            DrawerTagBadge(it.uppercase(Locale.US), accent = false, accentColor = palette.accent)
                                                        }
                                                    }
                                                }
                                                if (isCur) {
                                                    Icon(Icons.Default.Check, contentDescription = null, tint = palette.accent, modifier = Modifier.size(17.dp))
                                                }
                                            }
                                        }
                                    }
                                }
                            }

                            PlayerDrawer.AUDIO -> {
                                if (!isLive) Column(
                                    modifier = Modifier.fillMaxSize().verticalScroll(rememberScrollState()).padding(12.dp),
                                    verticalArrangement = Arrangement.spacedBy(8.dp),
                                ) {
                                    val firstAudioFocusIdx = audioTracks.indexOfFirst { it.selected }.let { if (it >= 0) it else 0 }
                                    audioTracks.forEachIndexed { idx, trk ->
                                        val itemMod = Modifier
                                            .then(if (idx == firstAudioFocusIdx) Modifier.focusRequester(drawerFirstFocusRequester) else Modifier)
                                            .then(if (idx == 0) Modifier.focusProperties { up = drawerCloseFocusRequester } else Modifier)
                                        DrawerItemCard(
                                            active = trk.selected,
                                            accentColor = palette.accent,
                                            modifier = itemMod,
                                            onClick = {
                                                controller.selectAudioTrack(trk.id)
                                                activeDrawer = null
                                                showHud("${VesselI18n.tr("player.audio", "Ses")}: ${trk.label}")
                                            },
                                        ) {
                                            Row(
                                                modifier = Modifier.fillMaxWidth(),
                                                verticalAlignment = Alignment.CenterVertically,
                                            ) {
                                                Column(modifier = Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(4.dp)) {
                                                    Text(trk.label, color = Color.White, fontWeight = FontWeight.SemiBold)
                                                    if (trk.meta.isNotBlank()) {
                                                        DrawerTagBadge(trk.meta, accent = false, accentColor = palette.accent)
                                                    }
                                                }
                                                if (trk.selected) {
                                                    Icon(Icons.Default.Check, contentDescription = null, tint = palette.accent, modifier = Modifier.size(17.dp))
                                                }
                                            }
                                        }
                                    }
                                    if (audioTracks.isEmpty()) {
                                        Text(
                                            text = VesselI18n.tr("player.audio_default_only"),
                                            style = MaterialTheme.typography.bodySmall,
                                            color = Color.White.copy(alpha = 0.65f),
                                            modifier = Modifier.padding(6.dp),
                                        )
                                    } else if (audioTracks.size == 1) {
                                        Text(
                                            text = VesselI18n.tr("player.audio_single_note"),
                                            style = MaterialTheme.typography.bodySmall,
                                            color = Color.White.copy(alpha = 0.65f),
                                            modifier = Modifier.padding(6.dp),
                                        )
                                    }
                                }
                            }

                            PlayerDrawer.SUBTITLES -> {
                                if (subtitleTranscriptOpen && externalCues.isNotEmpty()) {
                                    val filteredCues = remember(externalCues, subtitleTranscriptQuery) {
                                        val q = subtitleTranscriptQuery.trim().lowercase(Locale.US)
                                        if (q.isEmpty()) externalCues
                                        else externalCues.filter { cue ->
                                            cue.lines.any { it.lowercase(Locale.US).contains(q) }
                                        }
                                    }
                                    val targetTimelineMs = currentPosMs - subtitleDelayMs
                                    val nearestIndex = remember(filteredCues, targetTimelineMs) {
                                        if (filteredCues.isEmpty()) -1
                                        else {
                                            val activeIdx = filteredCues.indexOfFirst {
                                                targetTimelineMs >= it.startMs && targetTimelineMs <= it.endMs
                                            }
                                            if (activeIdx >= 0) activeIdx
                                            else {
                                                val upcoming = filteredCues.indexOfFirst { it.startMs >= targetTimelineMs }
                                                if (upcoming >= 0) upcoming else filteredCues.lastIndex
                                            }
                                        }
                                    }
                                    val transcriptListState = rememberLazyListState()
                                    LaunchedEffect(subtitleTranscriptOpen) {
                                        if (subtitleTranscriptOpen && nearestIndex >= 0) {
                                            transcriptListState.scrollToItem((nearestIndex - 1).coerceAtLeast(0))
                                        }
                                    }
                                    Column(
                                        modifier = Modifier.fillMaxSize().padding(10.dp),
                                        verticalArrangement = Arrangement.spacedBy(8.dp),
                                    ) {
                                        Text(
                                            text = VesselI18n.tr("player.sync_transcript_hint"),
                                            style = MaterialTheme.typography.labelSmall,
                                            color = Color.White.copy(alpha = 0.72f),
                                        )
                                        Row(
                                            modifier = Modifier.fillMaxWidth(),
                                            horizontalArrangement = Arrangement.SpaceBetween,
                                            verticalAlignment = Alignment.CenterVertically,
                                        ) {
                                            Text(
                                                text = "${formatClockMs(currentPosMs)} · ${subtitleDelayMs} ms",
                                                style = MaterialTheme.typography.labelSmall,
                                                fontWeight = FontWeight.Bold,
                                                color = palette.accent,
                                            )
                                            Row(horizontalArrangement = Arrangement.spacedBy(6.dp)) {
                                                if (subtitleDelayMs != 0L) {
                                                    PlayerHudPill(
                                                        icon = null,
                                                        label = VesselI18n.tr("player.sync_reset"),
                                                        active = false,
                                                        onClick = { subtitleDelayMs = 0L },
                                                    )
                                                }
                                                PlayerHudPill(
                                                    icon = null,
                                                    label = VesselI18n.tr("common.back"),
                                                    active = false,
                                                    modifier = Modifier.focusRequester(drawerFirstFocusRequester),
                                                    onClick = {
                                                        subtitleTranscriptOpen = false
                                                        subtitleTranscriptQuery = ""
                                                    },
                                                )
                                            }
                                        }
                                        VesselOutlinedTextField(
                                            value = subtitleTranscriptQuery,
                                            onValueChange = { subtitleTranscriptQuery = it },
                                            placeholder = {
                                                Text(
                                                    VesselI18n.tr("player.sync_transcript_search"),
                                                    maxLines = 1,
                                                    overflow = TextOverflow.Ellipsis,
                                                )
                                            },
                                            singleLine = true,
                                            modifier = Modifier
                                                .fillMaxWidth()
                                                .vesselDpadInputEscape(isTextEmpty = subtitleTranscriptQuery.isEmpty()),
                                        )
                                        LazyColumn(
                                            state = transcriptListState,
                                            modifier = Modifier.weight(1f).fillMaxWidth(),
                                            verticalArrangement = Arrangement.spacedBy(6.dp),
                                        ) {
                                            itemsIndexed(filteredCues) { idx, cue ->
                                                val isNearest = idx == nearestIndex
                                                DrawerItemCard(
                                                    active = isNearest,
                                                    accentColor = palette.accent,
                                                    onClick = {
                                                        val newDelay = currentPosMs - cue.startMs
                                                        subtitleDelayMs = newDelay
                                                        subtitleTranscriptOpen = false
                                                        subtitleTranscriptQuery = ""
                                                        showHud(VesselI18n.trf("player.sync_transcript_applied", "delay" to newDelay))
                                                    },
                                                ) {
                                                    Row(
                                                        modifier = Modifier.fillMaxWidth(),
                                                        horizontalArrangement = Arrangement.spacedBy(8.dp),
                                                        verticalAlignment = Alignment.Top,
                                                    ) {
                                                        DrawerTagBadge(
                                                            text = formatClockMs(cue.startMs),
                                                            accent = isNearest,
                                                            accentColor = palette.accent,
                                                        )
                                                        Text(
                                                            text = cue.lines.joinToString(" "),
                                                            color = Color.White,
                                                            style = MaterialTheme.typography.bodySmall,
                                                            maxLines = 3,
                                                            overflow = TextOverflow.Ellipsis,
                                                            modifier = Modifier.weight(1f),
                                                        )
                                                    }
                                                }
                                            }
                                        }
                                    }
                                } else Column(
                                    modifier = Modifier.fillMaxSize().verticalScroll(rememberScrollState()).padding(9.dp),
                                    verticalArrangement = Arrangement.spacedBy(7.dp),
                                ) {
                                    // Subtitle Delay (-500ms / +500ms) + Nuvio-style Transcript Sync
                                    DrawerSettingBlock(title = VesselI18n.tr("player.subtitles_delay")) {
                                        Row(
                                            modifier = Modifier.fillMaxWidth(),
                                            horizontalArrangement = Arrangement.SpaceBetween,
                                            verticalAlignment = Alignment.CenterVertically,
                                        ) {
                                            PlayerHudPill(
                                                icon = null,
                                                label = "-500ms",
                                                active = false,
                                                modifier = Modifier
                                                    .focusRequester(drawerFirstFocusRequester)
                                                    .focusProperties { up = drawerCloseFocusRequester },
                                                onClick = { subtitleDelayMs -= 500L },
                                            )
                                            Text(
                                                text = "$subtitleDelayMs ms",
                                                color = Color.White,
                                                fontWeight = FontWeight.Bold,
                                                style = MaterialTheme.typography.labelMedium,
                                            )
                                            PlayerHudPill(
                                                icon = null,
                                                label = "+500ms",
                                                active = false,
                                                onClick = { subtitleDelayMs += 500L },
                                            )
                                        }
                                        if (externalCues.isNotEmpty()) {
                                            Spacer(Modifier.height(4.dp))
                                            Row(
                                                modifier = Modifier.fillMaxWidth(),
                                                horizontalArrangement = Arrangement.spacedBy(6.dp),
                                                verticalAlignment = Alignment.CenterVertically,
                                            ) {
                                                PlayerHudPill(
                                                    icon = {
                                                        Icon(
                                                            Icons.AutoMirrored.Filled.List,
                                                            contentDescription = null,
                                                            tint = Color.White,
                                                            modifier = Modifier.size(13.dp),
                                                        )
                                                    },
                                                    label = VesselI18n.tr("player.sync_transcript"),
                                                    active = false,
                                                    onClick = { subtitleTranscriptOpen = true },
                                                )
                                                if (subtitleDelayMs != 0L) {
                                                    PlayerHudPill(
                                                        icon = null,
                                                        label = VesselI18n.tr("player.sync_reset"),
                                                        active = false,
                                                        onClick = { subtitleDelayMs = 0L },
                                                    )
                                                }
                                            }
                                        }
                                    }

                                    // Font Size Slider
                                    DrawerSettingBlock(title = "${VesselI18n.tr("subtitles.font_size")} (${prefs.subtitles.fontSizePx}px)") {
                                        Slider(
                                            value = prefs.subtitles.fontSizePx.toFloat().coerceIn(16f, 52f),
                                            onValueChange = { v ->
                                                val rounded = (v / 2f).roundToInt() * 2
                                                saveSubPrefs { it.copy(subtitles = it.subtitles.copy(fontSizePx = rounded.coerceIn(16, 52))) }
                                            },
                                            valueRange = 16f..52f,
                                            modifier = Modifier
                                                .height(24.dp)
                                                .tvStepSliderKeys(
                                                    current = prefs.subtitles.fontSizePx,
                                                    step = 2,
                                                    range = 16..52,
                                                    onStep = { next ->
                                                        saveSubPrefs { it.copy(subtitles = it.subtitles.copy(fontSizePx = next)) }
                                                    },
                                                ),
                                            colors = SliderDefaults.colors(
                                                thumbColor = palette.accent,
                                                activeTrackColor = palette.accent,
                                                inactiveTrackColor = Color.White.copy(alpha = 0.25f),
                                            ),
                                        )
                                    }

                                    // Bottom Offset Slider
                                    DrawerSettingBlock(title = "${VesselI18n.tr("subtitles.bottom_offset")} (${prefs.subtitles.bottomOffsetPx}px)") {
                                        Slider(
                                            value = prefs.subtitles.bottomOffsetPx.toFloat().coerceIn(16f, 220f),
                                            onValueChange = { v ->
                                                val rounded = (v / 4f).roundToInt() * 4
                                                saveSubPrefs { it.copy(subtitles = it.subtitles.copy(bottomOffsetPx = rounded.coerceIn(16, 220))) }
                                            },
                                            valueRange = 16f..220f,
                                            modifier = Modifier
                                                .height(24.dp)
                                                .tvStepSliderKeys(
                                                    current = prefs.subtitles.bottomOffsetPx,
                                                    step = 4,
                                                    range = 16..220,
                                                    onStep = { next ->
                                                        saveSubPrefs { it.copy(subtitles = it.subtitles.copy(bottomOffsetPx = next)) }
                                                    },
                                                ),
                                            colors = SliderDefaults.colors(
                                                thumbColor = palette.accent,
                                                activeTrackColor = palette.accent,
                                                inactiveTrackColor = Color.White.copy(alpha = 0.25f),
                                            ),
                                        )
                                    }

                                    // Background Opacity Slider
                                    DrawerSettingBlock(title = "${VesselI18n.tr("subtitles.bg_opacity")} (${prefs.subtitles.backgroundOpacity}%)") {
                                        Slider(
                                            value = prefs.subtitles.backgroundOpacity.toFloat().coerceIn(0f, 100f),
                                            onValueChange = { v ->
                                                val rounded = (v / 5f).roundToInt() * 5
                                                saveSubPrefs { it.copy(subtitles = it.subtitles.copy(backgroundOpacity = rounded.coerceIn(0, 100))) }
                                            },
                                            valueRange = 0f..100f,
                                            modifier = Modifier
                                                .height(24.dp)
                                                .tvStepSliderKeys(
                                                    current = prefs.subtitles.backgroundOpacity,
                                                    step = 5,
                                                    range = 0..100,
                                                    onStep = { next ->
                                                        saveSubPrefs { it.copy(subtitles = it.subtitles.copy(backgroundOpacity = next)) }
                                                    },
                                                ),
                                            colors = SliderDefaults.colors(
                                                thumbColor = palette.accent,
                                                activeTrackColor = palette.accent,
                                                inactiveTrackColor = Color.White.copy(alpha = 0.25f),
                                            ),
                                        )
                                    }

                                    // Font Family Chips
                                    DrawerSettingBlock(title = VesselI18n.tr("subtitles.font_family")) {
                                        FlowRow(
                                            horizontalArrangement = Arrangement.spacedBy(5.dp),
                                            verticalArrangement = Arrangement.spacedBy(5.dp),
                                        ) {
                                            listOf(
                                                "system-ui" to VesselI18n.tr("subtitles.font_sans"),
                                                "rounded" to VesselI18n.tr("subtitles.font_rounded"),
                                                "serif" to VesselI18n.tr("subtitles.font_serif"),
                                                "monospace" to VesselI18n.tr("subtitles.font_mono"),
                                            ).forEach { (id, label) ->
                                                DrawerChipButton(
                                                    label = label,
                                                    active = prefs.subtitles.fontFamily == id,
                                                    accentColor = palette.accent,
                                                    onClick = { saveSubPrefs { it.copy(subtitles = it.subtitles.copy(fontFamily = id)) } },
                                                )
                                            }
                                        }
                                    }

                                    // Outline Style Chips
                                    DrawerSettingBlock(title = VesselI18n.tr("subtitles.outline_style")) {
                                        FlowRow(
                                            horizontalArrangement = Arrangement.spacedBy(5.dp),
                                            verticalArrangement = Arrangement.spacedBy(5.dp),
                                        ) {
                                            listOf(
                                                "outline" to VesselI18n.tr("subtitles.style_outline"),
                                                "shadow" to VesselI18n.tr("subtitles.style_shadow"),
                                                "box" to VesselI18n.tr("subtitles.style_box"),
                                                "none" to VesselI18n.tr("subtitles.style_none"),
                                            ).forEach { (id, label) ->
                                                DrawerChipButton(
                                                    label = label,
                                                    active = prefs.subtitles.outlineStyle == id,
                                                    accentColor = palette.accent,
                                                    onClick = { saveSubPrefs { it.copy(subtitles = it.subtitles.copy(outlineStyle = id)) } },
                                                )
                                            }
                                        }
                                    }

                                    // Text Color Dots + Bold Toggle
                                    DrawerSettingBlock(title = VesselI18n.tr("subtitles.text_color")) {
                                        Row(
                                            horizontalArrangement = Arrangement.spacedBy(8.dp),
                                            verticalAlignment = Alignment.CenterVertically,
                                        ) {
                                            listOf("#ffffff", "#fde047", "#67e8f9", "#f9a8d4", "#a7f3d0").forEach { hex ->
                                                val sel = prefs.subtitles.textColor.equals(hex, ignoreCase = true)
                                                Surface(
                                                    shape = CircleShape,
                                                    color = parseHexColor(hex, Color.White),
                                                    border = BorderStroke(
                                                        width = if (sel) 2.dp else 1.dp,
                                                        color = if (sel) palette.accent else Color.White.copy(alpha = 0.35f),
                                                    ),
                                                    modifier = Modifier
                                                        .size(20.dp)
                                                        .clip(CircleShape)
                                                        .vesselFocusBorder(CircleShape)
                                                        .clickable {
                                                            saveSubPrefs { it.copy(subtitles = it.subtitles.copy(textColor = hex)) }
                                                        },
                                                ) {}
                                            }
                                            DrawerChipButton(
                                                label = "B",
                                                active = prefs.subtitles.bold,
                                                accentColor = palette.accent,
                                                onClick = {
                                                    saveSubPrefs { it.copy(subtitles = it.subtitles.copy(bold = !it.subtitles.bold)) }
                                                },
                                            )
                                        }
                                    }

                                    // Off Option
                                    val allOff = activeExternalSubIndex == -1 && embeddedSubsDisabled
                                    DrawerItemCard(
                                        active = allOff,
                                        accentColor = palette.accent,
                                        onClick = {
                                            activeExternalSubIndex = -1
                                            controller.selectSubtitleTrack(null)
                                            embeddedSubsDisabled = true
                                        },
                                    ) {
                                        Row(modifier = Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
                                            Text(VesselI18n.tr("player.subtitles_off"), color = Color.White, style = MaterialTheme.typography.bodySmall, fontWeight = FontWeight.SemiBold, modifier = Modifier.weight(1f))
                                            if (allOff) {
                                                Icon(Icons.Default.Check, contentDescription = null, tint = palette.accent, modifier = Modifier.size(15.dp))
                                            }
                                        }
                                    }

                                    // External Subtitle Tracks (with Sync Delay + Full Compose Rendering)
                                    availableExternalSubs.forEachIndexed { idx, sub ->
                                        val isSel = activeExternalSubIndex == idx
                                        DrawerItemCard(
                                            active = isSel,
                                            accentColor = palette.accent,
                                            onClick = { activeExternalSubIndex = idx },
                                        ) {
                                            Row(modifier = Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
                                                Column(modifier = Modifier.weight(1f)) {
                                                    Text(sub.label, color = Color.White, fontWeight = FontWeight.SemiBold)
                                                    Text(
                                                        "${sub.lang.uppercase(Locale.US)} · ${sub.format.uppercase(Locale.US)}",
                                                        style = MaterialTheme.typography.labelSmall,
                                                        color = Color.White.copy(alpha = 0.65f),
                                                    )
                                                }
                                                if (isSel) {
                                                    Icon(Icons.Default.Check, contentDescription = null, tint = palette.accent, modifier = Modifier.size(17.dp))
                                                }
                                            }
                                        }
                                    }

                                    // Embedded Muxed Subtitles (HLS/MKV)
                                    embeddedSubTracks.forEach { trk ->
                                        val isSel = activeExternalSubIndex == -1 && !embeddedSubsDisabled && trk.selected
                                        DrawerItemCard(
                                            active = isSel,
                                            accentColor = palette.accent,
                                            onClick = {
                                                activeExternalSubIndex = -1
                                                controller.selectSubtitleTrack(trk.id)
                                                embeddedSubsDisabled = false
                                                activeDrawer = null
                                            },
                                        ) {
                                            Row(modifier = Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
                                                Column(modifier = Modifier.weight(1f)) {
                                                    Text(trk.label, color = Color.White, fontWeight = FontWeight.SemiBold)
                                                    if (trk.meta.isNotBlank()) {
                                                        Text(trk.meta, style = MaterialTheme.typography.labelSmall, color = Color.White.copy(alpha = 0.65f))
                                                    }
                                                }
                                                if (isSel) {
                                                    Icon(Icons.Default.Check, contentDescription = null, tint = palette.accent, modifier = Modifier.size(17.dp))
                                                }
                                            }
                                        }
                                    }

                                    if (loadingSubtitle) {
                                        Text(
                                            text = VesselI18n.tr("common.loading", "Altyazı yükleniyor…"),
                                            style = MaterialTheme.typography.labelSmall,
                                            color = Color.White.copy(alpha = 0.7f),
                                            modifier = Modifier.padding(4.dp),
                                        )
                                    } else if (subtitleError != null) {
                                        Text(
                                            text = subtitleError.orEmpty(),
                                            style = MaterialTheme.typography.labelSmall,
                                            color = Color(0xFFFDA4AF),
                                            modifier = Modifier.padding(4.dp),
                                        )
                                    } else if (activeExternalSubIndex >= 0 && externalCues.isNotEmpty()) {
                                        Text(
                                            text = VesselI18n.trf("player.cues_loaded", "count" to externalCues.size),
                                            style = MaterialTheme.typography.labelSmall,
                                            color = Color.White.copy(alpha = 0.65f),
                                            modifier = Modifier.padding(4.dp),
                                        )
                                    } else if (availableExternalSubs.isEmpty() && embeddedSubTracks.isEmpty()) {
                                        Text(
                                            text = VesselI18n.tr("player.no_subtitles"),
                                            style = MaterialTheme.typography.bodySmall,
                                            color = Color.White.copy(alpha = 0.65f),
                                            modifier = Modifier.padding(4.dp),
                                        )
                                    }

                                    HorizontalDivider(color = Color.White.copy(alpha = 0.12f), modifier = Modifier.padding(vertical = 4.dp))

                                    DrawerItemCard(
                                        active = extSubOpen,
                                        accentColor = palette.accent,
                                        onClick = {
                                            extSubOpen = !extSubOpen
                                            if (extSubOpen) {
                                                if (extSubQueryText.isBlank()) {
                                                    extSubQueryText = activeDetail.originalTitle?.takeIf { it.isNotBlank() } ?: activeDetail.title
                                                }
                                                if (!extSubSearchedOnce && !extSubSearching) {
                                                    runExternalSubSearch()
                                                }
                                            }
                                        },
                                    ) {
                                        Row(modifier = Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
                                            Icon(Icons.Default.Search, contentDescription = null, tint = Color.White, modifier = Modifier.size(16.dp))
                                            Spacer(Modifier.width(8.dp))
                                            Text(
                                                text = VesselI18n.tr("player.ext_sub_search"),
                                                color = Color.White,
                                                fontWeight = FontWeight.SemiBold,
                                                modifier = Modifier.weight(1f),
                                            )
                                            if (extSubSearching) {
                                                CircularProgressIndicator(modifier = Modifier.size(14.dp), strokeWidth = 2.dp, color = palette.accent)
                                            }
                                        }
                                    }

                                    if (extSubOpen) {
                                        Column(verticalArrangement = Arrangement.spacedBy(6.dp)) {
                                            Row(
                                                modifier = Modifier.fillMaxWidth(),
                                                horizontalArrangement = Arrangement.spacedBy(6.dp),
                                                verticalAlignment = Alignment.CenterVertically,
                                            ) {
                                                VesselOutlinedTextField(
                                                    value = extSubQueryText,
                                                    onValueChange = { extSubQueryText = it },
                                                    placeholder = { Text(VesselI18n.tr("player.ext_sub_placeholder")) },
                                                    singleLine = true,
                                                    onSubmit = { runExternalSubSearch() },
                                                    modifier = Modifier
                                                        .weight(1f)
                                                        .vesselDpadInputEscape(
                                                            isTextEmpty = extSubQueryText.isEmpty(),
                                                            onSubmit = { runExternalSubSearch() },
                                                        ),
                                                )
                                                DrawerChipButton(
                                                    label = VesselI18n.tr("common.search"),
                                                    active = false,
                                                    accentColor = palette.accent,
                                                    onClick = { runExternalSubSearch() },
                                                )
                                            }

                                            FlowRow(
                                                horizontalArrangement = Arrangement.spacedBy(5.dp),
                                                verticalArrangement = Arrangement.spacedBy(5.dp),
                                            ) {
                                                val prefLabel = VesselI18n.trf(
                                                    "player.ext_sub_lang_pref",
                                                    "langs" to defaultPreferredSubLangs.joinToString(", ") { it.uppercase(Locale.US) },
                                                )
                                                listOf(
                                                    "preferred" to prefLabel,
                                                    "tr" to "TR",
                                                    "en" to "EN",
                                                    "all" to VesselI18n.tr("player.ext_sub_lang_all"),
                                                ).forEach { (id, label) ->
                                                    DrawerChipButton(
                                                        label = label,
                                                        active = extSubLangMode == id,
                                                        accentColor = palette.accent,
                                                        onClick = {
                                                            extSubLangMode = id
                                                            runExternalSubSearch()
                                                        },
                                                    )
                                                }
                                            }

                                            val extSources = extSubHits.map { it.candidate.sourceName ?: it.pluginName }.distinct()
                                            if (extSources.size > 1) {
                                                FlowRow(
                                                    horizontalArrangement = Arrangement.spacedBy(5.dp),
                                                    verticalArrangement = Arrangement.spacedBy(5.dp),
                                                ) {
                                                    DrawerChipButton(
                                                        label = "All (${extSubHits.size})",
                                                        active = extSubSourceFilter == "all",
                                                        accentColor = palette.accent,
                                                        onClick = { extSubSourceFilter = "all" },
                                                    )
                                                    extSources.forEach { src ->
                                                        DrawerChipButton(
                                                            label = src,
                                                            active = extSubSourceFilter == src,
                                                            accentColor = palette.accent,
                                                            onClick = { extSubSourceFilter = src },
                                                        )
                                                    }
                                                }
                                            }

                                            if (extSubErrors.isNotEmpty()) {
                                                Text(
                                                    text = extSubErrors.joinToString(" · ") { "${it.pluginName}: ${it.error}" },
                                                    style = MaterialTheme.typography.labelSmall,
                                                    color = Color(0xFFFDA4AF),
                                                    modifier = Modifier.padding(4.dp),
                                                )
                                            }

                                            val shownHits = if (extSubSourceFilter == "all") {
                                                extSubHits
                                            } else {
                                                extSubHits.filter { (it.candidate.sourceName ?: it.pluginName) == extSubSourceFilter }
                                            }

                                            if (extSubSearching) {
                                                Text(
                                                    text = VesselI18n.tr("player.ext_sub_searching"),
                                                    style = MaterialTheme.typography.labelSmall,
                                                    color = Color.White.copy(alpha = 0.7f),
                                                    modifier = Modifier.padding(4.dp),
                                                )
                                            } else if (extSubSearchedOnce && shownHits.isEmpty()) {
                                                Text(
                                                    text = VesselI18n.tr("player.ext_sub_empty"),
                                                    style = MaterialTheme.typography.bodySmall,
                                                    color = Color.White.copy(alpha = 0.65f),
                                                    modifier = Modifier.padding(4.dp),
                                                )
                                            }

                                            shownHits.forEach { hit ->
                                                val hitKey = "${hit.pluginId}:${hit.candidate.id}"
                                                val isResolving = extSubResolvingKey == hitKey
                                                DrawerItemCard(
                                                    active = false,
                                                    accentColor = palette.accent,
                                                    onClick = {
                                                        if (extSubResolvingKey != null) return@DrawerItemCard
                                                        extSubResolvingKey = hitKey
                                                        scope.launch {
                                                            val srcTag = hit.candidate.sourceName ?: hit.pluginName
                                                            val finalLabel = if (srcTag in hit.candidate.label) {
                                                                hit.candidate.label
                                                            } else {
                                                                "[$srcTag] ${hit.candidate.label}"
                                                            }
                                                            runCatching {
                                                                repo.resolveSubtitle(
                                                                    hit.pluginId,
                                                                    hit.candidate.id,
                                                                    hit.candidate.lang,
                                                                    finalLabel,
                                                                )
                                                            }.onSuccess { track ->
                                                                val updated = extraFetchedSubs.toMutableList()
                                                                val idxExisting = updated.indexOfFirst { it.id == track.id }
                                                                if (idxExisting >= 0) updated[idxExisting] = track else updated.add(track)
                                                                extraFetchedSubs = updated
                                                                val fullList = (activeCandidate?.subtitles ?: emptyList()) + activeStreamResult.subtitles + updated
                                                                val targetIdx = fullList.indexOfFirst { it.id == track.id }
                                                                if (targetIdx >= 0) activeExternalSubIndex = targetIdx
                                                                showHud(VesselI18n.tr("player.ext_sub_loaded"))
                                                            }.onFailure { err ->
                                                                repo.notifyError(err, "Failed to load subtitle")
                                                            }
                                                            extSubResolvingKey = null
                                                        }
                                                    },
                                                ) {
                                                    Row(modifier = Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
                                                        Column(modifier = Modifier.weight(1f)) {
                                                            Text(
                                                                text = hit.candidate.label,
                                                                color = Color.White,
                                                                fontWeight = FontWeight.SemiBold,
                                                                maxLines = 2,
                                                                overflow = TextOverflow.Ellipsis,
                                                            )
                                                            val meta = buildList {
                                                                add(hit.candidate.sourceName ?: hit.pluginName)
                                                                add(hit.candidate.lang.uppercase(Locale.US))
                                                                add(hit.candidate.format.uppercase(Locale.US))
                                                                if (hit.candidate.hearingImpaired == true) add("HI")
                                                                hit.candidate.downloads?.let { add("$it dl") }
                                                            }.joinToString(" · ")
                                                            Text(
                                                                text = meta,
                                                                style = MaterialTheme.typography.labelSmall,
                                                                color = Color.White.copy(alpha = 0.65f),
                                                            )
                                                        }
                                                        if (isResolving) {
                                                            CircularProgressIndicator(modifier = Modifier.size(15.dp), strokeWidth = 2.dp, color = palette.accent)
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }

                            PlayerDrawer.SETTINGS -> {
                                Column(
                                    modifier = Modifier.fillMaxSize().verticalScroll(rememberScrollState()).padding(9.dp),
                                    verticalArrangement = Arrangement.spacedBy(7.dp),
                                ) {
                                    if (!isLive) {
                                        DrawerSettingBlock(title = VesselI18n.tr("player.playback_speed")) {
                                            FlowRow(
                                                horizontalArrangement = Arrangement.spacedBy(5.dp),
                                                verticalArrangement = Arrangement.spacedBy(5.dp),
                                            ) {
                                                listOf(0.5f, 0.75f, 1.0f, 1.25f, 1.5f, 2.0f).forEachIndexed { idx, spd ->
                                                    val isSel = playbackSpeed == spd
                                                    val chipMod = Modifier
                                                        .then(if (isSel || (playbackSpeed !in listOf(0.5f, 0.75f, 1.0f, 1.25f, 1.5f, 2.0f) && idx == 0)) Modifier.focusRequester(drawerFirstFocusRequester) else Modifier)
                                                        .focusProperties { up = drawerCloseFocusRequester }
                                                    DrawerChipButton(
                                                        label = "${spd}x",
                                                        active = isSel,
                                                        accentColor = palette.accent,
                                                        modifier = chipMod,
                                                        onClick = {
                                                            playbackSpeed = spd
                                                            controller.setSpeed(spd)
                                                            bumpControls()
                                                        },
                                                    )
                                                }
                                            }
                                        }
                                    }

                                    DrawerSettingBlock(title = VesselI18n.tr("player.aspect_ratio")) {
                                        FlowRow(
                                            horizontalArrangement = Arrangement.spacedBy(5.dp),
                                            verticalArrangement = Arrangement.spacedBy(5.dp),
                                        ) {
                                            listOf(
                                                AspectRatioFrameLayout.RESIZE_MODE_FIT to VesselI18n.tr("player.fit_contain"),
                                                AspectRatioFrameLayout.RESIZE_MODE_ZOOM to VesselI18n.tr("player.fit_fill"),
                                                AspectRatioFrameLayout.RESIZE_MODE_FILL to VesselI18n.tr("player.fit_stretch"),
                                            ).forEachIndexed { idx, (mode, label) ->
                                                val isSel = resizeMode == mode
                                                val chipMod = if (isLive && isSel) {
                                                    Modifier
                                                        .focusRequester(drawerFirstFocusRequester)
                                                        .focusProperties { up = drawerCloseFocusRequester }
                                                } else if (isLive && idx == 0) {
                                                    Modifier.focusProperties { up = drawerCloseFocusRequester }
                                                } else Modifier
                                                DrawerChipButton(
                                                    label = label,
                                                    active = isSel,
                                                    accentColor = palette.accent,
                                                    modifier = chipMod,
                                                    onClick = {
                                                        resizeMode = mode
                                                        controller.setResizeMode(mode)
                                                        showHud(label)
                                                        bumpControls()
                                                    },
                                                )
                                            }
                                        }
                                    }
                                }
                            }

                            null -> Unit
                        }
                    }
                }
            }
        }
    }
}

