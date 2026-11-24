package dev.falsis.vessel.ui.screens

import androidx.compose.foundation.background
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
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.LazyRow
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Info
import androidx.compose.material.icons.filled.PlayArrow
import androidx.compose.material.icons.filled.Refresh
import androidx.compose.material.icons.filled.Tune
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.Checkbox
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
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
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import coil3.compose.AsyncImage
import dev.falsis.vessel.data.ContinueWatchingEntry
import dev.falsis.vessel.data.HomeShelfResult
import dev.falsis.vessel.data.LibraryItem
import dev.falsis.vessel.data.MediaRef
import dev.falsis.vessel.data.MediaSummary
import dev.falsis.vessel.data.VesselRepository
import dev.falsis.vessel.ui.components.ContinueWatchingCard
import dev.falsis.vessel.ui.components.MediaPosterCard
import dev.falsis.vessel.ui.components.VesselLogoMark
import dev.falsis.vessel.ui.i18n.VesselI18n
import dev.falsis.vessel.ui.theme.LocalVesselPalette
import kotlinx.coroutines.launch

@Composable
fun HomeScreen(
    repo: VesselRepository,
    onOpenDetails: (pluginId: String, nativeId: String) -> Unit,
    onResumeEntry: (ContinueWatchingEntry) -> Unit,
    onOpenExtensions: () -> Unit,
) {
    val palette = LocalVesselPalette.current
    val scope = rememberCoroutineScope()
    val isReady by repo.isReady.collectAsState()
    val initError by repo.initError.collectAsState()
    val plugins by repo.plugins.collectAsState()
    val prefs by repo.uiPreferences.collectAsState()

    var loading by remember { mutableStateOf(true) }
    var shelves by remember { mutableStateOf<List<HomeShelfResult>>(emptyList()) }
    var continueWatching by remember { mutableStateOf<List<ContinueWatchingEntry>>(emptyList()) }
    var showFeedsModal by remember { mutableStateOf(false) }

    suspend fun loadHome() {
        if (initError != null) {
            loading = false
            return
        }
        loading = true
        runCatching { continueWatching = repo.listContinueWatching(prefs.home.continueWatchingLimit) }
        runCatching { shelves = repo.getHomeFeeds(prefs.home.selectedPlugins) }
        loading = false
    }

    LaunchedEffect(isReady, plugins.size, prefs.home.selectedPlugins) {
        if (isReady) loadHome()
    }

    val heroItem: MediaSummary? = remember(shelves) {
        shelves.asSequence()
            .flatMap { it.sections.asSequence() }
            .flatMap { sec -> sec.items.asSequence().map { it.copy(pluginId = it.pluginId) } }
            .firstOrNull { !it.poster.isNullOrBlank() }
    }
    val heroPluginId: String? = remember(shelves, heroItem) {
        shelves.firstOrNull { res -> res.sections.any { sec -> sec.items.contains(heroItem) } }?.pluginId
    }

    LazyColumn(
        modifier = Modifier
            .fillMaxSize()
            .background(palette.bgBase),
        contentPadding = PaddingValues(bottom = 24.dp),
        verticalArrangement = Arrangement.spacedBy(20.dp),
    ) {
        // Top Brand Bar
        item {
            Row(
                modifier = Modifier
                    .fillMaxWidth()
                    .padding(horizontal = 16.dp, vertical = 12.dp),
                verticalAlignment = Alignment.CenterVertically,
                horizontalArrangement = Arrangement.SpaceBetween,
            ) {
                Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(10.dp)) {
                    VesselLogoMark(modifier = Modifier.height(28.dp).width(28.dp))
                    Text("Vessel", style = MaterialTheme.typography.titleLarge, fontWeight = FontWeight.Bold)
                }
                Row {
                    IconButton(onClick = { showFeedsModal = true }) {
                        Icon(Icons.Default.Tune, contentDescription = VesselI18n.tr("home.choose_feeds"))
                    }
                    IconButton(onClick = { scope.launch { loadHome() } }) {
                        Icon(Icons.Default.Refresh, contentDescription = VesselI18n.tr("common.refresh"))
                    }
                }
            }
        }

        // Native FFI warning banner if .so isn't built yet
        initError?.let { err ->
            item {
                Surface(
                    color = palette.danger.copy(alpha = 0.14f),
                    shape = RoundedCornerShape(14.dp),
                    modifier = Modifier
                        .fillMaxWidth()
                        .padding(horizontal = 16.dp),
                ) {
                    Column(modifier = Modifier.padding(14.dp), verticalArrangement = Arrangement.spacedBy(6.dp)) {
                        Text("Native Engine Setup Required", fontWeight = FontWeight.Bold, color = palette.danger)
                        Text(err, style = MaterialTheme.typography.bodySmall, color = palette.textPrimary)
                    }
                }
            }
        }

        // Spotlight Hero Banner
        if (heroItem != null && heroPluginId != null) {
            item {
                Box(
                    modifier = Modifier
                        .fillMaxWidth()
                        .padding(horizontal = 16.dp)
                        .height(260.dp)
                        .clip(RoundedCornerShape(18.dp))
                        .background(palette.bgElevated),
                ) {
                    AsyncImage(
                        model = heroItem.poster,
                        contentDescription = heroItem.title,
                        contentScale = ContentScale.Crop,
                        modifier = Modifier.fillMaxSize(),
                    )
                    Box(
                        modifier = Modifier
                            .fillMaxSize()
                            .background(
                                Brush.verticalGradient(
                                    listOf(Color.Transparent, Color.Black.copy(alpha = 0.85f)),
                                ),
                            ),
                    )
                    Column(
                        modifier = Modifier
                            .align(Alignment.BottomStart)
                            .padding(18.dp),
                        verticalArrangement = Arrangement.spacedBy(8.dp),
                    ) {
                        Text(
                            text = heroItem.title,
                            style = MaterialTheme.typography.headlineSmall,
                            fontWeight = FontWeight.Bold,
                            color = Color.White,
                            maxLines = 2,
                            overflow = TextOverflow.Ellipsis,
                        )
                        Row(horizontalArrangement = Arrangement.spacedBy(10.dp)) {
                            Button(onClick = { onOpenDetails(heroPluginId, heroItem.nativeId) }) {
                                Icon(Icons.Default.PlayArrow, contentDescription = null)
                                Spacer(Modifier.width(6.dp))
                                Text(VesselI18n.tr("home.watch_now"))
                            }
                            OutlinedButton(onClick = { onOpenDetails(heroPluginId, heroItem.nativeId) }) {
                                Icon(Icons.Default.Info, contentDescription = null, tint = Color.White)
                                Spacer(Modifier.width(6.dp))
                                Text(VesselI18n.tr("home.more_info"), color = Color.White)
                            }
                        }
                    }
                }
            }
        }

        // Continue Watching Rail
        if (prefs.home.showContinueWatching && continueWatching.isNotEmpty()) {
            item {
                Column(verticalArrangement = Arrangement.spacedBy(10.dp)) {
                    Text(
                        text = VesselI18n.tr("home.continue_watching"),
                        style = MaterialTheme.typography.titleMedium,
                        fontWeight = FontWeight.Bold,
                        modifier = Modifier.padding(horizontal = 16.dp),
                    )
                    LazyRow(
                        contentPadding = PaddingValues(horizontal = 16.dp),
                        horizontalArrangement = Arrangement.spacedBy(12.dp),
                    ) {
                        items(continueWatching, key = { "${it.progress.media.pluginId}:${it.progress.media.nativeId}" }) { entry ->
                            ContinueWatchingCard(
                                entry = entry,
                                onResume = { onResumeEntry(entry) },
                                onGoToDetails = {
                                    onOpenDetails(entry.progress.media.pluginId, entry.progress.media.nativeId)
                                },
                                onEdit = {
                                    onOpenDetails(entry.progress.media.pluginId, entry.progress.media.nativeId)
                                },
                                onRemove = {
                                    scope.launch {
                                        repo.deleteWatchProgress(
                                            entry.progress.media.pluginId,
                                            entry.progress.media.nativeId,
                                        )
                                        loadHome()
                                    }
                                },
                            )
                        }
                    }
                }
            }
        }

        if (loading) {
            item {
                Box(modifier = Modifier.fillMaxWidth().padding(32.dp), contentAlignment = Alignment.Center) {
                    CircularProgressIndicator()
                }
            }
        } else if (plugins.none { it.enabled }) {
            item {
                Column(
                    modifier = Modifier
                        .fillMaxWidth()
                        .padding(24.dp),
                    horizontalAlignment = Alignment.CenterHorizontally,
                    verticalArrangement = Arrangement.spacedBy(12.dp),
                ) {
                    Text(VesselI18n.tr("home.empty_plugins"), color = palette.textMuted)
                    Button(onClick = onOpenExtensions) {
                        Text(VesselI18n.tr("nav.extensions"))
                    }
                }
            }
        } else {
            shelves.forEach { result ->
                result.sections.forEach { section ->
                    if (section.items.isNotEmpty()) {
                        item(key = "${result.pluginId}:${section.id}") {
                            Column(verticalArrangement = Arrangement.spacedBy(10.dp)) {
                                Row(
                                    modifier = Modifier
                                        .fillMaxWidth()
                                        .padding(horizontal = 16.dp),
                                    horizontalArrangement = Arrangement.SpaceBetween,
                                    verticalAlignment = Alignment.CenterVertically,
                                ) {
                                    Text(
                                        text = section.title,
                                        style = MaterialTheme.typography.titleMedium,
                                        fontWeight = FontWeight.Bold,
                                    )
                                    Text(
                                        text = result.pluginName,
                                        style = MaterialTheme.typography.labelSmall,
                                        color = palette.textMuted,
                                    )
                                }
                                LazyRow(
                                    contentPadding = PaddingValues(horizontal = 16.dp),
                                    horizontalArrangement = Arrangement.spacedBy(12.dp),
                                ) {
                                    items(section.items, key = { "${result.pluginId}:${it.nativeId}" }) { item ->
                                        val pid = item.pluginId ?: result.pluginId
                                        MediaPosterCard(
                                            item = item,
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
            }
        }
    }

    if (showFeedsModal) {
        val enabled = plugins.filter { it.enabled }
        val currentSelection = prefs.home.selectedPlugins?.toSet() ?: enabled.map { it.manifest.id }.toSet()
        var draftSelection by remember(showFeedsModal) { mutableStateOf(currentSelection) }
        AlertDialog(
            onDismissRequest = { showFeedsModal = false },
            title = { Text(VesselI18n.tr("home.modal_title")) },
            text = {
                Column {
                    enabled.forEach { entry ->
                        val pid = entry.manifest.id
                        Row(
                            modifier = Modifier.fillMaxWidth(),
                            verticalAlignment = Alignment.CenterVertically,
                        ) {
                            Checkbox(
                                checked = pid in draftSelection,
                                onCheckedChange = { checked ->
                                    draftSelection = if (checked) draftSelection + pid else draftSelection - pid
                                },
                            )
                            Text(entry.manifest.name)
                        }
                    }
                }
            },
            confirmButton = {
                Button(onClick = {
                    showFeedsModal = false
                    scope.launch {
                        repo.saveUiPreferences(
                            prefs.copy(home = prefs.home.copy(selectedPlugins = draftSelection.toList())),
                        )
                    }
                }) {
                    Text(VesselI18n.tr("common.save"))
                }
            },
            dismissButton = {
                TextButton(onClick = { showFeedsModal = false }) {
                    Text(VesselI18n.tr("common.cancel"))
                }
            },
        )
    }
}
