package dev.falsis.vessel.ui.screens

import androidx.compose.animation.animateContentSize
import androidx.compose.animation.core.tween
import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.displayCutout
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.statusBarsIgnoringVisibility
import androidx.compose.foundation.layout.union
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.windowInsetsPadding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.automirrored.filled.List
import androidx.compose.material.icons.filled.Bookmark
import androidx.compose.material.icons.filled.Check
import androidx.compose.material.icons.filled.CheckCircle
import androidx.compose.material.icons.filled.Delete
import androidx.compose.material.icons.filled.Download
import androidx.compose.material.icons.filled.Edit
import androidx.compose.material.icons.filled.Layers
import androidx.compose.material.icons.filled.MoreVert
import androidx.compose.material.icons.filled.PlayArrow
import androidx.compose.material.icons.filled.RadioButtonUnchecked
import androidx.compose.material.icons.filled.Search
import androidx.compose.material.icons.filled.Star
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Checkbox
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.LinearProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateMapOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import coil3.compose.AsyncImage
import dev.falsis.vessel.data.DebridResolution
import dev.falsis.vessel.data.EnqueueDownloadPayload
import dev.falsis.vessel.data.Episode
import dev.falsis.vessel.data.LibraryItem
import dev.falsis.vessel.data.LinkedMediaSource
import dev.falsis.vessel.data.MediaDetail
import dev.falsis.vessel.data.MediaRef
import dev.falsis.vessel.data.MediaSummary
import dev.falsis.vessel.data.MetadataSearchHit
import dev.falsis.vessel.data.PlaybackProgress
import dev.falsis.vessel.data.PluginListEntry
import dev.falsis.vessel.data.ResolveDebridRequest
import dev.falsis.vessel.data.StreamCandidate
import dev.falsis.vessel.data.StreamResult
import dev.falsis.vessel.data.VesselRepository
import dev.falsis.vessel.ui.components.formatSizeBytes
import dev.falsis.vessel.ui.components.VesselButton
import dev.falsis.vessel.ui.components.VesselCompactIconButton
import dev.falsis.vessel.ui.components.VesselFilterChip
import dev.falsis.vessel.ui.components.VesselOutlinedButton
import dev.falsis.vessel.ui.components.VesselOutlinedTextField
import dev.falsis.vessel.ui.components.VesselTextButton
import dev.falsis.vessel.ui.components.vesselDpadInputEscape
import dev.falsis.vessel.ui.components.vesselFocusBorder
import dev.falsis.vessel.ui.i18n.VesselI18n
import dev.falsis.vessel.ui.isLiveMediaIdentifier
import dev.falsis.vessel.ui.isLiveTvPlugin
import dev.falsis.vessel.ui.theme.LocalVesselPalette
import java.util.Locale
import kotlinx.coroutines.launch

private data class PendingUncachedAction(
    val detail: MediaDetail,
    val episode: Episode,
    val streamResult: StreamResult,
    val index: Int,
    val savedPosMs: Long,
    val candidate: StreamCandidate,
    val resolution: DebridResolution,
    val isDownload: Boolean,
)

private fun isTorrentCandidate(cand: StreamCandidate): Boolean =
    cand.kind.equals("torrent", ignoreCase = true) ||
        cand.torrent != null ||
        cand.url.startsWith("magnet:", ignoreCase = true)

@OptIn(ExperimentalMaterial3Api::class, ExperimentalLayoutApi::class)
@Composable
fun DetailsScreen(
    repo: VesselRepository,
    pluginId: String,
    nativeId: String,
    onBack: () -> Unit,
    onOpenDebridSettings: () -> Unit = {},
    onPlayStream: (detail: MediaDetail, episode: Episode, streams: StreamResult, selectedIdx: Int, resumePosMs: Long) -> Unit,
    onPlayLiveReplace: ((detail: MediaDetail, episode: Episode, streams: StreamResult) -> Unit)? = null,
) {
    val palette = LocalVesselPalette.current
    val scope = rememberCoroutineScope()
    val plugins by repo.plugins.collectAsState()
    val prefs by repo.uiPreferences.collectAsState()
    val activePlugin = remember(plugins, pluginId) { plugins.firstOrNull { it.manifest.id == pluginId } }

    var loading by remember { mutableStateOf(true) }
    var detail by remember { mutableStateOf<MediaDetail?>(null) }
    var progressList by remember { mutableStateOf<List<PlaybackProgress>>(emptyList()) }
    var currentLibraryStatus by remember { mutableStateOf<String?>(null) }
    var selectedSeason by remember { mutableIntStateOf(1) }
    var expandSynopsis by remember { mutableStateOf(false) }
    var showFixMatch by remember { mutableStateOf(false) }
    var showStatusMenu by remember { mutableStateOf(false) }
    var showMoreMenu by remember { mutableStateOf(false) }
    var showIncludeSources by remember { mutableStateOf(false) }
    var linkedSources by remember { mutableStateOf<List<LinkedMediaSource>>(emptyList()) }

    var activeEpisodeForSources by remember { mutableStateOf<Episode?>(null) }
    var loadingStreams by remember { mutableStateOf(false) }
    var resolvedStreamResult by remember { mutableStateOf<StreamResult?>(null) }
    var quickStartingStreamId by remember { mutableStateOf<String?>(null) }
    var resolvingTorrent by remember { mutableStateOf(false) }
    var uncachedPrompt by remember { mutableStateOf<PendingUncachedAction?>(null) }
    var showNoDebridDialog by remember { mutableStateOf(false) }

    suspend fun loadAll() {
        loading = true
        runCatching {
            val isVirtual = pluginId == "vessel.metadata"
            val loaded = if (isVirtual) {
                repo.getVirtualMetadataDetail(nativeId)
            } else {
                repo.getDetails(pluginId, nativeId)
            }
            val isLiveRedirect = !isVirtual && isLiveMediaIdentifier(
                mediaType = loaded.type,
                pluginId = pluginId,
                streamId = loaded.episodes.firstOrNull()?.streamId,
                nativeId = nativeId,
                plugin = activePlugin,
            )
            if (isLiveRedirect) {
                val ep = loaded.episodes.firstOrNull() ?: Episode(
                    streamId = nativeId,
                    season = 1,
                    episode = 1,
                    title = loaded.title,
                    thumbnail = loaded.poster,
                )
                val sr = repo.getStreams(pluginId, ep.streamId)
                if (sr.streams.isEmpty()) {
                    repo.notify(VesselI18n.tr("details.no_streams"))
                    onBack()
                } else if (onPlayLiveReplace != null) {
                    onPlayLiveReplace(loaded.copy(type = "live"), ep, sr)
                } else {
                    onPlayStream(loaded.copy(type = "live"), ep, sr, 0, 0L)
                }
                return@runCatching
            }
            detail = loaded
            linkedSources = if (isVirtual) emptyList() else runCatching { repo.getLinkedSources(pluginId, nativeId) }.getOrDefault(emptyList())
            val seasons = loaded.episodes.map { it.season }.distinct().sorted()
            if (seasons.isNotEmpty() && selectedSeason !in seasons) {
                selectedSeason = seasons.first()
            }
            progressList = repo.listMediaWatchProgress(pluginId, nativeId)
            currentLibraryStatus = repo.listLibrary()
                .firstOrNull { it.media.pluginId == pluginId && it.media.nativeId == nativeId }
                ?.status
        }.onFailure {
            repo.notifyError(it, "Failed to load details")
        }
        loading = false
    }

    LaunchedEffect(pluginId, nativeId) { loadAll() }

    LaunchedEffect(pluginId, nativeId) {
        repo.metadataEnriched.collect { ev ->
            if (ev.pluginId == pluginId && ev.nativeId == nativeId) {
                detail = ev.detail
            }
        }
    }

    fun progressFor(ep: Episode): PlaybackProgress? =
        progressList.firstOrNull { it.season == ep.season && it.episode == ep.episode }

    suspend fun resolveAndLaunchCandidate(
        d: MediaDetail,
        ep: Episode,
        sr: StreamResult,
        idx: Int,
        savedPos: Long,
        allowUncachedQueue: Boolean = false,
    ) {
        val cand = sr.streams.getOrNull(idx) ?: return
        if (!isTorrentCandidate(cand)) {
            activeEpisodeForSources = null
            onPlayStream(d, ep, sr, idx, savedPos)
            return
        }
        resolvingTorrent = true
        try {
            val res = repo.resolveTorrentCandidate(
                ResolveDebridRequest(
                    url = cand.url,
                    torrent = cand.torrent,
                    preferredTarget = cand.debridCache?.takeIf { it.state == "cached" }?.target,
                    allowUncachedQueue = allowUncachedQueue,
                    season = if (d.type == "movie") null else ep.season,
                    episode = if (d.type == "movie") null else ep.episode,
                ),
            )
            when (res.status) {
                "ready" -> {
                    val resolvedUrl = res.url ?: run {
                        repo.notify("Debrid returned an empty URL")
                        return
                    }
                    val updatedStreams = sr.streams.mapIndexed { i, item ->
                        if (i == idx) {
                            item.copy(
                                url = resolvedUrl,
                                kind = "auto",
                                headers = res.headers,
                                flags = if (res.headers.isNotEmpty()) (item.flags + "requires_proxy").distinct() else item.flags,
                            )
                        } else {
                            item
                        }
                    }
                    activeEpisodeForSources = null
                    onPlayStream(d, ep, sr.copy(streams = updatedStreams), idx, savedPos)
                }
                "needs_confirmation" -> {
                    if (allowUncachedQueue) {
                        val pct = res.progressPct?.toInt() ?: 0
                        repo.notify("${res.providerLabel ?: "Debrid"}: ${res.cloudStatus ?: "queued"} ($pct%)")
                    } else {
                        uncachedPrompt = PendingUncachedAction(
                            detail = d,
                            episode = ep,
                            streamResult = sr,
                            index = idx,
                            savedPosMs = savedPos,
                            candidate = cand,
                            resolution = res,
                            isDownload = false,
                        )
                    }
                }
                "no_provider_configured" -> showNoDebridDialog = true
                else -> repo.notify(res.message ?: "Failed to resolve torrent")
            }
        } catch (err: Throwable) {
            repo.notifyError(err, "Failed to resolve torrent")
        } finally {
            resolvingTorrent = false
        }
    }

    suspend fun resolveAndDownloadCandidate(
        d: MediaDetail,
        ep: Episode,
        sr: StreamResult,
        idx: Int,
        allowUncachedQueue: Boolean = false,
    ) {
        val cand = sr.streams.getOrNull(idx) ?: return
        var targetUrl = cand.url
        var targetHeaders = cand.headers
        var targetKind = if (cand.kind == "hls" || cand.url.contains(".m3u8")) "hls" else cand.kind.ifBlank { "auto" }
        if (isTorrentCandidate(cand)) {
            resolvingTorrent = true
            try {
                val res = repo.resolveTorrentCandidate(
                    ResolveDebridRequest(
                        url = cand.url,
                        torrent = cand.torrent,
                        preferredTarget = cand.debridCache?.takeIf { it.state == "cached" }?.target,
                        allowUncachedQueue = allowUncachedQueue,
                        season = if (d.type == "movie") null else ep.season,
                        episode = if (d.type == "movie") null else ep.episode,
                    ),
                )
                when (res.status) {
                    "ready" -> {
                        targetUrl = res.url ?: return
                        targetHeaders = res.headers
                        targetKind = "auto"
                    }
                    "needs_confirmation" -> {
                        if (allowUncachedQueue) {
                            val pct = res.progressPct?.toInt() ?: 0
                            repo.notify("${res.providerLabel ?: "Debrid"}: ${res.cloudStatus ?: "queued"} ($pct%)")
                        } else {
                            uncachedPrompt = PendingUncachedAction(
                                detail = d,
                                episode = ep,
                                streamResult = sr,
                                index = idx,
                                savedPosMs = 0L,
                                candidate = cand,
                                resolution = res,
                                isDownload = true,
                            )
                        }
                        return
                    }
                    "no_provider_configured" -> {
                        showNoDebridDialog = true
                        return
                    }
                    else -> {
                        repo.notify(res.message ?: "Failed to resolve torrent")
                        return
                    }
                }
            } finally {
                resolvingTorrent = false
            }
        }

        runCatching {
            repo.enqueueDownload(
                EnqueueDownloadPayload(
                    pluginId = pluginId,
                    nativeId = nativeId,
                    title = d.title,
                    season = if (d.type == "movie") null else ep.season,
                    episode = if (d.type == "movie") null else ep.episode,
                    streamLabel = cand.label,
                    kind = targetKind,
                    sourceUrl = targetUrl,
                    headers = targetHeaders,
                    subtitles = cand.subtitles + sr.subtitles,
                ),
            )
        }.onSuccess {
            repo.notify("${VesselI18n.tr("details.download_episode")}: ${cand.label}")
        }.onFailure {
            repo.notifyError(it, "Download failed")
        }
    }

    fun openEpisodeSources(ep: Episode) {
        activeEpisodeForSources = ep
        resolvedStreamResult = null
        loadingStreams = true
        scope.launch {
            runCatching {
                repo.getStreamsWithExternalIds(
                    pluginId = pluginId,
                    nativeId = nativeId,
                    episodeId = ep.streamId,
                    season = ep.season,
                    episode = ep.episode,
                    absoluteEpisode = ep.absoluteEpisode,
                    isMovie = detail?.type == "movie",
                    externalIds = detail?.externalIds,
                )
            }
                .onSuccess { sr ->
                    resolvedStreamResult = sr
                    if (sr.streams.any(::isTorrentCandidate)) {
                        scope.launch {
                            runCatching { repo.probeTorrentCache(sr.streams) }
                                .onSuccess { probed ->
                                    if (activeEpisodeForSources?.streamId == ep.streamId) {
                                        resolvedStreamResult = sr.copy(streams = probed)
                                    }
                                }
                        }
                    }
                }
                .onFailure { repo.notifyError(it, "Failed to resolve streams") }
            loadingStreams = false
        }
    }

    fun handleEpisodePlayClick(ep: Episode) {
        val d = detail ?: return
        if (prefs.playback.clickBehavior == "always_ask_source") {
            openEpisodeSources(ep)
            return
        }
        quickStartingStreamId = ep.streamId
        scope.launch {
            runCatching {
                repo.getStreamsWithExternalIds(
                    pluginId = pluginId,
                    nativeId = nativeId,
                    episodeId = ep.streamId,
                    season = ep.season,
                    episode = ep.episode,
                    absoluteEpisode = ep.absoluteEpisode,
                    isMovie = d.type == "movie",
                    externalIds = d.externalIds,
                )
            }
                .onSuccess { sr ->
                    quickStartingStreamId = null
                    if (sr.streams.isEmpty()) {
                        repo.notify(VesselI18n.tr("details.no_streams"))
                    } else if (sr.streams.size == 1 || prefs.playback.clickBehavior == "quick_start_best") {
                        val bestIdx = sr.streams.indexOfFirst { it.isDefault }.takeIf { it >= 0 } ?: 0
                        val savedPos = progressFor(ep)?.takeIf { !it.completed }?.positionMs ?: 0L
                        resolveAndLaunchCandidate(d, ep, sr, bestIdx, savedPos)
                    } else {
                        activeEpisodeForSources = ep
                        resolvedStreamResult = sr
                        loadingStreams = false
                        if (sr.streams.any(::isTorrentCandidate)) {
                            scope.launch {
                                runCatching { repo.probeTorrentCache(sr.streams) }
                                    .onSuccess { probed ->
                                        if (activeEpisodeForSources?.streamId == ep.streamId) {
                                            resolvedStreamResult = sr.copy(streams = probed)
                                        }
                                    }
                            }
                        }
                    }
                }
                .onFailure {
                    quickStartingStreamId = null
                    repo.notifyError(it, "Failed to resolve streams")
                }
        }
    }

    fun toggleEpisodeWatched(ep: Episode) {
        val d = detail ?: return
        val existing = progressFor(ep)
        val markCompleted = existing?.completed != true
        scope.launch {
            runCatching {
                repo.recordProgress(
                    PlaybackProgress(
                        media = MediaRef(pluginId, nativeId),
                        streamId = ep.streamId,
                        season = ep.season,
                        episode = ep.episode,
                        absoluteEpisode = ep.absoluteEpisode,
                        title = d.title,
                        posterUrl = d.poster,
                        backdropUrl = d.backdrop,
                        episodeTitle = ep.title,
                        positionMs = if (markCompleted) 1000L else 0L,
                        durationMs = 1000L,
                        completed = markCompleted,
                        lastWatchedAt = java.time.Instant.now().toString(),
                    ),
                )
                progressList = repo.listMediaWatchProgress(pluginId, nativeId)
            }
        }
    }

    if (loading) {
        Box(modifier = Modifier.fillMaxSize().background(palette.bgBase), contentAlignment = Alignment.Center) {
            CircularProgressIndicator()
        }
        return
    }

    val d = detail ?: run {
        Box(modifier = Modifier.fillMaxSize().background(palette.bgBase), contentAlignment = Alignment.Center) {
            Column(horizontalAlignment = Alignment.CenterHorizontally, verticalArrangement = Arrangement.spacedBy(12.dp)) {
                Text(VesselI18n.tr("common.empty"))
                VesselButton(onClick = onBack) { Text(VesselI18n.tr("common.back")) }
            }
        }
        return
    }

    val seasons = d.episodes.map { it.season }.distinct().sorted()
    val filteredEpisodes = if (seasons.size > 1) {
        d.episodes.filter { it.season == selectedSeason }
    } else {
        d.episodes
    }
    val resumeCandidateEp = d.episodes.firstOrNull { ep ->
        val p = progressFor(ep)
        p != null && !p.completed && p.positionMs > 0
    } ?: d.episodes.firstOrNull { ep ->
        progressFor(ep)?.completed != true
    } ?: d.episodes.firstOrNull()

    LazyColumn(
        modifier = Modifier
            .fillMaxSize()
            .background(palette.bgBase),
        contentPadding = PaddingValues(bottom = 32.dp),
    ) {
        // Hero Backdrop Header + Poster
        item {
            Box(
                modifier = Modifier
                    .fillMaxWidth()
                    .height(320.dp)
                    .background(palette.bgSurface),
            ) {
                if (prefs.details.showBackdrop && !(d.backdrop ?: d.poster).isNullOrBlank()) {
                    AsyncImage(
                        model = d.backdrop ?: d.poster,
                        contentDescription = d.title,
                        contentScale = ContentScale.Crop,
                        modifier = Modifier.fillMaxSize(),
                    )
                }
                Box(
                    modifier = Modifier
                        .fillMaxSize()
                        .background(
                            Brush.verticalGradient(
                                0.0f to Color.Black.copy(alpha = 0.45f),
                                0.55f to palette.bgBase.copy(alpha = 0.72f),
                                1.0f to palette.bgBase,
                            ),
                        ),
                )
                Row(
                    modifier = Modifier
                        .fillMaxWidth()
                        .windowInsetsPadding(WindowInsets.statusBarsIgnoringVisibility.union(WindowInsets.displayCutout))
                        .padding(horizontal = 12.dp, vertical = 10.dp),
                    horizontalArrangement = Arrangement.SpaceBetween,
                    verticalAlignment = Alignment.CenterVertically,
                ) {
                    VesselCompactIconButton(
                        onClick = onBack,
                        size = 34.dp,
                        containerColor = Color.Black.copy(alpha = 0.45f),
                        borderColor = Color.White.copy(alpha = 0.16f),
                    ) {
                        Icon(
                            Icons.AutoMirrored.Filled.ArrowBack,
                            contentDescription = VesselI18n.tr("common.back"),
                            tint = Color.White,
                            modifier = Modifier.size(18.dp),
                        )
                    }

                    Row(
                        verticalAlignment = Alignment.CenterVertically,
                        horizontalArrangement = Arrangement.spacedBy(6.dp),
                    ) {
                        if (linkedSources.isNotEmpty()) {
                            Surface(
                                shape = RoundedCornerShape(999.dp),
                                color = Color.Black.copy(alpha = 0.55f),
                                border = BorderStroke(1.dp, palette.accent.copy(alpha = 0.45f)),
                                modifier = Modifier
                                    .clip(RoundedCornerShape(999.dp))
                                    .vesselFocusBorder(RoundedCornerShape(999.dp))
                                    .clickable { showIncludeSources = true },
                            ) {
                                Text(
                                    text = "+${linkedSources.size}",
                                    style = MaterialTheme.typography.labelSmall,
                                    fontWeight = FontWeight.Bold,
                                    color = palette.accent,
                                    modifier = Modifier.padding(horizontal = 8.dp, vertical = 4.dp),
                                )
                            }
                        }

                        Box {
                            VesselCompactIconButton(
                                onClick = { showMoreMenu = true },
                                size = 34.dp,
                                containerColor = Color.Black.copy(alpha = 0.45f),
                                borderColor = Color.White.copy(alpha = 0.16f),
                            ) {
                                Icon(
                                    Icons.Default.MoreVert,
                                    contentDescription = VesselI18n.tr("details.more_actions"),
                                    tint = Color.White,
                                    modifier = Modifier.size(18.dp),
                                )
                            }
                            DropdownMenu(expanded = showMoreMenu, onDismissRequest = { showMoreMenu = false }) {
                                DropdownMenuItem(
                                    leadingIcon = { Icon(Icons.Default.Layers, contentDescription = null, modifier = Modifier.size(18.dp)) },
                                    text = { Text(VesselI18n.tr("details.include_sources")) },
                                    onClick = {
                                        showMoreMenu = false
                                        showIncludeSources = true
                                    },
                                )
                                DropdownMenuItem(
                                    leadingIcon = { Icon(Icons.Default.Edit, contentDescription = null, modifier = Modifier.size(18.dp)) },
                                    text = { Text(VesselI18n.tr("details.fix_match")) },
                                    onClick = {
                                        showMoreMenu = false
                                        showFixMatch = true
                                    },
                                )
                            }
                        }
                    }
                }

                Row(
                    modifier = Modifier
                        .align(Alignment.BottomStart)
                        .fillMaxWidth()
                        .padding(16.dp),
                    horizontalArrangement = Arrangement.spacedBy(14.dp),
                    verticalAlignment = Alignment.Bottom,
                ) {
                    if (!d.poster.isNullOrBlank()) {
                        AsyncImage(
                            model = d.poster,
                            contentDescription = d.title,
                            contentScale = ContentScale.Crop,
                            modifier = Modifier
                                .width(96.dp)
                                .height(142.dp)
                                .clip(RoundedCornerShape(12.dp))
                                .background(palette.bgElevated),
                        )
                    }
                    Column(modifier = Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(6.dp)) {
                        Text(
                            text = d.title,
                            style = MaterialTheme.typography.headlineSmall,
                            fontWeight = FontWeight.ExtraBold,
                            maxLines = 2,
                            overflow = TextOverflow.Ellipsis,
                        )
                        Row(
                            verticalAlignment = Alignment.CenterVertically,
                            horizontalArrangement = Arrangement.spacedBy(8.dp),
                        ) {
                            d.year?.let { Text("$it", color = palette.textMuted, style = MaterialTheme.typography.labelMedium) }
                            d.rating?.takeIf { it > 0f }?.let { score ->
                                Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(2.dp)) {
                                    Icon(Icons.Default.Star, contentDescription = null, tint = palette.warning, modifier = Modifier.size(14.dp))
                                    Text(String.format("%.1f", score), style = MaterialTheme.typography.labelMedium, fontWeight = FontWeight.Bold)
                                }
                            }
                            d.status?.takeIf { it.isNotBlank() }?.let { st ->
                                Surface(color = palette.bgElevated, shape = RoundedCornerShape(6.dp)) {
                                    Text(st, style = MaterialTheme.typography.labelSmall, modifier = Modifier.padding(horizontal = 6.dp, vertical = 2.dp))
                                }
                            }
                        }
                        if (d.genres.isNotEmpty()) {
                            Text(
                                text = d.genres.take(3).joinToString(" · "),
                                color = palette.textMuted,
                                style = MaterialTheme.typography.labelSmall,
                                maxLines = 1,
                                overflow = TextOverflow.Ellipsis,
                            )
                        }
                    }
                }
            }
        }

        // Actions Row (Watch/Resume + Source Picker + Library Status + Fix Match)
        item {
            Row(
                modifier = Modifier
                    .fillMaxWidth()
                    .horizontalScroll(rememberScrollState())
                    .padding(horizontal = 16.dp, vertical = 6.dp),
                horizontalArrangement = Arrangement.spacedBy(8.dp),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                if (resumeCandidateEp != null) {
                    val saved = progressFor(resumeCandidateEp)
                    val isBusy = quickStartingStreamId == resumeCandidateEp.streamId
                    VesselButton(
                        enabled = !isBusy,
                        onClick = { handleEpisodePlayClick(resumeCandidateEp) },
                    ) {
                        if (isBusy) {
                            CircularProgressIndicator(modifier = Modifier.size(14.dp), strokeWidth = 2.dp, color = palette.bgBase)
                        } else {
                            Icon(Icons.Default.PlayArrow, contentDescription = null, modifier = Modifier.size(16.dp))
                        }
                        Spacer(Modifier.width(4.dp))
                        Text(
                            if (saved != null && saved.positionMs > 0 && !saved.completed) {
                                VesselI18n.tr("details.resume")
                            } else {
                                VesselI18n.tr("details.watch")
                            },
                        )
                    }
                    VesselOutlinedButton(onClick = { openEpisodeSources(resumeCandidateEp) }) {
                        Icon(Icons.AutoMirrored.Filled.List, contentDescription = null, modifier = Modifier.size(15.dp))
                        Spacer(Modifier.width(4.dp))
                        Text(VesselI18n.tr("details.sources"))
                    }
                }

                Box {
                    VesselOutlinedButton(onClick = { showStatusMenu = true }) {
                        Icon(Icons.Default.Bookmark, contentDescription = null, modifier = Modifier.size(14.dp))
                        Spacer(Modifier.width(4.dp))
                        Text(
                            currentLibraryStatus?.let { VesselI18n.tr("library.$it") }
                                ?: VesselI18n.tr("details.library_status"),
                        )
                    }
                    DropdownMenu(expanded = showStatusMenu, onDismissRequest = { showStatusMenu = false }) {
                        listOf("watching", "plan_to_watch", "completed", "on_hold", "dropped").forEach { status ->
                            DropdownMenuItem(
                                text = { Text(VesselI18n.tr("library.$status")) },
                                onClick = {
                                    showStatusMenu = false
                                    scope.launch {
                                        repo.upsertLibrary(
                                            LibraryItem(
                                                media = MediaRef(pluginId, nativeId),
                                                mediaType = d.type,
                                                isAnime = d.isAnime,
                                                title = d.title,
                                                posterUrl = d.poster,
                                                year = d.year,
                                                status = status,
                                            ),
                                        )
                                        currentLibraryStatus = status
                                        repo.notify("${d.title} · ${VesselI18n.tr("library.$status")}")
                                    }
                                },
                            )
                        }
                        if (currentLibraryStatus != null) {
                            DropdownMenuItem(
                                text = { Text(VesselI18n.tr("library.remove_item"), color = palette.danger) },
                                onClick = {
                                    showStatusMenu = false
                                    scope.launch {
                                        repo.deleteLibraryItem(pluginId, nativeId)
                                        currentLibraryStatus = null
                                    }
                                },
                            )
                        }
                    }
                }
            }
        }

        // External IDs Badges (matching Desktop DetailsView)
        val extBadges = buildList {
            d.externalIds.tmdb?.let { add("TMDB:$it") }
            d.externalIds.imdb?.let { add(it) }
            d.externalIds.simkl?.let { add("Simkl:$it") }
            d.externalIds.anilist?.let { add("AniList:$it") }
            d.externalIds.mal?.let { add("MAL:$it") }
            d.externalIds.kitsu?.let { add("Kitsu:$it") }
        }
        if (extBadges.isNotEmpty()) {
            item {
                Row(
                    modifier = Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 4.dp),
                    horizontalArrangement = Arrangement.spacedBy(6.dp),
                ) {
                    extBadges.forEach { badge ->
                        Surface(color = palette.bgSurface, shape = RoundedCornerShape(6.dp)) {
                            Text(
                                text = badge,
                                style = MaterialTheme.typography.labelSmall,
                                color = palette.textMuted,
                                modifier = Modifier.padding(horizontal = 8.dp, vertical = 3.dp),
                            )
                        }
                    }
                }
            }
        }

        // Synopsis (tap to expand/collapse with smooth height animation)
        d.synopsis?.takeIf { it.isNotBlank() }?.let { syn ->
            item {
                Text(
                    text = syn,
                    style = MaterialTheme.typography.bodyMedium,
                    color = palette.textMuted,
                    maxLines = if (expandSynopsis) Int.MAX_VALUE else 4,
                    overflow = TextOverflow.Ellipsis,
                    modifier = Modifier
                        .fillMaxWidth()
                        .animateContentSize(tween(240))
                        .clickable { expandSynopsis = !expandSynopsis }
                        .padding(horizontal = 16.dp, vertical = 8.dp),
                )
            }
        }

        // Season Selector
        if (seasons.size > 1) {
            item {
                Row(
                    modifier = Modifier
                        .fillMaxWidth()
                        .horizontalScroll(rememberScrollState())
                        .padding(horizontal = 16.dp, vertical = 8.dp),
                    horizontalArrangement = Arrangement.spacedBy(6.dp),
                ) {
                    seasons.forEach { s ->
                        VesselFilterChip(
                            selected = selectedSeason == s,
                            onClick = { selectedSeason = s },
                            label = "${VesselI18n.tr("details.season")} $s",
                        )
                    }
                }
            }
        }

        // Episodes List
        items(filteredEpisodes, key = { it.streamId }) { ep ->
            val prog = progressFor(ep)
            val ratio = if (prog != null && prog.durationMs > 0) {
                (prog.positionMs.toFloat() / prog.durationMs).coerceIn(0f, 1f)
            } else 0f
            val epLabel = ep.title?.takeIf { it.isNotBlank() }
                ?: "${VesselI18n.tr("downloads.episode_label")} ${ep.episode}"
            val isQuickBusy = quickStartingStreamId == ep.streamId

            Surface(
                shape = RoundedCornerShape(12.dp),
                color = palette.bgSurface,
                modifier = Modifier
                    .fillMaxWidth()
                    .padding(horizontal = 16.dp, vertical = 5.dp)
                    .clip(RoundedCornerShape(12.dp))
                    .vesselFocusBorder(RoundedCornerShape(12.dp))
                    .clickable { handleEpisodePlayClick(ep) },
            ) {
                Column(modifier = Modifier.padding(12.dp), verticalArrangement = Arrangement.spacedBy(6.dp)) {
                    Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(12.dp)) {
                        if (!prefs.details.hideEpisodeThumbnails) {
                            ep.thumbnail?.let { thumb ->
                                AsyncImage(
                                    model = thumb,
                                    contentDescription = null,
                                    contentScale = ContentScale.Crop,
                                    modifier = Modifier
                                        .width(104.dp)
                                        .height(60.dp)
                                        .clip(RoundedCornerShape(8.dp))
                                        .background(palette.bgElevated),
                                )
                            }
                        }
                        Column(modifier = Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(2.dp)) {
                            Text(
                                text = "${ep.episode}. $epLabel",
                                fontWeight = FontWeight.SemiBold,
                                maxLines = 1,
                                overflow = TextOverflow.Ellipsis,
                            )
                            if (!prefs.details.hideEpisodeOverviews) {
                                ep.overview?.takeIf { it.isNotBlank() }?.let { ov ->
                                    Text(
                                        text = ov,
                                        style = MaterialTheme.typography.bodySmall,
                                        color = palette.textMuted,
                                        maxLines = 2,
                                        overflow = TextOverflow.Ellipsis,
                                    )
                                }
                            }
                        }
                        Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(4.dp)) {
                            if (isQuickBusy) {
                                CircularProgressIndicator(modifier = Modifier.size(16.dp), strokeWidth = 2.dp)
                            }
                            VesselCompactIconButton(onClick = { toggleEpisodeWatched(ep) }, size = 30.dp) {
                                Icon(
                                    imageVector = if (prog?.completed == true) Icons.Default.CheckCircle else Icons.Default.RadioButtonUnchecked,
                                    contentDescription = VesselI18n.tr("details.watched_badge"),
                                    tint = if (prog?.completed == true) palette.success else palette.textMuted,
                                    modifier = Modifier.size(18.dp),
                                )
                            }
                            VesselCompactIconButton(onClick = { openEpisodeSources(ep) }, size = 30.dp) {
                                Icon(
                                    imageVector = Icons.AutoMirrored.Filled.List,
                                    contentDescription = VesselI18n.tr("details.select_stream"),
                                    tint = palette.textMuted,
                                    modifier = Modifier.size(18.dp),
                                )
                            }
                        }
                    }
                    if (ratio > 0f && prog?.completed != true) {
                        LinearProgressIndicator(
                            progress = { ratio },
                            color = palette.accent,
                            modifier = Modifier.fillMaxWidth().height(3.dp),
                        )
                    }
                }
            }
        }
    }

    // Source Picker Modal Bottom Sheet
    val epSheet = activeEpisodeForSources
    if (epSheet != null) {
        ModalBottomSheet(onDismissRequest = { activeEpisodeForSources = null }) {
            Column(
                modifier = Modifier
                    .fillMaxWidth()
                    .padding(horizontal = 16.dp, vertical = 12.dp),
                verticalArrangement = Arrangement.spacedBy(10.dp),
            ) {
                Text(VesselI18n.tr("details.select_stream"), style = MaterialTheme.typography.titleMedium, fontWeight = FontWeight.Bold)
                if (loadingStreams) {
                    Box(modifier = Modifier.fillMaxWidth().padding(24.dp), contentAlignment = Alignment.Center) {
                        CircularProgressIndicator()
                    }
                } else {
                    val sr = resolvedStreamResult
                    if (sr == null || sr.streams.isEmpty()) {
                        Text(VesselI18n.tr("details.no_streams"), color = palette.textMuted)
                    } else {
                        sr.streams.forEachIndexed { idx, candidate ->
                            Surface(
                                shape = RoundedCornerShape(10.dp),
                                color = palette.bgElevated,
                                modifier = Modifier
                                    .fillMaxWidth()
                                    .clip(RoundedCornerShape(10.dp))
                                    .clickable(enabled = !resolvingTorrent) {
                                        val savedPos = progressFor(epSheet)?.takeIf { !it.completed }?.positionMs ?: 0L
                                        scope.launch {
                                            resolveAndLaunchCandidate(d, epSheet, sr, idx, savedPos)
                                        }
                                    },
                            ) {
                                Row(
                                    modifier = Modifier.padding(12.dp),
                                    verticalAlignment = Alignment.CenterVertically,
                                    horizontalArrangement = Arrangement.SpaceBetween,
                                ) {
                                    Column(modifier = Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(4.dp)) {
                                        Text(candidate.label, style = MaterialTheme.typography.bodyMedium, fontWeight = FontWeight.SemiBold)
                                        Text(
                                            listOfNotNull(
                                                candidate.pluginName ?: candidate.pluginId,
                                                candidate.quality,
                                                candidate.kind.uppercase(Locale.US),
                                                candidate.audioType,
                                            ).joinToString(" · "),
                                            style = MaterialTheme.typography.labelSmall,
                                            color = palette.textMuted,
                                        )
                                        if (candidate.torrent != null || candidate.debridCache != null) {
                                            FlowRow(
                                                horizontalArrangement = Arrangement.spacedBy(6.dp),
                                                verticalArrangement = Arrangement.spacedBy(4.dp),
                                            ) {
                                                candidate.debridCache?.let { cache ->
                                                    val isCached = cache.state == "cached"
                                                    val isUncached = cache.state == "uncached"
                                                    Surface(
                                                        shape = RoundedCornerShape(6.dp),
                                                        color = when {
                                                            isCached -> Color(0x2822C55E)
                                                            isUncached -> Color(0x28F59E0B)
                                                            else -> palette.bgSurface
                                                        },
                                                    ) {
                                                        Text(
                                                            text = when {
                                                                isCached -> "⚡ ${cache.label}"
                                                                isUncached -> "☁️ ${cache.label}"
                                                                else -> cache.label
                                                            },
                                                            style = MaterialTheme.typography.labelSmall,
                                                            color = when {
                                                                isCached -> Color(0xFF4ADE80)
                                                                isUncached -> Color(0xFFFBBF24)
                                                                else -> palette.textMuted
                                                            },
                                                            modifier = Modifier.padding(horizontal = 6.dp, vertical = 2.dp),
                                                        )
                                                    }
                                                }
                                                candidate.torrent?.sizeBytes?.takeIf { it > 0L }?.let { bytes ->
                                                    Surface(shape = RoundedCornerShape(6.dp), color = palette.bgSurface) {
                                                        Text(
                                                            text = formatSizeBytes(bytes),
                                                            style = MaterialTheme.typography.labelSmall,
                                                            color = palette.textMuted,
                                                            modifier = Modifier.padding(horizontal = 6.dp, vertical = 2.dp),
                                                        )
                                                    }
                                                }
                                                candidate.torrent?.seeders?.let { seeders ->
                                                    Surface(shape = RoundedCornerShape(6.dp), color = palette.bgSurface) {
                                                        Text(
                                                            text = "👤 $seeders",
                                                            style = MaterialTheme.typography.labelSmall,
                                                            color = palette.textMuted,
                                                            modifier = Modifier.padding(horizontal = 6.dp, vertical = 2.dp),
                                                        )
                                                    }
                                                }
                                                candidate.torrent?.trackerSource?.takeIf { it.isNotBlank() }?.let { tr ->
                                                    Surface(shape = RoundedCornerShape(6.dp), color = palette.bgSurface) {
                                                        Text(
                                                            text = tr,
                                                            style = MaterialTheme.typography.labelSmall,
                                                            color = palette.textMuted,
                                                            modifier = Modifier.padding(horizontal = 6.dp, vertical = 2.dp),
                                                        )
                                                    }
                                                }
                                                candidate.torrent?.hdr?.takeIf { it.isNotBlank() }?.let { hdr ->
                                                    Surface(shape = RoundedCornerShape(6.dp), color = palette.accent.copy(alpha = 0.16f)) {
                                                        Text(
                                                            text = hdr,
                                                            style = MaterialTheme.typography.labelSmall,
                                                            color = palette.accent,
                                                            modifier = Modifier.padding(horizontal = 6.dp, vertical = 2.dp),
                                                        )
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    val isLiveMedia = isLiveMediaIdentifier(
                                        mediaType = d.type,
                                        pluginId = pluginId,
                                        streamId = epSheet.streamId,
                                        nativeId = nativeId,
                                        plugin = activePlugin,
                                    )
                                    if (!isLiveMedia) {
                                        VesselCompactIconButton(
                                            size = 34.dp,
                                            enabled = !resolvingTorrent,
                                            onClick = {
                                                scope.launch {
                                                    resolveAndDownloadCandidate(d, epSheet, sr, idx)
                                                }
                                            },
                                        ) {
                                            Icon(
                                                Icons.Default.Download,
                                                contentDescription = VesselI18n.tr("details.download_episode"),
                                                modifier = Modifier.size(18.dp),
                                            )
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                Spacer(Modifier.height(16.dp))
            }
        }
    }

    uncachedPrompt?.let { pending ->
        AlertDialog(
            onDismissRequest = { uncachedPrompt = null },
            title = { Text(VesselI18n.tr("debrid.uncached_title")) },
            text = {
                Text(
                    VesselI18n.trf(
                        "debrid.uncached_desc",
                        "provider" to (pending.resolution.providerLabel ?: "Debrid"),
                    ),
                )
            },
            confirmButton = {
                VesselButton(onClick = {
                    val act = pending
                    uncachedPrompt = null
                    scope.launch {
                        if (act.isDownload) {
                            resolveAndDownloadCandidate(act.detail, act.episode, act.streamResult, act.index, allowUncachedQueue = true)
                        } else {
                            resolveAndLaunchCandidate(act.detail, act.episode, act.streamResult, act.index, act.savedPosMs, allowUncachedQueue = true)
                        }
                    }
                }) {
                    Text(VesselI18n.tr("debrid.queue_cloud"))
                }
            },
            dismissButton = {
                VesselTextButton(onClick = { uncachedPrompt = null }) {
                    Text(VesselI18n.tr("common.cancel"))
                }
            },
        )
    }

    if (showNoDebridDialog) {
        AlertDialog(
            onDismissRequest = { showNoDebridDialog = false },
            title = { Text(VesselI18n.tr("debrid.no_provider_title")) },
            text = { Text(VesselI18n.tr("debrid.no_provider_desc")) },
            confirmButton = {
                VesselButton(onClick = {
                    showNoDebridDialog = false
                    activeEpisodeForSources = null
                    onOpenDebridSettings()
                }) {
                    Text(VesselI18n.tr("debrid.open_debrid_settings"))
                }
            },
            dismissButton = {
                VesselTextButton(onClick = { showNoDebridDialog = false }) {
                    Text(VesselI18n.tr("common.close"))
                }
            },
        )
    }

    if (showFixMatch) {
        val initialProvider = remember(showFixMatch, d.externalIds, d.type, d.isAnime) {
            when {
                d.externalIds.tmdb != null -> if (d.type == "movie") "tmdb_movie" else "tmdb_tv"
                d.externalIds.anilist != null -> "anilist"
                d.externalIds.mal != null -> "mal"
                d.externalIds.kitsu != null -> "kitsu"
                d.externalIds.simkl != null -> "simkl"
                d.externalIds.imdb != null -> "imdb"
                d.isAnime -> "anilist"
                d.type == "movie" -> "tmdb_movie"
                else -> "tmdb_tv"
            }
        }
        val initialId = remember(showFixMatch, d.externalIds) {
            d.externalIds.tmdb?.toString()
                ?: d.externalIds.anilist?.toString()
                ?: d.externalIds.mal?.toString()
                ?: d.externalIds.kitsu?.toString()
                ?: d.externalIds.simkl?.toString()
                ?: d.externalIds.imdb
                ?: ""
        }
        var fixProvider by remember(showFixMatch) { mutableStateOf(initialProvider) }
        var fixIdInput by remember(showFixMatch) { mutableStateOf(initialId) }
        var disableMeta by remember(showFixMatch) { mutableStateOf(false) }
        var searchQuery by remember(showFixMatch) { mutableStateOf(d.title) }
        var searchFilter by remember(showFixMatch) { mutableStateOf("all") }
        var searchingMeta by remember(showFixMatch) { mutableStateOf(false) }
        var hits by remember(showFixMatch) { mutableStateOf<List<MetadataSearchHit>>(emptyList()) }
        var notes by remember(showFixMatch) { mutableStateOf<List<String>>(emptyList()) }
        var showManual by remember(showFixMatch) { mutableStateOf(false) }

        fun runSearch() {
            val q = searchQuery.trim()
            if (q.isEmpty()) {
                hits = emptyList()
                notes = emptyList()
                return
            }
            searchingMeta = true
            scope.launch {
                runCatching { repo.searchMetadata(q, searchFilter) }
                    .onSuccess {
                        hits = it.hits
                        notes = it.notes
                    }
                    .onFailure { notes = listOf(it.message ?: "Search failed") }
                searchingMeta = false
            }
        }

        LaunchedEffect(showFixMatch) {
            if (showFixMatch && searchQuery.isNotBlank()) runSearch()
        }

        fun buildCanonicalId(): String? {
            val raw = fixIdInput.trim()
            if (raw.isEmpty()) return null
            if (':' in raw) return raw
            return when (fixProvider) {
                "anilist" -> "anilist:$raw"
                "mal" -> "mal:$raw"
                "kitsu" -> "kitsu:$raw"
                "simkl" -> "simkl:$raw"
                "tmdb_movie" -> "tmdb:movie:$raw"
                "imdb" -> "imdb:$raw"
                else -> "tmdb:tv:$raw"
            }
        }

        AlertDialog(
            onDismissRequest = { showFixMatch = false },
            title = { Text(VesselI18n.tr("details.fix_match"), style = MaterialTheme.typography.titleLarge) },
            text = {
                Column(
                    modifier = Modifier
                        .fillMaxWidth()
                        .heightIn(max = 460.dp)
                        .verticalScroll(rememberScrollState()),
                    verticalArrangement = Arrangement.spacedBy(8.dp),
                ) {
                    Text(VesselI18n.tr("details.fix_match_desc"), style = MaterialTheme.typography.bodySmall, color = palette.textMuted)

                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        horizontalArrangement = Arrangement.spacedBy(6.dp),
                        verticalAlignment = Alignment.CenterVertically,
                    ) {
                        VesselOutlinedTextField(
                            value = searchQuery,
                            onValueChange = { searchQuery = it },
                            placeholder = { Text(VesselI18n.tr("details.match_search_placeholder")) },
                            singleLine = true,
                            modifier = Modifier
                                .weight(1f)
                                .vesselDpadInputEscape(isTextEmpty = searchQuery.isEmpty(), onSubmit = { runSearch() }),
                        )
                        VesselCompactIconButton(
                            onClick = { runSearch() },
                            size = 40.dp,
                            containerColor = palette.accent,
                        ) {
                            Icon(Icons.Default.Search, contentDescription = VesselI18n.tr("common.search"), tint = Color(0xFF11111B), modifier = Modifier.size(18.dp))
                        }
                    }

                    Row(
                        modifier = Modifier.fillMaxWidth().horizontalScroll(rememberScrollState()),
                        horizontalArrangement = Arrangement.spacedBy(5.dp),
                    ) {
                        listOf(
                            "all" to VesselI18n.tr("common.all"),
                            "tmdb" to "TMDB",
                            "imdb" to "IMDb",
                            "simkl" to "Simkl",
                            "anilist" to "AniList",
                            "mal" to "MyAnimeList",
                            "kitsu" to "Kitsu",
                        ).forEach { (key, label) ->
                            VesselFilterChip(
                                selected = searchFilter == key,
                                onClick = {
                                    searchFilter = key
                                    runSearch()
                                },
                                label = label,
                            )
                        }
                    }

                    notes.forEach { note ->
                        Text(note, style = MaterialTheme.typography.labelSmall, color = palette.textMuted)
                    }

                    if (searchingMeta) {
                        Box(modifier = Modifier.fillMaxWidth().padding(vertical = 12.dp), contentAlignment = Alignment.Center) {
                            CircularProgressIndicator(modifier = Modifier.size(22.dp), strokeWidth = 2.dp)
                        }
                    } else if (hits.isNotEmpty()) {
                        Column(verticalArrangement = Arrangement.spacedBy(6.dp)) {
                            hits.forEach { hit ->
                                Surface(
                                    shape = RoundedCornerShape(10.dp),
                                    color = palette.bgElevated,
                                    modifier = Modifier
                                        .fillMaxWidth()
                                        .clip(RoundedCornerShape(10.dp))
                                        .clickable {
                                            showFixMatch = false
                                            scope.launch {
                                                runCatching {
                                                    repo.setMetadataOverride(pluginId, nativeId, hit.canonicalId, false)
                                                    loadAll()
                                                }.onSuccess {
                                                    repo.notify("${VesselI18n.tr("details.match_updated")}: ${hit.title}")
                                                }.onFailure {
                                                    repo.notifyError(it, "Failed to update match")
                                                }
                                            }
                                        },
                                ) {
                                    Row(
                                        modifier = Modifier.padding(8.dp),
                                        horizontalArrangement = Arrangement.spacedBy(8.dp),
                                        verticalAlignment = Alignment.CenterVertically,
                                    ) {
                                        if (!hit.poster.isNullOrBlank()) {
                                            AsyncImage(
                                                model = hit.poster,
                                                contentDescription = hit.title,
                                                contentScale = ContentScale.Crop,
                                                modifier = Modifier
                                                    .width(40.dp)
                                                    .height(58.dp)
                                                    .clip(RoundedCornerShape(6.dp))
                                                    .background(palette.bgSurface),
                                            )
                                        }
                                        Column(modifier = Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(2.dp)) {
                                            Text(hit.title, style = MaterialTheme.typography.bodySmall, fontWeight = FontWeight.SemiBold, maxLines = 1, overflow = TextOverflow.Ellipsis)
                                            Text(
                                                listOfNotNull(
                                                    hit.provider.uppercase(Locale.US),
                                                    hit.year?.toString(),
                                                    hit.mediaType,
                                                    hit.canonicalId,
                                                ).joinToString(" · "),
                                                style = MaterialTheme.typography.labelSmall,
                                                color = palette.textMuted,
                                                maxLines = 1,
                                                overflow = TextOverflow.Ellipsis,
                                            )
                                        }
                                    }
                                }
                            }
                        }
                    } else {
                        Text(VesselI18n.tr("details.match_no_hits"), style = MaterialTheme.typography.bodySmall, color = palette.textMuted)
                    }

                    VesselTextButton(onClick = { showManual = !showManual }) {
                        Text(VesselI18n.tr("details.match_manual_toggle"))
                    }

                    if (showManual) {
                        FlowRow(
                            horizontalArrangement = Arrangement.spacedBy(6.dp),
                            verticalArrangement = Arrangement.spacedBy(6.dp),
                        ) {
                            listOf(
                                "tmdb_tv" to "TMDB TV",
                                "tmdb_movie" to "TMDB Movie",
                                "imdb" to "IMDb",
                                "simkl" to "Simkl",
                                "anilist" to "AniList",
                                "mal" to "MyAnimeList",
                                "kitsu" to "Kitsu",
                            ).forEach { (key, label) ->
                                VesselFilterChip(
                                    selected = fixProvider == key,
                                    onClick = { fixProvider = key },
                                    label = label,
                                )
                            }
                        }
                        VesselOutlinedTextField(
                            value = fixIdInput,
                            onValueChange = { fixIdInput = it },
                            placeholder = { Text(VesselI18n.tr("details.match_id_placeholder")) },
                            enabled = !disableMeta,
                            singleLine = true,
                            modifier = Modifier
                                .fillMaxWidth()
                                .vesselDpadInputEscape(isTextEmpty = fixIdInput.isEmpty()),
                        )
                    }

                    Row(
                        verticalAlignment = Alignment.CenterVertically,
                        modifier = Modifier
                            .fillMaxWidth()
                            .clip(RoundedCornerShape(8.dp))
                            .clickable { disableMeta = !disableMeta }
                            .padding(vertical = 2.dp),
                    ) {
                        Checkbox(checked = disableMeta, onCheckedChange = { disableMeta = it })
                        Text(VesselI18n.tr("details.fix_match_disable"), style = MaterialTheme.typography.bodySmall)
                    }
                }
            },
            confirmButton = {
                if (showManual || disableMeta) {
                    VesselButton(onClick = {
                        showFixMatch = false
                        val canonical = if (disableMeta) null else buildCanonicalId()
                        scope.launch {
                            runCatching {
                                repo.setMetadataOverride(pluginId, nativeId, canonical, disableMeta)
                                loadAll()
                            }.onSuccess {
                                repo.notify(
                                    if (disableMeta) VesselI18n.tr("details.match_disabled")
                                    else VesselI18n.tr("details.match_updated"),
                                )
                            }.onFailure {
                                repo.notifyError(it, "Failed to update match")
                            }
                        }
                    }) {
                        Text(VesselI18n.tr("details.fix_match_apply"))
                    }
                }
            },
            dismissButton = {
                VesselTextButton(onClick = { showFixMatch = false }) { Text(VesselI18n.tr("common.cancel")) }
            },
        )
    }

    if (showIncludeSources) {
        val candidatePlugins = remember(plugins, pluginId) {
            plugins.filter { p ->
                p.enabled &&
                    p.manifest.id != pluginId &&
                    !isLiveTvPlugin(p)
            }
        }
        var targetQuery by remember(showIncludeSources) { mutableStateOf(d.title) }
        var activeSearchPid by remember(showIncludeSources) { mutableStateOf<String?>(null) }
        val pluginHits = remember(showIncludeSources) { mutableStateMapOf<String, List<MediaSummary>>() }
        val pluginBusy = remember(showIncludeSources) { mutableStateMapOf<String, Boolean>() }

        fun searchPlugin(pid: String, onAutoSelectFirst: Boolean = false, pluginObj: PluginListEntry? = null) {
            val q = targetQuery.trim()
            if (q.isEmpty()) return
            pluginBusy[pid] = true
            activeSearchPid = pid
            scope.launch {
                runCatching { repo.searchPluginSync(pid, q) }
                    .onSuccess { page ->
                        pluginHits[pid] = page.items
                        if (onAutoSelectFirst && pluginObj != null) {
                            val first = page.items.firstOrNull()
                            if (first != null) {
                                val next = linkedSources.filterNot { it.pluginId == pid } + LinkedMediaSource(
                                    pluginId = pid,
                                    pluginName = pluginObj.manifest.name,
                                    nativeId = first.nativeId,
                                    title = first.title,
                                    poster = first.poster,
                                )
                                linkedSources = repo.setLinkedSources(pluginId, nativeId, next)
                                repo.notify("${pluginObj.manifest.name}: ${first.title}")
                            } else {
                                repo.notify("${pluginObj.manifest.name}: ${VesselI18n.tr("common.empty")}")
                            }
                        }
                    }
                    .onFailure { repo.notifyError(it, "Search failed") }
                pluginBusy[pid] = false
            }
        }

        AlertDialog(
            onDismissRequest = { showIncludeSources = false },
            title = { Text(VesselI18n.tr("details.include_sources"), style = MaterialTheme.typography.titleLarge) },
            text = {
                Column(
                    modifier = Modifier
                        .fillMaxWidth()
                        .heightIn(max = 440.dp)
                        .verticalScroll(rememberScrollState()),
                    verticalArrangement = Arrangement.spacedBy(8.dp),
                ) {
                    Text(VesselI18n.tr("details.include_sources_desc"), style = MaterialTheme.typography.bodySmall, color = palette.textMuted)
                    VesselOutlinedTextField(
                        value = targetQuery,
                        onValueChange = { targetQuery = it },
                        placeholder = { Text(VesselI18n.tr("details.include_search_placeholder")) },
                        singleLine = true,
                        modifier = Modifier
                            .fillMaxWidth()
                            .vesselDpadInputEscape(isTextEmpty = targetQuery.isEmpty()),
                    )

                    if (candidatePlugins.isEmpty()) {
                        Text(VesselI18n.tr("common.empty"), color = palette.textMuted)
                    } else {
                        candidatePlugins.forEach { plug ->
                            val pid = plug.manifest.id
                            val linked = linkedSources.firstOrNull { it.pluginId == pid }
                            val isBusy = pluginBusy[pid] == true
                            Surface(shape = RoundedCornerShape(10.dp), color = palette.bgElevated) {
                                Column(modifier = Modifier.fillMaxWidth().padding(10.dp), verticalArrangement = Arrangement.spacedBy(6.dp)) {
                                    Row(
                                        modifier = Modifier.fillMaxWidth(),
                                        horizontalArrangement = Arrangement.SpaceBetween,
                                        verticalAlignment = Alignment.CenterVertically,
                                    ) {
                                        Column(modifier = Modifier.weight(1f)) {
                                            Text(plug.manifest.name, fontWeight = FontWeight.SemiBold, style = MaterialTheme.typography.bodySmall)
                                            if (linked != null) {
                                                Text(
                                                    "${VesselI18n.tr("details.linked_to")}: ${linked.title}",
                                                    style = MaterialTheme.typography.labelSmall,
                                                    color = palette.accent,
                                                    maxLines = 1,
                                                    overflow = TextOverflow.Ellipsis,
                                                )
                                            }
                                        }
                                        Row(horizontalArrangement = Arrangement.spacedBy(4.dp), verticalAlignment = Alignment.CenterVertically) {
                                            if (linked != null) {
                                                VesselCompactIconButton(size = 28.dp, onClick = { searchPlugin(pid) }) {
                                                    Icon(Icons.Default.Search, contentDescription = VesselI18n.tr("details.change_linked_match"), modifier = Modifier.size(15.dp))
                                                }
                                            }
                                            VesselCompactIconButton(
                                                size = 30.dp,
                                                containerColor = if (linked != null) palette.danger.copy(alpha = 0.16f) else palette.accent.copy(alpha = 0.18f),
                                                onClick = {
                                                    if (linked != null) {
                                                        scope.launch {
                                                            linkedSources = repo.setLinkedSources(
                                                                pluginId,
                                                                nativeId,
                                                                linkedSources.filterNot { it.pluginId == pid },
                                                            )
                                                        }
                                                    } else if (!isBusy) {
                                                        searchPlugin(pid, onAutoSelectFirst = true, pluginObj = plug)
                                                    }
                                                },
                                            ) {
                                                if (isBusy) {
                                                    CircularProgressIndicator(modifier = Modifier.size(14.dp), strokeWidth = 2.dp)
                                                } else if (linked != null) {
                                                    Icon(Icons.Default.Delete, contentDescription = VesselI18n.tr("common.remove"), tint = palette.danger, modifier = Modifier.size(15.dp))
                                                } else {
                                                    Icon(Icons.Default.Check, contentDescription = VesselI18n.tr("details.include_btn"), tint = palette.accent, modifier = Modifier.size(15.dp))
                                                }
                                            }
                                        }
                                    }

                                    val hitsList = pluginHits[pid].orEmpty()
                                    if (activeSearchPid == pid && hitsList.isNotEmpty()) {
                                        Row(
                                            modifier = Modifier.fillMaxWidth().horizontalScroll(rememberScrollState()),
                                            horizontalArrangement = Arrangement.spacedBy(6.dp),
                                        ) {
                                            hitsList.forEach { cand ->
                                                VesselFilterChip(
                                                    selected = linked?.nativeId == cand.nativeId,
                                                    onClick = {
                                                        scope.launch {
                                                            val next = linkedSources.filterNot { it.pluginId == pid } + LinkedMediaSource(
                                                                pluginId = pid,
                                                                pluginName = plug.manifest.name,
                                                                nativeId = cand.nativeId,
                                                                title = cand.title,
                                                                poster = cand.poster,
                                                            )
                                                            linkedSources = repo.setLinkedSources(pluginId, nativeId, next)
                                                            repo.notify("${plug.manifest.name}: ${cand.title}")
                                                        }
                                                    },
                                                    label = cand.title,
                                                )
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            },
            confirmButton = {
                VesselButton(onClick = { showIncludeSources = false }) {
                    Text(VesselI18n.tr("common.close"))
                }
            },
        )
    }
}
