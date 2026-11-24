package dev.falsis.vessel.ui.screens

import android.app.Activity
import android.content.pm.ActivityInfo
import android.graphics.Typeface
import android.net.Uri
import android.view.ViewGroup
import android.widget.FrameLayout
import androidx.annotation.OptIn
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.filled.AspectRatio
import androidx.compose.material.icons.filled.ClosedCaption
import androidx.compose.material.icons.filled.FastForward
import androidx.compose.material.icons.filled.Info
import androidx.compose.material.icons.filled.Lock
import androidx.compose.material.icons.filled.LockOpen
import androidx.compose.material.icons.filled.ScreenRotation
import androidx.compose.material.icons.filled.SkipNext
import androidx.compose.material.icons.filled.Speed
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.FilterChip
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.ModalBottomSheet
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
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.toArgb
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.viewinterop.AndroidView
import androidx.media3.common.C
import androidx.media3.common.MediaItem
import androidx.media3.common.MimeTypes
import androidx.media3.common.util.UnstableApi
import androidx.media3.datasource.DefaultHttpDataSource
import androidx.media3.exoplayer.ExoPlayer
import androidx.media3.exoplayer.source.DefaultMediaSourceFactory
import androidx.media3.ui.AspectRatioFrameLayout
import androidx.media3.ui.CaptionStyleCompat
import androidx.media3.ui.PlayerView
import dev.falsis.vessel.data.Episode
import dev.falsis.vessel.data.GetSkipSegmentsPayload
import dev.falsis.vessel.data.MediaDetail
import dev.falsis.vessel.data.MediaRef
import dev.falsis.vessel.data.PlaybackProgress
import dev.falsis.vessel.data.SkipSegment
import dev.falsis.vessel.data.StreamResult
import dev.falsis.vessel.data.VesselRepository
import dev.falsis.vessel.ui.i18n.VesselI18n
import dev.falsis.vessel.ui.theme.LocalVesselPalette
import dev.falsis.vessel.ui.theme.parseHexColor
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch

@kotlin.OptIn(ExperimentalMaterial3Api::class)
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
    onBack: () -> Unit,
    onGoToDetails: () -> Unit,
) {
    val context = LocalContext.current
    val activity = context as? Activity
    val palette = LocalVesselPalette.current
    val scope = rememberCoroutineScope()
    val prefs by repo.uiPreferences.collectAsState()

    var currentEpisode by remember(episode) { mutableStateOf(episode) }
    var activeStreamResult by remember(streamResult) { mutableStateOf(streamResult) }
    var selectedIndex by remember(streamResult) {
        mutableIntStateOf(initialStreamIndex.coerceIn(0, (streamResult.streams.size - 1).coerceAtLeast(0)))
    }
    var currentStartPosMs by remember(episode) { mutableLongStateOf(initialPositionMs) }

    var isLocked by remember { mutableStateOf(false) }
    var isLandscape by remember { mutableStateOf(false) }
    var resizeMode by remember { mutableIntStateOf(AspectRatioFrameLayout.RESIZE_MODE_FIT) }
    var showSourcesSheet by remember { mutableStateOf(false) }
    var showSpeedSheet by remember { mutableStateOf(false) }
    var playbackSpeed by remember { mutableFloatStateOf(1.0f) }

    var skipSegments by remember { mutableStateOf<List<SkipSegment>>(emptyList()) }
    var dismissedSkipKeys by remember { mutableStateOf<Set<String>>(emptySet()) }
    var dismissUpNext by remember(currentEpisode) { mutableStateOf(false) }
    var currentPosMs by remember { mutableLongStateOf(initialPositionMs) }
    var totalDurMs by remember { mutableLongStateOf(0L) }

    val activeCandidate = activeStreamResult.streams.getOrNull(selectedIndex) ?: activeStreamResult.streams.firstOrNull()

    val nextEpisode = remember(detail.episodes, currentEpisode) {
        val idx = detail.episodes.indexOfFirst { it.streamId == currentEpisode.streamId }
        if (idx >= 0 && idx + 1 < detail.episodes.size) detail.episodes[idx + 1] else null
    }

    val exoPlayer = remember {
        val httpFactory = DefaultHttpDataSource.Factory().setAllowCrossProtocolRedirects(true)
        ExoPlayer.Builder(context)
            .setMediaSourceFactory(DefaultMediaSourceFactory(httpFactory))
            .build()
            .apply { playWhenReady = true }
    }

    fun playEpisode(target: Episode) {
        scope.launch {
            runCatching { repo.getStreams(pluginId, target.streamId) }
                .onSuccess { sr ->
                    if (sr.streams.isEmpty()) {
                        repo.notify(VesselI18n.tr("details.no_streams"))
                    } else {
                        currentEpisode = target
                        activeStreamResult = sr
                        selectedIndex = sr.streams.indexOfFirst { it.isDefault }.takeIf { it >= 0 } ?: 0
                        currentStartPosMs = 0L
                        dismissedSkipKeys = emptySet()
                        dismissUpNext = false
                    }
                }
                .onFailure { repo.notify(it.message ?: "Failed to load next episode") }
        }
    }

    LaunchedEffect(activeCandidate, currentEpisode) {
        val cand = activeCandidate ?: return@LaunchedEffect
        val url = runCatching { repo.preparePlaybackUrl(cand) }.getOrElse { cand.url }

        val subs = (cand.subtitles + activeStreamResult.subtitles).mapNotNull { track ->
            val subUrl = track.source.url ?: return@mapNotNull null
            val mime = when (track.format.lowercase()) {
                "vtt" -> MimeTypes.TEXT_VTT
                "ass", "ssa" -> MimeTypes.TEXT_SSA
                else -> MimeTypes.APPLICATION_SUBRIP
            }
            MediaItem.SubtitleConfiguration.Builder(Uri.parse(subUrl))
                .setMimeType(mime)
                .setLanguage(track.lang)
                .setLabel(track.label)
                .setSelectionFlags(if (track.isDefault) C.SELECTION_FLAG_DEFAULT else 0)
                .build()
        }

        val item = MediaItem.Builder()
            .setUri(url)
            .setSubtitleConfigurations(subs)
            .build()
        exoPlayer.setMediaItem(item)
        if (currentStartPosMs > 0) {
            exoPlayer.seekTo(currentStartPosMs)
        }
        exoPlayer.prepare()
        exoPlayer.playWhenReady = true

        runCatching {
            skipSegments = repo.getSkipSegments(
                GetSkipSegmentsPayload(
                    pluginId = pluginId,
                    nativeId = nativeId,
                    mediaType = detail.type,
                    isAnime = detail.isAnime,
                    externalIds = detail.externalIds,
                    season = currentEpisode.season,
                    episode = currentEpisode.episode,
                    absoluteEpisode = currentEpisode.absoluteEpisode,
                    durationMs = totalDurMs.takeIf { it > 10_000L },
                    pluginSegments = cand.skipSegments,
                ),
            )
        }
    }

    suspend fun persistProgress(posMs: Long, durMs: Long) {
        if (posMs <= 0L) return
        val threshold = (prefs.playback.watchedThresholdPercent.coerceIn(50, 99)) / 100.0
        val completed = durMs > 0 && posMs >= (durMs * threshold).toLong()
        runCatching {
            repo.recordProgress(
                PlaybackProgress(
                    media = MediaRef(pluginId, nativeId),
                    streamId = currentEpisode.streamId,
                    season = currentEpisode.season,
                    episode = currentEpisode.episode,
                    absoluteEpisode = currentEpisode.absoluteEpisode,
                    title = detail.title,
                    posterUrl = detail.poster,
                    backdropUrl = detail.backdrop,
                    episodeTitle = currentEpisode.title,
                    positionMs = posMs,
                    durationMs = durMs,
                    completed = completed,
                ),
            )
        }
    }

    LaunchedEffect(exoPlayer, currentEpisode) {
        while (true) {
            currentPosMs = exoPlayer.currentPosition.coerceAtLeast(0L)
            totalDurMs = exoPlayer.duration.coerceAtLeast(0L)
            persistProgress(currentPosMs, totalDurMs)
            delay(5000L)
        }
    }

    DisposableEffect(Unit) {
        onDispose {
            val pos = exoPlayer.currentPosition.coerceAtLeast(0L)
            val dur = exoPlayer.duration.coerceAtLeast(0L)
            scope.launch { persistProgress(pos, dur) }
            activity?.requestedOrientation = ActivityInfo.SCREEN_ORIENTATION_UNSPECIFIED
            exoPlayer.release()
        }
    }

    val activeSkip = remember(skipSegments, currentPosMs, totalDurMs, dismissedSkipKeys) {
        skipSegments.firstOrNull { seg ->
            val key = "${seg.type}:${seg.startMs}:${seg.endMs}"
            if (key in dismissedSkipKeys) return@firstOrNull false
            val end = seg.endMs ?: totalDurMs
            currentPosMs >= seg.startMs && (end <= seg.startMs || currentPosMs < end - 1200L)
        }
    }

    val showUpNext = remember(nextEpisode, totalDurMs, currentPosMs, dismissUpNext, prefs.playback.autoNextDelaySecs) {
        if (nextEpisode == null || dismissUpNext || totalDurMs <= 30_000L) {
            false
        } else {
            val triggerWindowMs = (prefs.playback.autoNextDelaySecs.coerceIn(5, 30)) * 1000L
            (totalDurMs - currentPosMs) in 0..triggerWindowMs
        }
    }

    Box(
        modifier = Modifier
            .fillMaxSize()
            .background(Color.Black),
    ) {
        AndroidView(
            factory = { ctx ->
                PlayerView(ctx).apply {
                    player = exoPlayer
                    useController = !isLocked
                    this.resizeMode = resizeMode
                    setShowBuffering(PlayerView.SHOW_BUFFERING_WHEN_PLAYING)
                    layoutParams = FrameLayout.LayoutParams(
                        ViewGroup.LayoutParams.MATCH_PARENT,
                        ViewGroup.LayoutParams.MATCH_PARENT,
                    )
                }
            },
            update = { pv ->
                pv.player = exoPlayer
                pv.useController = !isLocked
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

        // Top Overlay Controls
        Row(
            modifier = Modifier
                .fillMaxWidth()
                .background(Color.Black.copy(alpha = 0.45f))
                .padding(horizontal = 8.dp, vertical = 8.dp)
                .align(Alignment.TopCenter),
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.SpaceBetween,
        ) {
            Row(verticalAlignment = Alignment.CenterVertically, modifier = Modifier.weight(1f)) {
                IconButton(onClick = onBack) {
                    Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = "Back", tint = Color.White)
                }
                Column(modifier = Modifier.weight(1f)) {
                    Text(
                        text = detail.title,
                        color = Color.White,
                        fontWeight = FontWeight.Bold,
                        maxLines = 1,
                        overflow = TextOverflow.Ellipsis,
                    )
                    if (detail.type != "movie" && detail.type != "live") {
                        Text(
                            text = "S${currentEpisode.season} · E${currentEpisode.episode}${currentEpisode.title?.let { " · $it" } ?: ""}",
                            color = Color.White.copy(alpha = 0.78f),
                            style = MaterialTheme.typography.labelSmall,
                            maxLines = 1,
                            overflow = TextOverflow.Ellipsis,
                        )
                    }
                }
            }

            Row(verticalAlignment = Alignment.CenterVertically) {
                if (!isLocked) {
                    if (nextEpisode != null) {
                        IconButton(onClick = { playEpisode(nextEpisode) }) {
                            Icon(Icons.Default.SkipNext, contentDescription = VesselI18n.tr("home.up_next", "Next"), tint = Color.White)
                        }
                    }
                    IconButton(onClick = {
                        resizeMode = when (resizeMode) {
                            AspectRatioFrameLayout.RESIZE_MODE_FIT -> AspectRatioFrameLayout.RESIZE_MODE_ZOOM
                            AspectRatioFrameLayout.RESIZE_MODE_ZOOM -> AspectRatioFrameLayout.RESIZE_MODE_FILL
                            else -> AspectRatioFrameLayout.RESIZE_MODE_FIT
                        }
                    }) {
                        Icon(Icons.Default.AspectRatio, contentDescription = "Aspect Ratio", tint = Color.White)
                    }
                    IconButton(onClick = {
                        isLandscape = !isLandscape
                        activity?.requestedOrientation = if (isLandscape) {
                            ActivityInfo.SCREEN_ORIENTATION_SENSOR_LANDSCAPE
                        } else {
                            ActivityInfo.SCREEN_ORIENTATION_UNSPECIFIED
                        }
                    }) {
                        Icon(Icons.Default.ScreenRotation, contentDescription = "Rotate", tint = Color.White)
                    }
                    IconButton(onClick = { showSpeedSheet = true }) {
                        Icon(Icons.Default.Speed, contentDescription = "Speed", tint = Color.White)
                    }
                    IconButton(onClick = { showSourcesSheet = true }) {
                        Icon(Icons.Default.ClosedCaption, contentDescription = "Sources", tint = Color.White)
                    }
                    IconButton(onClick = onGoToDetails) {
                        Icon(Icons.Default.Info, contentDescription = VesselI18n.tr("player.go_to_details"), tint = Color.White)
                    }
                }
                IconButton(onClick = { isLocked = !isLocked }) {
                    Icon(
                        if (isLocked) Icons.Default.Lock else Icons.Default.LockOpen,
                        contentDescription = "Lock",
                        tint = Color.White,
                    )
                }
            }
        }

        // Skip Intro / Recap / Credits / Preview Button
        if (!isLocked && activeSkip != null) {
            val skipLabel = when (activeSkip.type.lowercase()) {
                "recap" -> VesselI18n.tr("player.skip_recap")
                "credits" -> VesselI18n.tr("player.skip_credits")
                "preview" -> VesselI18n.tr("player.skip_preview")
                else -> VesselI18n.tr("player.skip_intro")
            }
            Surface(
                shape = RoundedCornerShape(14.dp),
                color = palette.bgSurface.copy(alpha = 0.92f),
                modifier = Modifier
                    .align(Alignment.BottomEnd)
                    .padding(end = 24.dp, bottom = 92.dp),
            ) {
                Row(
                    verticalAlignment = Alignment.CenterVertically,
                    modifier = Modifier.padding(horizontal = 6.dp, vertical = 4.dp),
                ) {
                    Button(
                        onClick = {
                            val target = activeSkip.endMs ?: totalDurMs
                            if (target > 0) exoPlayer.seekTo(target)
                            dismissedSkipKeys = dismissedSkipKeys + "${activeSkip.type}:${activeSkip.startMs}:${activeSkip.endMs}"
                        },
                        colors = ButtonDefaults.buttonColors(containerColor = palette.accent),
                    ) {
                        Icon(Icons.Default.FastForward, contentDescription = null)
                        Spacer(Modifier.width(6.dp))
                        Text(skipLabel, fontWeight = FontWeight.Bold)
                    }
                    TextButton(onClick = {
                        dismissedSkipKeys = dismissedSkipKeys + "${activeSkip.type}:${activeSkip.startMs}:${activeSkip.endMs}"
                    }) {
                        Text("×", color = palette.textPrimary)
                    }
                }
            }
        }

        // Up Next Episode Overlay (Desktop PlayerView parity)
        if (!isLocked && showUpNext && nextEpisode != null) {
            Surface(
                shape = RoundedCornerShape(14.dp),
                color = palette.bgSurface.copy(alpha = 0.94f),
                modifier = Modifier
                    .align(Alignment.BottomEnd)
                    .padding(end = 24.dp, bottom = 92.dp),
            ) {
                Row(
                    verticalAlignment = Alignment.CenterVertically,
                    horizontalArrangement = Arrangement.spacedBy(8.dp),
                    modifier = Modifier.padding(12.dp),
                ) {
                    Column {
                        Text(VesselI18n.tr("home.up_next", "Up Next"), style = MaterialTheme.typography.labelSmall, color = palette.accent)
                        Text(
                            text = "S${nextEpisode.season}·E${nextEpisode.episode} ${nextEpisode.title ?: ""}".trim(),
                            fontWeight = FontWeight.SemiBold,
                            maxLines = 1,
                            overflow = TextOverflow.Ellipsis,
                        )
                    }
                    Button(onClick = { playEpisode(nextEpisode) }) {
                        Icon(Icons.Default.SkipNext, contentDescription = null)
                        Spacer(Modifier.width(4.dp))
                        Text(VesselI18n.tr("common.play", "Play"))
                    }
                    TextButton(onClick = { dismissUpNext = true }) {
                        Text("×")
                    }
                }
            }
        }
    }

    if (showSourcesSheet) {
        ModalBottomSheet(onDismissRequest = { showSourcesSheet = false }) {
            Column(
                modifier = Modifier.fillMaxWidth().padding(16.dp),
                verticalArrangement = Arrangement.spacedBy(8.dp),
            ) {
                Text(VesselI18n.tr("details.sources"), style = MaterialTheme.typography.titleMedium, fontWeight = FontWeight.Bold)
                activeStreamResult.streams.forEachIndexed { idx, cand ->
                    Surface(
                        shape = RoundedCornerShape(10.dp),
                        color = if (idx == selectedIndex) palette.accent.copy(alpha = 0.25f) else palette.bgElevated,
                        modifier = Modifier
                            .fillMaxWidth()
                            .clickable {
                                selectedIndex = idx
                                showSourcesSheet = false
                            },
                    ) {
                        Text(
                            text = cand.label,
                            fontWeight = FontWeight.SemiBold,
                            modifier = Modifier.padding(12.dp),
                        )
                    }
                }
                Spacer(Modifier.height(16.dp))
            }
        }
    }

    if (showSpeedSheet) {
        ModalBottomSheet(onDismissRequest = { showSpeedSheet = false }) {
            Row(
                modifier = Modifier
                    .fillMaxWidth()
                    .horizontalScroll(rememberScrollState())
                    .padding(16.dp),
                horizontalArrangement = Arrangement.spacedBy(8.dp),
            ) {
                listOf(0.5f, 0.75f, 1.0f, 1.25f, 1.5f, 2.0f).forEach { spd ->
                    FilterChip(
                        selected = playbackSpeed == spd,
                        onClick = {
                            playbackSpeed = spd
                            exoPlayer.setPlaybackSpeed(spd)
                            showSpeedSheet = false
                        },
                        label = { Text("${spd}x") },
                    )
                }
            }
        }
    }
}
