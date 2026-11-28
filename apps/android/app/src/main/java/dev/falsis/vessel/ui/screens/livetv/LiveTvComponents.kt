package dev.falsis.vessel.ui.screens

import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.layout.aspectRatio
import androidx.compose.runtime.Stable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Shape
import androidx.compose.ui.unit.dp
import dev.falsis.vessel.data.HomeSection
import dev.falsis.vessel.data.PluginListEntry
import dev.falsis.vessel.data.StoredIptvChannel
import dev.falsis.vessel.data.StoredIptvProfile
import dev.falsis.vessel.ui.isLiveTvPlugin

internal fun isEnabledLiveTvPlugin(p: PluginListEntry): Boolean = p.enabled && isLiveTvPlugin(p)

internal fun <T> orderBySavedIds(items: List<T>, savedOrder: List<String>, getId: (T) -> String): List<T> {
    if (savedOrder.isEmpty() || items.size < 2) return items
    val rank = HashMap<String, Int>(savedOrder.size)
    savedOrder.forEachIndexed { idx, id -> rank[id] = idx }
    return items.withIndex()
        .sortedWith(compareBy<IndexedValue<T>> { rank[getId(it.value)] ?: Int.MAX_VALUE }.thenBy { it.index })
        .map { it.value }
}

@Stable
class LiveTvSessionState {
    var tab by mutableIntStateOf(0)
    var profiles by mutableStateOf<List<StoredIptvProfile>>(emptyList())
    var activeProfileId by mutableStateOf<String?>(null)
    var groups by mutableStateOf<List<String>>(emptyList())
    var selectedGroup by mutableStateOf<String?>(null)
    var channels by mutableStateOf<List<StoredIptvChannel>>(emptyList())
    var favoriteIptvChannels by mutableStateOf<List<StoredIptvChannel>>(emptyList())
    var query by mutableStateOf("")
    var favoritesOnly by mutableStateOf(false)

    var selectedLivePluginId by mutableStateOf<String?>(null)
    var pluginLiveSections by mutableStateOf<List<HomeSection>>(emptyList())
    var selectedSectionId by mutableStateOf<String?>(null)
    var tvCategoriesExpanded by mutableStateOf(false)
    var pluginChannelFilter by mutableStateOf("")
    var pluginFavoritesOnly by mutableStateOf(false)
}

internal fun liveLogoShape(style: String): Shape = when (style) {
    "circle" -> CircleShape
    "square" -> RoundedCornerShape(2.dp)
    else -> RoundedCornerShape(8.dp)
}

internal fun liveAspectModifier(aspect: String): Modifier = when (aspect) {
    "square" -> Modifier.aspectRatio(1f)
    "wide_16x9" -> Modifier.aspectRatio(16f / 9f)
    "classic_4x3" -> Modifier.aspectRatio(4f / 3f)
    else -> Modifier
}
