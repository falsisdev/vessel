package dev.falsis.vessel.ui.screens

import androidx.compose.animation.AnimatedContent
import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.core.tween
import androidx.compose.animation.expandVertically
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.shrinkVertically
import androidx.compose.animation.togetherWith
import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.gestures.detectVerticalDragGestures
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.aspectRatio
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.LazyRow
import androidx.compose.foundation.lazy.grid.GridCells
import androidx.compose.foundation.lazy.grid.GridItemSpan
import androidx.compose.foundation.lazy.grid.LazyVerticalGrid
import androidx.compose.foundation.lazy.grid.items
import androidx.compose.foundation.lazy.grid.itemsIndexed
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.lazy.itemsIndexed
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Add
import androidx.compose.material.icons.filled.ArrowDownward
import androidx.compose.material.icons.filled.ArrowUpward
import androidx.compose.material.icons.filled.Delete
import androidx.compose.material.icons.filled.DragIndicator
import androidx.compose.material.icons.filled.ExpandLess
import androidx.compose.material.icons.filled.ExpandMore
import androidx.compose.material.icons.filled.FilterList
import androidx.compose.material.icons.filled.LiveTv
import androidx.compose.material.icons.filled.PlayArrow
import androidx.compose.material.icons.filled.RestartAlt
import androidx.compose.material.icons.filled.Star
import androidx.compose.material.icons.filled.StarBorder
import androidx.compose.material.icons.filled.SwapVert
import androidx.compose.material.icons.filled.Sync
import androidx.compose.material.icons.filled.Tune
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Switch
import androidx.compose.material3.Tab
import androidx.compose.material3.TabRow
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.Stable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.key
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.focus.FocusRequester
import androidx.compose.ui.focus.focusRequester
import androidx.compose.ui.graphics.Shape
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.layout.onSizeChanged
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import coil3.compose.AsyncImage
import dev.falsis.vessel.data.EpgNowNext
import dev.falsis.vessel.data.Episode
import dev.falsis.vessel.data.FavoriteLiveChannel
import dev.falsis.vessel.data.HomeSection
import dev.falsis.vessel.data.LiveTvPreferences
import dev.falsis.vessel.data.MediaDetail
import dev.falsis.vessel.data.MediaSummary
import dev.falsis.vessel.data.PluginListEntry
import dev.falsis.vessel.data.StoredIptvChannel
import dev.falsis.vessel.data.StoredIptvProfile
import dev.falsis.vessel.data.StreamCandidate
import dev.falsis.vessel.data.StreamResult
import dev.falsis.vessel.data.UpsertIptvProfilePayload
import dev.falsis.vessel.data.VesselRepository
import dev.falsis.vessel.ui.components.VesselButton
import dev.falsis.vessel.ui.components.VesselCompactIconButton
import dev.falsis.vessel.ui.components.VesselFilterChip
import dev.falsis.vessel.ui.components.VesselOutlinedButton
import dev.falsis.vessel.ui.components.VesselOutlinedTextField
import dev.falsis.vessel.ui.components.VesselSearchField
import dev.falsis.vessel.ui.components.VesselTextButton
import dev.falsis.vessel.ui.components.vesselDpadInputEscape
import dev.falsis.vessel.ui.components.vesselFocusBorder
import dev.falsis.vessel.ui.components.vesselTopSafeInset
import dev.falsis.vessel.ui.i18n.VesselI18n
import dev.falsis.vessel.ui.isLiveTvPlugin
import dev.falsis.vessel.ui.theme.LocalIsTvDevice
import dev.falsis.vessel.ui.theme.LocalVesselFloatingNavInset
import dev.falsis.vessel.ui.theme.LocalVesselPalette
import dev.falsis.vessel.ui.theme.LocalVesselUiTokens
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch

private fun isEnabledLiveTvPlugin(p: PluginListEntry): Boolean = p.enabled && isLiveTvPlugin(p)

private fun <T> orderBySavedIds(items: List<T>, savedOrder: List<String>, getId: (T) -> String): List<T> {
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

private fun liveLogoShape(style: String): Shape = when (style) {
    "circle" -> CircleShape
    "square" -> RoundedCornerShape(2.dp)
    else -> RoundedCornerShape(8.dp)
}

private fun liveAspectModifier(aspect: String): Modifier = when (aspect) {
    "square" -> Modifier.aspectRatio(1f)
    "wide_16x9" -> Modifier.aspectRatio(16f / 9f)
    "classic_4x3" -> Modifier.aspectRatio(4f / 3f)
    else -> Modifier
}

@OptIn(ExperimentalLayoutApi::class)
@Composable
fun LiveTvScreen(
    repo: VesselRepository,
    state: LiveTvSessionState = remember { LiveTvSessionState() },
    onPlayUrl: (title: String, url: String) -> Unit,
    onPlayLiveStream: (
        pluginId: String,
        detail: MediaDetail,
        episode: Episode,
        streams: StreamResult,
        channelStreams: Map<String, List<StreamCandidate>>,
    ) -> Unit = { _, d, _, _, _ ->
        onPlayUrl(d.title, d.nativeId)
    },
    onOpenDetails: (pluginId: String, nativeId: String) -> Unit,
) {
    val palette = LocalVesselPalette.current
    val isTv = LocalIsTvDevice.current
    val scope = rememberCoroutineScope()
    val plugins by repo.plugins.collectAsState()
    val prefs by repo.uiPreferences.collectAsState()
    val livePrefs = prefs.liveTv
    var tab by state::tab

    // Tab 1: IPTV Playlists (backed by hoisted session state)
    var profiles by state::profiles
    var activeProfileId by state::activeProfileId
    var groups by state::groups
    var selectedGroup by state::selectedGroup
    var channels by state::channels
    var favoriteIptvChannels by state::favoriteIptvChannels
    var query by state::query
    var favoritesOnly by state::favoritesOnly
    var syncingProfileId by remember { mutableStateOf<String?>(null) }
    var confirmDeleteProfileId by remember { mutableStateOf<String?>(null) }
    var selectedEpgChannel by remember { mutableStateOf<StoredIptvChannel?>(null) }
    var epgNowNext by remember { mutableStateOf<EpgNowNext?>(null) }
    var showAddProfile by remember { mutableStateOf(false) }

    // Tab 0: Live Plugins (backed by hoisted session state)
    val livePlugins = remember(plugins, livePrefs.pluginOrder) {
        orderBySavedIds(plugins.filter(::isEnabledLiveTvPlugin), livePrefs.pluginOrder) { it.manifest.id }
    }
    var selectedLivePluginId by state::selectedLivePluginId
    var pluginLiveSections by state::pluginLiveSections
    var selectedSectionId by state::selectedSectionId
    var pluginChannelFilter by state::pluginChannelFilter
    var pluginFavoritesOnly by state::pluginFavoritesOnly
    var loadingPluginLive by remember { mutableStateOf(false) }
    var launchingChannelId by remember { mutableStateOf<String?>(null) }
    var showReorderDialog by remember { mutableStateOf(false) }
    var showViewDialog by remember { mutableStateOf(false) }

    val pluginFavorites = livePrefs.favoriteChannels
    val favoriteKeySet = remember(pluginFavorites) {
        pluginFavorites.mapTo(HashSet()) { "${it.pluginId}::${it.nativeId}" }
    }
    fun isPluginFavorite(pid: String, nid: String): Boolean = "$pid::$nid" in favoriteKeySet

    fun saveLivePrefs(updated: LiveTvPreferences) {
        scope.launch {
            repo.saveUiPreferences(prefs.copy(liveTv = updated))
        }
    }

    fun togglePluginFavorite(item: MediaSummary, sectionTitle: String?) {
        val pid = item.pluginId ?: selectedLivePluginId ?: return
        val current = livePrefs.favoriteChannels
        val exists = current.any { it.pluginId == pid && it.nativeId == item.nativeId }
        val next = if (exists) {
            current.filterNot { it.pluginId == pid && it.nativeId == item.nativeId }
        } else {
            listOf(
                FavoriteLiveChannel(
                    pluginId = pid,
                    nativeId = item.nativeId,
                    title = item.title,
                    poster = item.poster,
                    sectionTitle = sectionTitle,
                ),
            ) + current
        }
        saveLivePrefs(livePrefs.copy(favoriteChannels = next))
    }

    val orderedSections = remember(pluginLiveSections, selectedLivePluginId, livePrefs.sectionOrderByPlugin) {
        val pid = selectedLivePluginId
        val saved = if (pid != null) livePrefs.sectionOrderByPlugin[pid].orEmpty() else emptyList()
        orderBySavedIds(pluginLiveSections, saved) { it.id }
    }

    LaunchedEffect(confirmDeleteProfileId) {
        val target = confirmDeleteProfileId ?: return@LaunchedEffect
        delay(3000)
        if (confirmDeleteProfileId == target) confirmDeleteProfileId = null
    }

    suspend fun reloadProfilesAndChannels() {
        runCatching { profiles = repo.listIptvProfiles() }
        if (activeProfileId == null || profiles.none { it.id == activeProfileId }) {
            activeProfileId = profiles.firstOrNull()?.id
        }
        val pid = activeProfileId
        groups = if (pid != null) runCatching { repo.listIptvGroups(pid) }.getOrDefault(emptyList()) else emptyList()
        if (selectedGroup != null && selectedGroup !in groups) selectedGroup = null
        runCatching {
            channels = repo.searchIptvChannels(
                profileId = pid,
                query = query.ifBlank { null },
                groupTitle = selectedGroup,
                favoritesOnly = favoritesOnly,
            )
        }
        runCatching {
            favoriteIptvChannels = repo.searchIptvChannels(
                profileId = null,
                favoritesOnly = true,
            )
        }
    }

    LaunchedEffect(activeProfileId, selectedGroup, query, favoritesOnly) {
        reloadProfilesAndChannels()
    }

    suspend fun loadPluginSections(pid: String, resetSection: Boolean = true) {
        loadingPluginLive = true
        if (resetSection) {
            selectedSectionId = null
        }
        runCatching { repo.getHomeFeeds(listOf(pid)) }
            .onSuccess { list ->
                val feed = list.firstOrNull()
                if (!feed?.error.isNullOrBlank()) {
                    repo.notify("${feed?.pluginName ?: pid}: ${feed?.error}")
                }
                val loaded = feed?.sections ?: emptyList()
                pluginLiveSections = loaded
                if (selectedSectionId != null && loaded.none { it.id == selectedSectionId }) {
                    selectedSectionId = null
                }
            }
            .onFailure {
                pluginLiveSections = emptyList()
                repo.notifyError(it, "Failed to load live plugin")
            }
        loadingPluginLive = false
    }

    LaunchedEffect(tab, livePlugins) {
        if (tab == 0 && livePlugins.isNotEmpty()) {
            val current = selectedLivePluginId?.takeIf { id -> livePlugins.any { it.manifest.id == id } }
                ?: livePlugins.first().manifest.id
            val switchedPlugin = selectedLivePluginId != current
            if (switchedPlugin || pluginLiveSections.isEmpty()) {
                selectedLivePluginId = current
                loadPluginSections(current, resetSection = switchedPlugin)
            }
        }
    }

    fun playPluginLiveItem(
        item: MediaSummary,
        sectionTitle: String,
        siblingChannels: List<Pair<String, MediaSummary>> = emptyList(),
    ) {
        val pid = item.pluginId ?: selectedLivePluginId ?: return
        if (launchingChannelId != null) return
        launchingChannelId = item.nativeId
        scope.launch {
            runCatching {
                val detail = repo.getDetails(pid, item.nativeId)
                val siblingEpisodes = siblingChannels.mapIndexed { idx, (secTitle, ch) ->
                    Episode(
                        streamId = ch.nativeId,
                        season = 1,
                        episode = idx + 1,
                        title = ch.title,
                        overview = secTitle,
                        thumbnail = ch.poster,
                    )
                }
                val ep = siblingEpisodes.firstOrNull { it.streamId == item.nativeId }
                    ?: detail.episodes.firstOrNull()
                    ?: Episode(
                        streamId = item.nativeId,
                        season = 1,
                        episode = 1,
                        title = item.title,
                        overview = sectionTitle,
                        thumbnail = item.poster,
                    )
                val finalEpisodes = if (siblingEpisodes.any { it.streamId == ep.streamId }) {
                    siblingEpisodes
                } else if (detail.episodes.isNotEmpty()) {
                    detail.episodes
                } else {
                    listOf(ep)
                }
                val sr = repo.getStreams(pid, ep.streamId)
                if (sr.streams.isEmpty()) {
                    repo.notify(VesselI18n.trf("iptv.no_streams_for", "title" to item.title))
                } else {
                    onPlayLiveStream(
                        pid,
                        detail.copy(
                            type = "live",
                            title = detail.title.ifBlank { item.title },
                            poster = detail.poster ?: item.poster,
                            backdrop = detail.backdrop ?: item.poster,
                            episodes = finalEpisodes,
                        ),
                        ep,
                        sr,
                        emptyMap(),
                    )
                }
            }.onFailure {
                repo.notifyError(it, "Failed to play ${item.title}")
            }
            launchingChannelId = null
        }
    }

    fun playIptvChannel(ch: StoredIptvChannel, siblingChannels: List<StoredIptvChannel> = channels) {
        selectedEpgChannel = ch
        scope.launch {
            val tvg = ch.tvgId
            val guide = if (!tvg.isNullOrBlank()) {
                runCatching { repo.getEpgNowNext(ch.profileId, tvg) }.getOrNull()
            } else {
                null
            }
            epgNowNext = guide

            fun buildChannelCandidates(item: StoredIptvChannel): List<StreamCandidate> {
                val itemProf = profiles.firstOrNull { it.id == item.profileId }
                val ua = itemProf?.userAgent?.takeIf { it.isNotBlank() }
                    ?: item.userAgent?.takeIf { it.isNotBlank() }
                val headers = if (ua != null) mapOf("User-Agent" to ua) else emptyMap()
                val rawUrl = item.streamUrl
                val lowerUrl = rawUrl.lowercase()
                val isXtreamLive = item.streamId.startsWith("xtream-") && lowerUrl.contains("/live/")
                return when {
                    isXtreamLive && lowerUrl.endsWith(".ts") -> {
                        val hlsUrl = rawUrl.dropLast(3) + ".m3u8"
                        listOf(
                            StreamCandidate(
                                url = hlsUrl,
                                kind = "hls",
                                label = "${item.name} (HLS)",
                                isDefault = true,
                                flags = listOf("requires_proxy"),
                                headers = headers,
                            ),
                            StreamCandidate(
                                url = rawUrl,
                                kind = "mpeg_ts",
                                label = "${item.name} (MPEG-TS)",
                                flags = listOf("requires_proxy"),
                                headers = headers,
                            ),
                        )
                    }
                    isXtreamLive && lowerUrl.endsWith(".m3u8") -> {
                        val tsUrl = rawUrl.dropLast(5) + ".ts"
                        listOf(
                            StreamCandidate(
                                url = rawUrl,
                                kind = "hls",
                                label = "${item.name} (HLS)",
                                isDefault = true,
                                flags = listOf("requires_proxy"),
                                headers = headers,
                            ),
                            StreamCandidate(
                                url = tsUrl,
                                kind = "mpeg_ts",
                                label = "${item.name} (MPEG-TS)",
                                flags = listOf("requires_proxy"),
                                headers = headers,
                            ),
                        )
                    }
                    else -> {
                        val inferredKind = when {
                            lowerUrl.contains(".m3u8") -> "hls"
                            lowerUrl.contains(".ts") -> "mpeg_ts"
                            else -> "auto"
                        }
                        listOf(
                            StreamCandidate(
                                url = rawUrl,
                                kind = inferredKind,
                                label = when (inferredKind) {
                                    "hls" -> "${item.name} (HLS)"
                                    "mpeg_ts" -> "${item.name} (MPEG-TS)"
                                    else -> item.name
                                },
                                isDefault = true,
                                flags = listOf("requires_proxy"),
                                headers = headers,
                            ),
                        )
                    }
                }
            }

            val channelStreamsMap = mutableMapOf<String, List<StreamCandidate>>()
            val episodes = siblingChannels.mapIndexed { idx, item ->
                val sid = item.streamId.ifBlank { item.streamUrl }
                channelStreamsMap[sid] = buildChannelCandidates(item)
                Episode(
                    streamId = sid,
                    season = 1,
                    episode = idx + 1,
                    title = item.name,
                    overview = item.groupTitle,
                    thumbnail = item.logoUrl,
                )
            }
            val targetSid = ch.streamId.ifBlank { ch.streamUrl }
            val activeEp = episodes.firstOrNull { it.streamId == targetSid }
                ?: Episode(
                    streamId = targetSid,
                    season = 1,
                    episode = 1,
                    title = ch.name,
                    overview = ch.groupTitle,
                    thumbnail = ch.logoUrl,
                )
            val activeCandidates = buildChannelCandidates(ch)
            channelStreamsMap.putIfAbsent(targetSid, activeCandidates)

            val liveDetail = MediaDetail(
                nativeId = targetSid,
                pluginId = "iptv",
                type = "live",
                title = ch.name,
                poster = ch.logoUrl,
                backdrop = ch.logoUrl,
                synopsis = guide?.now?.let { "Now: ${it.title}" } ?: ch.groupTitle,
                genres = listOfNotNull(ch.groupTitle?.takeIf { it.isNotBlank() } ?: "Live TV"),
                episodes = episodes,
            )
            onPlayLiveStream(
                "iptv",
                liveDetail,
                activeEp,
                StreamResult(streams = activeCandidates),
                channelStreamsMap,
            )
        }
    }

    Column(
        modifier = Modifier
            .fillMaxSize()
            .background(palette.bgBase)
            .vesselTopSafeInset()
            .padding(top = 6.dp),
    ) {
        TabRow(selectedTabIndex = tab) {
            Tab(selected = tab == 0, onClick = { tab = 0 }, text = { Text(VesselI18n.tr("iptv.tab_plugins")) })
            Tab(selected = tab == 1, onClick = { tab = 1 }, text = { Text(VesselI18n.tr("iptv.tab_playlists")) })
        }

        // Immediate Top Favorites Section + View Customizer (always accessible)
        val totalFavs = pluginFavorites.size + favoriteIptvChannels.size
        if (!livePrefs.showFavoritesBar) {
            Row(
                modifier = Modifier
                    .fillMaxWidth()
                    .padding(horizontal = 16.dp, vertical = 6.dp),
                horizontalArrangement = Arrangement.End,
                verticalAlignment = Alignment.CenterVertically,
            ) {
                VesselOutlinedButton(
                    onClick = { showViewDialog = true },
                    height = 28.dp,
                    contentPadding = PaddingValues(horizontal = 9.dp, vertical = 0.dp),
                ) {
                    Icon(Icons.Default.Tune, contentDescription = null, modifier = Modifier.size(13.dp))
                    Spacer(Modifier.width(4.dp))
                    Text(VesselI18n.tr("iptv.customize_view"), style = MaterialTheme.typography.labelSmall)
                }
            }
        } else {
        Surface(
            shape = RoundedCornerShape(12.dp),
            color = palette.bgSurface,
            border = BorderStroke(1.dp, palette.warning.copy(alpha = 0.25f)),
            modifier = Modifier
                .fillMaxWidth()
                .padding(horizontal = 16.dp, vertical = 8.dp),
        ) {
            Column(
                modifier = Modifier.padding(horizontal = 12.dp, vertical = 10.dp),
                verticalArrangement = Arrangement.spacedBy(8.dp),
            ) {
                Row(
                    modifier = Modifier.fillMaxWidth(),
                    verticalAlignment = Alignment.CenterVertically,
                    horizontalArrangement = Arrangement.SpaceBetween,
                ) {
                    Row(
                        verticalAlignment = Alignment.CenterVertically,
                        horizontalArrangement = Arrangement.spacedBy(6.dp),
                    ) {
                        Icon(Icons.Default.Star, contentDescription = null, tint = palette.warning, modifier = Modifier.size(16.dp))
                        Text(
                            text = "${VesselI18n.tr("iptv.favorites_section")} ($totalFavs)",
                            style = MaterialTheme.typography.titleSmall,
                            fontWeight = FontWeight.Bold,
                        )
                    }
                    VesselOutlinedButton(
                        onClick = { showViewDialog = true },
                        height = 28.dp,
                        contentPadding = PaddingValues(horizontal = 9.dp, vertical = 0.dp),
                    ) {
                        Icon(Icons.Default.Tune, contentDescription = null, modifier = Modifier.size(13.dp))
                        Spacer(Modifier.width(4.dp))
                        Text(VesselI18n.tr("iptv.customize_view"), style = MaterialTheme.typography.labelSmall)
                    }
                }
                if (totalFavs == 0) {
                    Text(
                        text = VesselI18n.tr("iptv.favorites_empty_hint"),
                        style = MaterialTheme.typography.labelSmall,
                        color = palette.textMuted,
                    )
                } else {
                    val favSiblings = remember(pluginFavorites) {
                        pluginFavorites.map { f ->
                            (f.sectionTitle ?: f.pluginId) to MediaSummary(
                                nativeId = f.nativeId,
                                pluginId = f.pluginId,
                                type = "live",
                                title = f.title,
                                poster = f.poster,
                            )
                        }
                    }
                    LazyRow(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                        items(pluginFavorites, key = { "fp:${it.pluginId}:${it.nativeId}" }) { fav ->
                            Surface(
                                shape = RoundedCornerShape(10.dp),
                                color = palette.bgElevated,
                                modifier = Modifier
                                    .width(156.dp)
                                    .clip(RoundedCornerShape(10.dp))
                                    .vesselFocusBorder(RoundedCornerShape(10.dp))
                                    .clickable {
                                        playPluginLiveItem(
                                            MediaSummary(
                                                nativeId = fav.nativeId,
                                                pluginId = fav.pluginId,
                                                type = "live",
                                                title = fav.title,
                                                poster = fav.poster,
                                            ),
                                            fav.sectionTitle ?: fav.pluginId,
                                            siblingChannels = favSiblings.filter { it.second.pluginId == fav.pluginId },
                                        )
                                    },
                            ) {
                                Row(
                                    modifier = Modifier.padding(horizontal = 8.dp, vertical = 7.dp),
                                    verticalAlignment = Alignment.CenterVertically,
                                    horizontalArrangement = Arrangement.spacedBy(6.dp),
                                ) {
                                    if (livePrefs.showLogos) {
                                        Box(
                                            modifier = Modifier
                                                .size(28.dp)
                                                .clip(liveLogoShape(livePrefs.logoStyle))
                                                .then(if (livePrefs.logoStyle == "plain") Modifier else Modifier.background(palette.bgSurface)),
                                            contentAlignment = Alignment.Center,
                                        ) {
                                            if (!fav.poster.isNullOrBlank()) {
                                                AsyncImage(
                                                    model = fav.poster,
                                                    contentDescription = fav.title,
                                                    contentScale = ContentScale.Fit,
                                                    modifier = Modifier.fillMaxSize().padding(3.dp),
                                                )
                                            } else {
                                                Icon(Icons.Default.LiveTv, contentDescription = null, tint = palette.textMuted, modifier = Modifier.size(14.dp))
                                            }
                                        }
                                    }
                                    Column(modifier = Modifier.weight(1f)) {
                                        Text(fav.title, style = MaterialTheme.typography.labelMedium, fontWeight = FontWeight.SemiBold, maxLines = 1, overflow = TextOverflow.Ellipsis)
                                        if (livePrefs.showCategories) {
                                            Text(fav.sectionTitle ?: fav.pluginId, style = MaterialTheme.typography.labelSmall, color = palette.textMuted, maxLines = 1, overflow = TextOverflow.Ellipsis)
                                        }
                                    }
                                    if (livePrefs.showActionButtons) {
                                        VesselCompactIconButton(
                                            size = 24.dp,
                                            onClick = {
                                                saveLivePrefs(
                                                    livePrefs.copy(
                                                        favoriteChannels = livePrefs.favoriteChannels.filterNot {
                                                            it.pluginId == fav.pluginId && it.nativeId == fav.nativeId
                                                        },
                                                    ),
                                                )
                                            },
                                        ) {
                                            Icon(Icons.Default.Star, contentDescription = VesselI18n.tr("iptv.unfavorite_channel"), tint = palette.warning, modifier = Modifier.size(14.dp))
                                        }
                                    }
                                }
                            }
                        }
                        items(favoriteIptvChannels, key = { "fi:${it.id}" }) { ch ->
                            Surface(
                                shape = RoundedCornerShape(10.dp),
                                color = palette.bgElevated,
                                modifier = Modifier
                                    .width(156.dp)
                                    .clip(RoundedCornerShape(10.dp))
                                    .vesselFocusBorder(RoundedCornerShape(10.dp))
                                    .clickable { playIptvChannel(ch, favoriteIptvChannels) },
                            ) {
                                Row(
                                    modifier = Modifier.padding(horizontal = 8.dp, vertical = 7.dp),
                                    verticalAlignment = Alignment.CenterVertically,
                                    horizontalArrangement = Arrangement.spacedBy(6.dp),
                                ) {
                                    if (livePrefs.showLogos) {
                                        Box(
                                            modifier = Modifier
                                                .size(28.dp)
                                                .clip(liveLogoShape(livePrefs.logoStyle))
                                                .then(if (livePrefs.logoStyle == "plain") Modifier else Modifier.background(palette.bgSurface)),
                                            contentAlignment = Alignment.Center,
                                        ) {
                                            if (!ch.logoUrl.isNullOrBlank()) {
                                                AsyncImage(
                                                    model = ch.logoUrl,
                                                    contentDescription = ch.name,
                                                    contentScale = ContentScale.Fit,
                                                    modifier = Modifier.fillMaxSize().padding(3.dp),
                                                )
                                            } else {
                                                Icon(Icons.Default.LiveTv, contentDescription = null, tint = palette.textMuted, modifier = Modifier.size(14.dp))
                                            }
                                        }
                                    }
                                    Column(modifier = Modifier.weight(1f)) {
                                        Text(ch.name, style = MaterialTheme.typography.labelMedium, fontWeight = FontWeight.SemiBold, maxLines = 1, overflow = TextOverflow.Ellipsis)
                                        if (livePrefs.showCategories && !ch.groupTitle.isNullOrBlank()) {
                                            Text(ch.groupTitle, style = MaterialTheme.typography.labelSmall, color = palette.textMuted, maxLines = 1, overflow = TextOverflow.Ellipsis)
                                        }
                                    }
                                    if (livePrefs.showActionButtons) {
                                        VesselCompactIconButton(
                                            size = 24.dp,
                                            onClick = {
                                                scope.launch {
                                                    repo.setIptvFavorite(ch.id, false)
                                                    reloadProfilesAndChannels()
                                                }
                                            },
                                        ) {
                                            Icon(Icons.Default.Star, contentDescription = VesselI18n.tr("iptv.unfavorite_channel"), tint = palette.warning, modifier = Modifier.size(14.dp))
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

        AnimatedContent(
            targetState = tab,
            transitionSpec = { fadeIn(tween(200)).togetherWith(fadeOut(tween(150))) },
            label = "livetv_tab_transition",
            modifier = Modifier.weight(1f),
        ) { activeTab ->
        if (activeTab == 1) {
            Column(
                modifier = Modifier.fillMaxSize().padding(16.dp),
                verticalArrangement = Arrangement.spacedBy(10.dp),
            ) {
                // Profiles Row
                Row(
                    modifier = Modifier.fillMaxWidth().horizontalScroll(rememberScrollState()),
                    horizontalArrangement = Arrangement.spacedBy(6.dp),
                    verticalAlignment = Alignment.CenterVertically,
                ) {
                    VesselButton(onClick = { showAddProfile = true }, height = LocalVesselUiTokens.current.compactButtonHeight, contentPadding = PaddingValues(horizontal = 12.dp, vertical = 0.dp)) {
                        Icon(Icons.Default.Add, contentDescription = null, modifier = Modifier.size(15.dp))
                        Spacer(Modifier.width(5.dp))
                        Text(VesselI18n.tr("iptv.add_profile"), style = MaterialTheme.typography.labelMedium)
                    }
                    profiles.forEach { profile ->
                        VesselFilterChip(
                            selected = activeProfileId == profile.id,
                            onClick = {
                                activeProfileId = profile.id
                                selectedGroup = null
                            },
                            label = profile.name,
                        )
                    }
                }

                // Active Profile Actions + Favorites Filter
                Row(
                    modifier = Modifier.fillMaxWidth(),
                    horizontalArrangement = Arrangement.SpaceBetween,
                    verticalAlignment = Alignment.CenterVertically,
                ) {
                    VesselFilterChip(
                        selected = favoritesOnly,
                        onClick = { favoritesOnly = !favoritesOnly },
                        label = VesselI18n.tr("iptv.favorites_only"),
                    )
                    activeProfileId?.let { pid ->
                        Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(6.dp)) {
                            VesselOutlinedButton(
                                enabled = syncingProfileId != pid,
                                height = LocalVesselUiTokens.current.compactButtonHeight,
                                contentPadding = PaddingValues(horizontal = 12.dp, vertical = 0.dp),
                                onClick = {
                                    scope.launch {
                                        syncingProfileId = pid
                                        runCatching {
                                            val count = repo.syncIptvProfile(pid)
                                            runCatching { repo.syncIptvEpg(pid) }
                                            reloadProfilesAndChannels()
                                            repo.notify(VesselI18n.trf("iptv.synced_count", "count" to count))
                                        }.onFailure { repo.notifyError(it, "Sync failed") }
                                        syncingProfileId = null
                                    }
                                },
                            ) {
                                Icon(Icons.Default.Sync, contentDescription = null, modifier = Modifier.size(14.dp))
                                Spacer(Modifier.width(4.dp))
                                Text(VesselI18n.tr("iptv.sync_channels"), style = MaterialTheme.typography.labelSmall)
                            }
                            val confirming = confirmDeleteProfileId == pid
                            VesselCompactIconButton(
                                size = 30.dp,
                                onClick = {
                                    if (!confirming) {
                                        confirmDeleteProfileId = pid
                                    } else {
                                        confirmDeleteProfileId = null
                                        scope.launch {
                                            repo.deleteIptvProfile(pid)
                                            activeProfileId = null
                                            reloadProfilesAndChannels()
                                        }
                                    }
                                },
                            ) {
                                Icon(
                                    Icons.Default.Delete,
                                    contentDescription = VesselI18n.tr("common.delete"),
                                    tint = if (confirming) palette.warning else palette.danger,
                                    modifier = Modifier.size(17.dp),
                                )
                            }
                        }
                    }
                }

                // Group Chips (collapsible on Android TV, hidden in split_directory mode)
                if (groups.isNotEmpty() && livePrefs.viewMode != "split_directory") {
                    if (isTv) {
                        Row(
                            modifier = Modifier.fillMaxWidth(),
                            horizontalArrangement = Arrangement.End,
                        ) {
                            val grpActive = state.tvCategoriesExpanded || selectedGroup != null
                            VesselOutlinedButton(
                                onClick = { state.tvCategoriesExpanded = !state.tvCategoriesExpanded },
                                height = 34.dp,
                                contentPadding = PaddingValues(horizontal = 10.dp, vertical = 0.dp),
                            ) {
                                Icon(
                                    Icons.Default.FilterList,
                                    contentDescription = null,
                                    tint = if (grpActive) palette.accent else palette.textMuted,
                                    modifier = Modifier.size(15.dp),
                                )
                                Spacer(Modifier.width(5.dp))
                                Text(
                                    text = if (!state.tvCategoriesExpanded && selectedGroup != null) {
                                        selectedGroup!!
                                    } else {
                                        VesselI18n.tr("iptv.categories_toggle")
                                    },
                                    style = MaterialTheme.typography.labelSmall,
                                    color = if (grpActive) palette.accent else palette.textPrimary,
                                    maxLines = 1,
                                    overflow = TextOverflow.Ellipsis,
                                )
                                Spacer(Modifier.width(3.dp))
                                Icon(
                                    if (state.tvCategoriesExpanded) Icons.Default.ExpandLess else Icons.Default.ExpandMore,
                                    contentDescription = null,
                                    modifier = Modifier.size(15.dp),
                                )
                            }
                        }
                    }
                    AnimatedVisibility(
                        visible = !isTv || state.tvCategoriesExpanded,
                        enter = expandVertically(tween(200)) + fadeIn(tween(180)),
                        exit = shrinkVertically(tween(180)) + fadeOut(tween(140)),
                    ) {
                        Row(
                            modifier = Modifier.fillMaxWidth().horizontalScroll(rememberScrollState()),
                            horizontalArrangement = Arrangement.spacedBy(6.dp),
                        ) {
                            VesselFilterChip(
                                selected = selectedGroup == null,
                                onClick = { selectedGroup = null },
                                label = VesselI18n.tr("common.all"),
                            )
                            groups.forEach { grp ->
                                VesselFilterChip(
                                    selected = selectedGroup == grp,
                                    onClick = { selectedGroup = grp },
                                    label = grp,
                                )
                            }
                        }
                    }
                }

                VesselSearchField(
                    value = query,
                    onValueChange = { query = it },
                    placeholder = VesselI18n.tr("iptv.search_channels"),
                    imeAction = ImeAction.Done,
                    modifier = Modifier.fillMaxWidth(),
                )

                // EPG Bar for selected channel if available
                selectedEpgChannel?.let { ch ->
                    epgNowNext?.takeIf { it.now != null || it.next != null }?.let { guide ->
                        Surface(shape = RoundedCornerShape(10.dp), color = palette.bgSurface) {
                            Column(modifier = Modifier.fillMaxWidth().padding(10.dp), verticalArrangement = Arrangement.spacedBy(2.dp)) {
                                Text(ch.name, fontWeight = FontWeight.Bold, style = MaterialTheme.typography.labelLarge)
                                guide.now?.let {
                                    Text("${VesselI18n.tr("iptv.now")}: ${it.title}", style = MaterialTheme.typography.bodySmall, color = palette.accent)
                                }
                                guide.next?.let {
                                    Text("${VesselI18n.tr("iptv.next")}: ${it.title}", style = MaterialTheme.typography.labelSmall, color = palette.textMuted)
                                }
                            }
                        }
                    }
                }

                if (channels.isEmpty()) {
                    Box(modifier = Modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
                        Text(VesselI18n.tr("common.empty"), color = palette.textMuted)
                    }
                } else {
                    val iptvMode = livePrefs.viewMode
                    val iptvIsRow = iptvMode == "compact_list" || iptvMode == "guide_rows"
                    val iptvGridCols = when {
                        iptvIsRow -> GridCells.Fixed(1)
                        livePrefs.columns != null && livePrefs.columns >= 1 -> GridCells.Fixed(livePrefs.columns.coerceIn(1, 5))
                        iptvMode == "showcase_tiles" -> GridCells.Adaptive(minSize = 180.dp)
                        iptvMode == "logo_wall" -> GridCells.Adaptive(minSize = 118.dp)
                        else -> GridCells.Fixed(1)
                    }
                    val iptvSpacing = when (livePrefs.density) {
                        "compact" -> 6.dp
                        "spacious" -> 12.dp
                        else -> 8.dp
                    }
                    val iptvPad = when (livePrefs.density) {
                        "compact" -> 7.dp
                        "spacious" -> 14.dp
                        else -> 10.dp
                    }
                    val iptvLogoShape = liveLogoShape(livePrefs.logoStyle)
                    val iptvTitleLines = livePrefs.titleLines.coerceIn(1, 2)

                    @Composable
                    fun IptvChannelsContent(modifier: Modifier = Modifier) {
                        LazyVerticalGrid(
                            columns = iptvGridCols,
                            verticalArrangement = Arrangement.spacedBy(iptvSpacing),
                            horizontalArrangement = Arrangement.spacedBy(iptvSpacing),
                            contentPadding = PaddingValues(bottom = 24.dp + LocalVesselFloatingNavInset.current),
                            modifier = modifier,
                        ) {
                            itemsIndexed(channels, key = { _, it -> it.id }) { idx, ch ->
                                val chNum = idx + 1
                                Surface(
                                    shape = RoundedCornerShape(10.dp),
                                    color = palette.bgSurface,
                                    modifier = Modifier
                                        .fillMaxWidth()
                                        .then(if (!iptvIsRow && iptvGridCols != GridCells.Fixed(1)) liveAspectModifier(livePrefs.cardAspect) else Modifier)
                                        .clip(RoundedCornerShape(10.dp))
                                        .vesselFocusBorder(RoundedCornerShape(10.dp))
                                        .clickable { playIptvChannel(ch, channels) },
                                ) {
                                    if (iptvMode == "grid" || iptvMode == "showcase_tiles" || iptvMode == "logo_wall") {
                                        Column(
                                            modifier = Modifier.padding(iptvPad),
                                            verticalArrangement = Arrangement.spacedBy(6.dp),
                                            horizontalAlignment = Alignment.CenterHorizontally,
                                        ) {
                                            if (livePrefs.showLogos) {
                                                Box(
                                                    modifier = Modifier
                                                        .fillMaxWidth()
                                                        .height(if (iptvMode == "showcase_tiles") 90.dp else 72.dp)
                                                        .clip(iptvLogoShape)
                                                        .then(if (livePrefs.logoStyle == "plain") Modifier else Modifier.background(palette.bgElevated)),
                                                    contentAlignment = Alignment.Center,
                                                ) {
                                                    if (!ch.logoUrl.isNullOrBlank()) {
                                                        AsyncImage(
                                                            model = ch.logoUrl,
                                                            contentDescription = ch.name,
                                                            contentScale = ContentScale.Fit,
                                                            modifier = Modifier.fillMaxSize().padding(6.dp),
                                                        )
                                                    } else {
                                                        Icon(Icons.Default.LiveTv, contentDescription = null, tint = palette.textMuted, modifier = Modifier.size(22.dp))
                                                    }
                                                }
                                            }
                                            Row(
                                                modifier = Modifier.fillMaxWidth(),
                                                verticalAlignment = Alignment.CenterVertically,
                                                horizontalArrangement = Arrangement.spacedBy(4.dp),
                                            ) {
                                                Column(modifier = Modifier.weight(1f)) {
                                                    Text(
                                                        text = if (livePrefs.showChannelNumbers) "$chNum. ${ch.name}" else ch.name,
                                                        style = MaterialTheme.typography.bodySmall,
                                                        fontWeight = FontWeight.SemiBold,
                                                        maxLines = iptvTitleLines,
                                                        overflow = TextOverflow.Ellipsis,
                                                    )
                                                    if (livePrefs.showCategories && iptvMode != "logo_wall") {
                                                        ch.groupTitle?.let {
                                                            Text(it, style = MaterialTheme.typography.labelSmall, color = palette.textMuted, maxLines = 1, overflow = TextOverflow.Ellipsis)
                                                        }
                                                    }
                                                }
                                                if (livePrefs.showActionButtons && iptvMode != "logo_wall") {
                                                    VesselCompactIconButton(
                                                        size = 28.dp,
                                                        onClick = {
                                                            scope.launch {
                                                                repo.setIptvFavorite(ch.id, !ch.isFavorite)
                                                                reloadProfilesAndChannels()
                                                            }
                                                        },
                                                    ) {
                                                        Icon(
                                                            if (ch.isFavorite) Icons.Default.Star else Icons.Default.StarBorder,
                                                            contentDescription = null,
                                                            tint = if (ch.isFavorite) palette.warning else palette.textMuted,
                                                            modifier = Modifier.size(16.dp),
                                                        )
                                                    }
                                                }
                                            }
                                        }
                                    } else {
                                        Row(
                                            modifier = Modifier.padding(iptvPad),
                                            verticalAlignment = Alignment.CenterVertically,
                                            horizontalArrangement = Arrangement.spacedBy(10.dp),
                                        ) {
                                            if (livePrefs.showChannelNumbers) {
                                                Surface(
                                                    shape = RoundedCornerShape(6.dp),
                                                    color = palette.accent.copy(alpha = 0.16f),
                                                ) {
                                                    Text(
                                                        text = chNum.toString().padStart(2, '0'),
                                                        style = MaterialTheme.typography.labelSmall,
                                                        fontWeight = FontWeight.Bold,
                                                        color = palette.accent,
                                                        modifier = Modifier.padding(horizontal = 7.dp, vertical = 3.dp),
                                                    )
                                                }
                                            }
                                            if (livePrefs.showLogos) {
                                                Box(
                                                    modifier = Modifier
                                                        .size(if (livePrefs.density == "compact") 34.dp else 42.dp)
                                                        .clip(iptvLogoShape)
                                                        .then(if (livePrefs.logoStyle == "plain") Modifier else Modifier.background(palette.bgElevated)),
                                                    contentAlignment = Alignment.Center,
                                                ) {
                                                    if (!ch.logoUrl.isNullOrBlank()) {
                                                        AsyncImage(
                                                            model = ch.logoUrl,
                                                            contentDescription = ch.name,
                                                            contentScale = ContentScale.Fit,
                                                            modifier = Modifier.fillMaxSize().padding(4.dp),
                                                        )
                                                    } else {
                                                        Icon(Icons.Default.LiveTv, contentDescription = null, tint = palette.textMuted, modifier = Modifier.size(20.dp))
                                                    }
                                                }
                                            }
                                            Column(modifier = Modifier.weight(1f)) {
                                                Text(ch.name, fontWeight = FontWeight.SemiBold, maxLines = iptvTitleLines, overflow = TextOverflow.Ellipsis)
                                                if (livePrefs.showCategories) {
                                                    ch.groupTitle?.let {
                                                        Text(it, style = MaterialTheme.typography.labelSmall, color = palette.textMuted, maxLines = 1, overflow = TextOverflow.Ellipsis)
                                                    }
                                                }
                                            }
                                            if (livePrefs.showActionButtons) {
                                                VesselCompactIconButton(
                                                    size = 32.dp,
                                                    onClick = {
                                                        scope.launch {
                                                            repo.setIptvFavorite(ch.id, !ch.isFavorite)
                                                            reloadProfilesAndChannels()
                                                        }
                                                    },
                                                ) {
                                                    Icon(
                                                        if (ch.isFavorite) Icons.Default.Star else Icons.Default.StarBorder,
                                                        contentDescription = null,
                                                        tint = if (ch.isFavorite) palette.warning else palette.textMuted,
                                                        modifier = Modifier.size(18.dp),
                                                    )
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    if (iptvMode == "split_directory" && groups.isNotEmpty()) {
                        Row(modifier = Modifier.fillMaxSize(), horizontalArrangement = Arrangement.spacedBy(12.dp)) {
                            Surface(
                                shape = RoundedCornerShape(12.dp),
                                color = palette.bgSurface,
                                border = BorderStroke(1.dp, palette.border.copy(alpha = 0.45f)),
                                modifier = Modifier
                                    .width(if (isTv) 200.dp else 140.dp)
                                    .fillMaxHeight()
                                    .padding(bottom = 16.dp + LocalVesselFloatingNavInset.current),
                            ) {
                                LazyColumn(
                                    contentPadding = PaddingValues(8.dp),
                                    verticalArrangement = Arrangement.spacedBy(4.dp),
                                ) {
                                    item(key = "iptv_split:all") {
                                        val sel = selectedGroup == null
                                        Surface(
                                            shape = RoundedCornerShape(9.dp),
                                            color = if (sel) palette.accent.copy(alpha = 0.2f) else palette.bgElevated.copy(alpha = 0.35f),
                                            modifier = Modifier
                                                .fillMaxWidth()
                                                .clip(RoundedCornerShape(9.dp))
                                                .vesselFocusBorder(RoundedCornerShape(9.dp))
                                                .clickable { selectedGroup = null },
                                        ) {
                                            Text(
                                                text = VesselI18n.tr("common.all"),
                                                style = MaterialTheme.typography.labelMedium,
                                                fontWeight = if (sel) FontWeight.Bold else FontWeight.Medium,
                                                color = if (sel) palette.accent else palette.textPrimary,
                                                modifier = Modifier.padding(horizontal = 10.dp, vertical = 9.dp),
                                            )
                                        }
                                    }
                                    items(groups, key = { "iptv_split:$it" }) { grp ->
                                        val sel = selectedGroup == grp
                                        Surface(
                                            shape = RoundedCornerShape(9.dp),
                                            color = if (sel) palette.accent.copy(alpha = 0.2f) else palette.bgElevated.copy(alpha = 0.35f),
                                            modifier = Modifier
                                                .fillMaxWidth()
                                                .clip(RoundedCornerShape(9.dp))
                                                .vesselFocusBorder(RoundedCornerShape(9.dp))
                                                .clickable { selectedGroup = grp },
                                        ) {
                                            Text(
                                                text = grp,
                                                style = MaterialTheme.typography.labelMedium,
                                                fontWeight = if (sel) FontWeight.Bold else FontWeight.Medium,
                                                color = if (sel) palette.accent else palette.textPrimary,
                                                maxLines = 1,
                                                overflow = TextOverflow.Ellipsis,
                                                modifier = Modifier.padding(horizontal = 10.dp, vertical = 9.dp),
                                            )
                                        }
                                    }
                                }
                            }
                            IptvChannelsContent(modifier = Modifier.weight(1f).fillMaxHeight())
                        }
                    } else {
                        IptvChannelsContent(modifier = Modifier.fillMaxSize())
                    }
                }
            }
        } else {
            // Tab 0: Live Plugins (Full Desktop Parity)
            if (livePlugins.isEmpty()) {
                Box(modifier = Modifier.fillMaxSize().padding(24.dp), contentAlignment = Alignment.Center) {
                    Text(VesselI18n.tr("iptv.no_live_plugins"), color = palette.textMuted)
                }
            } else {
                val filterQ = pluginChannelFilter.trim().lowercase()
                val filteredSections = remember(orderedSections, selectedSectionId, filterQ, pluginFavoritesOnly, favoriteKeySet, selectedLivePluginId) {
                    orderedSections
                        .filter { selectedSectionId == null || it.id == selectedSectionId }
                        .map { sec ->
                            val items = sec.items.filter { item ->
                                val matchQ = filterQ.isEmpty() || item.title.lowercase().contains(filterQ)
                                val resolvedPid = item.pluginId ?: selectedLivePluginId.orEmpty()
                                val matchFav = !pluginFavoritesOnly || isPluginFavorite(resolvedPid, item.nativeId)
                                matchQ && matchFav
                            }
                            sec.copy(items = items)
                        }
                        .filter { it.items.isNotEmpty() }
                }
                val siblingPairs = remember(filteredSections) {
                    filteredSections.flatMap { sec -> sec.items.map { sec.title to it } }
                }
                val selectedSectionTitle = remember(orderedSections, selectedSectionId) {
                    orderedSections.firstOrNull { it.id == selectedSectionId }?.title
                }

                Column(
                    modifier = Modifier.fillMaxSize().padding(horizontal = 16.dp, vertical = 4.dp),
                    verticalArrangement = Arrangement.spacedBy(10.dp),
                ) {
                    // Live plugin selector chips
                    Row(
                        modifier = Modifier.fillMaxWidth().horizontalScroll(rememberScrollState()),
                        horizontalArrangement = Arrangement.spacedBy(6.dp),
                    ) {
                        livePlugins.forEach { plugin ->
                            VesselFilterChip(
                                selected = selectedLivePluginId == plugin.manifest.id,
                                onClick = {
                                    val switched = selectedLivePluginId != plugin.manifest.id
                                    selectedLivePluginId = plugin.manifest.id
                                    scope.launch { loadPluginSections(plugin.manifest.id, resetSection = switched) }
                                },
                                label = plugin.manifest.name,
                            )
                        }
                    }

                    // Channel search + favorites filter + custom reorder + TV category collapse toggle
                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        horizontalArrangement = Arrangement.spacedBy(8.dp),
                        verticalAlignment = Alignment.CenterVertically,
                    ) {
                        VesselSearchField(
                            value = pluginChannelFilter,
                            onValueChange = { pluginChannelFilter = it },
                            placeholder = VesselI18n.tr("iptv.search_channels"),
                            imeAction = ImeAction.Done,
                            modifier = Modifier.weight(1f),
                        )
                        VesselCompactIconButton(
                            onClick = { pluginFavoritesOnly = !pluginFavoritesOnly },
                            size = 36.dp,
                            containerColor = if (pluginFavoritesOnly) palette.warning.copy(alpha = 0.2f) else palette.bgSurface,
                            borderColor = if (pluginFavoritesOnly) palette.warning else palette.border.copy(alpha = 0.6f),
                        ) {
                            Icon(
                                if (pluginFavoritesOnly) Icons.Default.Star else Icons.Default.StarBorder,
                                contentDescription = VesselI18n.tr("iptv.favorites_only"),
                                tint = if (pluginFavoritesOnly) palette.warning else palette.textMuted,
                                modifier = Modifier.size(18.dp),
                            )
                        }
                        if (livePlugins.size > 1 || orderedSections.size > 1) {
                            VesselCompactIconButton(
                                onClick = { showReorderDialog = true },
                                size = 36.dp,
                                containerColor = palette.bgSurface,
                                borderColor = palette.border.copy(alpha = 0.6f),
                            ) {
                                Icon(
                                    Icons.Default.SwapVert,
                                    contentDescription = VesselI18n.tr("iptv.reorder"),
                                    tint = palette.accent,
                                    modifier = Modifier.size(18.dp),
                                )
                            }
                        }
                        if (isTv && orderedSections.size > 1 && livePrefs.viewMode != "split_directory") {
                            val catBtnActive = state.tvCategoriesExpanded || selectedSectionTitle != null
                            VesselOutlinedButton(
                                onClick = { state.tvCategoriesExpanded = !state.tvCategoriesExpanded },
                                height = 36.dp,
                                contentPadding = PaddingValues(horizontal = 10.dp, vertical = 0.dp),
                            ) {
                                Icon(
                                    Icons.Default.FilterList,
                                    contentDescription = null,
                                    tint = if (catBtnActive) palette.accent else palette.textMuted,
                                    modifier = Modifier.size(15.dp),
                                )
                                Spacer(Modifier.width(5.dp))
                                Text(
                                    text = if (!state.tvCategoriesExpanded && selectedSectionTitle != null) {
                                        selectedSectionTitle
                                    } else {
                                        VesselI18n.tr("iptv.categories_toggle")
                                    },
                                    style = MaterialTheme.typography.labelSmall,
                                    color = if (catBtnActive) palette.accent else palette.textPrimary,
                                    maxLines = 1,
                                    overflow = TextOverflow.Ellipsis,
                                )
                                Spacer(Modifier.width(3.dp))
                                Icon(
                                    if (state.tvCategoriesExpanded) Icons.Default.ExpandLess else Icons.Default.ExpandMore,
                                    contentDescription = null,
                                    modifier = Modifier.size(15.dp),
                                )
                            }
                        }
                    }

                    // Category chips (always visible on mobile, collapsible on Android TV; hidden in split_directory mode)
                    AnimatedVisibility(
                        visible = orderedSections.size > 1 && livePrefs.viewMode != "split_directory" && (!isTv || state.tvCategoriesExpanded),
                        enter = expandVertically(tween(200)) + fadeIn(tween(180)),
                        exit = shrinkVertically(tween(180)) + fadeOut(tween(140)),
                    ) {
                        Row(
                            modifier = Modifier.fillMaxWidth().horizontalScroll(rememberScrollState()),
                            horizontalArrangement = Arrangement.spacedBy(6.dp),
                        ) {
                            VesselFilterChip(
                                selected = selectedSectionId == null,
                                onClick = { selectedSectionId = null },
                                label = VesselI18n.tr("common.all"),
                            )
                            orderedSections.forEach { sec ->
                                VesselFilterChip(
                                    selected = selectedSectionId == sec.id,
                                    onClick = { selectedSectionId = sec.id },
                                    label = "${sec.title} (${sec.items.size})",
                                )
                            }
                        }
                    }

                    if (loadingPluginLive) {
                        Box(modifier = Modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
                            CircularProgressIndicator()
                        }
                    } else if (filteredSections.isEmpty()) {
                        Box(modifier = Modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
                            Text(VesselI18n.tr("common.empty"), color = palette.textMuted)
                        }
                    } else {
                        val effectiveMode = livePrefs.viewMode
                        val isRowMode = effectiveMode == "compact_list" || effectiveMode == "guide_rows"
                        val gridCols = when {
                            isRowMode -> GridCells.Fixed(1)
                            livePrefs.columns != null && livePrefs.columns >= 1 -> GridCells.Fixed(livePrefs.columns.coerceIn(1, 5))
                            effectiveMode == "showcase_tiles" -> GridCells.Adaptive(minSize = 180.dp)
                            effectiveMode == "logo_wall" -> GridCells.Adaptive(minSize = 118.dp)
                            else -> GridCells.Adaptive(minSize = 152.dp)
                        }
                        val cardSpacing = when (livePrefs.density) {
                            "compact" -> 6.dp
                            "spacious" -> 14.dp
                            else -> 10.dp
                        }
                        val cardPadding = when (livePrefs.density) {
                            "compact" -> 7.dp
                            "spacious" -> 13.dp
                            else -> 10.dp
                        }
                        val logoHeight = when {
                            effectiveMode == "showcase_tiles" -> 96.dp
                            effectiveMode == "logo_wall" -> 86.dp
                            livePrefs.density == "compact" -> 58.dp
                            else -> 78.dp
                        }
                        val logoShape = liveLogoShape(livePrefs.logoStyle)
                        val titleMaxLines = livePrefs.titleLines.coerceIn(1, 2)

                        @Composable
                        fun ChannelsGridContent(modifier: Modifier = Modifier) {
                            LazyVerticalGrid(
                                columns = gridCols,
                                contentPadding = PaddingValues(bottom = 24.dp + LocalVesselFloatingNavInset.current),
                                horizontalArrangement = Arrangement.spacedBy(cardSpacing),
                                verticalArrangement = Arrangement.spacedBy(cardSpacing),
                                modifier = modifier,
                            ) {
                                var runningChannelNum = 0
                                filteredSections.forEach { sec ->
                                    if (livePrefs.showSectionHeaders) {
                                        item(key = "sec:${sec.id}", span = { GridItemSpan(maxLineSpan) }) {
                                            Text(
                                                text = "${sec.title} (${sec.items.size})",
                                                style = MaterialTheme.typography.titleMedium,
                                                fontWeight = FontWeight.Bold,
                                                modifier = Modifier.padding(top = 6.dp),
                                            )
                                        }
                                    }
                                    val startNum = runningChannelNum
                                    runningChannelNum += sec.items.size
                                    itemsIndexed(sec.items, key = { idx, it -> "${sec.id}:${idx}:${it.nativeId}" }) { idx, ch ->
                                        val chNum = startNum + idx + 1
                                        val isLaunching = launchingChannelId == ch.nativeId
                                        val resolvedPid = ch.pluginId ?: selectedLivePluginId.orEmpty()
                                        val isFav = isPluginFavorite(resolvedPid, ch.nativeId)
                                        Surface(
                                            shape = RoundedCornerShape(12.dp),
                                            color = palette.bgSurface,
                                            modifier = Modifier
                                                .fillMaxWidth()
                                                .then(if (!isRowMode) liveAspectModifier(livePrefs.cardAspect) else Modifier)
                                                .clip(RoundedCornerShape(12.dp))
                                                .vesselFocusBorder(RoundedCornerShape(12.dp))
                                                .clickable { playPluginLiveItem(ch, sec.title, siblingPairs) },
                                        ) {
                                            when (effectiveMode) {
                                                "compact_list", "guide_rows" -> {
                                                    val isGuide = effectiveMode == "guide_rows"
                                                    Row(
                                                        modifier = Modifier.padding(if (isGuide) cardPadding + 2.dp else cardPadding),
                                                        verticalAlignment = Alignment.CenterVertically,
                                                        horizontalArrangement = Arrangement.spacedBy(10.dp),
                                                    ) {
                                                        if (livePrefs.showChannelNumbers) {
                                                            Surface(
                                                                shape = RoundedCornerShape(6.dp),
                                                                color = palette.accent.copy(alpha = 0.16f),
                                                            ) {
                                                                Text(
                                                                    text = chNum.toString().padStart(2, '0'),
                                                                    style = MaterialTheme.typography.labelSmall,
                                                                    fontWeight = FontWeight.Bold,
                                                                    color = palette.accent,
                                                                    modifier = Modifier.padding(horizontal = 7.dp, vertical = 3.dp),
                                                                )
                                                            }
                                                        }
                                                        if (livePrefs.showLogos) {
                                                            Box(
                                                                modifier = Modifier
                                                                    .size(if (isGuide) 48.dp else 40.dp)
                                                                    .clip(logoShape)
                                                                    .then(if (livePrefs.logoStyle == "plain") Modifier else Modifier.background(palette.bgElevated)),
                                                                contentAlignment = Alignment.Center,
                                                            ) {
                                                                if (!ch.poster.isNullOrBlank()) {
                                                                    AsyncImage(
                                                                        model = ch.poster,
                                                                        contentDescription = ch.title,
                                                                        contentScale = ContentScale.Fit,
                                                                        modifier = Modifier.fillMaxSize().padding(4.dp),
                                                                    )
                                                                } else {
                                                                    Icon(Icons.Default.LiveTv, contentDescription = null, tint = palette.textMuted, modifier = Modifier.size(18.dp))
                                                                }
                                                            }
                                                        }
                                                        Column(modifier = Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(2.dp)) {
                                                            Text(
                                                                text = ch.title,
                                                                style = if (isGuide) MaterialTheme.typography.bodyMedium else MaterialTheme.typography.bodySmall,
                                                                fontWeight = FontWeight.SemiBold,
                                                                maxLines = titleMaxLines,
                                                                overflow = TextOverflow.Ellipsis,
                                                            )
                                                            if (livePrefs.showCategories) {
                                                                Text(
                                                                    text = sec.title,
                                                                    style = MaterialTheme.typography.labelSmall,
                                                                    color = palette.textMuted,
                                                                    maxLines = 1,
                                                                    overflow = TextOverflow.Ellipsis,
                                                                )
                                                            }
                                                        }
                                                        if (livePrefs.showActionButtons) {
                                                            VesselCompactIconButton(
                                                                size = 28.dp,
                                                                onClick = { togglePluginFavorite(ch, sec.title) },
                                                            ) {
                                                                Icon(
                                                                    if (isFav) Icons.Default.Star else Icons.Default.StarBorder,
                                                                    contentDescription = VesselI18n.tr(if (isFav) "iptv.unfavorite_channel" else "iptv.favorite_channel"),
                                                                    tint = if (isFav) palette.warning else palette.textMuted,
                                                                    modifier = Modifier.size(16.dp),
                                                                )
                                                            }
                                                        }
                                                        if (isLaunching) {
                                                            CircularProgressIndicator(modifier = Modifier.size(16.dp), strokeWidth = 2.dp)
                                                        } else if (livePrefs.showActionButtons) {
                                                            Icon(Icons.Default.PlayArrow, contentDescription = null, tint = palette.accent, modifier = Modifier.size(18.dp))
                                                        }
                                                    }
                                                }
                                                "logo_wall" -> {
                                                    Box(modifier = Modifier.fillMaxWidth().padding(cardPadding)) {
                                                        Column(
                                                            horizontalAlignment = Alignment.CenterHorizontally,
                                                            verticalArrangement = Arrangement.spacedBy(6.dp),
                                                            modifier = Modifier.fillMaxWidth(),
                                                        ) {
                                                            Box(
                                                                modifier = Modifier
                                                                    .fillMaxWidth()
                                                                    .height(logoHeight)
                                                                    .clip(logoShape)
                                                                    .then(if (livePrefs.logoStyle == "plain") Modifier else Modifier.background(palette.bgElevated)),
                                                                contentAlignment = Alignment.Center,
                                                            ) {
                                                                if (!ch.poster.isNullOrBlank()) {
                                                                    AsyncImage(
                                                                        model = ch.poster,
                                                                        contentDescription = ch.title,
                                                                        contentScale = ContentScale.Fit,
                                                                        modifier = Modifier.fillMaxSize().padding(10.dp),
                                                                    )
                                                                } else {
                                                                    Icon(Icons.Default.LiveTv, contentDescription = null, tint = palette.textMuted, modifier = Modifier.size(26.dp))
                                                                }
                                                                if (isLaunching) {
                                                                    Box(
                                                                        modifier = Modifier.fillMaxSize().background(palette.bgBase.copy(alpha = 0.7f)),
                                                                        contentAlignment = Alignment.Center,
                                                                    ) {
                                                                        CircularProgressIndicator(modifier = Modifier.size(22.dp), strokeWidth = 2.dp)
                                                                    }
                                                                }
                                                            }
                                                            Text(
                                                                text = if (livePrefs.showChannelNumbers) "$chNum · ${ch.title}" else ch.title,
                                                                style = MaterialTheme.typography.labelSmall,
                                                                fontWeight = FontWeight.SemiBold,
                                                                textAlign = TextAlign.Center,
                                                                maxLines = titleMaxLines,
                                                                overflow = TextOverflow.Ellipsis,
                                                            )
                                                        }
                                                        if (isFav) {
                                                            Icon(
                                                                Icons.Default.Star,
                                                                contentDescription = null,
                                                                tint = palette.warning,
                                                                modifier = Modifier.align(Alignment.TopEnd).size(14.dp),
                                                            )
                                                        }
                                                    }
                                                }
                                                else -> {
                                                    Column(
                                                        modifier = Modifier.padding(cardPadding),
                                                        verticalArrangement = Arrangement.spacedBy(8.dp),
                                                        horizontalAlignment = Alignment.CenterHorizontally,
                                                    ) {
                                                        if (livePrefs.showLogos) {
                                                            Box(
                                                                modifier = Modifier
                                                                    .fillMaxWidth()
                                                                    .height(logoHeight)
                                                                    .clip(logoShape)
                                                                    .then(if (livePrefs.logoStyle == "plain") Modifier else Modifier.background(palette.bgElevated)),
                                                                contentAlignment = Alignment.Center,
                                                            ) {
                                                                if (!ch.poster.isNullOrBlank()) {
                                                                    AsyncImage(
                                                                        model = ch.poster,
                                                                        contentDescription = ch.title,
                                                                        contentScale = ContentScale.Fit,
                                                                        modifier = Modifier.fillMaxSize().padding(8.dp),
                                                                    )
                                                                } else {
                                                                    Icon(Icons.Default.LiveTv, contentDescription = null, tint = palette.textMuted)
                                                                }
                                                                if (livePrefs.showChannelNumbers) {
                                                                    Surface(
                                                                        shape = RoundedCornerShape(6.dp),
                                                                        color = palette.bgBase.copy(alpha = 0.82f),
                                                                        modifier = Modifier.align(Alignment.TopStart).padding(5.dp),
                                                                    ) {
                                                                        Text(
                                                                            text = "$chNum",
                                                                            style = MaterialTheme.typography.labelSmall,
                                                                            fontWeight = FontWeight.Bold,
                                                                            color = palette.accent,
                                                                            modifier = Modifier.padding(horizontal = 5.dp, vertical = 1.dp),
                                                                        )
                                                                    }
                                                                }
                                                                if (isLaunching) {
                                                                    Box(
                                                                        modifier = Modifier.fillMaxSize().background(palette.bgBase.copy(alpha = 0.7f)),
                                                                        contentAlignment = Alignment.Center,
                                                                    ) {
                                                                        CircularProgressIndicator(modifier = Modifier.size(22.dp), strokeWidth = 2.dp)
                                                                    }
                                                                }
                                                            }
                                                        }
                                                        Row(
                                                            modifier = Modifier.fillMaxWidth(),
                                                            verticalAlignment = Alignment.CenterVertically,
                                                            horizontalArrangement = Arrangement.spacedBy(4.dp),
                                                        ) {
                                                            Column(modifier = Modifier.weight(1f)) {
                                                                Text(
                                                                    text = if (!livePrefs.showLogos && livePrefs.showChannelNumbers) "$chNum. ${ch.title}" else ch.title,
                                                                    style = MaterialTheme.typography.bodySmall,
                                                                    fontWeight = FontWeight.SemiBold,
                                                                    maxLines = titleMaxLines,
                                                                    overflow = TextOverflow.Ellipsis,
                                                                )
                                                                if (livePrefs.showCategories) {
                                                                    Text(
                                                                        text = sec.title,
                                                                        style = MaterialTheme.typography.labelSmall,
                                                                        color = palette.textMuted,
                                                                        maxLines = 1,
                                                                        overflow = TextOverflow.Ellipsis,
                                                                    )
                                                                }
                                                            }
                                                            if (livePrefs.showActionButtons) {
                                                                VesselCompactIconButton(
                                                                    size = 26.dp,
                                                                    onClick = { togglePluginFavorite(ch, sec.title) },
                                                                ) {
                                                                    Icon(
                                                                        if (isFav) Icons.Default.Star else Icons.Default.StarBorder,
                                                                        contentDescription = VesselI18n.tr(if (isFav) "iptv.unfavorite_channel" else "iptv.favorite_channel"),
                                                                        tint = if (isFav) palette.warning else palette.textMuted,
                                                                        modifier = Modifier.size(15.dp),
                                                                    )
                                                                }
                                                                Icon(Icons.Default.PlayArrow, contentDescription = null, tint = palette.accent, modifier = Modifier.size(16.dp))
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

                        if (effectiveMode == "split_directory" && orderedSections.size > 1) {
                            Row(
                                modifier = Modifier.fillMaxSize(),
                                horizontalArrangement = Arrangement.spacedBy(12.dp),
                            ) {
                                Surface(
                                    shape = RoundedCornerShape(12.dp),
                                    color = palette.bgSurface,
                                    border = BorderStroke(1.dp, palette.border.copy(alpha = 0.45f)),
                                    modifier = Modifier
                                        .width(if (isTv) 200.dp else 140.dp)
                                        .fillMaxHeight()
                                        .padding(bottom = 16.dp + LocalVesselFloatingNavInset.current),
                                ) {
                                    LazyColumn(
                                        contentPadding = PaddingValues(8.dp),
                                        verticalArrangement = Arrangement.spacedBy(4.dp),
                                    ) {
                                        item(key = "split:all") {
                                            val sel = selectedSectionId == null
                                            Surface(
                                                shape = RoundedCornerShape(9.dp),
                                                color = if (sel) palette.accent.copy(alpha = 0.2f) else palette.bgElevated.copy(alpha = 0.35f),
                                                modifier = Modifier
                                                    .fillMaxWidth()
                                                    .clip(RoundedCornerShape(9.dp))
                                                    .vesselFocusBorder(RoundedCornerShape(9.dp))
                                                    .clickable { selectedSectionId = null },
                                            ) {
                                                Text(
                                                    text = VesselI18n.tr("common.all"),
                                                    style = MaterialTheme.typography.labelMedium,
                                                    fontWeight = if (sel) FontWeight.Bold else FontWeight.Medium,
                                                    color = if (sel) palette.accent else palette.textPrimary,
                                                    maxLines = 1,
                                                    overflow = TextOverflow.Ellipsis,
                                                    modifier = Modifier.padding(horizontal = 10.dp, vertical = 9.dp),
                                                )
                                            }
                                        }
                                        items(orderedSections, key = { "split:${it.id}" }) { sec ->
                                            val sel = selectedSectionId == sec.id
                                            Surface(
                                                shape = RoundedCornerShape(9.dp),
                                                color = if (sel) palette.accent.copy(alpha = 0.2f) else palette.bgElevated.copy(alpha = 0.35f),
                                                modifier = Modifier
                                                    .fillMaxWidth()
                                                    .clip(RoundedCornerShape(9.dp))
                                                    .vesselFocusBorder(RoundedCornerShape(9.dp))
                                                    .clickable { selectedSectionId = sec.id },
                                            ) {
                                                Row(
                                                    modifier = Modifier.padding(horizontal = 10.dp, vertical = 9.dp),
                                                    horizontalArrangement = Arrangement.SpaceBetween,
                                                    verticalAlignment = Alignment.CenterVertically,
                                                ) {
                                                    Text(
                                                        text = sec.title,
                                                        style = MaterialTheme.typography.labelMedium,
                                                        fontWeight = if (sel) FontWeight.Bold else FontWeight.Medium,
                                                        color = if (sel) palette.accent else palette.textPrimary,
                                                        maxLines = 1,
                                                        overflow = TextOverflow.Ellipsis,
                                                        modifier = Modifier.weight(1f),
                                                    )
                                                    Spacer(Modifier.width(4.dp))
                                                    Text(
                                                        text = "${sec.items.size}",
                                                        style = MaterialTheme.typography.labelSmall,
                                                        color = palette.textMuted,
                                                    )
                                                }
                                            }
                                        }
                                    }
                                }
                                ChannelsGridContent(modifier = Modifier.weight(1f).fillMaxHeight())
                            }
                        } else {
                            ChannelsGridContent(modifier = Modifier.fillMaxSize())
                        }
                    }
                }
            }
        }
        }
    }

    if (showAddProfile) {
        var name by remember { mutableStateOf("") }
        var serverUrl by remember { mutableStateOf("") }
        var epgUrl by remember { mutableStateOf("") }
        var username by remember { mutableStateOf("") }
        var password by remember { mutableStateOf("") }
        var isXtream by remember { mutableStateOf(false) }

        AlertDialog(
            onDismissRequest = { showAddProfile = false },
            title = { Text(VesselI18n.tr("iptv.add_profile")) },
            text = {
                Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                    Row(horizontalArrangement = Arrangement.spacedBy(6.dp)) {
                        VesselFilterChip(selected = !isXtream, onClick = { isXtream = false }, label = "M3U+")
                        VesselFilterChip(selected = isXtream, onClick = { isXtream = true }, label = "Xtream")
                    }
                    VesselOutlinedTextField(value = name, onValueChange = { name = it }, label = { Text(VesselI18n.tr("iptv.name")) }, singleLine = true, modifier = Modifier.fillMaxWidth().vesselDpadInputEscape(isTextEmpty = name.isEmpty()))
                    VesselOutlinedTextField(value = serverUrl, onValueChange = { serverUrl = it }, label = { Text(VesselI18n.tr("iptv.server_url")) }, singleLine = true, modifier = Modifier.fillMaxWidth().vesselDpadInputEscape(isTextEmpty = serverUrl.isEmpty()))
                    if (isXtream) {
                        VesselOutlinedTextField(value = username, onValueChange = { username = it }, label = { Text(VesselI18n.tr("iptv.username")) }, singleLine = true, modifier = Modifier.fillMaxWidth().vesselDpadInputEscape(isTextEmpty = username.isEmpty()))
                        VesselOutlinedTextField(value = password, onValueChange = { password = it }, label = { Text(VesselI18n.tr("iptv.password")) }, singleLine = true, modifier = Modifier.fillMaxWidth().vesselDpadInputEscape(isTextEmpty = password.isEmpty()))
                    }
                    VesselOutlinedTextField(value = epgUrl, onValueChange = { epgUrl = it }, label = { Text(VesselI18n.tr("iptv.epg_url")) }, singleLine = true, modifier = Modifier.fillMaxWidth().vesselDpadInputEscape(isTextEmpty = epgUrl.isEmpty()))
                }
            },
            confirmButton = {
                VesselButton(onClick = {
                    if (name.isBlank() || serverUrl.isBlank()) return@VesselButton
                    showAddProfile = false
                    scope.launch {
                        val id = "iptv-${System.currentTimeMillis()}"
                        val created = repo.upsertIptvProfile(
                            UpsertIptvProfilePayload(
                                id = id,
                                name = name.trim(),
                                kind = if (isXtream) "xtream" else "m3u",
                                serverUrl = serverUrl.trim(),
                                username = username.ifBlank { null },
                                password = password.ifBlank { null },
                                epgUrl = epgUrl.ifBlank { null },
                            ),
                        )
                        activeProfileId = created.id
                        repo.syncIptvProfile(created.id)
                        reloadProfilesAndChannels()
                    }
                }) {
                    Text(VesselI18n.tr("common.save"))
                }
            },
            dismissButton = {
                VesselTextButton(onClick = { showAddProfile = false }) { Text(VesselI18n.tr("common.cancel")) }
            },
        )
    }

    if (showViewDialog) {
        AlertDialog(
            onDismissRequest = { showViewDialog = false },
            title = { Text(VesselI18n.tr("iptv.customize_view"), fontWeight = FontWeight.Bold) },
            text = {
                Column(
                    modifier = Modifier
                        .fillMaxWidth()
                        .heightIn(max = 460.dp)
                        .verticalScroll(rememberScrollState()),
                    verticalArrangement = Arrangement.spacedBy(12.dp),
                ) {
                    Text(VesselI18n.tr("iptv.view_layout"), style = MaterialTheme.typography.labelLarge, color = palette.accent, fontWeight = FontWeight.SemiBold)
                    FlowRow(horizontalArrangement = Arrangement.spacedBy(6.dp), verticalArrangement = Arrangement.spacedBy(6.dp)) {
                        listOf("grid", "compact_list", "showcase_tiles", "guide_rows", "logo_wall", "split_directory").forEach { mode ->
                            VesselFilterChip(
                                selected = livePrefs.viewMode == mode,
                                onClick = { saveLivePrefs(livePrefs.copy(viewMode = mode)) },
                                label = VesselI18n.tr("iptv.layout_$mode"),
                            )
                        }
                    }
                    Text(VesselI18n.tr("settings.density"), style = MaterialTheme.typography.labelLarge, color = palette.accent, fontWeight = FontWeight.SemiBold)
                    FlowRow(horizontalArrangement = Arrangement.spacedBy(6.dp), verticalArrangement = Arrangement.spacedBy(6.dp)) {
                        listOf("compact", "comfortable", "spacious").forEach { dens ->
                            VesselFilterChip(
                                selected = livePrefs.density == dens,
                                onClick = { saveLivePrefs(livePrefs.copy(density = dens)) },
                                label = VesselI18n.tr("settings.density_$dens"),
                            )
                        }
                    }
                    Text(VesselI18n.tr("iptv.columns_label"), style = MaterialTheme.typography.labelLarge, color = palette.accent, fontWeight = FontWeight.SemiBold)
                    FlowRow(horizontalArrangement = Arrangement.spacedBy(6.dp), verticalArrangement = Arrangement.spacedBy(6.dp)) {
                        VesselFilterChip(
                            selected = livePrefs.columns == null,
                            onClick = { saveLivePrefs(livePrefs.copy(columns = null)) },
                            label = VesselI18n.tr("iptv.columns_auto"),
                        )
                        listOf(2, 3, 4, 5).forEach { cols ->
                            VesselFilterChip(
                                selected = livePrefs.columns == cols,
                                onClick = { saveLivePrefs(livePrefs.copy(columns = cols)) },
                                label = "$cols",
                            )
                        }
                    }
                    Text(VesselI18n.tr("iptv.logo_style"), style = MaterialTheme.typography.labelLarge, color = palette.accent, fontWeight = FontWeight.SemiBold)
                    FlowRow(horizontalArrangement = Arrangement.spacedBy(6.dp), verticalArrangement = Arrangement.spacedBy(6.dp)) {
                        listOf("rounded", "circle", "square", "plain").forEach { ls ->
                            VesselFilterChip(
                                selected = livePrefs.logoStyle == ls,
                                onClick = { saveLivePrefs(livePrefs.copy(logoStyle = ls)) },
                                label = VesselI18n.tr("iptv.logo_$ls"),
                            )
                        }
                    }
                    Text(VesselI18n.tr("iptv.card_aspect"), style = MaterialTheme.typography.labelLarge, color = palette.accent, fontWeight = FontWeight.SemiBold)
                    FlowRow(horizontalArrangement = Arrangement.spacedBy(6.dp), verticalArrangement = Arrangement.spacedBy(6.dp)) {
                        listOf("auto", "square", "wide_16x9", "classic_4x3").forEach { ar ->
                            VesselFilterChip(
                                selected = livePrefs.cardAspect == ar,
                                onClick = { saveLivePrefs(livePrefs.copy(cardAspect = ar)) },
                                label = VesselI18n.tr("iptv.aspect_$ar"),
                            )
                        }
                    }
                    Text(VesselI18n.tr("iptv.title_lines"), style = MaterialTheme.typography.labelLarge, color = palette.accent, fontWeight = FontWeight.SemiBold)
                    FlowRow(horizontalArrangement = Arrangement.spacedBy(6.dp), verticalArrangement = Arrangement.spacedBy(6.dp)) {
                        listOf(1 to VesselI18n.tr("iptv.lines_1"), 2 to VesselI18n.tr("iptv.lines_2")).forEach { (lines, label) ->
                            VesselFilterChip(
                                selected = livePrefs.titleLines == lines,
                                onClick = { saveLivePrefs(livePrefs.copy(titleLines = lines)) },
                                label = label,
                            )
                        }
                    }
                    listOf(
                        Triple(VesselI18n.tr("iptv.show_logos"), livePrefs.showLogos) { v: Boolean -> saveLivePrefs(livePrefs.copy(showLogos = v)) },
                        Triple(VesselI18n.tr("iptv.show_categories"), livePrefs.showCategories) { v: Boolean -> saveLivePrefs(livePrefs.copy(showCategories = v)) },
                        Triple(VesselI18n.tr("iptv.show_channel_numbers"), livePrefs.showChannelNumbers) { v: Boolean -> saveLivePrefs(livePrefs.copy(showChannelNumbers = v)) },
                        Triple(VesselI18n.tr("iptv.show_favorites_bar"), livePrefs.showFavoritesBar) { v: Boolean -> saveLivePrefs(livePrefs.copy(showFavoritesBar = v)) },
                        Triple(VesselI18n.tr("iptv.show_section_headers"), livePrefs.showSectionHeaders) { v: Boolean -> saveLivePrefs(livePrefs.copy(showSectionHeaders = v)) },
                        Triple(VesselI18n.tr("iptv.show_action_buttons"), livePrefs.showActionButtons) { v: Boolean -> saveLivePrefs(livePrefs.copy(showActionButtons = v)) },
                    ).forEach { (label, checked, onChanged) ->
                        Row(
                            modifier = Modifier.fillMaxWidth(),
                            horizontalArrangement = Arrangement.SpaceBetween,
                            verticalAlignment = Alignment.CenterVertically,
                        ) {
                            Text(label, style = MaterialTheme.typography.bodySmall, modifier = Modifier.weight(1f).padding(end = 8.dp))
                            Switch(checked = checked, onCheckedChange = onChanged)
                        }
                    }
                }
            },
            confirmButton = {
                VesselButton(onClick = { showViewDialog = false }, height = LocalVesselUiTokens.current.compactButtonHeight) {
                    Text(VesselI18n.tr("common.close"), style = MaterialTheme.typography.labelMedium)
                }
            },
        )
    }

    if (showReorderDialog) {
        val activePluginId = selectedLivePluginId
        var pluginDraft by remember(showReorderDialog, livePlugins) { mutableStateOf(livePlugins.map { it.manifest.id }) }
        var sectionDraft by remember(showReorderDialog, orderedSections) { mutableStateOf(orderedSections.map { it.id }) }
        val pluginMapById = remember(livePlugins) { livePlugins.associateBy { it.manifest.id } }
        val sectionMapById = remember(orderedSections) { orderedSections.associateBy { it.id } }
        var draggingPid by remember { mutableStateOf<String?>(null) }
        var dragAccumPluginY by remember { mutableFloatStateOf(0f) }
        var pluginRowHeightPx by remember { mutableIntStateOf(110) }
        var draggingSid by remember { mutableStateOf<String?>(null) }
        var dragAccumSectionY by remember { mutableFloatStateOf(0f) }
        var sectionRowHeightPx by remember { mutableIntStateOf(110) }
        val focusRequesters = remember(showReorderDialog) { mutableMapOf<String, Pair<FocusRequester, FocusRequester>>() }
        var pendingFocusKey by remember { mutableStateOf<Pair<String, Boolean>?>(null) }

        LaunchedEffect(pendingFocusKey, pluginDraft, sectionDraft) {
            val target = pendingFocusKey ?: return@LaunchedEffect
            kotlinx.coroutines.delay(16)
            val pair = focusRequesters[target.first]
            if (pair != null) {
                runCatching {
                    (if (target.second) pair.first else pair.second).requestFocus()
                }
            }
            pendingFocusKey = null
        }

        AlertDialog(
            onDismissRequest = { showReorderDialog = false },
            title = {
                Column(verticalArrangement = Arrangement.spacedBy(2.dp)) {
                    Text(VesselI18n.tr("iptv.reorder_title"), fontWeight = FontWeight.Bold)
                    Text(VesselI18n.tr("home.shelf_order_hint"), style = MaterialTheme.typography.labelSmall, color = palette.textMuted)
                }
            },
            text = {
                Column(
                    modifier = Modifier
                        .fillMaxWidth()
                        .heightIn(max = 420.dp)
                        .verticalScroll(rememberScrollState()),
                    verticalArrangement = Arrangement.spacedBy(10.dp),
                ) {
                    if (pluginDraft.size > 1) {
                        Text(
                            text = VesselI18n.tr("iptv.plugin_order"),
                            style = MaterialTheme.typography.labelLarge,
                            color = palette.accent,
                            fontWeight = FontWeight.SemiBold,
                        )
                        pluginDraft.forEachIndexed { idx, pid ->
                            key("plug:$pid") {
                            val plugin = pluginMapById[pid]
                            val isDragging = draggingPid == pid
                            val (upReq, downReq) = focusRequesters.getOrPut("plug:$pid") { FocusRequester() to FocusRequester() }
                            Surface(
                                shape = RoundedCornerShape(10.dp),
                                color = if (isDragging) palette.accent.copy(alpha = 0.18f) else palette.bgElevated,
                                border = if (isDragging) BorderStroke(1.dp, palette.accent) else null,
                                modifier = Modifier
                                    .fillMaxWidth()
                                    .onSizeChanged { if (it.height > 0) pluginRowHeightPx = it.height },
                            ) {
                                Row(
                                    modifier = Modifier.fillMaxWidth().padding(horizontal = 8.dp, vertical = 6.dp),
                                    verticalAlignment = Alignment.CenterVertically,
                                    horizontalArrangement = Arrangement.spacedBy(6.dp),
                                ) {
                                    Icon(
                                        Icons.Default.DragIndicator,
                                        contentDescription = VesselI18n.tr("home.shelf_order_hint"),
                                        tint = palette.textMuted,
                                        modifier = Modifier
                                            .size(22.dp)
                                            .pointerInput(pid) {
                                                detectVerticalDragGestures(
                                                    onDragStart = {
                                                        draggingPid = pid
                                                        dragAccumPluginY = 0f
                                                    },
                                                    onDragEnd = {
                                                        draggingPid = null
                                                        dragAccumPluginY = 0f
                                                        saveLivePrefs(livePrefs.copy(pluginOrder = pluginDraft))
                                                    },
                                                    onDragCancel = {
                                                        draggingPid = null
                                                        dragAccumPluginY = 0f
                                                    },
                                                    onVerticalDrag = { _, dragAmount ->
                                                        val cur = pluginDraft.indexOf(pid)
                                                        if (cur == -1) return@detectVerticalDragGestures
                                                        dragAccumPluginY += dragAmount
                                                        val stepPx = (pluginRowHeightPx + 12).coerceAtLeast(56)
                                                        if (dragAccumPluginY > stepPx * 0.55f && cur < pluginDraft.lastIndex) {
                                                            val next = pluginDraft.toMutableList()
                                                            next.add(cur + 1, next.removeAt(cur))
                                                            pluginDraft = next
                                                            dragAccumPluginY -= stepPx
                                                        } else if (dragAccumPluginY < -stepPx * 0.55f && cur > 0) {
                                                            val next = pluginDraft.toMutableList()
                                                            next.add(cur - 1, next.removeAt(cur))
                                                            pluginDraft = next
                                                            dragAccumPluginY += stepPx
                                                        }
                                                    },
                                                )
                                            },
                                    )
                                    Text(
                                        text = plugin?.manifest?.name ?: pid,
                                        style = MaterialTheme.typography.bodySmall,
                                        maxLines = 1,
                                        overflow = TextOverflow.Ellipsis,
                                        modifier = Modifier.weight(1f),
                                    )
                                    Row(horizontalArrangement = Arrangement.spacedBy(4.dp)) {
                                        VesselCompactIconButton(
                                            size = 28.dp,
                                            enabled = idx > 0,
                                            modifier = Modifier.focusRequester(upReq),
                                            onClick = {
                                                val ids = pluginDraft.toMutableList()
                                                val newIdx = idx - 1
                                                ids.add(newIdx, ids.removeAt(idx))
                                                pluginDraft = ids
                                                saveLivePrefs(livePrefs.copy(pluginOrder = ids))
                                                pendingFocusKey = "plug:$pid" to (newIdx > 0)
                                            },
                                        ) {
                                            Icon(
                                                Icons.Default.ArrowUpward,
                                                contentDescription = VesselI18n.tr("iptv.move_up"),
                                                tint = if (idx > 0) palette.textPrimary else palette.textMuted.copy(alpha = 0.35f),
                                                modifier = Modifier.size(16.dp),
                                            )
                                        }
                                        VesselCompactIconButton(
                                            size = 28.dp,
                                            enabled = idx < pluginDraft.lastIndex,
                                            modifier = Modifier.focusRequester(downReq),
                                            onClick = {
                                                val ids = pluginDraft.toMutableList()
                                                val newIdx = idx + 1
                                                ids.add(newIdx, ids.removeAt(idx))
                                                pluginDraft = ids
                                                saveLivePrefs(livePrefs.copy(pluginOrder = ids))
                                                pendingFocusKey = "plug:$pid" to (newIdx >= ids.lastIndex)
                                            },
                                        ) {
                                            Icon(
                                                Icons.Default.ArrowDownward,
                                                contentDescription = VesselI18n.tr("iptv.move_down"),
                                                tint = if (idx < pluginDraft.lastIndex) palette.textPrimary else palette.textMuted.copy(alpha = 0.35f),
                                                modifier = Modifier.size(16.dp),
                                            )
                                        }
                                    }
                                }
                            }
                            }
                        }
                    }

                    if (sectionDraft.size > 1 && activePluginId != null) {
                        if (pluginDraft.size > 1) {
                            HorizontalDivider(color = palette.border.copy(alpha = 0.35f))
                        }
                        Text(
                            text = VesselI18n.tr("iptv.section_order"),
                            style = MaterialTheme.typography.labelLarge,
                            color = palette.accent,
                            fontWeight = FontWeight.SemiBold,
                        )
                        sectionDraft.forEachIndexed { idx, sid ->
                            key("sec:$sid") {
                            val sec = sectionMapById[sid]
                            val isDragging = draggingSid == sid
                            val (upReq, downReq) = focusRequesters.getOrPut("sec:$sid") { FocusRequester() to FocusRequester() }
                            Surface(
                                shape = RoundedCornerShape(10.dp),
                                color = if (isDragging) palette.accent.copy(alpha = 0.18f) else palette.bgElevated,
                                border = if (isDragging) BorderStroke(1.dp, palette.accent) else null,
                                modifier = Modifier
                                    .fillMaxWidth()
                                    .onSizeChanged { if (it.height > 0) sectionRowHeightPx = it.height },
                            ) {
                                Row(
                                    modifier = Modifier.fillMaxWidth().padding(horizontal = 8.dp, vertical = 6.dp),
                                    verticalAlignment = Alignment.CenterVertically,
                                    horizontalArrangement = Arrangement.spacedBy(6.dp),
                                ) {
                                    Icon(
                                        Icons.Default.DragIndicator,
                                        contentDescription = VesselI18n.tr("home.shelf_order_hint"),
                                        tint = palette.textMuted,
                                        modifier = Modifier
                                            .size(22.dp)
                                            .pointerInput(sid) {
                                                detectVerticalDragGestures(
                                                    onDragStart = {
                                                        draggingSid = sid
                                                        dragAccumSectionY = 0f
                                                    },
                                                    onDragEnd = {
                                                        draggingSid = null
                                                        dragAccumSectionY = 0f
                                                        val map = livePrefs.sectionOrderByPlugin.toMutableMap()
                                                        map[activePluginId] = sectionDraft
                                                        saveLivePrefs(livePrefs.copy(sectionOrderByPlugin = map))
                                                    },
                                                    onDragCancel = {
                                                        draggingSid = null
                                                        dragAccumSectionY = 0f
                                                    },
                                                    onVerticalDrag = { _, dragAmount ->
                                                        val cur = sectionDraft.indexOf(sid)
                                                        if (cur == -1) return@detectVerticalDragGestures
                                                        dragAccumSectionY += dragAmount
                                                        val stepPx = (sectionRowHeightPx + 12).coerceAtLeast(56)
                                                        if (dragAccumSectionY > stepPx * 0.55f && cur < sectionDraft.lastIndex) {
                                                            val next = sectionDraft.toMutableList()
                                                            next.add(cur + 1, next.removeAt(cur))
                                                            sectionDraft = next
                                                            dragAccumSectionY -= stepPx
                                                        } else if (dragAccumSectionY < -stepPx * 0.55f && cur > 0) {
                                                            val next = sectionDraft.toMutableList()
                                                            next.add(cur - 1, next.removeAt(cur))
                                                            sectionDraft = next
                                                            dragAccumSectionY += stepPx
                                                        }
                                                    },
                                                )
                                            },
                                    )
                                    Text(
                                        text = sec?.title ?: sid,
                                        style = MaterialTheme.typography.bodySmall,
                                        maxLines = 1,
                                        overflow = TextOverflow.Ellipsis,
                                        modifier = Modifier.weight(1f),
                                    )
                                    Row(horizontalArrangement = Arrangement.spacedBy(4.dp)) {
                                        VesselCompactIconButton(
                                            size = 28.dp,
                                            enabled = idx > 0,
                                            modifier = Modifier.focusRequester(upReq),
                                            onClick = {
                                                val ids = sectionDraft.toMutableList()
                                                val newIdx = idx - 1
                                                ids.add(newIdx, ids.removeAt(idx))
                                                sectionDraft = ids
                                                val map = livePrefs.sectionOrderByPlugin.toMutableMap()
                                                map[activePluginId] = ids
                                                saveLivePrefs(livePrefs.copy(sectionOrderByPlugin = map))
                                                pendingFocusKey = "sec:$sid" to (newIdx > 0)
                                            },
                                        ) {
                                            Icon(
                                                Icons.Default.ArrowUpward,
                                                contentDescription = VesselI18n.tr("iptv.move_up"),
                                                tint = if (idx > 0) palette.textPrimary else palette.textMuted.copy(alpha = 0.35f),
                                                modifier = Modifier.size(16.dp),
                                            )
                                        }
                                        VesselCompactIconButton(
                                            size = 28.dp,
                                            enabled = idx < sectionDraft.lastIndex,
                                            modifier = Modifier.focusRequester(downReq),
                                            onClick = {
                                                val ids = sectionDraft.toMutableList()
                                                val newIdx = idx + 1
                                                ids.add(newIdx, ids.removeAt(idx))
                                                sectionDraft = ids
                                                val map = livePrefs.sectionOrderByPlugin.toMutableMap()
                                                map[activePluginId] = ids
                                                saveLivePrefs(livePrefs.copy(sectionOrderByPlugin = map))
                                                pendingFocusKey = "sec:$sid" to (newIdx >= ids.lastIndex)
                                            },
                                        ) {
                                            Icon(
                                                Icons.Default.ArrowDownward,
                                                contentDescription = VesselI18n.tr("iptv.move_down"),
                                                tint = if (idx < sectionDraft.lastIndex) palette.textPrimary else palette.textMuted.copy(alpha = 0.35f),
                                                modifier = Modifier.size(16.dp),
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
                VesselButton(onClick = { showReorderDialog = false }, height = LocalVesselUiTokens.current.compactButtonHeight) {
                    Text(VesselI18n.tr("common.close"), style = MaterialTheme.typography.labelMedium)
                }
            },
            dismissButton = {
                VesselTextButton(onClick = {
                    val map = livePrefs.sectionOrderByPlugin.toMutableMap()
                    if (activePluginId != null) map.remove(activePluginId)
                    saveLivePrefs(livePrefs.copy(pluginOrder = emptyList(), sectionOrderByPlugin = map))
                    showReorderDialog = false
                }) {
                    Icon(Icons.Default.RestartAlt, contentDescription = null, modifier = Modifier.size(15.dp))
                    Spacer(Modifier.width(4.dp))
                    Text(VesselI18n.tr("iptv.reset_order"))
                }
            },
        )
    }
}
