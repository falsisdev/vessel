package dev.falsis.vessel.ui.screens

import androidx.compose.animation.animateContentSize
import androidx.compose.animation.core.tween
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
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
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.filled.ArrowDownward
import androidx.compose.material.icons.filled.ArrowUpward
import androidx.compose.material.icons.filled.CheckCircle
import androidx.compose.material.icons.filled.Delete
import androidx.compose.material.icons.filled.ExpandLess
import androidx.compose.material.icons.filled.ExpandMore
import androidx.compose.material.icons.filled.Pause
import androidx.compose.material.icons.filled.PlayArrow
import androidx.compose.material.icons.filled.Refresh
import androidx.compose.material3.Icon
import androidx.compose.material3.LinearProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.focus.FocusRequester
import androidx.compose.ui.focus.focusRequester
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import dev.falsis.vessel.data.LocalPlaybackBundle
import dev.falsis.vessel.data.StoredDownload
import dev.falsis.vessel.data.VesselRepository
import dev.falsis.vessel.ui.components.VesselCompactIconButton
import dev.falsis.vessel.ui.components.vesselTopSafeInset
import dev.falsis.vessel.ui.i18n.VesselI18n
import dev.falsis.vessel.ui.theme.LocalVesselPalette
import kotlinx.coroutines.launch

private fun formatBytes(bytes: Long): String {
    if (bytes <= 0L) return "0 B"
    val units = arrayOf("B", "KB", "MB", "GB")
    var value = bytes.toDouble()
    var unitIdx = 0
    while (value >= 1024.0 && unitIdx < units.lastIndex) {
        value /= 1024.0
        unitIdx++
    }
    return if (unitIdx == 0) "$bytes B" else String.format("%.1f %s", value, units[unitIdx])
}

@Composable
fun DownloadsScreen(
    repo: VesselRepository,
    onPlayOffline: (dl: StoredDownload, bundle: LocalPlaybackBundle) -> Unit,
    onBack: (() -> Unit)? = null,
) {
    val palette = LocalVesselPalette.current
    val scope = rememberCoroutineScope()
    val liveProgress by repo.downloadProgress.collectAsState()
    var downloads by remember { mutableStateOf<List<StoredDownload>>(emptyList()) }
    var expandedContents by remember { mutableStateOf<Set<String>>(emptySet()) }
    var expandedSeasons by remember { mutableStateOf<Set<String>>(emptySet()) }
    var expandedEpisodes by remember { mutableStateOf<Set<String>>(emptySet()) }

    suspend fun reload() {
        runCatching { downloads = repo.listDownloads() }
    }

    LaunchedEffect(Unit) { reload() }

    val merged = remember(downloads, liveProgress) {
        downloads.map { dl ->
            val snap = liveProgress[dl.id]
            if (snap == null) dl else dl.copy(
                status = snap.status,
                bytesDownloaded = snap.bytesDownloaded,
                totalBytes = snap.totalBytes ?: dl.totalBytes,
                segmentsDone = snap.segmentsDone,
                segmentsTotal = snap.segmentsTotal,
            )
        }
    }

    val queue = remember(merged) {
        merged.filter { it.status in setOf("queued", "downloading", "paused", "failed") }
    }
    val queueFocusRequesters = remember { mutableMapOf<String, Pair<FocusRequester, FocusRequester>>() }
    var pendingQueueFocus by remember { mutableStateOf<Pair<String, Boolean>?>(null) }

    LaunchedEffect(pendingQueueFocus, queue) {
        val target = pendingQueueFocus ?: return@LaunchedEffect
        kotlinx.coroutines.delay(16)
        val pair = queueFocusRequesters[target.first]
        if (pair != null) {
            runCatching { (if (target.second) pair.first else pair.second).requestFocus() }
        }
        pendingQueueFocus = null
    }

    val groupedByContent = remember(merged) {
        merged.groupBy { "${it.pluginId}:${it.nativeId}" }
    }

    LazyColumn(
        modifier = Modifier
            .fillMaxSize()
            .background(palette.bgBase)
            .vesselTopSafeInset(),
        contentPadding = PaddingValues(start = 16.dp, top = 14.dp, end = 16.dp, bottom = 24.dp),
        verticalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        item {
            Row(
                modifier = Modifier.fillMaxWidth(),
                verticalAlignment = Alignment.CenterVertically,
                horizontalArrangement = Arrangement.SpaceBetween,
            ) {
                Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                    if (onBack != null) {
                        VesselCompactIconButton(onClick = onBack, size = 34.dp, containerColor = palette.bgSurface) {
                            Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = VesselI18n.tr("common.back"), modifier = Modifier.size(18.dp))
                        }
                    }
                    Text(VesselI18n.tr("downloads.title"), style = MaterialTheme.typography.headlineSmall, fontWeight = FontWeight.Bold)
                }
                VesselCompactIconButton(onClick = { scope.launch { reload() } }, size = 34.dp, containerColor = palette.bgSurface) {
                    Icon(Icons.Default.Refresh, contentDescription = VesselI18n.tr("common.refresh"), modifier = Modifier.size(18.dp))
                }
            }
        }

        // Active & Queued Downloads with priority reordering
        if (queue.isNotEmpty()) {
            item {
                Text(VesselI18n.tr("downloads.queue_pos"), style = MaterialTheme.typography.titleMedium, fontWeight = FontWeight.Bold)
            }
            items(queue, key = { "q:${it.id}" }) { dl ->
                val pct = if (dl.segmentsTotal > 0) {
                    dl.segmentsDone.toFloat() / dl.segmentsTotal
                } else if ((dl.totalBytes ?: 0L) > 0L) {
                    dl.bytesDownloaded.toFloat() / dl.totalBytes!!
                } else 0f

                Surface(shape = RoundedCornerShape(12.dp), color = palette.bgSurface) {
                    Column(modifier = Modifier.fillMaxWidth().padding(12.dp), verticalArrangement = Arrangement.spacedBy(6.dp)) {
                        Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.SpaceBetween, modifier = Modifier.fillMaxWidth()) {
                            Column(modifier = Modifier.weight(1f)) {
                                Text(dl.title, fontWeight = FontWeight.SemiBold)
                                val sizeInfo = if ((dl.totalBytes ?: 0L) > 0L) {
                                    "${formatBytes(dl.bytesDownloaded)} / ${formatBytes(dl.totalBytes!!)}"
                                } else if (dl.segmentsTotal > 0) {
                                    "${dl.segmentsDone}/${dl.segmentsTotal}"
                                } else if (dl.bytesDownloaded > 0L) {
                                    formatBytes(dl.bytesDownloaded)
                                } else null
                                Text(
                                    listOfNotNull(dl.streamLabel, dl.status.uppercase(), sizeInfo).joinToString(" · "),
                                    style = MaterialTheme.typography.labelSmall,
                                    color = palette.textMuted,
                                )
                            }
                            Row(horizontalArrangement = Arrangement.spacedBy(4.dp), verticalAlignment = Alignment.CenterVertically) {
                                val idx = queue.indexOfFirst { it.id == dl.id }
                                val (upReq, downReq) = queueFocusRequesters.getOrPut(dl.id) { FocusRequester() to FocusRequester() }
                                if (idx > 0) {
                                    VesselCompactIconButton(
                                        size = 28.dp,
                                        modifier = Modifier.focusRequester(upReq),
                                        onClick = {
                                            val ids = queue.map { it.id }.toMutableList()
                                            val newIdx = idx - 1
                                            ids.add(newIdx, ids.removeAt(idx))
                                            pendingQueueFocus = dl.id to (newIdx > 0)
                                            scope.launch { repo.reorderDownloads(ids); reload() }
                                        },
                                    ) {
                                        Icon(Icons.Default.ArrowUpward, contentDescription = "Move Up", modifier = Modifier.size(16.dp))
                                    }
                                }
                                if (idx >= 0 && idx < queue.lastIndex) {
                                    VesselCompactIconButton(
                                        size = 28.dp,
                                        modifier = Modifier.focusRequester(downReq),
                                        onClick = {
                                            val ids = queue.map { it.id }.toMutableList()
                                            val newIdx = idx + 1
                                            ids.add(newIdx, ids.removeAt(idx))
                                            pendingQueueFocus = dl.id to (newIdx >= ids.lastIndex)
                                            scope.launch { repo.reorderDownloads(ids); reload() }
                                        },
                                    ) {
                                        Icon(Icons.Default.ArrowDownward, contentDescription = "Move Down", modifier = Modifier.size(16.dp))
                                    }
                                }
                                if (dl.status == "downloading" || dl.status == "queued") {
                                    VesselCompactIconButton(size = 28.dp, onClick = { scope.launch { repo.pauseDownload(dl.id); reload() } }) {
                                        Icon(Icons.Default.Pause, contentDescription = VesselI18n.tr("common.pause"), modifier = Modifier.size(16.dp))
                                    }
                                } else {
                                    VesselCompactIconButton(size = 28.dp, onClick = { scope.launch { repo.resumeDownload(dl.id); reload() } }) {
                                        Icon(Icons.Default.PlayArrow, contentDescription = VesselI18n.tr("common.resume"), modifier = Modifier.size(16.dp))
                                    }
                                }
                                VesselCompactIconButton(size = 28.dp, onClick = { scope.launch { repo.deleteDownload(dl.id); reload() } }) {
                                    Icon(Icons.Default.Delete, contentDescription = VesselI18n.tr("common.delete"), tint = palette.danger, modifier = Modifier.size(16.dp))
                                }
                            }
                        }
                        LinearProgressIndicator(progress = { pct.coerceIn(0f, 1f) }, modifier = Modifier.fillMaxWidth())
                    }
                }
            }
        }

        if (groupedByContent.isEmpty()) {
            item {
                Box(modifier = Modifier.fillMaxWidth().padding(32.dp), contentAlignment = Alignment.Center) {
                    Text(VesselI18n.tr("downloads.empty"), color = palette.textMuted)
                }
            }
        } else {
            // Level 1: Content -> Level 2: Season -> Level 3: Episode -> Level 4: Stream
            groupedByContent.forEach { (cKey, items) ->
                item(key = "c:$cKey") {
                    val sample = items.first()
                    val isExpanded = cKey in expandedContents
                    val cleanTitle = sample.title.replace(Regex("\\s+—\\s+S\\d+E\\d+$", RegexOption.IGNORE_CASE), "")
                    val isMovie = items.all { it.season == null && it.episode == null }

                    Surface(shape = RoundedCornerShape(14.dp), color = palette.bgSurface) {
                        Column(
                            modifier = Modifier
                                .fillMaxWidth()
                                .animateContentSize(tween(220))
                                .padding(12.dp),
                            verticalArrangement = Arrangement.spacedBy(8.dp),
                        ) {
                            Row(
                                modifier = Modifier
                                    .fillMaxWidth()
                                    .clickable {
                                        expandedContents = if (isExpanded) expandedContents - cKey else expandedContents + cKey
                                    },
                                verticalAlignment = Alignment.CenterVertically,
                                horizontalArrangement = Arrangement.SpaceBetween,
                            ) {
                                Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(8.dp), modifier = Modifier.weight(1f)) {
                                    Icon(if (isExpanded) Icons.Default.ExpandLess else Icons.Default.ExpandMore, contentDescription = null)
                                    Column {
                                        Text(cleanTitle, fontWeight = FontWeight.Bold)
                                        Text("${items.size} ${VesselI18n.tr("downloads.streams_count")}", style = MaterialTheme.typography.labelSmall, color = palette.textMuted)
                                    }
                                }
                            }

                            if (isExpanded) {
                                if (isMovie) {
                                    items.forEach { streamItem ->
                                        OfflineStreamRow(streamItem, onPlay = {
                                            scope.launch {
                                                runCatching { repo.prepareLocalDownloadPlayback(streamItem.id) }
                                                    .onSuccess { onPlayOffline(streamItem, it) }
                                            }
                                        }, onDelete = {
                                            scope.launch { repo.deleteDownload(streamItem.id); reload() }
                                        })
                                    }
                                } else {
                                    val bySeason = items.groupBy { it.season ?: 1 }.toSortedMap()
                                    bySeason.forEach { (seasonNum, seasonItems) ->
                                        val sKey = "$cKey:s:$seasonNum"
                                        val sOpen = sKey in expandedSeasons
                                        Surface(shape = RoundedCornerShape(10.dp), color = palette.bgElevated) {
                                            Column(modifier = Modifier.fillMaxWidth().padding(10.dp), verticalArrangement = Arrangement.spacedBy(6.dp)) {
                                                Row(
                                                    modifier = Modifier
                                                        .fillMaxWidth()
                                                        .clickable {
                                                            expandedSeasons = if (sOpen) expandedSeasons - sKey else expandedSeasons + sKey
                                                        },
                                                    verticalAlignment = Alignment.CenterVertically,
                                                ) {
                                                    Icon(if (sOpen) Icons.Default.ExpandLess else Icons.Default.ExpandMore, contentDescription = null, modifier = Modifier.size(18.dp))
                                                    Spacer(Modifier.width(6.dp))
                                                    Text("${VesselI18n.tr("details.season")} $seasonNum", fontWeight = FontWeight.SemiBold)
                                                }

                                                if (sOpen) {
                                                    val byEp = seasonItems.groupBy { it.episode ?: 1 }.toSortedMap()
                                                    byEp.forEach { (epNum, epStreams) ->
                                                        val eKey = "$sKey:e:$epNum"
                                                        val eOpen = eKey in expandedEpisodes
                                                        Column(modifier = Modifier.padding(start = 12.dp), verticalArrangement = Arrangement.spacedBy(4.dp)) {
                                                            Row(
                                                                modifier = Modifier
                                                                    .fillMaxWidth()
                                                                    .clickable {
                                                                        expandedEpisodes = if (eOpen) expandedEpisodes - eKey else expandedEpisodes + eKey
                                                                    }
                                                                    .padding(vertical = 4.dp),
                                                                verticalAlignment = Alignment.CenterVertically,
                                                            ) {
                                                                Icon(if (eOpen) Icons.Default.ExpandLess else Icons.Default.ExpandMore, contentDescription = null, modifier = Modifier.size(16.dp))
                                                                Spacer(Modifier.width(6.dp))
                                                                Text("${VesselI18n.tr("downloads.episode_label")} $epNum", fontWeight = FontWeight.Medium)
                                                            }

                                                            if (eOpen) {
                                                                epStreams.forEach { st ->
                                                                    OfflineStreamRow(st, onPlay = {
                                                                        scope.launch {
                                                                            runCatching { repo.prepareLocalDownloadPlayback(st.id) }
                                                                                .onSuccess { onPlayOffline(st, it) }
                                                                        }
                                                                    }, onDelete = {
                                                                        scope.launch { repo.deleteDownload(st.id); reload() }
                                                                    })
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

@Composable
private fun OfflineStreamRow(
    dl: StoredDownload,
    onPlay: () -> Unit,
    onDelete: () -> Unit,
) {
    val palette = LocalVesselPalette.current
    Surface(
        shape = RoundedCornerShape(8.dp),
        color = palette.bgBase,
        modifier = Modifier
            .fillMaxWidth()
            .padding(start = 8.dp)
            .clickable(enabled = dl.status == "completed", onClick = onPlay),
    ) {
        Row(
            modifier = Modifier.padding(horizontal = 10.dp, vertical = 8.dp),
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.SpaceBetween,
        ) {
            Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(8.dp), modifier = Modifier.weight(1f)) {
                if (dl.status == "completed") {
                    Icon(Icons.Default.CheckCircle, contentDescription = null, tint = palette.success, modifier = Modifier.size(16.dp))
                }
                Text(dl.streamLabel ?: VesselI18n.tr("downloads.stream_default"), fontWeight = FontWeight.Medium)
            }
            VesselCompactIconButton(onClick = onDelete, size = 28.dp) {
                Icon(Icons.Default.Delete, contentDescription = VesselI18n.tr("common.delete"), tint = palette.danger, modifier = Modifier.size(16.dp))
            }
        }
    }
}
