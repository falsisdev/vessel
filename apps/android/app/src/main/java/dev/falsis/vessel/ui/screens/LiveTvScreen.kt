package dev.falsis.vessel.ui.screens

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
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Add
import androidx.compose.material.icons.filled.Star
import androidx.compose.material.icons.filled.StarBorder
import androidx.compose.material.icons.filled.Sync
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.FilterChip
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
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
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import dev.falsis.vessel.data.StoredIptvChannel
import dev.falsis.vessel.data.StoredIptvProfile
import dev.falsis.vessel.data.UpsertIptvProfilePayload
import dev.falsis.vessel.data.VesselRepository
import dev.falsis.vessel.ui.i18n.VesselI18n
import dev.falsis.vessel.ui.theme.LocalVesselPalette
import kotlinx.coroutines.launch

@Composable
fun LiveTvScreen(
    repo: VesselRepository,
    onPlayUrl: (title: String, url: String) -> Unit,
    onOpenDetails: (pluginId: String, nativeId: String) -> Unit,
) {
    val palette = LocalVesselPalette.current
    val scope = rememberCoroutineScope()
    val plugins by repo.plugins.collectAsState()
    var tab by remember { mutableIntStateOf(0) }
    var profiles by remember { mutableStateOf<List<StoredIptvProfile>>(emptyList()) }
    var channels by remember { mutableStateOf<List<StoredIptvChannel>>(emptyList()) }
    var query by remember { mutableStateOf("") }
    var favoritesOnly by remember { mutableStateOf(false) }
    var showAddProfile by remember { mutableStateOf(false) }

    suspend fun reloadIptv() {
        runCatching { profiles = repo.listIptvProfiles() }
        runCatching { channels = repo.searchIptvChannels(query = query.ifBlank { null }, favoritesOnly = favoritesOnly) }
    }

    LaunchedEffect(query, favoritesOnly) { reloadIptv() }

    Column(modifier = Modifier.fillMaxSize().background(palette.bgBase)) {
        TabRow(selectedTabIndex = tab) {
            Tab(selected = tab == 0, onClick = { tab = 0 }, text = { Text(VesselI18n.tr("iptv.tab_playlists")) })
            Tab(selected = tab == 1, onClick = { tab = 1 }, text = { Text(VesselI18n.tr("iptv.tab_plugins")) })
        }

        if (tab == 0) {
            Column(modifier = Modifier.fillMaxSize().padding(16.dp), verticalArrangement = Arrangement.spacedBy(10.dp)) {
                Row(
                    modifier = Modifier.fillMaxWidth(),
                    horizontalArrangement = Arrangement.SpaceBetween,
                    verticalAlignment = Alignment.CenterVertically,
                ) {
                    Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                        Button(onClick = { showAddProfile = true }) {
                            Icon(Icons.Default.Add, contentDescription = null)
                            Text(VesselI18n.tr("iptv.add_profile"))
                        }
                        FilterChip(
                            selected = favoritesOnly,
                            onClick = { favoritesOnly = !favoritesOnly },
                            label = { Text(VesselI18n.tr("iptv.favorites_only")) },
                        )
                    }
                    profiles.firstOrNull()?.let { first ->
                        IconButton(onClick = {
                            scope.launch {
                                repo.syncIptvProfile(first.id)
                                runCatching { repo.syncIptvEpg(first.id) }
                                reloadIptv()
                            }
                        }) {
                            Icon(Icons.Default.Sync, contentDescription = VesselI18n.tr("iptv.sync_channels"))
                        }
                    }
                }

                OutlinedTextField(
                    value = query,
                    onValueChange = { query = it },
                    placeholder = { Text(VesselI18n.tr("iptv.search_channels")) },
                    singleLine = true,
                    modifier = Modifier.fillMaxWidth(),
                )

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
                                    scope.launch {
                                        runCatching { repo.playIptvChannel(ch, ch.userAgent) }
                                            .onSuccess { url -> onPlayUrl(ch.name, url) }
                                    }
                                },
                        ) {
                            Row(
                                modifier = Modifier.padding(12.dp),
                                verticalAlignment = Alignment.CenterVertically,
                                horizontalArrangement = Arrangement.SpaceBetween,
                            ) {
                                Column(modifier = Modifier.weight(1f)) {
                                    Text(ch.name, fontWeight = FontWeight.SemiBold)
                                    ch.groupTitle?.let {
                                        Text(it, style = MaterialTheme.typography.labelSmall, color = palette.textMuted)
                                    }
                                }
                                IconButton(onClick = {
                                    scope.launch {
                                        repo.setIptvFavorite(ch.id, !ch.isFavorite)
                                        reloadIptv()
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
        } else {
            val livePlugins = remember(plugins) {
                plugins.filter { it.enabled && ("live_tv" in it.manifest.capabilities || "live" in it.manifest.contentTypes) }
            }
            if (livePlugins.isEmpty()) {
                Box(modifier = Modifier.fillMaxSize().padding(24.dp), contentAlignment = Alignment.Center) {
                    Text(VesselI18n.tr("iptv.no_live_plugins"), color = palette.textMuted)
                }
            } else {
                LazyColumn(
                    modifier = Modifier.fillMaxSize().padding(16.dp),
                    verticalArrangement = Arrangement.spacedBy(10.dp),
                ) {
                    items(livePlugins, key = { it.manifest.id }) { plugin ->
                        Surface(
                            shape = RoundedCornerShape(12.dp),
                            color = palette.bgSurface,
                            modifier = Modifier
                                .fillMaxWidth()
                                .clickable {
                                    scope.launch {
                                        val feeds = runCatching { repo.getHomeFeeds(listOf(plugin.manifest.id)) }.getOrNull()
                                        val first = feeds?.firstOrNull()?.sections?.firstOrNull()?.items?.firstOrNull()
                                        if (first != null) {
                                            onOpenDetails(plugin.manifest.id, first.nativeId)
                                        }
                                    }
                                },
                        ) {
                            Column(modifier = Modifier.padding(14.dp)) {
                                Text(plugin.manifest.name, fontWeight = FontWeight.Bold)
                                plugin.manifest.description?.let {
                                    Spacer(Modifier.height(4.dp))
                                    Text(it, style = MaterialTheme.typography.bodySmall, color = palette.textMuted)
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
                        repo.syncIptvProfile(created.id)
                        reloadIptv()
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
