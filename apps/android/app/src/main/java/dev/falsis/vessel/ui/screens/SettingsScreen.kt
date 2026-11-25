package dev.falsis.vessel.ui.screens

import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.core.tween
import androidx.compose.animation.expandVertically
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.shrinkVertically
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.FlowRow
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
import androidx.compose.material.icons.automirrored.filled.OpenInNew
import androidx.compose.material.icons.automirrored.filled.RotateLeft
import androidx.compose.material.icons.filled.Backup
import androidx.compose.material.icons.filled.CleaningServices
import androidx.compose.material.icons.filled.ContentCopy
import androidx.compose.material.icons.filled.Download
import androidx.compose.material.icons.filled.Extension
import androidx.compose.material.icons.filled.Palette
import androidx.compose.material.icons.filled.Refresh
import androidx.compose.material.icons.filled.Security
import androidx.compose.material.icons.filled.SystemUpdate
import androidx.compose.material.icons.filled.Tune
import androidx.compose.material3.Checkbox
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.LinearProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Slider
import androidx.compose.material3.Surface
import androidx.compose.material3.Switch
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
import androidx.compose.ui.platform.LocalClipboardManager
import dev.falsis.vessel.ui.theme.LocalVesselFloatingNavInset
import androidx.compose.ui.platform.LocalUriHandler
import androidx.compose.ui.text.AnnotatedString
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import dev.falsis.vessel.BuildConfig
import dev.falsis.vessel.data.VesselRepository
import dev.falsis.vessel.ui.components.VesselButton
import dev.falsis.vessel.ui.components.VesselFilterChip
import dev.falsis.vessel.ui.components.VesselOutlinedButton
import dev.falsis.vessel.ui.components.VesselTextButton
import dev.falsis.vessel.ui.components.vesselTopSafeInset
import dev.falsis.vessel.ui.i18n.VesselI18n
import dev.falsis.vessel.ui.theme.DYNAMIC_MATERIAL_THEME_ID
import dev.falsis.vessel.ui.theme.LocalVesselPalette
import dev.falsis.vessel.ui.theme.LocalVesselUiTokens
import dev.falsis.vessel.ui.theme.parseHexColor
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch

@OptIn(ExperimentalLayoutApi::class)
@Composable
fun SettingsScreen(
    repo: VesselRepository,
    onOpenDownloads: () -> Unit = {},
    onOpenExtensions: () -> Unit = {},
) {
    val palette = LocalVesselPalette.current
    val tokens = LocalVesselUiTokens.current
    val scope = rememberCoroutineScope()
    val clipboard = LocalClipboardManager.current
    val uriHandler = LocalUriHandler.current
    val themes by repo.themes.collectAsState()
    val activeTheme by repo.activeTheme.collectAsState()
    val activeLocale by repo.locale.collectAsState()
    val engineInfo by repo.engineInfo.collectAsState()
    val plugins by repo.plugins.collectAsState()
    val prefs by repo.uiPreferences.collectAsState()
    val autoCheckUpdates by repo.autoCheckUpdates.collectAsState()
    val updateInfo by repo.updateInfo.collectAsState()
    val checkingUpdate by repo.checkingUpdate.collectAsState()
    val downloadingUpdate by repo.downloadingUpdate.collectAsState()
    val updatePercent by repo.updateDownloadPercent.collectAsState()
    val updateError by repo.updateError.collectAsState()

    var showThemeMenu by remember { mutableStateOf(false) }
    var showLocaleMenu by remember { mutableStateOf(false) }
    var showNavLayoutMenu by remember { mutableStateOf(false) }
    var showHeroModeMenu by remember { mutableStateOf(false) }
    var showPlayClickMenu by remember { mutableStateOf(false) }
    var showAspectMenu by remember { mutableStateOf(false) }
    var showSubOutlineMenu by remember { mutableStateOf(false) }
    var showSubFontMenu by remember { mutableStateOf(false) }

    var themeJsonInput by remember { mutableStateOf("") }
    var showThemeImporter by remember { mutableStateOf(false) }
    var tmdbKey by remember { mutableStateOf("") }
    var simklClientId by remember { mutableStateOf("") }
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
            .vesselTopSafeInset(),
        contentPadding = PaddingValues(start = 16.dp, top = 14.dp, end = 16.dp, bottom = 36.dp + LocalVesselFloatingNavInset.current),
        verticalArrangement = Arrangement.spacedBy(14.dp),
    ) {
        item {
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween,
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Text(VesselI18n.tr("settings.title"), style = MaterialTheme.typography.headlineSmall, fontWeight = FontWeight.Bold)
                VesselOutlinedButton(
                    height = tokens.compactButtonHeight,
                    contentPadding = PaddingValues(horizontal = 10.dp, vertical = 0.dp),
                    onClick = {
                        scope.launch {
                            repo.resetUiPreferences()
                            repo.notify(VesselI18n.tr("settings.reset_ui"))
                        }
                    },
                ) {
                    Icon(Icons.AutoMirrored.Filled.RotateLeft, contentDescription = null, modifier = Modifier.size(14.dp))
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
                                    VesselI18n.tr("settings.dynamic_theme_desc"),
                                    style = MaterialTheme.typography.labelSmall,
                                    color = palette.textMuted,
                                )
                            }
                        }
                        Box {
                            VesselOutlinedButton(onClick = { showThemeMenu = true }, height = tokens.compactButtonHeight) {
                                Text(activeTheme.name, style = MaterialTheme.typography.labelMedium)
                            }
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
                            VesselOutlinedButton(onClick = { showLocaleMenu = true }, height = tokens.compactButtonHeight) {
                                Text(VesselI18n.tr("languages.$activeLocale", activeLocale), style = MaterialTheme.typography.labelMedium)
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

                    VesselTextButton(onClick = { showThemeImporter = !showThemeImporter }) {
                        Text(VesselI18n.tr("settings.import_theme"))
                    }
                    AnimatedVisibility(
                        visible = showThemeImporter,
                        enter = expandVertically(tween(220)) + fadeIn(tween(180)),
                        exit = shrinkVertically(tween(180)) + fadeOut(tween(140)),
                    ) {
                        Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                            OutlinedTextField(
                                value = themeJsonInput,
                                onValueChange = { themeJsonInput = it },
                                placeholder = { Text(VesselI18n.tr("settings.theme_paste_hint")) },
                                minLines = 3,
                                modifier = Modifier.fillMaxWidth(),
                            )
                            VesselButton(onClick = {
                                if (themeJsonInput.isBlank()) return@VesselButton
                                scope.launch {
                                    runCatching { repo.importThemeJson(themeJsonInput) }
                                        .onSuccess {
                                            themeJsonInput = ""
                                            showThemeImporter = false
                                            repo.notify(VesselI18n.trf("settings.theme_imported", "name" to it.name))
                                        }
                                        .onFailure { repo.notifyError(it, "Invalid theme JSON") }
                                }
                            }) {
                                Text(VesselI18n.tr("settings.import_theme"))
                            }
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
                        Text(VesselI18n.tr("settings.nav_layout"), modifier = Modifier.weight(1f))
                        Box {
                            val navLabel = if (prefs.shell.mobileNavLayout == "floating_bottom") {
                                VesselI18n.tr("settings.layout_floating_bottom")
                            } else {
                                VesselI18n.tr("settings.nav_standard")
                            }
                            VesselOutlinedButton(onClick = { showNavLayoutMenu = true }, height = tokens.compactButtonHeight) {
                                Text(navLabel, style = MaterialTheme.typography.labelMedium)
                            }
                            DropdownMenu(expanded = showNavLayoutMenu, onDismissRequest = { showNavLayoutMenu = false }) {
                                listOf(
                                    "standard" to VesselI18n.tr("settings.nav_standard"),
                                    "floating_bottom" to VesselI18n.tr("settings.layout_floating_bottom"),
                                ).forEach { (layoutKey, txt) ->
                                    DropdownMenuItem(
                                        text = { Text(txt) },
                                        onClick = {
                                            showNavLayoutMenu = false
                                            scope.launch {
                                                repo.saveUiPreferences(
                                                    prefs.copy(shell = prefs.shell.copy(mobileNavLayout = layoutKey)),
                                                )
                                            }
                                        },
                                    )
                                }
                            }
                        }
                    }

                    Column(verticalArrangement = Arrangement.spacedBy(6.dp)) {
                        Text(VesselI18n.tr("settings.density"))
                        FlowRow(
                            horizontalArrangement = Arrangement.spacedBy(8.dp),
                            verticalArrangement = Arrangement.spacedBy(8.dp),
                        ) {
                            listOf("compact", "comfortable", "spacious").forEach { densityKey ->
                                VesselFilterChip(
                                    selected = prefs.shell.density == densityKey,
                                    onClick = {
                                        scope.launch {
                                            repo.saveUiPreferences(
                                                prefs.copy(shell = prefs.shell.copy(density = densityKey)),
                                            )
                                        }
                                    },
                                    label = VesselI18n.tr("settings.density_$densityKey"),
                                )
                            }
                        }
                    }

                    Column(verticalArrangement = Arrangement.spacedBy(4.dp)) {
                        Row(
                            modifier = Modifier.fillMaxWidth(),
                            verticalAlignment = Alignment.CenterVertically,
                            horizontalArrangement = Arrangement.SpaceBetween,
                        ) {
                            Text(
                                "${VesselI18n.tr("settings.ui_scale")}: ${prefs.shell.fontScalePercent}%",
                                modifier = Modifier.weight(1f),
                            )
                            if (prefs.shell.fontScalePercent != 100) {
                                VesselTextButton(
                                    onClick = {
                                        scope.launch {
                                            repo.saveUiPreferences(
                                                prefs.copy(shell = prefs.shell.copy(fontScalePercent = 100)),
                                            )
                                        }
                                    },
                                ) {
                                    Text("100%")
                                }
                            }
                        }
                        Slider(
                            value = prefs.shell.fontScalePercent.coerceIn(85, 130).toFloat(),
                            onValueChange = { raw ->
                                val snapped = ((raw / 5f).toInt() * 5).coerceIn(85, 130)
                                if (snapped != prefs.shell.fontScalePercent) {
                                    scope.launch {
                                        repo.saveUiPreferences(
                                            prefs.copy(shell = prefs.shell.copy(fontScalePercent = snapped)),
                                        )
                                    }
                                }
                            },
                            valueRange = 85f..130f,
                            steps = 8,
                        )
                    }

                    Column(verticalArrangement = Arrangement.spacedBy(6.dp)) {
                        Text(VesselI18n.tr("settings.corner_style"))
                        FlowRow(
                            horizontalArrangement = Arrangement.spacedBy(8.dp),
                            verticalArrangement = Arrangement.spacedBy(8.dp),
                        ) {
                            listOf("sharp", "rounded", "pill").forEach { cKey ->
                                VesselFilterChip(
                                    selected = prefs.shell.cornerStyle == cKey,
                                    onClick = {
                                        scope.launch {
                                            repo.saveUiPreferences(
                                                prefs.copy(shell = prefs.shell.copy(cornerStyle = cKey)),
                                            )
                                        }
                                    },
                                    label = VesselI18n.tr("settings.corner_$cKey"),
                                )
                            }
                        }
                    }

                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        verticalAlignment = Alignment.CenterVertically,
                        horizontalArrangement = Arrangement.SpaceBetween,
                    ) {
                        Text(VesselI18n.tr("settings.hero_banner"), modifier = Modifier.weight(1f))
                        Box {
                            val heroModeLabel = when (prefs.home.heroBanner) {
                                "compact" -> VesselI18n.tr("settings.hero_compact")
                                "hidden" -> VesselI18n.tr("settings.hero_hidden")
                                else -> VesselI18n.tr("settings.hero_full")
                            }
                            VesselOutlinedButton(onClick = { showHeroModeMenu = true }, height = tokens.compactButtonHeight) {
                                Text(heroModeLabel, style = MaterialTheme.typography.labelMedium)
                            }
                            DropdownMenu(expanded = showHeroModeMenu, onDismissRequest = { showHeroModeMenu = false }) {
                                listOf(
                                    "full" to VesselI18n.tr("settings.hero_full"),
                                    "compact" to VesselI18n.tr("settings.hero_compact"),
                                    "hidden" to VesselI18n.tr("settings.hero_hidden"),
                                ).forEach { (mode, label) ->
                                    DropdownMenuItem(
                                        text = { Text(label) },
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
                            VesselOutlinedButton(onClick = { showPlayClickMenu = true }, height = tokens.compactButtonHeight) {
                                Text(label, style = MaterialTheme.typography.labelMedium)
                            }
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
                        Text(VesselI18n.tr("settings.card_aspect_ratio"), modifier = Modifier.weight(1f))
                        Box {
                            val ratioOptions = listOf(
                                "poster_2x3" to VesselI18n.tr("settings.ratio_2x3"),
                                "portrait_3x4" to VesselI18n.tr("settings.ratio_3x4"),
                                "portrait_4x5" to VesselI18n.tr("settings.ratio_4x5"),
                                "square" to VesselI18n.tr("settings.ratio_1x1"),
                                "classic_4x3" to VesselI18n.tr("settings.ratio_4x3"),
                                "wide_16x9" to VesselI18n.tr("settings.ratio_16x9"),
                                "cinema_21x9" to VesselI18n.tr("settings.ratio_21x9"),
                            )
                            val activeAspectKey = when (prefs.cards.aspectRatio) {
                                "wide16x9" -> "wide_16x9"
                                "poster2x3" -> "poster_2x3"
                                else -> prefs.cards.aspectRatio
                            }
                            val label = ratioOptions.firstOrNull { it.first == activeAspectKey }?.second
                                ?: VesselI18n.tr("settings.ratio_2x3")
                            VesselOutlinedButton(onClick = { showAspectMenu = true }, height = tokens.compactButtonHeight) {
                                Text(label, style = MaterialTheme.typography.labelMedium)
                            }
                            DropdownMenu(expanded = showAspectMenu, onDismissRequest = { showAspectMenu = false }) {
                                ratioOptions.forEach { (key, txt) ->
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
                            val styleLabel = when (sub.outlineStyle) {
                                "shadow" -> VesselI18n.tr("subtitles.style_shadow")
                                "box" -> VesselI18n.tr("subtitles.style_box")
                                "none" -> VesselI18n.tr("subtitles.style_none")
                                else -> VesselI18n.tr("subtitles.style_outline")
                            }
                            VesselOutlinedButton(onClick = { showSubOutlineMenu = true }, height = tokens.compactButtonHeight) {
                                Text(styleLabel, style = MaterialTheme.typography.labelMedium)
                            }
                            DropdownMenu(expanded = showSubOutlineMenu, onDismissRequest = { showSubOutlineMenu = false }) {
                                listOf(
                                    "outline" to VesselI18n.tr("subtitles.style_outline"),
                                    "shadow" to VesselI18n.tr("subtitles.style_shadow"),
                                    "box" to VesselI18n.tr("subtitles.style_box"),
                                    "none" to VesselI18n.tr("subtitles.style_none"),
                                ).forEach { (styleKey, styleText) ->
                                    DropdownMenuItem(
                                        text = { Text(styleText) },
                                        onClick = {
                                            showSubOutlineMenu = false
                                            scope.launch { repo.saveUiPreferences(prefs.copy(subtitles = sub.copy(outlineStyle = styleKey))) }
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
                            val fontLabel = when (sub.fontFamily) {
                                "rounded" -> VesselI18n.tr("subtitles.font_rounded")
                                "serif" -> VesselI18n.tr("subtitles.font_serif")
                                "monospace" -> VesselI18n.tr("subtitles.font_mono")
                                else -> VesselI18n.tr("subtitles.font_sans")
                            }
                            VesselOutlinedButton(onClick = { showSubFontMenu = true }, height = tokens.compactButtonHeight) {
                                Text(fontLabel, style = MaterialTheme.typography.labelMedium)
                            }
                            DropdownMenu(expanded = showSubFontMenu, onDismissRequest = { showSubFontMenu = false }) {
                                listOf(
                                    "system-ui" to VesselI18n.tr("subtitles.font_sans"),
                                    "rounded" to VesselI18n.tr("subtitles.font_rounded"),
                                    "serif" to VesselI18n.tr("subtitles.font_serif"),
                                    "monospace" to VesselI18n.tr("subtitles.font_mono"),
                                ).forEach { (famKey, famLabel) ->
                                    DropdownMenuItem(
                                        text = { Text(famLabel) },
                                        onClick = {
                                            showSubFontMenu = false
                                            scope.launch { repo.saveUiPreferences(prefs.copy(subtitles = sub.copy(fontFamily = famKey))) }
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
                        OutlinedTextField(
                            value = simklClientId,
                            onValueChange = { simklClientId = it },
                            label = { Text(VesselI18n.tr("settings.simkl_client_id")) },
                            singleLine = true,
                            modifier = Modifier.fillMaxWidth(),
                        )
                        Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                            VesselButton(onClick = {
                                scope.launch {
                                    repo.setTmdbApiKey(tmdbKey.ifBlank { null })
                                    repo.setSimklClientId(simklClientId.ifBlank { null })
                                    tmdbKey = ""
                                    simklClientId = ""
                                    repo.notify(VesselI18n.tr("settings.tmdb_saved"))
                                }
                            }) {
                                Text(VesselI18n.tr("settings.save_api_key"))
                            }
                            VesselOutlinedButton(onClick = {
                                scope.launch {
                                    repo.clearRuntimeCaches()
                                    repo.notify(VesselI18n.tr("settings.caches_cleared"))
                                }
                            }) {
                                Icon(Icons.Default.CleaningServices, contentDescription = null, modifier = Modifier.size(15.dp))
                                Spacer(Modifier.width(4.dp))
                                Text(VesselI18n.tr("settings.clear_cache"))
                            }
                        }

                        Text("${VesselI18n.tr("settings.root_label")}: ${info.rootDir}", style = MaterialTheme.typography.labelSmall, color = palette.textMuted)
                        Text("${VesselI18n.tr("settings.downloads_label")}: ${info.downloadsDir}", style = MaterialTheme.typography.labelSmall, color = palette.textMuted)
                        Text("${VesselI18n.tr("settings.proxy_label")}: ${info.proxyBaseUrl}", style = MaterialTheme.typography.labelSmall, color = palette.textMuted)
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
                        Text(VesselI18n.tr("settings.backup_title"), fontWeight = FontWeight.Bold)
                    }

                    Row(
                        modifier = Modifier.fillMaxWidth().clickable { includeSecretsInBackup = !includeSecretsInBackup },
                        verticalAlignment = Alignment.CenterVertically,
                    ) {
                        Checkbox(checked = includeSecretsInBackup, onCheckedChange = { includeSecretsInBackup = it })
                        Text(VesselI18n.tr("settings.backup_include_secrets"), style = MaterialTheme.typography.bodySmall)
                    }

                    Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                        VesselButton(onClick = {
                            scope.launch {
                                runCatching { repo.exportBackupJson(includeSecretsInBackup) }
                                    .onSuccess {
                                        backupJsonInput = it
                                        repo.notify(VesselI18n.tr("settings.backup_exported"))
                                    }
                                    .onFailure { repo.notifyError(it, "Export failed") }
                            }
                        }) {
                            Text(VesselI18n.tr("common.export"))
                        }
                        if (backupJsonInput.isNotBlank()) {
                            VesselOutlinedButton(onClick = {
                                clipboard.setText(AnnotatedString(backupJsonInput))
                                repo.notify(VesselI18n.tr("common.copied"))
                            }) {
                                Icon(Icons.Default.ContentCopy, contentDescription = null, modifier = Modifier.size(14.dp))
                                Spacer(Modifier.width(4.dp))
                                Text(VesselI18n.tr("common.copy"))
                            }
                        }
                    }

                    OutlinedTextField(
                        value = backupJsonInput,
                        onValueChange = { backupJsonInput = it },
                        placeholder = { Text(VesselI18n.tr("settings.backup_paste_hint")) },
                        minLines = 3,
                        maxLines = 6,
                        modifier = Modifier.fillMaxWidth(),
                    )

                    Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                        VesselOutlinedButton(
                            enabled = backupJsonInput.isNotBlank(),
                            onClick = {
                                scope.launch {
                                    runCatching { repo.importBackupJson(backupJsonInput, overwrite = false) }
                                        .onSuccess { repo.notify(VesselI18n.tr("settings.backup_merged")) }
                                        .onFailure { repo.notifyError(it, "Import failed") }
                                }
                            },
                        ) {
                            Text(VesselI18n.tr("settings.backup_merge"))
                        }
                        VesselButton(
                            enabled = backupJsonInput.isNotBlank(),
                            onClick = {
                                if (!confirmResetBackup) {
                                    confirmResetBackup = true
                                } else {
                                    confirmResetBackup = false
                                    scope.launch {
                                        runCatching { repo.importBackupJson(backupJsonInput, overwrite = true) }
                                            .onSuccess { repo.notify(VesselI18n.tr("settings.backup_restored")) }
                                            .onFailure { repo.notifyError(it, "Restore failed") }
                                    }
                                }
                            },
                        ) {
                            Text(
                                if (confirmResetBackup) {
                                    VesselI18n.tr("common.confirm")
                                } else {
                                    VesselI18n.tr("settings.backup_reset")
                                },
                            )
                        }
                    }
                }
            }
        }

        // Updates & Release Notes
        item {
            Surface(shape = RoundedCornerShape(14.dp), color = palette.bgSurface) {
                Column(modifier = Modifier.fillMaxWidth().padding(14.dp), verticalArrangement = Arrangement.spacedBy(10.dp)) {
                    Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                        Icon(Icons.Default.SystemUpdate, contentDescription = null, tint = palette.accent, modifier = Modifier.size(18.dp))
                        Text(VesselI18n.tr("settings.updates_title"), fontWeight = FontWeight.Bold)
                    }

                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        verticalAlignment = Alignment.CenterVertically,
                        horizontalArrangement = Arrangement.SpaceBetween,
                    ) {
                        Column(modifier = Modifier.weight(1f).padding(end = 8.dp)) {
                            Text(VesselI18n.tr("settings.updates_auto"), fontWeight = FontWeight.Medium)
                            Text(
                                text = VesselI18n.tr("settings.updates_auto_mobile_desc"),
                                style = MaterialTheme.typography.labelSmall,
                                color = palette.textMuted,
                            )
                        }
                        Switch(
                            checked = autoCheckUpdates,
                            onCheckedChange = { repo.setAutoCheckUpdates(it) },
                        )
                    }

                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        verticalAlignment = Alignment.CenterVertically,
                        horizontalArrangement = Arrangement.spacedBy(10.dp),
                    ) {
                        VesselButton(
                            height = 34.dp,
                            enabled = !checkingUpdate,
                            onClick = { scope.launch { repo.checkForAppUpdates(silent = false) } },
                        ) {
                            Icon(Icons.Default.Refresh, contentDescription = null, modifier = Modifier.size(15.dp))
                            Spacer(Modifier.width(5.dp))
                            Text(
                                text = if (checkingUpdate) VesselI18n.tr("settings.checking_updates") else VesselI18n.tr("settings.check_updates"),
                                style = MaterialTheme.typography.labelMedium,
                            )
                        }
                        Text(
                            text = updateInfo?.let {
                                VesselI18n.trf(
                                    "settings.current_latest_ver",
                                    "current" to "v${it.currentVersion}",
                                    "latest" to "v${it.latestVersion}",
                                )
                            } ?: "v${BuildConfig.VERSION_NAME}",
                            style = MaterialTheme.typography.labelSmall,
                            color = palette.textMuted,
                            modifier = Modifier.weight(1f),
                        )
                    }

                    updateError?.let { err ->
                        Text(err, style = MaterialTheme.typography.labelSmall, color = palette.danger)
                    }

                    updateInfo?.let { info ->
                        Surface(
                            shape = RoundedCornerShape(10.dp),
                            color = palette.bgElevated,
                        ) {
                            Column(
                                modifier = Modifier.fillMaxWidth().padding(12.dp),
                                verticalArrangement = Arrangement.spacedBy(8.dp),
                            ) {
                                Row(
                                    modifier = Modifier.fillMaxWidth(),
                                    horizontalArrangement = Arrangement.SpaceBetween,
                                    verticalAlignment = Alignment.CenterVertically,
                                ) {
                                    Text(
                                        text = info.releaseName,
                                        fontWeight = FontWeight.SemiBold,
                                        style = MaterialTheme.typography.bodyMedium,
                                        modifier = Modifier.weight(1f),
                                    )
                                    Surface(
                                        shape = RoundedCornerShape(999.dp),
                                        color = if (info.updateAvailable) palette.accent.copy(alpha = 0.2f) else palette.success.copy(alpha = 0.2f),
                                    ) {
                                        Text(
                                            text = if (info.updateAvailable) VesselI18n.tr("settings.update_available") else VesselI18n.tr("settings.up_to_date"),
                                            style = MaterialTheme.typography.labelSmall,
                                            color = if (info.updateAvailable) palette.accent else palette.success,
                                            modifier = Modifier.padding(horizontal = 8.dp, vertical = 2.dp),
                                        )
                                    }
                                }

                                Text(
                                    text = info.releaseNotes,
                                    style = MaterialTheme.typography.bodySmall,
                                    color = palette.textMuted,
                                )

                                if (downloadingUpdate) {
                                    Column(verticalArrangement = Arrangement.spacedBy(4.dp)) {
                                        Text(
                                            text = VesselI18n.trf(
                                                "settings.downloading_update",
                                                "file" to (info.assetName ?: "vessel.apk"),
                                                "percent" to (updatePercent ?: 0),
                                            ),
                                            style = MaterialTheme.typography.labelSmall,
                                            color = palette.textMuted,
                                        )
                                        if (updatePercent != null) {
                                            LinearProgressIndicator(
                                                progress = { (updatePercent ?: 0) / 100f },
                                                modifier = Modifier.fillMaxWidth().height(5.dp).clip(RoundedCornerShape(999.dp)),
                                            )
                                        } else {
                                            LinearProgressIndicator(
                                                modifier = Modifier.fillMaxWidth().height(5.dp).clip(RoundedCornerShape(999.dp)),
                                            )
                                        }
                                    }
                                }

                                if (info.updateAvailable && info.downloadUrl == null) {
                                    Text(
                                        text = VesselI18n.tr("settings.update_asset_pending"),
                                        style = MaterialTheme.typography.labelSmall,
                                        color = palette.textMuted,
                                    )
                                }

                                Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                                    if (info.updateAvailable && info.downloadUrl != null) {
                                        VesselButton(
                                            height = tokens.compactButtonHeight,
                                            enabled = !downloadingUpdate,
                                            onClick = { scope.launch { repo.downloadAndInstallUpdate(info) } },
                                        ) {
                                            Icon(Icons.Default.Download, contentDescription = null, modifier = Modifier.size(14.dp))
                                            Spacer(Modifier.width(4.dp))
                                            Text(
                                                text = if (downloadingUpdate) {
                                                    VesselI18n.tr("settings.downloading_short")
                                                } else {
                                                    VesselI18n.trf("settings.install_update", "asset" to (info.assetName ?: "APK"))
                                                },
                                                style = MaterialTheme.typography.labelSmall,
                                            )
                                        }
                                    }
                                    VesselOutlinedButton(
                                        height = tokens.compactButtonHeight,
                                        onClick = { runCatching { uriHandler.openUri(info.htmlUrl) } },
                                    ) {
                                        Icon(Icons.AutoMirrored.Filled.OpenInNew, contentDescription = null, modifier = Modifier.size(14.dp))
                                        Spacer(Modifier.width(4.dp))
                                        Text(VesselI18n.tr("settings.open_releases"), style = MaterialTheme.typography.labelSmall)
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
