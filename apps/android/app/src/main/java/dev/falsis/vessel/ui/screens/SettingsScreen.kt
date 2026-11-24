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
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.KeyboardArrowRight
import androidx.compose.material.icons.filled.Download
import androidx.compose.material.icons.filled.Extension
import androidx.compose.material3.Button
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Surface
import androidx.compose.material3.Switch
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import dev.falsis.vessel.data.VesselRepository
import dev.falsis.vessel.ui.i18n.VesselI18n
import dev.falsis.vessel.ui.theme.DYNAMIC_MATERIAL_THEME_ID
import dev.falsis.vessel.ui.theme.LocalVesselPalette
import kotlinx.coroutines.launch

@Composable
fun SettingsScreen(
    repo: VesselRepository,
    onOpenDownloads: () -> Unit = {},
    onOpenExtensions: () -> Unit = {},
) {
    val palette = LocalVesselPalette.current
    val scope = rememberCoroutineScope()
    val themes by repo.themes.collectAsState()
    val activeTheme by repo.activeTheme.collectAsState()
    val activeLocale by repo.locale.collectAsState()
    val engineInfo by repo.engineInfo.collectAsState()
    val plugins by repo.plugins.collectAsState()

    var showThemeMenu by remember { mutableStateOf(false) }
    var showLocaleMenu by remember { mutableStateOf(false) }
    var tmdbKey by remember { mutableStateOf("") }

    LazyColumn(
        modifier = Modifier
            .fillMaxSize()
            .background(palette.bgBase)
            .padding(16.dp),
        contentPadding = PaddingValues(bottom = 32.dp),
        verticalArrangement = Arrangement.spacedBy(14.dp),
    ) {
        item {
            Text(VesselI18n.tr("settings.title"), style = MaterialTheme.typography.headlineSmall, fontWeight = FontWeight.Bold)
        }

        // Quick Tools: Extensions & Downloads
        item {
            Surface(shape = RoundedCornerShape(14.dp), color = palette.bgSurface) {
                Column(modifier = Modifier.fillMaxWidth()) {
                    Row(
                        modifier = Modifier
                            .fillMaxWidth()
                            .clickable(onClick = onOpenExtensions)
                            .padding(14.dp),
                        verticalAlignment = Alignment.CenterVertically,
                        horizontalArrangement = Arrangement.SpaceBetween,
                    ) {
                        Row(verticalAlignment = Alignment.CenterVertically) {
                            Icon(Icons.Default.Extension, contentDescription = null, tint = palette.accent, modifier = Modifier.size(22.dp))
                            Spacer(Modifier.width(12.dp))
                            Column {
                                Text(VesselI18n.tr("nav.extensions"), fontWeight = FontWeight.SemiBold)
                                Text(
                                    "${plugins.count { it.enabled }} / ${plugins.size}",
                                    style = MaterialTheme.typography.labelSmall,
                                    color = palette.textMuted,
                                )
                            }
                        }
                        Icon(Icons.AutoMirrored.Filled.KeyboardArrowRight, contentDescription = null, tint = palette.textMuted)
                    }

                    Row(
                        modifier = Modifier
                            .fillMaxWidth()
                            .clickable(onClick = onOpenDownloads)
                            .padding(14.dp),
                        verticalAlignment = Alignment.CenterVertically,
                        horizontalArrangement = Arrangement.SpaceBetween,
                    ) {
                        Row(verticalAlignment = Alignment.CenterVertically) {
                            Icon(Icons.Default.Download, contentDescription = null, tint = palette.accent, modifier = Modifier.size(22.dp))
                            Spacer(Modifier.width(12.dp))
                            Text(VesselI18n.tr("nav.downloads"), fontWeight = FontWeight.SemiBold)
                        }
                        Icon(Icons.AutoMirrored.Filled.KeyboardArrowRight, contentDescription = null, tint = palette.textMuted)
                    }
                }
            }
        }

        // Theme & Language
        item {
            Surface(shape = RoundedCornerShape(14.dp), color = palette.bgSurface) {
                Column(modifier = Modifier.fillMaxWidth().padding(14.dp), verticalArrangement = Arrangement.spacedBy(12.dp)) {
                    Text(VesselI18n.tr("settings.appearance"), fontWeight = FontWeight.Bold)

                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        verticalAlignment = Alignment.CenterVertically,
                        horizontalArrangement = Arrangement.SpaceBetween,
                    ) {
                        Column(modifier = Modifier.weight(1f).padding(end = 8.dp)) {
                            Text(VesselI18n.tr("settings.theme_label"))
                            if (activeTheme.id == DYNAMIC_MATERIAL_THEME_ID) {
                                Text(
                                    "Material You • Dynamic wallpaper & system theme",
                                    style = MaterialTheme.typography.labelSmall,
                                    color = palette.textMuted,
                                )
                            }
                        }
                        Box {
                            OutlinedButton(onClick = { showThemeMenu = true }) { Text(activeTheme.name) }
                            DropdownMenu(expanded = showThemeMenu, onDismissRequest = { showThemeMenu = false }) {
                                themes.forEach { t ->
                                    DropdownMenuItem(
                                        text = { Text(t.name) },
                                        onClick = { repo.selectTheme(t.id); showThemeMenu = false },
                                    )
                                }
                            }
                        }
                    }

                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        verticalAlignment = Alignment.CenterVertically,
                        horizontalArrangement = Arrangement.SpaceBetween,
                    ) {
                        Text(VesselI18n.tr("settings.locale_label"))
                        Box {
                            OutlinedButton(onClick = { showLocaleMenu = true }) {
                                Text(VesselI18n.tr("languages.$activeLocale", activeLocale))
                            }
                            DropdownMenu(expanded = showLocaleMenu, onDismissRequest = { showLocaleMenu = false }) {
                                VesselI18n.bundledCodes.forEach { code ->
                                    DropdownMenuItem(
                                        text = { Text(VesselI18n.tr("languages.$code", code)) },
                                        onClick = { repo.setLocale(code); showLocaleMenu = false },
                                    )
                                }
                            }
                        }
                    }
                }
            }
        }

        // Playback & Privacy
        engineInfo?.let { info ->
            item {
                Surface(shape = RoundedCornerShape(14.dp), color = palette.bgSurface) {
                    Column(modifier = Modifier.fillMaxWidth().padding(14.dp), verticalArrangement = Arrangement.spacedBy(12.dp)) {
                        Text(VesselI18n.tr("settings.playback_privacy"), fontWeight = FontWeight.Bold)

                        Row(
                            modifier = Modifier.fillMaxWidth(),
                            verticalAlignment = Alignment.CenterVertically,
                            horizontalArrangement = Arrangement.SpaceBetween,
                        ) {
                            Text(VesselI18n.tr("settings.always_proxy"), modifier = Modifier.weight(1f))
                            Switch(
                                checked = info.alwaysProxy,
                                onCheckedChange = { scope.launch { repo.setAlwaysProxy(it) } },
                            )
                        }

                        Row(
                            modifier = Modifier.fillMaxWidth(),
                            verticalAlignment = Alignment.CenterVertically,
                            horizontalArrangement = Arrangement.SpaceBetween,
                        ) {
                            Text(VesselI18n.tr("settings.metadata_enabled"), modifier = Modifier.weight(1f))
                            Switch(
                                checked = info.metadataEnabled,
                                onCheckedChange = { scope.launch { repo.setMetadataEnabled(it) } },
                            )
                        }

                        OutlinedTextField(
                            value = tmdbKey,
                            onValueChange = { tmdbKey = it },
                            label = { Text(VesselI18n.tr("settings.tmdb_key")) },
                            singleLine = true,
                            modifier = Modifier.fillMaxWidth(),
                        )
                        Button(onClick = {
                            scope.launch {
                                repo.setTmdbApiKey(tmdbKey.ifBlank { null })
                                repo.notify("TMDB API key saved")
                            }
                        }) {
                            Text(VesselI18n.tr("settings.save_api_key"))
                        }

                        Text("Root: ${info.rootDir}", style = MaterialTheme.typography.labelSmall, color = palette.textMuted)
                        Text("Proxy: ${info.proxyBaseUrl}", style = MaterialTheme.typography.labelSmall, color = palette.textMuted)
                    }
                }
            }
        }
    }
}
