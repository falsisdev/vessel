package dev.falsis.vessel.ui.screens

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
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.grid.GridCells
import androidx.compose.foundation.lazy.grid.GridItemSpan
import androidx.compose.foundation.lazy.grid.LazyVerticalGrid
import androidx.compose.foundation.lazy.grid.items
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Clear
import androidx.compose.material.icons.filled.Close
import androidx.compose.material.icons.filled.Search
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.FilterChip
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.LinearProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
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
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import dev.falsis.vessel.data.LibraryItem
import dev.falsis.vessel.data.MediaRef
import dev.falsis.vessel.data.MediaSummary
import dev.falsis.vessel.data.VesselRepository
import dev.falsis.vessel.ui.components.MediaPosterCard
import dev.falsis.vessel.ui.i18n.VesselI18n
import dev.falsis.vessel.ui.theme.LocalVesselPalette
import kotlinx.coroutines.launch

@Composable
fun SearchScreen(
    repo: VesselRepository,
    onOpenDetails: (pluginId: String, nativeId: String) -> Unit,
) {
    val palette = LocalVesselPalette.current
    val scope = rememberCoroutineScope()
    val plugins by repo.plugins.collectAsState()
    val prefs by repo.uiPreferences.collectAsState()
    val enabledPlugins = remember(plugins) { plugins.filter { it.enabled } }
    val pluginNameMap = remember(plugins) { plugins.associate { it.manifest.id to it.manifest.name } }

    var query by remember { mutableStateOf("") }
    var selectedType by remember { mutableStateOf("all") }
    var selectedPluginId by remember { mutableStateOf<String?>(null) }
    var showPluginMenu by remember { mutableStateOf(false) }
    var activeSearchId by remember { mutableStateOf<String?>(null) }
    var searching by remember { mutableStateOf(false) }
    var results by remember { mutableStateOf<List<Pair<String, MediaSummary>>>(emptyList()) }

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

    Column(
        modifier = Modifier
            .fillMaxSize()
            .background(palette.bgBase)
            .padding(16.dp),
        verticalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        Row(
            modifier = Modifier.fillMaxWidth(),
            horizontalArrangement = Arrangement.SpaceBetween,
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Text(VesselI18n.tr("search.title"), style = MaterialTheme.typography.headlineSmall, fontWeight = FontWeight.Bold)
            if (searching) {
                Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(6.dp)) {
                    Surface(shape = RoundedCornerShape(50), color = palette.accent.copy(alpha = 0.16f)) {
                        Row(
                            modifier = Modifier.padding(horizontal = 10.dp, vertical = 4.dp),
                            verticalAlignment = Alignment.CenterVertically,
                            horizontalArrangement = Arrangement.spacedBy(6.dp),
                        ) {
                            CircularProgressIndicator(modifier = Modifier.size(12.dp), strokeWidth = 2.dp, color = palette.accent)
                            Text(VesselI18n.tr("search.searching"), style = MaterialTheme.typography.labelSmall, color = palette.accent)
                        }
                    }
                    TextButton(onClick = { stopSearch() }) {
                        Icon(Icons.Default.Close, contentDescription = null, modifier = Modifier.size(15.dp))
                        Spacer(Modifier.width(2.dp))
                        Text(VesselI18n.tr("common.cancel"))
                    }
                }
            }
        }

        OutlinedTextField(
            value = query,
            onValueChange = { query = it },
            placeholder = { Text(VesselI18n.tr("search.placeholder")) },
            leadingIcon = { Icon(Icons.Default.Search, contentDescription = null) },
            trailingIcon = {
                if (query.isNotEmpty()) {
                    IconButton(onClick = {
                        query = ""
                        results = emptyList()
                        stopSearch()
                    }) {
                        Icon(Icons.Default.Clear, contentDescription = null)
                    }
                }
            },
            singleLine = true,
            keyboardOptions = KeyboardOptions(imeAction = ImeAction.Search),
            keyboardActions = KeyboardActions(onSearch = { triggerSearch() }),
            modifier = Modifier.fillMaxWidth(),
        )

        Row(
            modifier = Modifier
                .fillMaxWidth()
                .horizontalScroll(rememberScrollState()),
            horizontalArrangement = Arrangement.spacedBy(8.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            listOf(
                "all" to VesselI18n.tr("search.filter_all"),
                "movie" to VesselI18n.tr("search.filter_movies"),
                "series" to VesselI18n.tr("search.filter_series"),
                "anime" to VesselI18n.tr("search.filter_anime"),
            ).forEach { (key, label) ->
                FilterChip(
                    selected = selectedType == key,
                    onClick = { selectedType = key },
                    label = { Text(label) },
                )
            }

            Box {
                val pluginLabel = enabledPlugins.firstOrNull { it.manifest.id == selectedPluginId }?.manifest?.name
                    ?: VesselI18n.tr("search.all_plugins")
                OutlinedButton(onClick = { showPluginMenu = true }) {
                    Text(pluginLabel, maxLines = 1, overflow = TextOverflow.Ellipsis)
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
            LinearProgressIndicator(modifier = Modifier.fillMaxWidth())
        }

        if (groupedBuckets.isEmpty() && !searching) {
            Box(modifier = Modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
                Text(VesselI18n.tr("search.empty"), color = palette.textMuted)
            }
        } else {
            LazyVerticalGrid(
                columns = GridCells.Adaptive(minSize = if (prefs.cards.aspectRatio == "wide_16x9") 180.dp else 124.dp),
                contentPadding = PaddingValues(bottom = 24.dp),
                horizontalArrangement = Arrangement.spacedBy(12.dp),
                verticalArrangement = Arrangement.spacedBy(14.dp),
                modifier = Modifier.fillMaxSize(),
            ) {
                groupedBuckets.forEach { (pluginId, bucketItems) ->
                    item(key = "hdr:$pluginId", span = { GridItemSpan(maxLineSpan) }) {
                        Row(
                            modifier = Modifier.fillMaxWidth().padding(top = 6.dp),
                            verticalAlignment = Alignment.CenterVertically,
                            horizontalArrangement = Arrangement.spacedBy(8.dp),
                        ) {
                            Text(
                                text = pluginNameMap[pluginId] ?: pluginId,
                                style = MaterialTheme.typography.titleMedium,
                                fontWeight = FontWeight.Bold,
                            )
                            Surface(color = palette.bgSurface, shape = RoundedCornerShape(50)) {
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
                        MediaPosterCard(
                            item = item,
                            cards = prefs.cards,
                            onClick = { onOpenDetails(pid, item.nativeId) },
                            onQuickStatus = { status ->
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
                                    repo.notify("${item.title} → $status")
                                }
                            },
                        )
                    }
                }
            }
        }
    }
}
