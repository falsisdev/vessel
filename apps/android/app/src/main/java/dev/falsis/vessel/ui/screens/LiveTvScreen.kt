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
import androidx.compose.foundation.lazy.grid.GridCells
import androidx.compose.foundation.lazy.grid.GridItemSpan
import androidx.compose.foundation.lazy.grid.LazyVerticalGrid
import androidx.compose.foundation.lazy.grid.items
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Add
import androidx.compose.material.icons.filled.Delete
import androidx.compose.material.icons.filled.LiveTv
import androidx.compose.material.icons.filled.PlayArrow
import androidx.compose.material.icons.filled.Search
import androidx.compose.material.icons.filled.Star
import androidx.compose.material.icons.filled.StarBorder
import androidx.compose.material.icons.filled.Sync
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.FilterChip
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Surface
import androidx.compose.material3.Tab
import androidx.compose.material3.TabRow
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import coil3.compose.AsyncImage
import dev.falsis.vessel.data.EpgNowNext
import dev.falsis.vessel.data.Episode
import dev.falsis.vessel.data.HomeSection
import dev.falsis.vessel.data.MediaDetail
import dev.falsis.vessel.data.MediaSummary
import dev.falsis.vessel.data.PluginListEntry
import dev.falsis.vessel.data.StoredIptvChannel
import dev.falsis.vessel.data.StoredIptvProfile
import dev.falsis.vessel.data.StreamCandidate
import dev.falsis.vessel.data.StreamResult
import dev.falsis.vessel.data.UpsertIptvProfilePayload
import dev.falsis.vessel.data.VesselRepository
import dev.falsis.vessel.ui.i18n.VesselI18n
import dev.falsis.vessel.ui.theme.LocalVesselPalette
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch

private fun isLiveTvPlugin(p: PluginListEntry): Boolean {
    if (!p.enabled) return false
    if ("live_tv" in p.manifest.capabilities || "live" in p.manifest.contentTypes) return true
    val hay = "${p.manifest.id} ${p.manifest.name} ${p.manifest.description ?: ""}".lowercase()
    return "iptv" in hay || "canlı tv" in hay || "canli tv" in hay || "live tv" in hay || "_spor" in hay
}

@Composable
fun LiveTvScreen(
    repo: VesselRepository,
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
    val scope = rememberCoroutineScope()
    val plugins by repo.plugins.collectAsState()
    var tab by remember { mutableIntStateOf(0) }

    // Tab 0: IPTV Playlists
    var profiles by remember { mutableStateOf<List<StoredIptvProfile>>(emptyList()) }
    var activeProfileId by remember { mutableStateOf<String?>(null) }
    var groups by remember { mutableStateOf<List<String>>(emptyList()) }
    var selectedGroup by remember { mutableStateOf<String?>(null) }
    var channels by remember { mutableStateOf<List<StoredIptvChannel>>(emptyList()) }
    var query by remember { mutableStateOf("") }
    var favoritesOnly by remember { mutableStateOf(false) }
    var syncingProfileId by remember { mutableStateOf<String?>(null) }
    var confirmDeleteProfileId by remember { mutableStateOf<String?>(null) }
    var selectedEpgChannel by remember { mutableStateOf<StoredIptvChannel?>(null) }
    var epgNowNext by remember { mutableStateOf<EpgNowNext?>(null) }
    var showAddProfile by remember { mutableStateOf(false) }

    // Tab 1: Live Plugins (exact parity with Desktop IptvView.vue)
    val livePlugins = remember(plugins) { plugins.filter(::isLiveTvPlugin) }
    var selectedLivePluginId by remember { mutableStateOf<String?>(null) }
    var pluginLiveSections by remember { mutableStateOf<List<HomeSection>>(emptyList()) }
    var selectedSectionId by remember { mutableStateOf<String?>(null) }
    var pluginChannelFilter by remember { mutableStateOf("") }
    var loadingPluginLive by remember { mutableStateOf(false) }
    var launchingChannelId by remember { mutableStateOf<String?>(null) }

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
    }

    LaunchedEffect(activeProfileId, selectedGroup, query, favoritesOnly) {
        reloadProfilesAndChannels()
    }

    suspend fun loadPluginSections(pid: String) {
        loadingPluginLive = true
        selectedSectionId = null
        runCatching { repo.getHomeFeeds(listOf(pid)) }
            .onSuccess { list ->
                val feed = list.firstOrNull()
                if (!feed?.error.isNullOrBlank()) {
                    repo.notify("${feed?.pluginName ?: pid}: ${feed?.error}")
                }
                pluginLiveSections = feed?.sections ?: emptyList()
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
            if (selectedLivePluginId != current || pluginLiveSections.isEmpty()) {
                selectedLivePluginId = current
                loadPluginSections(current)
            }
        }
    }

    fun playPluginLiveItem(item: MediaSummary, sectionTitle: String) {
        val pid = item.pluginId ?: selectedLivePluginId ?: return
        if (launchingChannelId != null) return
        launchingChannelId = item.nativeId
        scope.launch {
            runCatching {
                val detail = repo.getDetails(pid, item.nativeId)
                val ep = detail.episodes.firstOrNull() ?: Episode(
                    streamId = item.nativeId,
                    season = 1,
                    episode = 1,
                    title = item.title,
                    overview = sectionTitle,
                    thumbnail = item.poster,
                )
                val sr = repo.getStreams(pid, ep.streamId)
                if (sr.streams.isEmpty()) {
                    repo.notify("No playable live streams returned for ${item.title}")
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
                        emptyMap(),
                    )
                }
            }.onFailure {
                repo.notifyError(it, "Failed to play ${item.title}")
            }
            launchingChannelId = null
        }
    }

    Column(modifier = Modifier.fillMaxSize().background(palette.bgBase)) {
        TabRow(selectedTabIndex = tab) {
            Tab(selected = tab == 0, onClick = { tab = 0 }, text = { Text(VesselI18n.tr("iptv.tab_plugins")) })
            Tab(selected = tab == 1, onClick = { tab = 1 }, text = { Text(VesselI18n.tr("iptv.tab_playlists")) })
        }

        if (tab == 1) {
            Column(
                modifier = Modifier.fillMaxSize().padding(16.dp),
                verticalArrangement = Arrangement.spacedBy(10.dp),
            ) {
                // Profiles Row
                Row(
                    modifier = Modifier.fillMaxWidth().horizontalScroll(rememberScrollState()),
                    horizontalArrangement = Arrangement.spacedBy(8.dp),
                    verticalAlignment = Alignment.CenterVertically,
                ) {
                    Button(onClick = { showAddProfile = true }) {
                        Icon(Icons.Default.Add, contentDescription = null, modifier = Modifier.size(16.dp))
                        Spacer(Modifier.width(4.dp))
                        Text(VesselI18n.tr("iptv.add_profile"))
                    }
                    profiles.forEach { profile ->
                        FilterChip(
                            selected = activeProfileId == profile.id,
                            onClick = {
                                activeProfileId = profile.id
                                selectedGroup = null
                            },
                            label = { Text(profile.name) },
                        )
                    }
                }

                // Active Profile Actions + Favorites Filter
                Row(
                    modifier = Modifier.fillMaxWidth(),
                    horizontalArrangement = Arrangement.SpaceBetween,
                    verticalAlignment = Alignment.CenterVertically,
                ) {
                    FilterChip(
                        selected = favoritesOnly,
                        onClick = { favoritesOnly = !favoritesOnly },
                        label = { Text(VesselI18n.tr("iptv.favorites_only")) },
                    )
                    activeProfileId?.let { pid ->
                        Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(4.dp)) {
                            OutlinedButton(
                                enabled = syncingProfileId != pid,
                                onClick = {
                                    scope.launch {
                                        syncingProfileId = pid
                                        runCatching {
                                            val count = repo.syncIptvProfile(pid)
                                            runCatching { repo.syncIptvEpg(pid) }
                                            reloadProfilesAndChannels()
                                            repo.notify("Synced $count channels")
                                        }.onFailure { repo.notify(it.message ?: "Sync failed") }
                                        syncingProfileId = null
                                    }
                                },
                            ) {
                                Icon(Icons.Default.Sync, contentDescription = null, modifier = Modifier.size(15.dp))
                                Spacer(Modifier.width(4.dp))
                                Text(VesselI18n.tr("iptv.sync_channels"))
                            }
                            val confirming = confirmDeleteProfileId == pid
                            IconButton(onClick = {
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
                            }) {
                                Icon(
                                    Icons.Default.Delete,
                                    contentDescription = VesselI18n.tr("common.delete"),
                                    tint = if (confirming) palette.warning else palette.danger,
                                )
                            }
                        }
                    }
                }

                // Group Chips
                if (groups.isNotEmpty()) {
                    Row(
                        modifier = Modifier.fillMaxWidth().horizontalScroll(rememberScrollState()),
                        horizontalArrangement = Arrangement.spacedBy(6.dp),
                    ) {
                        FilterChip(
                            selected = selectedGroup == null,
                            onClick = { selectedGroup = null },
                            label = { Text(VesselI18n.tr("common.all", "All")) },
                        )
                        groups.forEach { grp ->
                            FilterChip(
                                selected = selectedGroup == grp,
                                onClick = { selectedGroup = grp },
                                label = { Text(grp) },
                            )
                        }
                    }
                }

                OutlinedTextField(
                    value = query,
                    onValueChange = { query = it },
                    placeholder = { Text(VesselI18n.tr("iptv.search_channels")) },
                    leadingIcon = { Icon(Icons.Default.Search, contentDescription = null) },
                    singleLine = true,
                    modifier = Modifier.fillMaxWidth(),
                )

                // EPG Bar for selected channel if available
                selectedEpgChannel?.let { ch ->
                    epgNowNext?.takeIf { it.now != null || it.next != null }?.let { guide ->
                        Surface(shape = RoundedCornerShape(10.dp), color = palette.bgSurface) {
                            Column(modifier = Modifier.fillMaxWidth().padding(10.dp), verticalArrangement = Arrangement.spacedBy(2.dp)) {
                                Text(ch.name, fontWeight = FontWeight.Bold, style = MaterialTheme.typography.labelLarge)
                                guide.now?.let {
                                    Text("Now: ${it.title}", style = MaterialTheme.typography.bodySmall, color = palette.accent)
                                }
                                guide.next?.let {
                                    Text("Next: ${it.title}", style = MaterialTheme.typography.labelSmall, color = palette.textMuted)
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
                    LazyColumn(
                        verticalArrangement = Arrangement.spacedBy(8.dp),
                        contentPadding = PaddingValues(bottom = 24.dp),
                    ) {
                        items(channels, key = { it.id }) { ch ->
                            Surface(
                                shape = RoundedCornerShape(10.dp),
                                color = palette.bgSurface,
                                modifier = Modifier
                                    .fillMaxWidth()
                                    .clickable {
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
                                            val episodes = channels.mapIndexed { idx, item ->
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
                                    },
                            ) {
                                Row(
                                    modifier = Modifier.padding(10.dp),
                                    verticalAlignment = Alignment.CenterVertically,
                                    horizontalArrangement = Arrangement.spacedBy(10.dp),
                                ) {
                                    Box(
                                        modifier = Modifier
                                            .size(42.dp)
                                            .clip(RoundedCornerShape(8.dp))
                                            .background(palette.bgElevated),
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
                                    Column(modifier = Modifier.weight(1f)) {
                                        Text(ch.name, fontWeight = FontWeight.SemiBold, maxLines = 1, overflow = TextOverflow.Ellipsis)
                                        ch.groupTitle?.let {
                                            Text(it, style = MaterialTheme.typography.labelSmall, color = palette.textMuted)
                                        }
                                    }
                                    IconButton(onClick = {
                                        scope.launch {
                                            repo.setIptvFavorite(ch.id, !ch.isFavorite)
                                            reloadProfilesAndChannels()
                                        }
                                    }) {
                                        Icon(
                                            if (ch.isFavorite) Icons.Default.Star else Icons.Default.StarBorder,
                                            contentDescription = null,
                                            tint = if (ch.isFavorite) palette.warning else palette.textMuted,
                                        )
                                    }
                                }
                            }
                        }
                    }
                }
            }
        } else {
            // Tab 1: Live Plugins (Full Desktop Parity)
            if (livePlugins.isEmpty()) {
                Box(modifier = Modifier.fillMaxSize().padding(24.dp), contentAlignment = Alignment.Center) {
                    Text(VesselI18n.tr("iptv.no_live_plugins"), color = palette.textMuted)
                }
            } else {
                val filterQ = pluginChannelFilter.trim().lowercase()
                val filteredSections = remember(pluginLiveSections, selectedSectionId, filterQ) {
                    pluginLiveSections
                        .filter { selectedSectionId == null || it.id == selectedSectionId }
                        .map { sec ->
                            val items = if (filterQ.isEmpty()) sec.items else sec.items.filter { it.title.lowercase().contains(filterQ) }
                            sec.copy(items = items)
                        }
                        .filter { it.items.isNotEmpty() }
                }

                Column(
                    modifier = Modifier.fillMaxSize().padding(16.dp),
                    verticalArrangement = Arrangement.spacedBy(10.dp),
                ) {
                    // Live plugin selector chips
                    Row(
                        modifier = Modifier.fillMaxWidth().horizontalScroll(rememberScrollState()),
                        horizontalArrangement = Arrangement.spacedBy(8.dp),
                    ) {
                        livePlugins.forEach { plugin ->
                            FilterChip(
                                selected = selectedLivePluginId == plugin.manifest.id,
                                onClick = {
                                    selectedLivePluginId = plugin.manifest.id
                                    scope.launch { loadPluginSections(plugin.manifest.id) }
                                },
                                label = { Text(plugin.manifest.name) },
                            )
                        }
                    }

                    // Channel search
                    OutlinedTextField(
                        value = pluginChannelFilter,
                        onValueChange = { pluginChannelFilter = it },
                        placeholder = { Text(VesselI18n.tr("iptv.search_channels")) },
                        leadingIcon = { Icon(Icons.Default.Search, contentDescription = null) },
                        singleLine = true,
                        modifier = Modifier.fillMaxWidth(),
                    )

                    // Category chips (Ulusal, Spor, Haber, Belgesel...)
                    if (pluginLiveSections.size > 1) {
                        Row(
                            modifier = Modifier.fillMaxWidth().horizontalScroll(rememberScrollState()),
                            horizontalArrangement = Arrangement.spacedBy(6.dp),
                        ) {
                            FilterChip(
                                selected = selectedSectionId == null,
                                onClick = { selectedSectionId = null },
                                label = { Text(VesselI18n.tr("common.all", "All")) },
                            )
                            pluginLiveSections.forEach { sec ->
                                FilterChip(
                                    selected = selectedSectionId == sec.id,
                                    onClick = { selectedSectionId = sec.id },
                                    label = { Text("${sec.title} (${sec.items.size})") },
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
                        LazyVerticalGrid(
                            columns = GridCells.Adaptive(minSize = 152.dp),
                            contentPadding = PaddingValues(bottom = 24.dp),
                            horizontalArrangement = Arrangement.spacedBy(10.dp),
                            verticalArrangement = Arrangement.spacedBy(10.dp),
                            modifier = Modifier.fillMaxSize(),
                        ) {
                            filteredSections.forEach { sec ->
                                item(key = "sec:${sec.id}", span = { GridItemSpan(maxLineSpan) }) {
                                    Text(
                                        text = "${sec.title} (${sec.items.size})",
                                        style = MaterialTheme.typography.titleMedium,
                                        fontWeight = FontWeight.Bold,
                                        modifier = Modifier.padding(top = 6.dp),
                                    )
                                }
                                items(sec.items, key = { "${sec.id}:${it.nativeId}" }) { ch ->
                                    val isLaunching = launchingChannelId == ch.nativeId
                                    Surface(
                                        shape = RoundedCornerShape(12.dp),
                                        color = palette.bgSurface,
                                        modifier = Modifier
                                            .fillMaxWidth()
                                            .clip(RoundedCornerShape(12.dp))
                                            .clickable { playPluginLiveItem(ch, sec.title) },
                                    ) {
                                        Column(
                                            modifier = Modifier.padding(10.dp),
                                            verticalArrangement = Arrangement.spacedBy(8.dp),
                                            horizontalAlignment = Alignment.CenterHorizontally,
                                        ) {
                                            Box(
                                                modifier = Modifier
                                                    .fillMaxWidth()
                                                    .height(78.dp)
                                                    .clip(RoundedCornerShape(8.dp))
                                                    .background(palette.bgElevated),
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
                                                if (isLaunching) {
                                                    Box(
                                                        modifier = Modifier.fillMaxSize().background(palette.bgBase.copy(alpha = 0.7f)),
                                                        contentAlignment = Alignment.Center,
                                                    ) {
                                                        CircularProgressIndicator(modifier = Modifier.size(22.dp), strokeWidth = 2.dp)
                                                    }
                                                }
                                            }
                                            Row(
                                                modifier = Modifier.fillMaxWidth(),
                                                verticalAlignment = Alignment.CenterVertically,
                                                horizontalArrangement = Arrangement.SpaceBetween,
                                            ) {
                                                Text(
                                                    text = ch.title,
                                                    style = MaterialTheme.typography.bodySmall,
                                                    fontWeight = FontWeight.SemiBold,
                                                    maxLines = 1,
                                                    overflow = TextOverflow.Ellipsis,
                                                    modifier = Modifier.weight(1f),
                                                )
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
                    Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                        FilterChip(selected = !isXtream, onClick = { isXtream = false }, label = { Text("M3U+") })
                        FilterChip(selected = isXtream, onClick = { isXtream = true }, label = { Text("Xtream") })
                    }
                    OutlinedTextField(value = name, onValueChange = { name = it }, label = { Text("Name") }, singleLine = true)
                    OutlinedTextField(value = serverUrl, onValueChange = { serverUrl = it }, label = { Text("URL / Server") }, singleLine = true)
                    if (isXtream) {
                        OutlinedTextField(value = username, onValueChange = { username = it }, label = { Text("Username") }, singleLine = true)
                        OutlinedTextField(value = password, onValueChange = { password = it }, label = { Text("Password") }, singleLine = true)
                    }
                    OutlinedTextField(value = epgUrl, onValueChange = { epgUrl = it }, label = { Text("XMLTV EPG URL (optional)") }, singleLine = true)
                }
            },
            confirmButton = {
                Button(onClick = {
                    if (name.isBlank() || serverUrl.isBlank()) return@Button
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
                TextButton(onClick = { showAddProfile = false }) { Text(VesselI18n.tr("common.cancel")) }
            },
        )
    }
}
