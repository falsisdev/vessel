package dev.falsis.vessel.ui.screens

import androidx.compose.animation.AnimatedContent
import androidx.compose.animation.core.tween
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.togetherWith
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
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
import androidx.compose.material3.Icon
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
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.unit.dp
import dev.falsis.vessel.data.LibraryItem
import dev.falsis.vessel.data.MediaSummary
import dev.falsis.vessel.data.VesselRepository
import dev.falsis.vessel.ui.components.MediaPosterCard
import dev.falsis.vessel.ui.components.VesselFilterChip
import dev.falsis.vessel.ui.components.VesselSearchField
import dev.falsis.vessel.ui.components.isLandscapeCardRatio
import dev.falsis.vessel.ui.components.vesselTopSafeInset
import dev.falsis.vessel.ui.i18n.VesselI18n
import dev.falsis.vessel.ui.theme.LocalVesselFloatingNavInset
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
            Text(VesselI18n.tr("library.title"), style = MaterialTheme.typography.headlineSmall, fontWeight = FontWeight.Bold)
            Surface(color = palette.bgSurface, shape = RoundedCornerShape(999.dp)) {
                Text(
                    text = "${filteredItems.size}",
                    style = MaterialTheme.typography.labelSmall,
                    color = palette.textMuted,
                    modifier = Modifier.padding(horizontal = 9.dp, vertical = 3.dp),
                )
            }
        }

        Row(
            modifier = Modifier
                .fillMaxWidth()
                .horizontalScroll(rememberScrollState()),
            horizontalArrangement = Arrangement.spacedBy(6.dp),
        ) {
            listOf(
                null to VesselI18n.tr("library.all"),
                "watching" to VesselI18n.tr("library.watching"),
                "plan_to_watch" to VesselI18n.tr("library.plan_to_watch"),
                "completed" to VesselI18n.tr("library.completed"),
                "on_hold" to VesselI18n.tr("library.on_hold"),
                "dropped" to VesselI18n.tr("library.dropped"),
            ).forEach { (status, label) ->
                VesselFilterChip(
                    selected = selectedFilter == status,
                    onClick = { selectedFilter = status },
                    label = label,
                )
            }
        }

        if (itemsList.isNotEmpty()) {
            VesselSearchField(
                value = searchText,
                onValueChange = { searchText = it },
                placeholder = VesselI18n.tr("common.search"),
                imeAction = ImeAction.Done,
                modifier = Modifier.fillMaxWidth(),
            )
        }

        AnimatedContent(
            targetState = filteredItems.isEmpty(),
            transitionSpec = { fadeIn(tween(200)).togetherWith(fadeOut(tween(150))) },
            label = "library_grid_transition",
            modifier = Modifier.fillMaxSize(),
        ) { isEmpty ->
            if (isEmpty) {
                Box(modifier = Modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
                    Text(VesselI18n.tr("common.empty"), color = palette.textMuted)
                }
            } else {
                LazyVerticalGrid(
                    columns = GridCells.Adaptive(minSize = if (isWideCard) 168.dp else 110.dp),
                    contentPadding = PaddingValues(top = 2.dp, bottom = 24.dp + LocalVesselFloatingNavInset.current),
                    horizontalArrangement = Arrangement.spacedBy(10.dp),
                    verticalArrangement = Arrangement.spacedBy(14.dp),
                    modifier = Modifier.fillMaxSize(),
                ) {
                    items(filteredItems, key = { "${it.media.pluginId}:${it.media.nativeId}" }) { item ->
                        Box(modifier = Modifier.fillMaxWidth()) {
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
                                currentStatus = item.status,
                                fillGridWidth = true,
                                onClick = { onOpenDetails(item.media.pluginId, item.media.nativeId) },
                                onQuickStatus = { newStatus ->
                                    scope.launch {
                                        repo.upsertLibrary(item.copy(status = newStatus))
                                        reload()
                                    }
                                },
                            )
                            // Minimal background-free remove icon at top-left (non-overlapping with bottom title)
                            Box(
                                contentAlignment = Alignment.Center,
                                modifier = Modifier
                                    .align(Alignment.TopStart)
                                    .padding(3.dp)
                                    .size(24.dp)
                                    .clip(CircleShape)
                                    .clickable {
                                        scope.launch {
                                            repo.deleteLibraryItem(item.media.pluginId, item.media.nativeId)
                                            reload()
                                        }
                                    },
                            ) {
                                Icon(
                                    Icons.Default.Close,
                                    contentDescription = VesselI18n.tr("library.remove_item"),
                                    tint = Color.White.copy(alpha = 0.88f),
                                    modifier = Modifier.size(14.dp),
                                )
                            }
                        }
                    }
                }
            }
        }
    }
}
