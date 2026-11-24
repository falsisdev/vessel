package dev.falsis.vessel.ui.screens

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.filled.CheckCircle
import androidx.compose.material.icons.filled.Download
import androidx.compose.material.icons.filled.Edit
import androidx.compose.material.icons.filled.PlayArrow
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.Checkbox
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.FilterChip
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.LinearProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
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
import dev.falsis.vessel.data.EnqueueDownloadPayload
import dev.falsis.vessel.data.Episode
import dev.falsis.vessel.data.LibraryItem
import dev.falsis.vessel.data.MediaDetail
import dev.falsis.vessel.data.MediaRef
import dev.falsis.vessel.data.PlaybackProgress
import dev.falsis.vessel.data.StreamCandidate
import dev.falsis.vessel.data.StreamResult
import dev.falsis.vessel.data.VesselRepository
import dev.falsis.vessel.ui.i18n.VesselI18n
import dev.falsis.vessel.ui.theme.LocalVesselPalette
import kotlinx.coroutines.launch

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun DetailsScreen(
    repo: VesselRepository,
    pluginId: String,
    nativeId: String,
    onBack: () -> Unit,
    onPlayStream: (detail: MediaDetail, episode: Episode, streams: StreamResult, selectedIdx: Int, resumePosMs: Long) -> Unit,
) {
    val palette = LocalVesselPalette.current
    val scope = rememberCoroutineScope()

    var loading by remember { mutableStateOf(true) }
    var detail by remember { mutableStateOf<MediaDetail?>(null) }
    var progressList by remember { mutableStateOf<List<PlaybackProgress>>(emptyList()) }
    var selectedSeason by remember { mutableIntStateOf(1) }
    var showFixMatch by remember { mutableStateOf(false) }
    var showStatusMenu by remember { mutableStateOf(false) }

    var activeEpisodeForSources by remember { mutableStateOf<Episode?>(null) }
    var loadingStreams by remember { mutableStateOf(false) }
    var resolvedStreamResult by remember { mutableStateOf<StreamResult?>(null) }

    suspend fun loadAll() {
        loading = true
        runCatching {
            val loaded = repo.getDetails(pluginId, nativeId)
            detail = loaded
            val seasons = loaded.episodes.map { it.season }.distinct().sorted()
            if (seasons.isNotEmpty() && selectedSeason !in seasons) {
                selectedSeason = seasons.first()
            }
            progressList = repo.listMediaWatchProgress(pluginId, nativeId)
        }.onFailure {
            repo.notify(it.message ?: "Failed to load details")
        }
        loading = false
    }

    LaunchedEffect(pluginId, nativeId) { loadAll() }

    fun progressFor(ep: Episode): PlaybackProgress? =
        progressList.firstOrNull { it.season == ep.season && it.episode == ep.episode }

    fun openEpisodeSources(ep: Episode) {
        activeEpisodeForSources = ep
        resolvedStreamResult = null
        loadingStreams = true
        scope.launch {
            runCatching { repo.getStreams(pluginId, ep.streamId) }
                .onSuccess { resolvedStreamResult = it }
                .onFailure { repo.notify(it.message ?: "Failed to resolve streams") }
            loadingStreams = false
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
            Text(VesselI18n.tr("common.empty"))
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
    } ?: d.episodes.firstOrNull()

    LazyColumn(
        modifier = Modifier
            .fillMaxSize()
            .background(palette.bgBase),
        contentPadding = PaddingValues(bottom = 32.dp),
    ) {
        // Hero Backdrop Header
        item {
            Box(
                modifier = Modifier
                    .fillMaxWidth()
                    .height(280.dp)
                    .background(palette.bgSurface),
            ) {
                AsyncImage(
                    model = d.backdrop ?: d.poster,
                    contentDescription = d.title,
                    contentScale = ContentScale.Crop,
                    modifier = Modifier.fillMaxSize(),
                )
                Box(
                    modifier = Modifier
                        .fillMaxSize()
                        .background(
                            Brush.verticalGradient(
                                listOf(Color.Black.copy(alpha = 0.35f), palette.bgBase),
                            ),
                        ),
                )
                IconButton(
                    onClick = onBack,
                    modifier = Modifier.padding(12.dp),
                ) {
                    Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = "Back", tint = Color.White)
                }

                Column(
                    modifier = Modifier
                        .align(Alignment.BottomStart)
                        .padding(16.dp),
                    verticalArrangement = Arrangement.spacedBy(8.dp),
                ) {
                    Text(d.title, style = MaterialTheme.typography.headlineSmall, fontWeight = FontWeight.Bold)
                    Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                        d.year?.let { Text("$it", color = palette.textMuted, style = MaterialTheme.typography.labelMedium) }
                        if (d.genres.isNotEmpty()) {
                            Text(
                                d.genres.take(3).joinToString(" · "),
                                color = palette.textMuted,
                                style = MaterialTheme.typography.labelMedium,
                                maxLines = 1,
                                overflow = TextOverflow.Ellipsis,
                            )
                        }
                    }
                    Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                        if (resumeCandidateEp != null) {
                            val saved = progressFor(resumeCandidateEp)
                            Button(onClick = { openEpisodeSources(resumeCandidateEp) }) {
                                Icon(Icons.Default.PlayArrow, contentDescription = null)
                                Spacer(Modifier.width(4.dp))
                                Text(
                                    if (saved != null && saved.positionMs > 0 && !saved.completed) {
                                        VesselI18n.tr("details.resume")
                                    } else {
                                        VesselI18n.tr("details.watch")
                                    },
                                )
                            }
                        }
                        Box {
                            OutlinedButton(onClick = { showStatusMenu = true }) {
                                Text(VesselI18n.tr("details.library_status"))
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
                                                        status = status,
                                                    ),
                                                )
                                                repo.notify("${d.title} → $status")
                                            }
                                        },
                                    )
                                }
                            }
                        }
                        OutlinedButton(onClick = { showFixMatch = true }) {
                            Icon(Icons.Default.Edit, contentDescription = null, modifier = Modifier.size(16.dp))
                        }
                    }
                }
            }
        }

        // Synopsis
        d.synopsis?.takeIf { it.isNotBlank() }?.let { syn ->
            item {
                Text(
                    text = syn,
                    style = MaterialTheme.typography.bodyMedium,
                    color = palette.textMuted,
                    maxLines = 4,
                    overflow = TextOverflow.Ellipsis,
                    modifier = Modifier.padding(horizontal = 16.dp, vertical = 8.dp),
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
                    horizontalArrangement = Arrangement.spacedBy(8.dp),
                ) {
                    seasons.forEach { s ->
                        FilterChip(
                            selected = selectedSeason == s,
                            onClick = { selectedSeason = s },
                            label = { Text("${VesselI18n.tr("details.season")} $s") },
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

            Surface(
                shape = RoundedCornerShape(12.dp),
                color = palette.bgSurface,
                modifier = Modifier
                    .fillMaxWidth()
                    .padding(horizontal = 16.dp, vertical = 5.dp)
                    .clip(RoundedCornerShape(12.dp))
                    .clickable { openEpisodeSources(ep) },
            ) {
                Column(modifier = Modifier.padding(12.dp), verticalArrangement = Arrangement.spacedBy(6.dp)) {
                    Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(12.dp)) {
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
                        Column(modifier = Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(2.dp)) {
                            Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(6.dp)) {
                                Text(
                                    text = "${ep.episode}. $epLabel",
                                    fontWeight = FontWeight.SemiBold,
                                    maxLines = 1,
                                    overflow = TextOverflow.Ellipsis,
                                    modifier = Modifier.weight(1f, fill = false),
                                )
                                if (prog?.completed == true) {
                                    Icon(
                                        Icons.Default.CheckCircle,
                                        contentDescription = VesselI18n.tr("details.watched_badge"),
                                        tint = palette.success,
                                        modifier = Modifier.size(16.dp),
                                    )
                                }
                            }
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
                                    .clickable {
                                        val savedPos = progressFor(epSheet)?.takeIf { !it.completed }?.positionMs ?: 0L
                                        activeEpisodeForSources = null
                                        onPlayStream(d, epSheet, sr, idx, savedPos)
                                    },
                            ) {
                                Row(
                                    modifier = Modifier.padding(12.dp),
                                    verticalAlignment = Alignment.CenterVertically,
                                    horizontalArrangement = Arrangement.SpaceBetween,
                                ) {
                                    Column(modifier = Modifier.weight(1f)) {
                                        Text(candidate.label, fontWeight = FontWeight.SemiBold)
                                        Text(
                                            listOfNotNull(candidate.quality, candidate.kind.uppercase(), candidate.audioType).joinToString(" · "),
                                            style = MaterialTheme.typography.labelSmall,
                                            color = palette.textMuted,
                                        )
                                    }
                                    IconButton(onClick = {
                                        scope.launch {
                                            runCatching {
                                                repo.enqueueDownload(
                                                    EnqueueDownloadPayload(
                                                        pluginId = pluginId,
                                                        nativeId = nativeId,
                                                        title = d.title,
                                                        season = if (d.type == "movie") null else epSheet.season,
                                                        episode = if (d.type == "movie") null else epSheet.episode,
                                                        streamLabel = candidate.label,
                                                        kind = if (candidate.kind == "hls" || candidate.url.contains(".m3u8")) "hls" else "direct",
                                                        sourceUrl = candidate.url,
                                                        headers = candidate.headers,
                                                        subtitles = candidate.subtitles + sr.subtitles,
                                                    ),
                                                )
                                            }.onSuccess {
                                                repo.notify("${VesselI18n.tr("details.download_episode")}: ${candidate.label}")
                                            }.onFailure {
                                                repo.notify(it.message ?: "Download failed")
                                            }
                                        }
                                    }) {
                                        Icon(Icons.Default.Download, contentDescription = VesselI18n.tr("details.download_episode"))
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

    if (showFixMatch) {
        var canonicalInput by remember { mutableStateOf("") }
        var disableMeta by remember { mutableStateOf(false) }
        AlertDialog(
            onDismissRequest = { showFixMatch = false },
            title = { Text(VesselI18n.tr("details.fix_match")) },
            text = {
                Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                    Text(VesselI18n.tr("details.fix_match_desc"), style = MaterialTheme.typography.bodySmall)
                    OutlinedTextField(
                        value = canonicalInput,
                        onValueChange = { canonicalInput = it },
                        placeholder = { Text(VesselI18n.tr("details.canonical_hint")) },
                        singleLine = true,
                    )
                    Row(verticalAlignment = Alignment.CenterVertically) {
                        Checkbox(checked = disableMeta, onCheckedChange = { disableMeta = it })
                        Text(VesselI18n.tr("details.fix_match_disable"))
                    }
                }
            },
            confirmButton = {
                Button(onClick = {
                    showFixMatch = false
                    scope.launch {
                        repo.setMetadataOverride(pluginId, nativeId, canonicalInput.ifBlank { null }, disableMeta)
                        loadAll()
                    }
                }) {
                    Text(VesselI18n.tr("details.fix_match_apply"))
                }
            },
            dismissButton = {
                TextButton(onClick = { showFixMatch = false }) { Text(VesselI18n.tr("common.cancel")) }
            },
        )
    }
}
