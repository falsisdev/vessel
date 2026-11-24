package dev.falsis.vessel.ui.screens

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
import androidx.compose.material.icons.filled.ClosedCaption
import androidx.compose.material.icons.filled.FastForward
import androidx.compose.material.icons.filled.Info
import androidx.compose.material.icons.filled.Lock
import androidx.compose.material.icons.filled.LockOpen
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
    val palette = LocalVesselPalette.current
    val scope = rememberCoroutineScope()

    var selectedIndex by remember { mutableIntStateOf(initialStreamIndex.coerceIn(0, (streamResult.streams.size - 1).coerceAtLeast(0))) }
    var resolvedUrl by remember { mutableStateOf<String?>(null) }
    var isLocked by remember { mutableStateOf(false) }
    var showSourcesSheet by remember { mutableStateOf(false) }
    var showSpeedSheet by remember { mutableStateOf(false) }
    var playbackSpeed by remember { mutableFloatStateOf(1.0f) }

    var skipSegments by remember { mutableStateOf<List<SkipSegment>>(emptyList()) }
    var dismissedSkipKeys by remember { mutableStateOf<Set<String>>(emptySet()) }
    var currentPosMs by remember { mutableLongStateOf(initialPositionMs) }
    var totalDurMs by remember { mutableLongStateOf(0L) }

    val activeCandidate = streamResult.streams.getOrNull(selectedIndex) ?: streamResult.streams.firstOrNull()

    val exoPlayer = remember {
        val httpFactory = DefaultHttpDataSource.Factory().setAllowCrossProtocolRedirects(true)
        ExoPlayer.Builder(context)
            .setMediaSourceFactory(DefaultMediaSourceFactory(httpFactory))
            .build()
            .apply { playWhenReady = true }
    }

    LaunchedEffect(activeCandidate) {
        val cand = activeCandidate ?: return@LaunchedEffect
        val url = runCatching { repo.preparePlaybackUrl(cand) }.getOrElse { cand.url }
        resolvedUrl = url

        val subs = (cand.subtitles + streamResult.subtitles).mapNotNull { track ->
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
        if (initialPositionMs > 0) {
            exoPlayer.seekTo(initialPositionMs)
        }
        exoPlayer.prepare()
        exoPlayer.playWhenReady = true

        // Fetch validated skip segments (Plugin > TheIntroDB > IntroDB > AniSkip)
        runCatching {
            skipSegments = repo.getSkipSegments(
                GetSkipSegmentsPayload(
                    pluginId = pluginId,
                    nativeId = nativeId,
                    mediaType = detail.type,
                    isAnime = detail.isAnime,
                    externalIds = detail.externalIds,
                    season = episode.season,
                    episode = episode.episode,
                    absoluteEpisode = episode.absoluteEpisode,
                    durationMs = totalDurMs.takeIf { it > 10_000L },
                    pluginSegments = cand.skipSegments,
                ),
            )
        }
    }

    suspend fun persistProgress(posMs: Long, durMs: Long) {
        if (posMs <= 0L) return
        val completed = durMs > 0 && posMs >= (durMs * 0.90).toLong()
        runCatching {
            repo.recordProgress(
                PlaybackProgress(
                    media = MediaRef(pluginId, nativeId),
                    streamId = episode.streamId,
                    season = episode.season,
                    episode = episode.episode,
                    absoluteEpisode = episode.absoluteEpisode,
                    title = detail.title,
                    posterUrl = detail.poster,
                    backdropUrl = detail.backdrop,
                    episodeTitle = episode.title,
                    positionMs = posMs,
                    durationMs = durMs,
                    completed = completed,
                ),
            )
        }
    }

    LaunchedEffect(exoPlayer) {
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
            },
            modifier = Modifier.fillMaxSize(),
        )

        // Top Overlay Controls
        Row(
            modifier = Modifier
                .fillMaxWidth()
                .background(Color.Black.copy(alpha = 0.45f))
                .padding(horizontal = 12.dp, vertical = 10.dp)
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
                    if (detail.type != "movie") {
                        Text(
                            text = "S${episode.season} · E${episode.episode}${episode.title?.let { " · $it" } ?: ""}",
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
                    TextButton(onClick = onGoToDetails) {
                        Icon(Icons.Default.Info, contentDescription = null, tint = Color.White)
                        Spacer(Modifier.width(4.dp))
                        Text(VesselI18n.tr("player.go_to_details"), color = Color.White)
                    }
                    IconButton(onClick = { showSpeedSheet = true }) {
                        Icon(Icons.Default.Speed, contentDescription = "Speed", tint = Color.White)
                    }
                    IconButton(onClick = { showSourcesSheet = true }) {
                        Icon(Icons.Default.ClosedCaption, contentDescription = "Sources", tint = Color.White)
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
    }

    if (showSourcesSheet) {
        ModalBottomSheet(onDismissRequest = { showSourcesSheet = false }) {
            Column(
                modifier = Modifier.fillMaxWidth().padding(16.dp),
                verticalArrangement = Arrangement.spacedBy(8.dp),
            ) {
                Text(VesselI18n.tr("details.sources"), style = MaterialTheme.typography.titleMedium, fontWeight = FontWeight.Bold)
                streamResult.streams.forEachIndexed { idx, cand ->
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
