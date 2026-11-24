package dev.falsis.vessel.ui.screens

import androidx.compose.animation.AnimatedContent
import androidx.compose.animation.core.tween
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.togetherWith
import androidx.compose.foundation.background
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
import androidx.compose.foundation.lazy.grid.GridCells
import androidx.compose.foundation.lazy.grid.GridItemSpan
import androidx.compose.foundation.lazy.grid.LazyVerticalGrid
import androidx.compose.foundation.lazy.grid.items
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Close
import androidx.compose.material.icons.filled.ExpandMore
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
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
import androidx.compose.ui.draw.clip
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import dev.falsis.vessel.data.Episode
import dev.falsis.vessel.data.LibraryItem
import dev.falsis.vessel.data.MediaDetail
import dev.falsis.vessel.data.MediaRef
import dev.falsis.vessel.data.MediaSummary
import dev.falsis.vessel.data.StreamResult
import dev.falsis.vessel.data.VesselRepository
import dev.falsis.vessel.ui.components.MediaPosterCard
import dev.falsis.vessel.ui.components.VesselFilterChip
import dev.falsis.vessel.ui.components.VesselOutlinedButton
import dev.falsis.vessel.ui.components.VesselSearchField
import dev.falsis.vessel.ui.components.VesselTextButton
import dev.falsis.vessel.ui.components.isLandscapeCardRatio
import dev.falsis.vessel.ui.components.vesselTopSafeInset
import dev.falsis.vessel.ui.i18n.VesselI18n
import dev.falsis.vessel.ui.isLiveMediaSummary
import dev.falsis.vessel.ui.isLiveTvPlugin
import dev.falsis.vessel.ui.theme.LocalVesselPalette
import kotlinx.coroutines.launch

@Composable
fun SearchScreen(
    repo: VesselRepository,
    onOpenDetails: (pluginId: String, nativeId: String) -> Unit,
    onPlayLiveStream: (pluginId: String, detail: MediaDetail, episode: Episode, streams: StreamResult) -> Unit = { _, _, _, _ -> },
) {
    val palette = LocalVesselPalette.current
    val scope = rememberCoroutineScope()
    val plugins by repo.plugins.collectAsState()
    val prefs by repo.uiPreferences.collectAsState()
    val enabledPlugins = remember(plugins) { plugins.filter { it.enabled } }
    val pluginMap = remember(plugins) { plugins.associateBy { it.manifest.id } }
    val pluginNameMap = remember(plugins) { plugins.associate { it.manifest.id to it.manifest.name } }
    var launchingLiveKey by remember { mutableStateOf<String?>(null) }

    var query by remember { mutableStateOf("") }
    var selectedType by remember { mutableStateOf("all") }
    var selectedPluginId by remember { mutableStateOf<String?>(null) }
    var showPluginMenu by remember { mutableStateOf(false) }
    var activeSearchId by remember { mutableStateOf<String?>(null) }
    var searching by remember { mutableStateOf(false) }
    var results by remember { mutableStateOf<List<Pair<String, MediaSummary>>>(emptyList()) }
    var libraryStatusMap by remember { mutableStateOf<Map<String, String>>(emptyMap()) }

    LaunchedEffect(Unit) {
        runCatching {
            libraryStatusMap = repo.listLibrary().associate {
                "${it.media.pluginId}:${it.media.nativeId}" to it.status
            }
        }
    }

    LaunchedEffect(Unit) {
        repo.searchEvents.collect { ev ->
            if (ev.searchId != activeSearchId) return@collect
            if (ev.items.isNotEmpty()) {
                val mapped = ev.items.map { item -> (item.pluginId ?: ev.pluginId) to item }
                results = (results + mapped).distinctBy { "${it.first}:${it.second.nativeId}" }
            }
            if (ev.done) searching = false
        }
    }

    fun triggerSearch() {
        val q = query.trim()
        if (q.isEmpty()) return
        results = emptyList()
        searching = true
        scope.launch {
            runCatching {
                repo.cancelSearch()
                activeSearchId = repo.searchStreams(
                    query = q,
                    pluginIds = selectedPluginId?.let { listOf(it) },
                )
            }.onFailure {
                searching = false
                repo.notifyError(it, "Search failed")
            }
        }
    }

    fun stopSearch() {
        searching = false
        scope.launch { repo.cancelSearch() }
    }

    val groupedBuckets = remember(results, selectedType, selectedPluginId) {
        val filtered = results.filter { (pid, item) ->
            val pluginMatch = selectedPluginId == null || pid == selectedPluginId
            val typeMatch = when (selectedType) {
                "movie" -> item.type == "movie"
                "series" -> item.type == "series" && !item.isAnime
                "anime" -> item.isAnime
                else -> true
            }
            pluginMatch && typeMatch
        }
        filtered.groupBy { it.first }.toList()
    }

    val isWideCard = isLandscapeCardRatio(prefs.cards.aspectRatio)

    Column(
        modifier = Modifier
            .fillMaxSize()
            .background(palette.bgBase)
            .vesselTopSafeInset()
            .padding(start = 16.dp, top = 14.dp, end = 16.dp, bottom = 12.dp),
        verticalArrangement = Arrangement.spacedBy(10.dp),
    ) {
        Row(
            modifier = Modifier.fillMaxWidth(),
            horizontalArrangement = Arrangement.SpaceBetween,
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Text(VesselI18n.tr("search.title"), style = MaterialTheme.typography.headlineSmall, fontWeight = FontWeight.Bold)
            if (searching) {
                Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(6.dp)) {
                    Surface(shape = RoundedCornerShape(999.dp), color = palette.accent.copy(alpha = 0.16f)) {
                        Row(
                            modifier = Modifier.padding(horizontal = 9.dp, vertical = 4.dp),
                            verticalAlignment = Alignment.CenterVertically,
                            horizontalArrangement = Arrangement.spacedBy(5.dp),
                        ) {
                            CircularProgressIndicator(modifier = Modifier.size(11.dp), strokeWidth = 2.dp, color = palette.accent)
                            Text(VesselI18n.tr("search.searching"), style = MaterialTheme.typography.labelSmall, color = palette.accent)
                        }
                    }
                    VesselTextButton(onClick = { stopSearch() }, height = 28.dp) {
                        Icon(Icons.Default.Close, contentDescription = null, modifier = Modifier.size(14.dp))
                        Spacer(Modifier.width(3.dp))
                        Text(VesselI18n.tr("common.cancel"))
                    }
                }
            }
        }

        VesselSearchField(
            value = query,
            onValueChange = { query = it },
            placeholder = VesselI18n.tr("search.placeholder"),
            imeAction = ImeAction.Search,
            onSearch = { triggerSearch() },
            onClear = {
                results = emptyList()
                stopSearch()
            },
            modifier = Modifier.fillMaxWidth(),
        )

        Row(
            modifier = Modifier
                .fillMaxWidth()
                .horizontalScroll(rememberScrollState()),
            horizontalArrangement = Arrangement.spacedBy(6.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            listOf(
                "all" to VesselI18n.tr("search.filter_all"),
                "movie" to VesselI18n.tr("search.filter_movies"),
                "series" to VesselI18n.tr("search.filter_series"),
                "anime" to VesselI18n.tr("search.filter_anime"),
            ).forEach { (key, label) ->
                VesselFilterChip(
                    selected = selectedType == key,
                    onClick = { selectedType = key },
                    label = label,
                )
            }

            Box {
                val pluginLabel = enabledPlugins.firstOrNull { it.manifest.id == selectedPluginId }?.manifest?.name
                    ?: VesselI18n.tr("search.all_plugins")
                VesselOutlinedButton(
                    onClick = { showPluginMenu = true },
                    height = 30.dp,
                    contentPadding = PaddingValues(horizontal = 10.dp, vertical = 0.dp),
                ) {
                    Text(
                        text = pluginLabel,
                        style = MaterialTheme.typography.labelMedium,
                        maxLines = 1,
                        overflow = TextOverflow.Ellipsis,
                    )
                    Spacer(Modifier.width(3.dp))
                    Icon(Icons.Default.ExpandMore, contentDescription = null, modifier = Modifier.size(15.dp))
                }
                DropdownMenu(expanded = showPluginMenu, onDismissRequest = { showPluginMenu = false }) {
                    DropdownMenuItem(
                        text = { Text(VesselI18n.tr("search.all_plugins")) },
                        onClick = {
                            selectedPluginId = null
                            showPluginMenu = false
                            if (query.isNotBlank()) triggerSearch()
                        },
                    )
                    enabledPlugins.forEach { p ->
                        DropdownMenuItem(
                            text = { Text(p.manifest.name) },
                            onClick = {
                                selectedPluginId = p.manifest.id
                                showPluginMenu = false
                                if (query.isNotBlank()) triggerSearch()
                            },
                        )
                    }
                }
            }
        }

        if (searching) {
            LinearProgressIndicator(
                modifier = Modifier
                    .fillMaxWidth()
                    .height(2.dp)
                    .clip(RoundedCornerShape(999.dp)),
                color = palette.accent,
            )
        }

        AnimatedContent(
            targetState = groupedBuckets.isEmpty() && !searching,
            transitionSpec = { fadeIn(tween(200)).togetherWith(fadeOut(tween(150))) },
            label = "search_results_transition",
            modifier = Modifier.fillMaxSize(),
        ) { showEmpty ->
            if (showEmpty) {
                Box(modifier = Modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
                    Text(VesselI18n.tr("search.empty"), color = palette.textMuted)
                }
            } else {
                LazyVerticalGrid(
                    columns = GridCells.Adaptive(minSize = if (isWideCard) 168.dp else 110.dp),
                    contentPadding = PaddingValues(top = 2.dp, bottom = 24.dp),
                    horizontalArrangement = Arrangement.spacedBy(10.dp),
                    verticalArrangement = Arrangement.spacedBy(14.dp),
                    modifier = Modifier.fillMaxSize(),
                ) {
                    groupedBuckets.forEach { (pluginId, bucketItems) ->
                        item(key = "hdr:$pluginId", span = { GridItemSpan(maxLineSpan) }) {
                            Row(
                                modifier = Modifier
                                    .fillMaxWidth()
                                    .padding(top = 4.dp),
                                verticalAlignment = Alignment.CenterVertically,
                                horizontalArrangement = Arrangement.spacedBy(8.dp),
                            ) {
                                Text(
                                    text = pluginNameMap[pluginId] ?: pluginId,
                                    style = MaterialTheme.typography.titleMedium,
                                    fontWeight = FontWeight.Bold,
                                )
                                Surface(color = palette.bgSurface, shape = RoundedCornerShape(999.dp)) {
                                    Text(
                                        text = "${bucketItems.size}",
                                        style = MaterialTheme.typography.labelSmall,
                                        color = palette.textMuted,
                                        modifier = Modifier.padding(horizontal = 8.dp, vertical = 2.dp),
                                    )
                                }
                            }
                        }

                        items(bucketItems, key = { "${it.first}:${it.second.nativeId}" }) { (pid, item) ->
                            val itemKey = "$pid:${item.nativeId}"
                            val plug = pluginMap[pid]
                            val isLiveItem = (plug != null && isLiveTvPlugin(plug)) || isLiveMediaSummary(item, plug)
                            MediaPosterCard(
                                item = if (isLiveItem) item.copy(type = "live") else item,
                                cards = prefs.cards,
                                currentStatus = if (isLiveItem) null else libraryStatusMap[itemKey],
                                fillGridWidth = true,
                                onClick = {
                                    if (!isLiveItem) {
                                        onOpenDetails(pid, item.nativeId)
                                    } else if (launchingLiveKey == null) {
                                        launchingLiveKey = itemKey
                                        scope.launch {
                                            runCatching {
                                                val detail = repo.getDetails(pid, item.nativeId)
                                                val ep = detail.episodes.firstOrNull() ?: Episode(
                                                    streamId = item.nativeId,
                                                    season = 1,
                                                    episode = 1,
                                                    title = item.title,
                                                    thumbnail = item.poster,
                                                )
                                                val sr = repo.getStreams(pid, ep.streamId)
                                                if (sr.streams.isEmpty()) {
                                                    repo.notify(VesselI18n.tr("details.no_streams"))
                                                } else {
                                                    onPlayLiveStream(
                                                        pid,
                                                        detail.copy(
                                                            type = "live",
                                                            title = detail.title.ifBlank { item.title },
                                                            poster = detail.poster ?: item.poster,
                                                            backdrop = detail.backdrop ?: item.poster,
                                                        ),
                                                        ep,
                                                        sr,
                                                    )
                                                }
                                            }.onFailure {
                                                repo.notifyError(it, "Failed to play ${item.title}")
                                            }
                                            launchingLiveKey = null
                                        }
                                    }
                                },
                                onQuickStatus = if (isLiveItem) null else { status ->
                                    scope.launch {
                                        repo.upsertLibrary(
                                            LibraryItem(
                                                media = MediaRef(pid, item.nativeId),
                                                mediaType = item.type,
                                                isAnime = item.isAnime,
                                                title = item.title,
                                                posterUrl = item.poster,
                                                status = status,
                                            ),
                                        )
                                        libraryStatusMap = libraryStatusMap + (itemKey to status)
                                        repo.notify("${item.title} · ${VesselI18n.tr("library.$status")}")
                                    }
                                },
                            )
                        }
                    }
                }
            }
        }
    }
}
