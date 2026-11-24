package dev.falsis.vessel.ui.screens

import androidx.compose.foundation.background
import androidx.compose.foundation.border
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
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.KeyboardArrowRight
import androidx.compose.material.icons.automirrored.filled.RotateLeft
import androidx.compose.material.icons.filled.Backup
import androidx.compose.material.icons.filled.CleaningServices
import androidx.compose.material.icons.filled.ContentCopy
import androidx.compose.material.icons.filled.Download
import androidx.compose.material.icons.filled.Extension
import androidx.compose.material.icons.filled.Palette
import androidx.compose.material.icons.filled.Security
import androidx.compose.material.icons.filled.Tune
import androidx.compose.material3.Button
import androidx.compose.material3.Checkbox
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Slider
import androidx.compose.material3.Surface
import androidx.compose.material3.Switch
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
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalClipboardManager
import androidx.compose.ui.text.AnnotatedString
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import dev.falsis.vessel.data.VesselRepository
import dev.falsis.vessel.ui.i18n.VesselI18n
import dev.falsis.vessel.ui.theme.DYNAMIC_MATERIAL_THEME_ID
import dev.falsis.vessel.ui.theme.LocalVesselPalette
import dev.falsis.vessel.ui.theme.parseHexColor
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch

@Composable
fun SettingsScreen(
    repo: VesselRepository,
    onOpenDownloads: () -> Unit = {},
    onOpenExtensions: () -> Unit = {},
) {
    val palette = LocalVesselPalette.current
    val scope = rememberCoroutineScope()
    val clipboard = LocalClipboardManager.current
    val themes by repo.themes.collectAsState()
    val activeTheme by repo.activeTheme.collectAsState()
    val activeLocale by repo.locale.collectAsState()
    val engineInfo by repo.engineInfo.collectAsState()
    val plugins by repo.plugins.collectAsState()
    val prefs by repo.uiPreferences.collectAsState()

    var showThemeMenu by remember { mutableStateOf(false) }
    var showLocaleMenu by remember { mutableStateOf(false) }
    var showHeroModeMenu by remember { mutableStateOf(false) }
    var showPlayClickMenu by remember { mutableStateOf(false) }
    var showAspectMenu by remember { mutableStateOf(false) }
    var showSubOutlineMenu by remember { mutableStateOf(false) }
    var showSubFontMenu by remember { mutableStateOf(false) }

    var themeJsonInput by remember { mutableStateOf("") }
    var showThemeImporter by remember { mutableStateOf(false) }
    var tmdbKey by remember { mutableStateOf("") }
    var backupJsonInput by remember { mutableStateOf("") }
    var includeSecretsInBackup by remember { mutableStateOf(false) }
    var confirmResetBackup by remember { mutableStateOf(false) }

    LaunchedEffect(confirmResetBackup) {
        if (!confirmResetBackup) return@LaunchedEffect
        delay(3500)
        confirmResetBackup = false
    }

    LazyColumn(
        modifier = Modifier
            .fillMaxSize()
            .background(palette.bgBase)
            .padding(16.dp),
        contentPadding = PaddingValues(bottom = 36.dp),
        verticalArrangement = Arrangement.spacedBy(14.dp),
    ) {
        item {
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween,
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Text(VesselI18n.tr("settings.title"), style = MaterialTheme.typography.headlineSmall, fontWeight = FontWeight.Bold)
                OutlinedButton(onClick = {
                    scope.launch {
                        repo.resetUiPreferences()
                        repo.notify(VesselI18n.tr("settings.reset_ui"))
                    }
                }) {
                    Icon(Icons.AutoMirrored.Filled.RotateLeft, contentDescription = null, modifier = Modifier.size(16.dp))
                    Spacer(Modifier.width(4.dp))
                    Text(VesselI18n.tr("settings.reset_ui"), style = MaterialTheme.typography.labelSmall)
                }
            }
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

                    HorizontalDivider(color = palette.border.copy(alpha = 0.35f))

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

        // Appearance & Language + Theme JSON Import
        item {
            Surface(shape = RoundedCornerShape(14.dp), color = palette.bgSurface) {
                Column(modifier = Modifier.fillMaxWidth().padding(14.dp), verticalArrangement = Arrangement.spacedBy(12.dp)) {
                    Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                        Icon(Icons.Default.Palette, contentDescription = null, tint = palette.accent, modifier = Modifier.size(18.dp))
                        Text(VesselI18n.tr("settings.appearance"), fontWeight = FontWeight.Bold)
                    }

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

                    TextButton(onClick = { showThemeImporter = !showThemeImporter }) {
                        Text(VesselI18n.tr("settings.import_theme"))
                    }
                    if (showThemeImporter) {
                        OutlinedTextField(
                            value = themeJsonInput,
                            onValueChange = { themeJsonInput = it },
                            placeholder = { Text("Paste .theme.vessel JSON…") },
                            minLines = 3,
                            modifier = Modifier.fillMaxWidth(),
                        )
                        Button(onClick = {
                            if (themeJsonInput.isBlank()) return@Button
                            scope.launch {
                                runCatching { repo.importThemeJson(themeJsonInput) }
                                    .onSuccess {
                                        themeJsonInput = ""
                                        showThemeImporter = false
                                        repo.notify("Imported theme ${it.name}")
                                    }
                                    .onFailure { repo.notify(it.message ?: "Invalid theme JSON") }
                            }
                        }) {
                            Text(VesselI18n.tr("settings.import_theme"))
                        }
                    }
                }
            }
        }

        // UI & Playback Customization + Subtitle Live Preview (Desktop SettingsView parity)
        item {
            Surface(shape = RoundedCornerShape(14.dp), color = palette.bgSurface) {
                Column(modifier = Modifier.fillMaxWidth().padding(14.dp), verticalArrangement = Arrangement.spacedBy(12.dp)) {
                    Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                        Icon(Icons.Default.Tune, contentDescription = null, tint = palette.accent, modifier = Modifier.size(18.dp))
                        Text(VesselI18n.tr("settings.ui_customization"), fontWeight = FontWeight.Bold)
                    }

                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        verticalAlignment = Alignment.CenterVertically,
                        horizontalArrangement = Arrangement.SpaceBetween,
                    ) {
                        Text("Hero Banner", modifier = Modifier.weight(1f))
                        Box {
                            OutlinedButton(onClick = { showHeroModeMenu = true }) { Text(prefs.home.heroBanner.uppercase()) }
                            DropdownMenu(expanded = showHeroModeMenu, onDismissRequest = { showHeroModeMenu = false }) {
                                listOf("full", "compact", "hidden").forEach { mode ->
                                    DropdownMenuItem(
                                        text = { Text(mode.uppercase()) },
                                        onClick = {
                                            showHeroModeMenu = false
                                            scope.launch { repo.saveUiPreferences(prefs.copy(home = prefs.home.copy(heroBanner = mode))) }
                                        },
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
                        Text(VesselI18n.tr("settings.play_click"), modifier = Modifier.weight(1f))
                        Box {
                            val label = if (prefs.playback.clickBehavior == "always_ask_source") {
                                VesselI18n.tr("settings.play_ask")
                            } else {
                                VesselI18n.tr("settings.play_quick")
                            }
                            OutlinedButton(onClick = { showPlayClickMenu = true }) { Text(label) }
                            DropdownMenu(expanded = showPlayClickMenu, onDismissRequest = { showPlayClickMenu = false }) {
                                listOf(
                                    "quick_start_best" to VesselI18n.tr("settings.play_quick"),
                                    "always_ask_source" to VesselI18n.tr("settings.play_ask"),
                                ).forEach { (key, txt) ->
                                    DropdownMenuItem(
                                        text = { Text(txt) },
                                        onClick = {
                                            showPlayClickMenu = false
                                            scope.launch { repo.saveUiPreferences(prefs.copy(playback = prefs.playback.copy(clickBehavior = key))) }
                                        },
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
                        Text(VesselI18n.tr("home.continue_watching"), modifier = Modifier.weight(1f))
                        Switch(
                            checked = prefs.home.showContinueWatching,
                            onCheckedChange = { v ->
                                scope.launch { repo.saveUiPreferences(prefs.copy(home = prefs.home.copy(showContinueWatching = v))) }
                            },
                        )
                    }

                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        verticalAlignment = Alignment.CenterVertically,
                        horizontalArrangement = Arrangement.SpaceBetween,
                    ) {
                        Text(VesselI18n.tr("settings.show_rating_chip"), modifier = Modifier.weight(1f))
                        Switch(
                            checked = prefs.cards.showRating,
                            onCheckedChange = { v ->
                                scope.launch { repo.saveUiPreferences(prefs.copy(cards = prefs.cards.copy(showRating = v))) }
                            },
                        )
                    }

                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        verticalAlignment = Alignment.CenterVertically,
                        horizontalArrangement = Arrangement.SpaceBetween,
                    ) {
                        Text(VesselI18n.tr("settings.spoiler_free"), modifier = Modifier.weight(1f))
                        Switch(
                            checked = prefs.details.hideEpisodeOverviews,
                            onCheckedChange = { v ->
                                scope.launch {
                                    repo.saveUiPreferences(
                                        prefs.copy(details = prefs.details.copy(hideEpisodeOverviews = v, hideEpisodeThumbnails = v)),
                                    )
                                }
                            },
                        )
                    }

                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        verticalAlignment = Alignment.CenterVertically,
                        horizontalArrangement = Arrangement.SpaceBetween,
                    ) {
                        Text("Card Aspect Ratio", modifier = Modifier.weight(1f))
                        Box {
                            val label = when (prefs.cards.aspectRatio) {
                                "wide_16x9" -> "16:9"
                                "square" -> "1:1"
                                else -> "2:3"
                            }
                            OutlinedButton(onClick = { showAspectMenu = true }) { Text(label) }
                            DropdownMenu(expanded = showAspectMenu, onDismissRequest = { showAspectMenu = false }) {
                                listOf("poster_2x3" to "2:3", "wide_16x9" to "16:9", "square" to "1:1").forEach { (key, txt) ->
                                    DropdownMenuItem(
                                        text = { Text(txt) },
                                        onClick = {
                                            showAspectMenu = false
                                            scope.launch { repo.saveUiPreferences(prefs.copy(cards = prefs.cards.copy(aspectRatio = key))) }
                                        },
                                    )
                                }
                            }
                        }
                    }

                    HorizontalDivider(color = palette.border.copy(alpha = 0.35f))

                    // Subtitle Styling + Live Preview Box
                    Text(VesselI18n.tr("settings.subtitles_title"), fontWeight = FontWeight.Bold)
                    val sub = prefs.subtitles
                    val previewColor = parseHexColor(sub.textColor, Color.White)
                    val previewBgAlpha = (sub.backgroundOpacity.coerceIn(0, 100) / 100f)
                    val previewBg = if (sub.outlineStyle == "none") Color.Transparent else Color(0xFF080A10).copy(alpha = previewBgAlpha)
                    Box(
                        modifier = Modifier
                            .fillMaxWidth()
                            .height(84.dp)
                            .clip(RoundedCornerShape(10.dp))
                            .background(Color(0xFF10121A))
                            .padding(bottom = (sub.bottomOffsetPx / 12).coerceIn(4, 24).dp),
                        contentAlignment = Alignment.BottomCenter,
                    ) {
                        Box(
                            modifier = Modifier
                                .clip(RoundedCornerShape(6.dp))
                                .background(previewBg)
                                .padding(horizontal = 10.dp, vertical = 4.dp),
                        ) {
                            Text(
                                text = VesselI18n.tr("subtitles.preview_sample", "Sample Subtitle Line"),
                                color = previewColor,
                                fontSize = (sub.fontSizePx * 0.58f).coerceIn(12f, 24f).sp,
                                fontWeight = if (sub.bold) FontWeight.Bold else FontWeight.Medium,
                                fontFamily = when (sub.fontFamily) {
                                    "serif" -> FontFamily.Serif
                                    "monospace" -> FontFamily.Monospace
                                    else -> FontFamily.SansSerif
                                },
                            )
                        }
                    }

                    Text("${VesselI18n.tr("subtitles.font_size")}: ${sub.fontSizePx}px", style = MaterialTheme.typography.labelMedium)
                    Slider(
                        value = sub.fontSizePx.toFloat(),
                        onValueChange = { v ->
                            scope.launch { repo.saveUiPreferences(prefs.copy(subtitles = sub.copy(fontSizePx = v.toInt()))) }
                        },
                        valueRange = 16f..52f,
                    )

                    Text("${VesselI18n.tr("subtitles.bg_opacity")}: ${sub.backgroundOpacity}%", style = MaterialTheme.typography.labelMedium)
                    Slider(
                        value = sub.backgroundOpacity.toFloat(),
                        onValueChange = { v ->
                            scope.launch { repo.saveUiPreferences(prefs.copy(subtitles = sub.copy(backgroundOpacity = v.toInt()))) }
                        },
                        valueRange = 0f..100f,
                    )

                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        verticalAlignment = Alignment.CenterVertically,
                        horizontalArrangement = Arrangement.SpaceBetween,
                    ) {
                        Text(VesselI18n.tr("subtitles.outline_style"))
                        Box {
                            OutlinedButton(onClick = { showSubOutlineMenu = true }) { Text(sub.outlineStyle) }
                            DropdownMenu(expanded = showSubOutlineMenu, onDismissRequest = { showSubOutlineMenu = false }) {
                                listOf("outline", "shadow", "box", "none").forEach { style ->
                                    DropdownMenuItem(
                                        text = { Text(style) },
                                        onClick = {
                                            showSubOutlineMenu = false
                                            scope.launch { repo.saveUiPreferences(prefs.copy(subtitles = sub.copy(outlineStyle = style))) }
                                        },
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
                        Text(VesselI18n.tr("subtitles.font_family"))
                        Box {
                            OutlinedButton(onClick = { showSubFontMenu = true }) { Text(sub.fontFamily) }
                            DropdownMenu(expanded = showSubFontMenu, onDismissRequest = { showSubFontMenu = false }) {
                                listOf("system-ui", "rounded", "serif", "monospace").forEach { fam ->
                                    DropdownMenuItem(
                                        text = { Text(fam) },
                                        onClick = {
                                            showSubFontMenu = false
                                            scope.launch { repo.saveUiPreferences(prefs.copy(subtitles = sub.copy(fontFamily = fam))) }
                                        },
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
                        Text(VesselI18n.tr("subtitles.text_color"))
                        Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                            listOf("#ffffff", "#fde047", "#67e8f9", "#f9a8d4", "#a7f3d0").forEach { hex ->
                                val selected = sub.textColor.equals(hex, ignoreCase = true)
                                Box(
                                    modifier = Modifier
                                        .size(26.dp)
                                        .clip(CircleShape)
                                        .background(parseHexColor(hex, Color.White))
                                        .border(
                                            width = if (selected) 2.dp else 1.dp,
                                            color = if (selected) palette.accent else palette.border,
                                            shape = CircleShape,
                                        )
                                        .clickable {
                                            scope.launch { repo.saveUiPreferences(prefs.copy(subtitles = sub.copy(textColor = hex))) }
                                        },
                                )
                            }
                        }
                    }

                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        verticalAlignment = Alignment.CenterVertically,
                        horizontalArrangement = Arrangement.SpaceBetween,
                    ) {
                        Text(VesselI18n.tr("subtitles.bold"))
                        Switch(
                            checked = sub.bold,
                            onCheckedChange = { b ->
                                scope.launch { repo.saveUiPreferences(prefs.copy(subtitles = sub.copy(bold = b))) }
                            },
                        )
                    }
                }
            }
        }

        // Playback, Privacy & Storage
        engineInfo?.let { info ->
            item {
                Surface(shape = RoundedCornerShape(14.dp), color = palette.bgSurface) {
                    Column(modifier = Modifier.fillMaxWidth().padding(14.dp), verticalArrangement = Arrangement.spacedBy(12.dp)) {
                        Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                            Icon(Icons.Default.Security, contentDescription = null, tint = palette.accent, modifier = Modifier.size(18.dp))
                            Text(VesselI18n.tr("settings.playback_privacy"), fontWeight = FontWeight.Bold)
                        }

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
                        Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                            Button(onClick = {
                                scope.launch {
                                    repo.setTmdbApiKey(tmdbKey.ifBlank { null })
                                    tmdbKey = ""
                                    repo.notify("TMDB API key saved")
                                }
                            }) {
                                Text(VesselI18n.tr("settings.save_api_key"))
                            }
                            OutlinedButton(onClick = {
                                scope.launch {
                                    repo.clearRuntimeCaches()
                                    repo.notify("Runtime caches cleared")
                                }
                            }) {
                                Icon(Icons.Default.CleaningServices, contentDescription = null, modifier = Modifier.size(16.dp))
                                Spacer(Modifier.width(4.dp))
                                Text("Clear Cache")
                            }
                        }

                        Text("Root: ${info.rootDir}", style = MaterialTheme.typography.labelSmall, color = palette.textMuted)
                        Text("Downloads: ${info.downloadsDir}", style = MaterialTheme.typography.labelSmall, color = palette.textMuted)
                        Text("Proxy: ${info.proxyBaseUrl}", style = MaterialTheme.typography.labelSmall, color = palette.textMuted)
                    }
                }
            }
        }

        // Backup & Restore (Desktop SettingsView parity)
        item {
            Surface(shape = RoundedCornerShape(14.dp), color = palette.bgSurface) {
                Column(modifier = Modifier.fillMaxWidth().padding(14.dp), verticalArrangement = Arrangement.spacedBy(10.dp)) {
                    Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                        Icon(Icons.Default.Backup, contentDescription = null, tint = palette.accent, modifier = Modifier.size(18.dp))
                        Text(VesselI18n.tr("settings.backup_title", "Backup & Restore"), fontWeight = FontWeight.Bold)
                    }

                    Row(
                        modifier = Modifier.fillMaxWidth().clickable { includeSecretsInBackup = !includeSecretsInBackup },
                        verticalAlignment = Alignment.CenterVertically,
                    ) {
                        Checkbox(checked = includeSecretsInBackup, onCheckedChange = { includeSecretsInBackup = it })
                        Text(VesselI18n.tr("settings.backup_include_secrets", "Include API keys / secrets"), style = MaterialTheme.typography.bodySmall)
                    }

                    Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                        Button(onClick = {
                            scope.launch {
                                runCatching { repo.exportBackupJson(includeSecretsInBackup) }
                                    .onSuccess {
                                        backupJsonInput = it
                                        repo.notify("Backup exported")
                                    }
                                    .onFailure { repo.notify(it.message ?: "Export failed") }
                            }
                        }) {
                            Text(VesselI18n.tr("common.export", "Export"))
                        }
                        if (backupJsonInput.isNotBlank()) {
                            OutlinedButton(onClick = {
                                clipboard.setText(AnnotatedString(backupJsonInput))
                                repo.notify("Copied backup JSON")
                            }) {
                                Icon(Icons.Default.ContentCopy, contentDescription = null, modifier = Modifier.size(15.dp))
                                Spacer(Modifier.width(4.dp))
                                Text("Copy")
                            }
                        }
                    }

                    OutlinedTextField(
                        value = backupJsonInput,
                        onValueChange = { backupJsonInput = it },
                        placeholder = { Text("Vessel backup JSON…") },
                        minLines = 3,
                        maxLines = 6,
                        modifier = Modifier.fillMaxWidth(),
                    )

                    Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                        OutlinedButton(
                            enabled = backupJsonInput.isNotBlank(),
                            onClick = {
                                scope.launch {
                                    runCatching { repo.importBackupJson(backupJsonInput, overwrite = false) }
                                        .onSuccess { repo.notify("Merged backup") }
                                        .onFailure { repo.notify(it.message ?: "Import failed") }
                                }
                            },
                        ) {
                            Text(VesselI18n.tr("settings.backup_merge", "Merge Import"))
                        }
                        Button(
                            enabled = backupJsonInput.isNotBlank(),
                            onClick = {
                                if (!confirmResetBackup) {
                                    confirmResetBackup = true
                                } else {
                                    confirmResetBackup = false
                                    scope.launch {
                                        runCatching { repo.importBackupJson(backupJsonInput, overwrite = true) }
                                            .onSuccess { repo.notify("Restored backup") }
                                            .onFailure { repo.notify(it.message ?: "Restore failed") }
                                    }
                                }
                            },
                        ) {
                            Text(
                                if (confirmResetBackup) {
                                    VesselI18n.tr("common.confirm", "Confirm")
                                } else {
                                    VesselI18n.tr("settings.backup_reset", "Reset & Import")
                                },
                            )
                        }
                    }
                }
            }
        }
    }
}
