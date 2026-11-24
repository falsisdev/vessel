package dev.falsis.vessel.ui.screens

import androidx.compose.foundation.background
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.lazy.grid.GridCells
import androidx.compose.foundation.lazy.grid.LazyVerticalGrid
import androidx.compose.foundation.lazy.grid.items
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Close
import androidx.compose.material.icons.filled.Search
import androidx.compose.material3.FilterChip
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
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
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import dev.falsis.vessel.data.LibraryItem
import dev.falsis.vessel.data.MediaSummary
import dev.falsis.vessel.data.VesselRepository
import dev.falsis.vessel.ui.components.MediaPosterCard
import dev.falsis.vessel.ui.i18n.VesselI18n
import dev.falsis.vessel.ui.theme.LocalVesselPalette
import kotlinx.coroutines.launch

@Composable
fun LibraryScreen(
    repo: VesselRepository,
    onOpenDetails: (pluginId: String, nativeId: String) -> Unit,
) {
    val palette = LocalVesselPalette.current
    val scope = rememberCoroutineScope()
    val prefs by repo.uiPreferences.collectAsState()
    var selectedFilter by remember { mutableStateOf<String?>(null) }
    var searchText by remember { mutableStateOf("") }
    var itemsList by remember { mutableStateOf<List<LibraryItem>>(emptyList()) }

    suspend fun reload() {
        runCatching { itemsList = repo.listLibrary(selectedFilter) }
    }

    LaunchedEffect(selectedFilter) { reload() }

    val filteredItems = remember(itemsList, searchText) {
        val q = searchText.trim().lowercase()
        if (q.isEmpty()) itemsList else itemsList.filter { it.title.lowercase().contains(q) }
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
            Text(VesselI18n.tr("library.title"), style = MaterialTheme.typography.headlineSmall, fontWeight = FontWeight.Bold)
            Surface(color = palette.bgSurface, shape = RoundedCornerShape(50)) {
                Text(
                    text = "${filteredItems.size}",
                    style = MaterialTheme.typography.labelMedium,
                    color = palette.textMuted,
                    modifier = Modifier.padding(horizontal = 10.dp, vertical = 4.dp),
                )
            }
        }

        Row(
            modifier = Modifier
                .fillMaxWidth()
                .horizontalScroll(rememberScrollState()),
            horizontalArrangement = Arrangement.spacedBy(8.dp),
        ) {
            listOf(
                null to VesselI18n.tr("library.all"),
                "watching" to VesselI18n.tr("library.watching"),
                "plan_to_watch" to VesselI18n.tr("library.plan_to_watch"),
                "completed" to VesselI18n.tr("library.completed"),
                "on_hold" to VesselI18n.tr("library.on_hold"),
                "dropped" to VesselI18n.tr("library.dropped"),
            ).forEach { (status, label) ->
                FilterChip(
                    selected = selectedFilter == status,
                    onClick = { selectedFilter = status },
                    label = { Text(label) },
                )
            }
        }

        if (itemsList.isNotEmpty()) {
            OutlinedTextField(
                value = searchText,
                onValueChange = { searchText = it },
                placeholder = { Text(VesselI18n.tr("common.search", "Search…")) },
                leadingIcon = { Icon(Icons.Default.Search, contentDescription = null) },
                singleLine = true,
                modifier = Modifier.fillMaxWidth(),
            )
        }

        if (filteredItems.isEmpty()) {
            Box(modifier = Modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
                Text(VesselI18n.tr("common.empty"), color = palette.textMuted)
            }
        } else {
            LazyVerticalGrid(
                columns = GridCells.Adaptive(minSize = if (prefs.cards.aspectRatio == "wide_16x9") 180.dp else 124.dp),
                contentPadding = PaddingValues(bottom = 24.dp),
                horizontalArrangement = Arrangement.spacedBy(12.dp),
                verticalArrangement = Arrangement.spacedBy(14.dp),
                modifier = Modifier.fillMaxSize(),
            ) {
                items(filteredItems, key = { "${it.media.pluginId}:${it.media.nativeId}" }) { item ->
                    Box {
                        MediaPosterCard(
                            item = MediaSummary(
                                nativeId = item.media.nativeId,
                                pluginId = item.media.pluginId,
                                type = item.mediaType,
                                isAnime = item.isAnime,
                                title = item.title,
                                poster = item.posterUrl,
                                year = item.year,
                            ),
                            cards = prefs.cards,
                            onClick = { onOpenDetails(item.media.pluginId, item.media.nativeId) },
                            onQuickStatus = { newStatus ->
                                scope.launch {
                                    repo.upsertLibrary(item.copy(status = newStatus))
                                    reload()
                                }
                            },
                        )
                        // Top-left non-overlapping delete icon matching Desktop .lib-remove-btn
                        IconButton(
                            onClick = {
                                scope.launch {
                                    repo.deleteLibraryItem(item.media.pluginId, item.media.nativeId)
                                    reload()
                                }
                            },
                            modifier = Modifier
                                .align(Alignment.TopStart)
                                .padding(4.dp)
                                .size(26.dp)
                                .clip(CircleShape)
                                .background(Color.Black.copy(alpha = 0.68f)),
                        ) {
                            Icon(
                                Icons.Default.Close,
                                contentDescription = VesselI18n.tr("library.remove_item"),
                                tint = Color.White,
                                modifier = Modifier.size(14.dp),
                            )
                        }
                    }
                }
            }
        }
    }
}
