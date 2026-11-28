package dev.falsis.vessel.ui.screens

import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.gestures.detectVerticalDragGestures
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.ArrowDownward
import androidx.compose.material.icons.filled.ArrowUpward
import androidx.compose.material.icons.filled.DragIndicator
import androidx.compose.material.icons.filled.RestartAlt
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Switch
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.key
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.focus.FocusRequester
import androidx.compose.ui.focus.focusRequester
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.layout.onSizeChanged
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import dev.falsis.vessel.data.HomeSection
import dev.falsis.vessel.data.LiveTvPreferences
import dev.falsis.vessel.data.PluginListEntry
import dev.falsis.vessel.data.UpsertIptvProfilePayload
import dev.falsis.vessel.ui.components.VesselButton
import dev.falsis.vessel.ui.components.VesselCompactIconButton
import dev.falsis.vessel.ui.components.VesselFilterChip
import dev.falsis.vessel.ui.components.VesselOutlinedTextField
import dev.falsis.vessel.ui.components.VesselTextButton
import dev.falsis.vessel.ui.components.vesselDpadInputEscape
import dev.falsis.vessel.ui.i18n.VesselI18n
import dev.falsis.vessel.ui.theme.LocalVesselPalette
import dev.falsis.vessel.ui.theme.LocalVesselUiTokens

@Composable
internal fun AddIptvProfileDialog(
    onDismiss: () -> Unit,
    onSave: (UpsertIptvProfilePayload) -> Unit,
) {
    var name by remember { mutableStateOf("") }
    var serverUrl by remember { mutableStateOf("") }
    var epgUrl by remember { mutableStateOf("") }
    var username by remember { mutableStateOf("") }
    var password by remember { mutableStateOf("") }
    var isXtream by remember { mutableStateOf(false) }

    AlertDialog(
        onDismissRequest = onDismiss,
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
                onSave(
                    UpsertIptvProfilePayload(
                        id = "iptv-${System.currentTimeMillis()}",
                        name = name.trim(),
                        kind = if (isXtream) "xtream" else "m3u",
                        serverUrl = serverUrl.trim(),
                        username = username.ifBlank { null },
                        password = password.ifBlank { null },
                        epgUrl = epgUrl.ifBlank { null },
                    ),
                )
            }) {
                Text(VesselI18n.tr("common.save"))
            }
        },
        dismissButton = {
            VesselTextButton(onClick = onDismiss) { Text(VesselI18n.tr("common.cancel")) }
        },
    )
}

@OptIn(ExperimentalLayoutApi::class)
@Composable
internal fun LiveTvCustomizeViewDialog(
    livePrefs: LiveTvPreferences,
    onSavePrefs: (LiveTvPreferences) -> Unit,
    onDismiss: () -> Unit,
) {
    val palette = LocalVesselPalette.current
    AlertDialog(
        onDismissRequest = onDismiss,
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
                            onClick = { onSavePrefs(livePrefs.copy(viewMode = mode)) },
                            label = VesselI18n.tr("iptv.layout_$mode"),
                        )
                    }
                }
                Text(VesselI18n.tr("settings.density"), style = MaterialTheme.typography.labelLarge, color = palette.accent, fontWeight = FontWeight.SemiBold)
                FlowRow(horizontalArrangement = Arrangement.spacedBy(6.dp), verticalArrangement = Arrangement.spacedBy(6.dp)) {
                    listOf("compact", "comfortable", "spacious").forEach { dens ->
                        VesselFilterChip(
                            selected = livePrefs.density == dens,
                            onClick = { onSavePrefs(livePrefs.copy(density = dens)) },
                            label = VesselI18n.tr("settings.density_$dens"),
                        )
                    }
                }
                Text(VesselI18n.tr("iptv.columns_label"), style = MaterialTheme.typography.labelLarge, color = palette.accent, fontWeight = FontWeight.SemiBold)
                FlowRow(horizontalArrangement = Arrangement.spacedBy(6.dp), verticalArrangement = Arrangement.spacedBy(6.dp)) {
                    VesselFilterChip(
                        selected = livePrefs.columns == null,
                        onClick = { onSavePrefs(livePrefs.copy(columns = null)) },
                        label = VesselI18n.tr("iptv.columns_auto"),
                    )
                    listOf(2, 3, 4, 5).forEach { cols ->
                        VesselFilterChip(
                            selected = livePrefs.columns == cols,
                            onClick = { onSavePrefs(livePrefs.copy(columns = cols)) },
                            label = "$cols",
                        )
                    }
                }
                Text(VesselI18n.tr("iptv.logo_style"), style = MaterialTheme.typography.labelLarge, color = palette.accent, fontWeight = FontWeight.SemiBold)
                FlowRow(horizontalArrangement = Arrangement.spacedBy(6.dp), verticalArrangement = Arrangement.spacedBy(6.dp)) {
                    listOf("rounded", "circle", "square", "plain").forEach { ls ->
                        VesselFilterChip(
                            selected = livePrefs.logoStyle == ls,
                            onClick = { onSavePrefs(livePrefs.copy(logoStyle = ls)) },
                            label = VesselI18n.tr("iptv.logo_$ls"),
                        )
                    }
                }
                Text(VesselI18n.tr("iptv.card_aspect"), style = MaterialTheme.typography.labelLarge, color = palette.accent, fontWeight = FontWeight.SemiBold)
                FlowRow(horizontalArrangement = Arrangement.spacedBy(6.dp), verticalArrangement = Arrangement.spacedBy(6.dp)) {
                    listOf("auto", "square", "wide_16x9", "classic_4x3").forEach { ar ->
                        VesselFilterChip(
                            selected = livePrefs.cardAspect == ar,
                            onClick = { onSavePrefs(livePrefs.copy(cardAspect = ar)) },
                            label = VesselI18n.tr("iptv.aspect_$ar"),
                        )
                    }
                }
                Text(VesselI18n.tr("iptv.title_lines"), style = MaterialTheme.typography.labelLarge, color = palette.accent, fontWeight = FontWeight.SemiBold)
                FlowRow(horizontalArrangement = Arrangement.spacedBy(6.dp), verticalArrangement = Arrangement.spacedBy(6.dp)) {
                    listOf(1 to VesselI18n.tr("iptv.lines_1"), 2 to VesselI18n.tr("iptv.lines_2")).forEach { (lines, label) ->
                        VesselFilterChip(
                            selected = livePrefs.titleLines == lines,
                            onClick = { onSavePrefs(livePrefs.copy(titleLines = lines)) },
                            label = label,
                        )
                    }
                }
                listOf(
                    Triple(VesselI18n.tr("iptv.show_logos"), livePrefs.showLogos) { v: Boolean -> onSavePrefs(livePrefs.copy(showLogos = v)) },
                    Triple(VesselI18n.tr("iptv.show_categories"), livePrefs.showCategories) { v: Boolean -> onSavePrefs(livePrefs.copy(showCategories = v)) },
                    Triple(VesselI18n.tr("iptv.show_channel_numbers"), livePrefs.showChannelNumbers) { v: Boolean -> onSavePrefs(livePrefs.copy(showChannelNumbers = v)) },
                    Triple(VesselI18n.tr("iptv.show_favorites_bar"), livePrefs.showFavoritesBar) { v: Boolean -> onSavePrefs(livePrefs.copy(showFavoritesBar = v)) },
                    Triple(VesselI18n.tr("iptv.show_section_headers"), livePrefs.showSectionHeaders) { v: Boolean -> onSavePrefs(livePrefs.copy(showSectionHeaders = v)) },
                    Triple(VesselI18n.tr("iptv.show_action_buttons"), livePrefs.showActionButtons) { v: Boolean -> onSavePrefs(livePrefs.copy(showActionButtons = v)) },
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
            VesselButton(onClick = onDismiss, height = LocalVesselUiTokens.current.compactButtonHeight) {
                Text(VesselI18n.tr("common.close"), style = MaterialTheme.typography.labelMedium)
            }
        },
    )
}

@Composable
internal fun LiveTvReorderDialog(
    livePrefs: LiveTvPreferences,
    livePlugins: List<PluginListEntry>,
    orderedSections: List<HomeSection>,
    activePluginId: String?,
    onSavePrefs: (LiveTvPreferences) -> Unit,
    onDismiss: () -> Unit,
) {
    val palette = LocalVesselPalette.current
    var pluginDraft by remember(livePlugins) { mutableStateOf(livePlugins.map { it.manifest.id }) }
    var sectionDraft by remember(orderedSections) { mutableStateOf(orderedSections.map { it.id }) }
    val pluginMapById = remember(livePlugins) { livePlugins.associateBy { it.manifest.id } }
    val sectionMapById = remember(orderedSections) { orderedSections.associateBy { it.id } }
    var draggingPid by remember { mutableStateOf<String?>(null) }
    var dragAccumPluginY by remember { mutableFloatStateOf(0f) }
    var pluginRowHeightPx by remember { mutableIntStateOf(110) }
    var draggingSid by remember { mutableStateOf<String?>(null) }
    var dragAccumSectionY by remember { mutableFloatStateOf(0f) }
    var sectionRowHeightPx by remember { mutableIntStateOf(110) }
    val focusRequesters = remember { mutableMapOf<String, Pair<FocusRequester, FocusRequester>>() }
    var pendingFocusKey by remember { mutableStateOf<Pair<String, Boolean>?>(null) }

    LaunchedEffect(pendingFocusKey, pluginDraft, sectionDraft) {
        val target = pendingFocusKey ?: return@LaunchedEffect
        kotlinx.coroutines.delay(16)
        val pair = focusRequesters[target.first]
        if (pair != null) {
            runCatching { (if (target.second) pair.first else pair.second).requestFocus() }
        }
        pendingFocusKey = null
    }

    AlertDialog(
        onDismissRequest = onDismiss,
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
                                                    onDragStart = { draggingPid = pid; dragAccumPluginY = 0f },
                                                    onDragEnd = {
                                                        draggingPid = null
                                                        dragAccumPluginY = 0f
                                                        onSavePrefs(livePrefs.copy(pluginOrder = pluginDraft))
                                                    },
                                                    onDragCancel = { draggingPid = null; dragAccumPluginY = 0f },
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
                                                onSavePrefs(livePrefs.copy(pluginOrder = ids))
                                                pendingFocusKey = "plug:$pid" to (newIdx > 0)
                                            },
                                        ) {
                                            Icon(Icons.Default.ArrowUpward, contentDescription = VesselI18n.tr("iptv.move_up"), tint = if (idx > 0) palette.textPrimary else palette.textMuted.copy(alpha = 0.35f), modifier = Modifier.size(16.dp))
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
                                                onSavePrefs(livePrefs.copy(pluginOrder = ids))
                                                pendingFocusKey = "plug:$pid" to (newIdx >= ids.lastIndex)
                                            },
                                        ) {
                                            Icon(Icons.Default.ArrowDownward, contentDescription = VesselI18n.tr("iptv.move_down"), tint = if (idx < pluginDraft.lastIndex) palette.textPrimary else palette.textMuted.copy(alpha = 0.35f), modifier = Modifier.size(16.dp))
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
                                                    onDragStart = { draggingSid = sid; dragAccumSectionY = 0f },
                                                    onDragEnd = {
                                                        draggingSid = null
                                                        dragAccumSectionY = 0f
                                                        val map = livePrefs.sectionOrderByPlugin.toMutableMap()
                                                        map[activePluginId] = sectionDraft
                                                        onSavePrefs(livePrefs.copy(sectionOrderByPlugin = map))
                                                    },
                                                    onDragCancel = { draggingSid = null; dragAccumSectionY = 0f },
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
                                                onSavePrefs(livePrefs.copy(sectionOrderByPlugin = map))
                                                pendingFocusKey = "sec:$sid" to (newIdx > 0)
                                            },
                                        ) {
                                            Icon(Icons.Default.ArrowUpward, contentDescription = VesselI18n.tr("iptv.move_up"), tint = if (idx > 0) palette.textPrimary else palette.textMuted.copy(alpha = 0.35f), modifier = Modifier.size(16.dp))
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
                                                onSavePrefs(livePrefs.copy(sectionOrderByPlugin = map))
                                                pendingFocusKey = "sec:$sid" to (newIdx >= ids.lastIndex)
                                            },
                                        ) {
                                            Icon(Icons.Default.ArrowDownward, contentDescription = VesselI18n.tr("iptv.move_down"), tint = if (idx < sectionDraft.lastIndex) palette.textPrimary else palette.textMuted.copy(alpha = 0.35f), modifier = Modifier.size(16.dp))
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
            VesselButton(onClick = onDismiss, height = LocalVesselUiTokens.current.compactButtonHeight) {
                Text(VesselI18n.tr("common.close"), style = MaterialTheme.typography.labelMedium)
            }
        },
        dismissButton = {
            VesselTextButton(onClick = {
                val map = livePrefs.sectionOrderByPlugin.toMutableMap()
                if (activePluginId != null) map.remove(activePluginId)
                onSavePrefs(livePrefs.copy(pluginOrder = emptyList(), sectionOrderByPlugin = map))
                onDismiss()
            }) {
                Icon(Icons.Default.RestartAlt, contentDescription = null, modifier = Modifier.size(15.dp))
                Spacer(Modifier.width(4.dp))
                Text(VesselI18n.tr("iptv.reset_order"))
            }
        },
    )
}
