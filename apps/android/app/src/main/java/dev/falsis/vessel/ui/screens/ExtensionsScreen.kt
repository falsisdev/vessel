package dev.falsis.vessel.ui.screens

import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.animateContentSize
import androidx.compose.animation.core.tween
import androidx.compose.animation.expandVertically
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.shrinkVertically
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
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.automirrored.filled.KeyboardArrowRight
import androidx.compose.material.icons.filled.Close
import androidx.compose.material.icons.filled.Delete
import androidx.compose.material.icons.filled.Download
import androidx.compose.material.icons.filled.Extension
import androidx.compose.material.icons.filled.Home
import androidx.compose.material.icons.filled.KeyboardArrowDown
import androidx.compose.material.icons.filled.Language
import androidx.compose.material.icons.filled.Refresh
import androidx.compose.material.icons.filled.SystemUpdateAlt
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Surface
import androidx.compose.material3.Switch
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateMapOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import coil3.compose.AsyncImage
import dev.falsis.vessel.data.PluginListEntry
import dev.falsis.vessel.data.PluginUpdateInfo
import dev.falsis.vessel.data.ResolvedSource
import dev.falsis.vessel.data.VesselRepository
import dev.falsis.vessel.ui.components.VesselButton
import dev.falsis.vessel.ui.components.VesselCompactIconButton
import dev.falsis.vessel.ui.components.VesselFilterChip
import dev.falsis.vessel.ui.components.VesselOutlinedButton
import dev.falsis.vessel.ui.components.VesselSearchField
import dev.falsis.vessel.ui.components.VesselTextButton
import dev.falsis.vessel.ui.components.vesselTopSafeInset
import dev.falsis.vessel.ui.i18n.VesselI18n
import dev.falsis.vessel.ui.theme.LocalVesselPalette
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch

private data class PluginGroupUi(
    val name: String,
    val items: List<PluginListEntry>,
    val enabledCount: Int,
    val homeCount: Int,
    val iconUrl: String?,
)

private fun resolveGroupName(entry: PluginListEntry): String {
    val rid = entry.repoId?.trim()
    if (!rid.isNullOrEmpty()) return rid
    val domains = entry.allowedDomains
    if (
        "raw.githubusercontent.com" in domains &&
        (entry.manifest.author == "falsisdev" ||
            entry.manifest.author == "metemel" ||
            entry.manifest.id.startsWith("anthology") ||
            entry.manifest.id == "m3u.anthology.addon")
    ) {
        return "Anthology"
    }
    if (entry.manifest.id.startsWith("community.anthology")) {
        return "Anthology"
    }
    if (entry.sourceType == "dev_dir") {
        return "Local Dev Plugins"
    }
    return "Standalone Plugins"
}

private fun resolveGroupIcon(items: List<PluginListEntry>): String? {
    val counts = LinkedHashMap<String, Int>()
    for (item in items) {
        val icon = item.manifest.icon?.trim()
        if (!icon.isNullOrEmpty()) {
            counts[icon] = (counts[icon] ?: 0) + 1
        }
    }
    return counts.maxByOrNull { it.value }?.key
}

@Composable
fun ExtensionsScreen(
    repo: VesselRepository,
    initialSourceInput: String? = null,
    onBack: (() -> Unit)? = null,
) {
    val palette = LocalVesselPalette.current
    val scope = rememberCoroutineScope()
    val plugins by repo.plugins.collectAsState()
    val prefs by repo.uiPreferences.collectAsState()

    var sourceInput by remember(initialSourceInput) { mutableStateOf(initialSourceInput ?: "") }
    var resolving by remember { mutableStateOf(false) }
    var resolved by remember { mutableStateOf<ResolvedSource?>(null) }
    var repoFilter by remember { mutableStateOf("") }
    var installedFilter by remember { mutableStateOf("") }
    var installingTarget by remember { mutableStateOf<String?>(null) }
    var batchInstalling by remember { mutableStateOf(false) }
    var batchProgress by remember { mutableStateOf<String?>(null) }
    var updateMap by remember { mutableStateOf<Map<String, PluginUpdateInfo>>(emptyMap()) }
    var checkingUpdates by remember { mutableStateOf(false) }
    var updatingId by remember { mutableStateOf<String?>(null) }
    var updatingAll by remember { mutableStateOf(false) }

    val availableUpdateCount = remember(updateMap) {
        updateMap.values.count { it.updateAvailable }
    }

    fun runCheckUpdates(silent: Boolean) {
        if (checkingUpdates || plugins.isEmpty()) return
        checkingUpdates = true
        scope.launch {
            runCatching { repo.checkPluginUpdates() }
                .onSuccess { list ->
                    updateMap = list.associateBy { it.pluginId }
                    if (!silent) {
                        val count = list.count { it.updateAvailable }
                        repo.notify(
                            if (count > 0) VesselI18n.trf("extensions.updates_found", "count" to count)
                            else VesselI18n.tr("extensions.up_to_date"),
                        )
                    }
                }
                .onFailure {
                    if (!silent) repo.notifyError(it, VesselI18n.tr("extensions.update_failed"))
                }
            checkingUpdates = false
        }
    }

    LaunchedEffect(plugins.size) {
        if (plugins.isNotEmpty() && updateMap.isEmpty()) {
            runCheckUpdates(silent = true)
        }
    }

    val expandedGroups = remember { mutableStateMapOf<String, Boolean>() }
    var confirmRemoveGroup by remember { mutableStateOf<String?>(null) }
    var confirmUninstallId by remember { mutableStateOf<String?>(null) }

    LaunchedEffect(confirmRemoveGroup) {
        val target = confirmRemoveGroup ?: return@LaunchedEffect
        delay(3000)
        if (confirmRemoveGroup == target) confirmRemoveGroup = null
    }

    LaunchedEffect(confirmUninstallId) {
        val target = confirmUninstallId ?: return@LaunchedEffect
        delay(3000)
        if (confirmUninstallId == target) confirmUninstallId = null
    }

    val selectedHomeSet = remember(plugins, prefs.home.selectedPlugins) {
        val sel = prefs.home.selectedPlugins
        if (sel != null) {
            sel.toSet()
        } else {
            plugins.filter { it.enabled }.map { it.manifest.id }.toSet()
        }
    }

    val installedIds = remember(plugins) { plugins.map { it.manifest.id }.toSet() }

    val groupedPlugins = remember(plugins, installedFilter, selectedHomeSet) {
        val q = installedFilter.trim().lowercase()
        val buckets = LinkedHashMap<String, MutableList<PluginListEntry>>()
        for (entry in plugins) {
            val groupName = resolveGroupName(entry)
            if (
                q.isNotEmpty() &&
                !entry.manifest.name.lowercase().contains(q) &&
                !entry.manifest.id.lowercase().contains(q) &&
                !groupName.lowercase().contains(q)
            ) {
                continue
            }
            buckets.getOrPut(groupName) { mutableListOf() }.add(entry)
        }
        buckets.entries.map { (name, items) ->
            PluginGroupUi(
                name = name,
                items = items,
                enabledCount = items.count { it.enabled },
                homeCount = items.count { it.enabled && it.manifest.id in selectedHomeSet },
                iconUrl = resolveGroupIcon(items),
            )
        }
    }

    fun isGroupExpanded(group: PluginGroupUi): Boolean {
        if (installedFilter.trim().isNotEmpty()) return true
        return expandedGroups[group.name] ?: (group.items.size == 1)
    }

    suspend fun saveHomeSelection(ids: Set<String>) {
        val enabledIds = plugins.filter { it.enabled }.map { it.manifest.id }.toSet()
        val next = if (enabledIds.isNotEmpty() && ids.containsAll(enabledIds) && ids.size == enabledIds.size) {
            null
        } else {
            ids.toList()
        }
        runCatching {
            repo.saveUiPreferences(prefs.copy(home = prefs.home.copy(selectedPlugins = next)))
        }
    }

    fun inspectSource() {
        val raw = sourceInput.trim()
        if (raw.isEmpty()) return
        resolving = true
        repoFilter = ""
        scope.launch {
            runCatching { repo.resolveSource(raw) }
                .onSuccess { resolved = it }
                .onFailure { repo.notifyError(it, "Failed to inspect source") }
            resolving = false
        }
    }

    fun installAllFromRepo() {
        val res = resolved ?: return
        val repoObj = res.repo ?: return
        if (batchInstalling) return
        val q = repoFilter.trim().lowercase()
        val filtered = if (q.isEmpty()) {
            repoObj.plugins
        } else {
            repoObj.plugins.filter {
                it.name.lowercase().contains(q) ||
                    it.id.lowercase().contains(q) ||
                    (it.description ?: "").lowercase().contains(q)
            }
        }
        val targets = filtered.filter { entry ->
            val target = entry.downloadUrl ?: entry.sourcePath
            !target.isNullOrBlank() && !(entry.description ?: "").startsWith("[Disabled in repo]")
        }
        if (targets.isEmpty()) return
        batchInstalling = true
        scope.launch {
            var okCount = 0
            var failCount = 0
            val repoLabel = repoObj.name.ifBlank { repoObj.id }
            targets.forEachIndexed { idx, entry ->
                val target = entry.downloadUrl ?: entry.sourcePath ?: res.resolvedUrl
                installingTarget = target
                batchProgress = "${idx + 1}/${targets.size}"
                runCatching { repo.installSource(target, repoLabel) }
                    .onSuccess { okCount++ }
                    .onFailure { failCount++ }
            }
            installingTarget = null
            batchProgress = null
            batchInstalling = false
            val msg = if (failCount > 0) {
                "$repoLabel: $okCount installed ($failCount failed)"
            } else {
                "$repoLabel: $okCount installed"
            }
            repo.notify(msg)
        }
    }

    LazyColumn(
        modifier = Modifier
            .fillMaxSize()
            .background(palette.bgBase)
            .vesselTopSafeInset(),
        contentPadding = PaddingValues(start = 16.dp, top = 14.dp, end = 16.dp, bottom = 28.dp),
        verticalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        // Header
        item {
            Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                if (onBack != null) {
                    VesselCompactIconButton(onClick = onBack, size = 34.dp, containerColor = palette.bgSurface) {
                        Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = VesselI18n.tr("common.back"), modifier = Modifier.size(18.dp))
                    }
                }
                Text(VesselI18n.tr("extensions.title"), style = MaterialTheme.typography.headlineSmall, fontWeight = FontWeight.Bold)
            }
        }

        // Source Input + Quick Shortcodes
        item {
            Surface(shape = RoundedCornerShape(14.dp), color = palette.bgSurface) {
                Column(modifier = Modifier.fillMaxWidth().padding(14.dp), verticalArrangement = Arrangement.spacedBy(10.dp)) {
                    OutlinedTextField(
                        value = sourceInput,
                        onValueChange = { sourceInput = it },
                        placeholder = { Text(VesselI18n.tr("extensions.source_placeholder")) },
                        singleLine = true,
                        modifier = Modifier.fillMaxWidth(),
                    )
                    Row(
                        horizontalArrangement = Arrangement.spacedBy(8.dp),
                        verticalAlignment = Alignment.CenterVertically,
                    ) {
                        VesselButton(onClick = { inspectSource() }, enabled = !resolving && !batchInstalling) {
                            Text(VesselI18n.tr("extensions.resolve"))
                        }
                        listOf("anthology", "tr").forEach { code ->
                            VesselFilterChip(
                                selected = sourceInput.trim().equals(code, ignoreCase = true),
                                onClick = {
                                    sourceInput = code
                                    inspectSource()
                                },
                                label = code,
                            )
                        }
                        if (resolving) {
                            CircularProgressIndicator(modifier = Modifier.size(18.dp), strokeWidth = 2.dp)
                        }
                    }
                }
            }
        }

        // Resolved Repo / Single Plugin Preview Card
        resolved?.let { res ->
            item {
                Surface(shape = RoundedCornerShape(14.dp), color = palette.bgSurface) {
                    Column(
                        modifier = Modifier.fillMaxWidth().padding(14.dp),
                        verticalArrangement = Arrangement.spacedBy(10.dp),
                    ) {
                        val repoObj = res.repo
                        val pkgManifest = res.manifest
                        if (repoObj != null) {
                            val q = repoFilter.trim().lowercase()
                            val filteredRepo = remember(repoObj.plugins, q) {
                                if (q.isEmpty()) {
                                    repoObj.plugins
                                } else {
                                    repoObj.plugins.filter {
                                        it.name.lowercase().contains(q) ||
                                            it.id.lowercase().contains(q) ||
                                            (it.description ?: "").lowercase().contains(q)
                                    }
                                }
                            }
                            val enabledTargets = filteredRepo.count {
                                val target = it.downloadUrl ?: it.sourcePath
                                !target.isNullOrBlank() && !(it.description ?: "").startsWith("[Disabled in repo]")
                            }

                            Row(
                                modifier = Modifier.fillMaxWidth(),
                                horizontalArrangement = Arrangement.SpaceBetween,
                                verticalAlignment = Alignment.CenterVertically,
                            ) {
                                Column(modifier = Modifier.weight(1f).padding(end = 8.dp)) {
                                    Text(
                                        text = "${repoObj.name} (${repoObj.plugins.size})",
                                        style = MaterialTheme.typography.titleMedium,
                                        fontWeight = FontWeight.Bold,
                                    )
                                    Text(
                                        text = res.badge,
                                        style = MaterialTheme.typography.labelSmall,
                                        color = palette.accent,
                                    )
                                }
                                Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(6.dp)) {
                                    VesselButton(
                                        onClick = { installAllFromRepo() },
                                        enabled = !batchInstalling && enabledTargets > 0,
                                        height = 32.dp,
                                    ) {
                                        Icon(Icons.Default.Download, contentDescription = null, modifier = Modifier.size(14.dp))
                                        Spacer(Modifier.width(5.dp))
                                        Text(
                                            if (batchInstalling && batchProgress != null) {
                                                "${VesselI18n.tr("common.loading")} ($batchProgress)"
                                            } else {
                                                VesselI18n.tr("extensions.install_all")
                                            },
                                        )
                                    }
                                    VesselCompactIconButton(onClick = { resolved = null }, enabled = !batchInstalling, size = 30.dp) {
                                        Icon(Icons.Default.Close, contentDescription = VesselI18n.tr("common.close"), modifier = Modifier.size(16.dp))
                                    }
                                }
                            }

                            if (repoObj.plugins.size > 4) {
                                VesselSearchField(
                                    value = repoFilter,
                                    onValueChange = { repoFilter = it },
                                    placeholder = VesselI18n.tr("extensions.filter_repo"),
                                    imeAction = ImeAction.Done,
                                    modifier = Modifier.fillMaxWidth(),
                                )
                            }

                            HorizontalDivider(color = palette.border.copy(alpha = 0.45f))

                            filteredRepo.forEach { entry ->
                                val installTarget = entry.downloadUrl ?: entry.sourcePath ?: res.resolvedUrl
                                val isInstalled = entry.id in installedIds
                                val isDisabledInRepo = (entry.description ?: "").startsWith("[Disabled in repo]")
                                val isBusy = batchInstalling || installingTarget == installTarget

                                Row(
                                    modifier = Modifier.fillMaxWidth(),
                                    horizontalArrangement = Arrangement.SpaceBetween,
                                    verticalAlignment = Alignment.CenterVertically,
                                ) {
                                    Column(modifier = Modifier.weight(1f).padding(end = 8.dp)) {
                                        Row(
                                            verticalAlignment = Alignment.CenterVertically,
                                            horizontalArrangement = Arrangement.spacedBy(6.dp),
                                        ) {
                                            Text(
                                                text = "${entry.name} v${entry.version}",
                                                fontWeight = FontWeight.SemiBold,
                                                color = if (isDisabledInRepo) palette.textMuted else palette.textPrimary,
                                            )
                                            if (isInstalled) {
                                                Surface(
                                                    color = palette.success.copy(alpha = 0.16f),
                                                    shape = RoundedCornerShape(6.dp),
                                                ) {
                                                    Text(
                                                        text = VesselI18n.tr("extensions.installed_badge", "Installed"),
                                                        style = MaterialTheme.typography.labelSmall,
                                                        color = palette.success,
                                                        modifier = Modifier.padding(horizontal = 6.dp, vertical = 2.dp),
                                                    )
                                                }
                                            }
                                        }
                                        entry.description?.let {
                                            Text(
                                                text = it,
                                                style = MaterialTheme.typography.bodySmall,
                                                color = palette.textMuted,
                                                maxLines = 2,
                                                overflow = TextOverflow.Ellipsis,
                                            )
                                        }
                                    }
                                    VesselButton(
                                        enabled = !isBusy,
                                        height = 31.dp,
                                        contentPadding = PaddingValues(horizontal = 10.dp, vertical = 0.dp),
                                        onClick = {
                                            scope.launch {
                                                installingTarget = installTarget
                                                val repoLabel = repoObj.name.ifBlank { repoObj.id }
                                                runCatching { repo.installSource(installTarget, repoLabel) }
                                                    .onSuccess { repo.notify(VesselI18n.trf("extensions.installed_msg", "name" to entry.name)) }
                                                    .onFailure { repo.notifyError(it, VesselI18n.tr("extensions.install_failed")) }
                                                installingTarget = null
                                            }
                                        },
                                    ) {
                                        Text(
                                            when {
                                                installingTarget == installTarget -> batchProgress ?: "..."
                                                updateMap[entry.id]?.updateAvailable == true -> VesselI18n.tr("common.update")
                                                isInstalled -> VesselI18n.tr("common.reinstall")
                                                else -> VesselI18n.tr("extensions.install_now")
                                            },
                                            style = MaterialTheme.typography.labelSmall,
                                        )
                                    }
                                }
                            }
                        } else if (pkgManifest != null) {
                            Row(
                                modifier = Modifier.fillMaxWidth(),
                                horizontalArrangement = Arrangement.SpaceBetween,
                                verticalAlignment = Alignment.CenterVertically,
                            ) {
                                Column(modifier = Modifier.weight(1f).padding(end = 8.dp)) {
                                    Text("${pkgManifest.name} v${pkgManifest.version} (${res.badge})", fontWeight = FontWeight.Bold)
                                    pkgManifest.description?.let {
                                        Text(it, style = MaterialTheme.typography.bodySmall, color = palette.textMuted)
                                    }
                                }
                                Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(6.dp)) {
                                    VesselButton(
                                        enabled = !batchInstalling && installingTarget == null,
                                        height = 32.dp,
                                        onClick = {
                                            scope.launch {
                                                installingTarget = sourceInput.trim()
                                                runCatching { repo.installSource(sourceInput.trim(), null) }
                                                    .onSuccess {
                                                        repo.notify(VesselI18n.trf("extensions.installed_msg", "name" to pkgManifest.name))
                                                        resolved = null
                                                    }
                                                    .onFailure { repo.notifyError(it, VesselI18n.tr("extensions.install_failed")) }
                                                installingTarget = null
                                            }
                                        },
                                    ) {
                                        Text(VesselI18n.tr("extensions.install_now"))
                                    }
                                    VesselCompactIconButton(onClick = { resolved = null }, size = 30.dp) {
                                        Icon(Icons.Default.Close, contentDescription = VesselI18n.tr("common.close"), modifier = Modifier.size(16.dp))
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // Installed Plugins Toolbar (Search + Check/Update All + Home count chip)
        if (plugins.isNotEmpty()) {
            item {
                Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        horizontalArrangement = Arrangement.spacedBy(8.dp),
                        verticalAlignment = Alignment.CenterVertically,
                    ) {
                        VesselSearchField(
                            value = installedFilter,
                            onValueChange = { installedFilter = it },
                            placeholder = VesselI18n.tr("extensions.filter_installed"),
                            imeAction = ImeAction.Done,
                            modifier = Modifier.weight(1f),
                        )
                        Surface(
                            color = palette.accent.copy(alpha = 0.16f),
                            shape = RoundedCornerShape(10.dp),
                        ) {
                            Row(
                                modifier = Modifier.padding(horizontal = 10.dp, vertical = 8.dp),
                                verticalAlignment = Alignment.CenterVertically,
                                horizontalArrangement = Arrangement.spacedBy(4.dp),
                            ) {
                                Icon(Icons.Default.Home, contentDescription = null, tint = palette.accent, modifier = Modifier.size(15.dp))
                                Text(
                                    text = "${selectedHomeSet.size}",
                                    style = MaterialTheme.typography.labelMedium,
                                    fontWeight = FontWeight.Bold,
                                    color = palette.accent,
                                )
                            }
                        }
                    }

                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        horizontalArrangement = Arrangement.spacedBy(8.dp),
                        verticalAlignment = Alignment.CenterVertically,
                    ) {
                        VesselOutlinedButton(
                            onClick = { runCheckUpdates(silent = false) },
                            enabled = !checkingUpdates && !updatingAll,
                            height = 32.dp,
                            contentPadding = PaddingValues(horizontal = 10.dp, vertical = 0.dp),
                        ) {
                            if (checkingUpdates) {
                                CircularProgressIndicator(modifier = Modifier.size(13.dp), strokeWidth = 2.dp)
                            } else {
                                Icon(Icons.Default.Refresh, contentDescription = null, modifier = Modifier.size(14.dp))
                            }
                            Spacer(Modifier.width(5.dp))
                            Text(
                                text = if (checkingUpdates) {
                                    VesselI18n.tr("extensions.checking_updates")
                                } else {
                                    VesselI18n.tr("extensions.check_updates")
                                },
                                style = MaterialTheme.typography.labelSmall,
                            )
                        }

                        if (availableUpdateCount > 0) {
                            VesselButton(
                                onClick = {
                                    if (!updatingAll) {
                                        updatingAll = true
                                        scope.launch {
                                            runCatching { repo.updateAllPlugins() }
                                                .onSuccess { res ->
                                                    if (res.updated.isNotEmpty()) {
                                                        repo.notify(VesselI18n.trf("extensions.batch_updated_msg", "count" to res.updated.size))
                                                    }
                                                    runCheckUpdates(silent = true)
                                                }
                                                .onFailure { repo.notifyError(it, VesselI18n.tr("extensions.update_failed")) }
                                            updatingAll = false
                                        }
                                    }
                                },
                                enabled = !updatingAll && !checkingUpdates,
                                height = 32.dp,
                                contentPadding = PaddingValues(horizontal = 10.dp, vertical = 0.dp),
                            ) {
                                if (updatingAll) {
                                    CircularProgressIndicator(modifier = Modifier.size(13.dp), strokeWidth = 2.dp)
                                } else {
                                    Icon(Icons.Default.SystemUpdateAlt, contentDescription = null, modifier = Modifier.size(14.dp))
                                }
                                Spacer(Modifier.width(5.dp))
                                Text(
                                    text = VesselI18n.trf("extensions.update_all", "count" to availableUpdateCount),
                                    style = MaterialTheme.typography.labelSmall,
                                )
                            }
                        }
                    }
                }
            }
        }

        // Empty state
        if (plugins.isEmpty()) {
            item {
                Surface(shape = RoundedCornerShape(14.dp), color = palette.bgSurface) {
                    Text(
                        text = VesselI18n.tr("common.empty", "No plugins installed yet."),
                        color = palette.textMuted,
                        modifier = Modifier.fillMaxWidth().padding(18.dp),
                    )
                }
            }
        }

        // Grouped Installed Plugins (matching Desktop ExtensionsView grouping & controls)
        items(groupedPlugins, key = { it.name }) { group ->
            val expanded = isGroupExpanded(group)
            Surface(shape = RoundedCornerShape(14.dp), color = palette.bgSurface) {
                Column(modifier = Modifier.fillMaxWidth()) {
                    // Group Header (tap to expand/collapse)
                    Row(
                        modifier = Modifier
                            .fillMaxWidth()
                            .clickable { expandedGroups[group.name] = !expanded }
                            .padding(horizontal = 14.dp, vertical = 12.dp),
                        verticalAlignment = Alignment.CenterVertically,
                        horizontalArrangement = Arrangement.spacedBy(10.dp),
                    ) {
                        Icon(
                            imageVector = if (expanded) Icons.Default.KeyboardArrowDown else Icons.AutoMirrored.Filled.KeyboardArrowRight,
                            contentDescription = null,
                            tint = palette.textMuted,
                        )
                        Box(
                            modifier = Modifier
                                .size(36.dp)
                                .clip(RoundedCornerShape(8.dp))
                                .background(palette.bgElevated),
                            contentAlignment = Alignment.Center,
                        ) {
                            if (!group.iconUrl.isNullOrBlank()) {
                                AsyncImage(
                                    model = group.iconUrl,
                                    contentDescription = group.name,
                                    contentScale = ContentScale.Crop,
                                    modifier = Modifier.fillMaxSize(),
                                )
                            } else {
                                Icon(Icons.Default.Extension, contentDescription = null, tint = palette.accent, modifier = Modifier.size(20.dp))
                            }
                        }
                        Column(modifier = Modifier.weight(1f)) {
                            Text(
                                text = group.name,
                                style = MaterialTheme.typography.titleMedium,
                                fontWeight = FontWeight.Bold,
                                maxLines = 1,
                                overflow = TextOverflow.Ellipsis,
                            )
                            Text(
                                text = "${group.enabledCount}/${group.items.size} ${VesselI18n.tr("home.active_count")} · ${group.homeCount} ${VesselI18n.tr("extensions.home_label")}",
                                style = MaterialTheme.typography.labelSmall,
                                color = palette.textMuted,
                            )
                        }
                    }

                    // Clean Group Actions Bar (Pin/Clear Home · Enable/Disable All · Remove Group)
                    Row(
                        modifier = Modifier
                            .fillMaxWidth()
                            .padding(start = 14.dp, end = 14.dp, bottom = 10.dp),
                        horizontalArrangement = Arrangement.spacedBy(8.dp),
                        verticalAlignment = Alignment.CenterVertically,
                    ) {
                        VesselOutlinedButton(
                            onClick = {
                                val next = selectedHomeSet.toMutableSet()
                                val showAll = group.homeCount == 0
                                for (item in group.items) {
                                    if (showAll && item.enabled) next.add(item.manifest.id)
                                    else next.remove(item.manifest.id)
                                }
                                scope.launch {
                                    saveHomeSelection(next)
                                    repo.notify(
                                        if (showAll) VesselI18n.trf("extensions.pinned_home", "name" to group.name)
                                        else VesselI18n.trf("extensions.cleared_home", "name" to group.name),
                                    )
                                }
                            },
                            height = 30.dp,
                            contentPadding = PaddingValues(horizontal = 10.dp, vertical = 0.dp),
                        ) {
                            Icon(Icons.Default.Home, contentDescription = null, modifier = Modifier.size(13.dp))
                            Spacer(Modifier.width(4.dp))
                            Text(
                                text = if (group.homeCount == 0) {
                                    VesselI18n.tr("extensions.pin_home")
                                } else {
                                    VesselI18n.tr("extensions.clear_home")
                                },
                                style = MaterialTheme.typography.labelSmall,
                            )
                        }

                        val enableTarget = group.enabledCount < group.items.size
                        VesselOutlinedButton(
                            onClick = {
                                scope.launch {
                                    for (item in group.items) {
                                        if (item.enabled != enableTarget) {
                                            repo.setPluginEnabled(item.manifest.id, enableTarget)
                                        }
                                    }
                                    repo.notify(
                                        if (enableTarget) VesselI18n.trf("extensions.enabled_group", "name" to group.name)
                                        else VesselI18n.trf("extensions.disabled_group", "name" to group.name),
                                    )
                                }
                            },
                            height = 30.dp,
                            contentPadding = PaddingValues(horizontal = 10.dp, vertical = 0.dp),
                        ) {
                            Text(
                                text = if (enableTarget) {
                                    VesselI18n.tr("extensions.enable_all")
                                } else {
                                    VesselI18n.tr("extensions.disable_all")
                                },
                                style = MaterialTheme.typography.labelSmall,
                            )
                        }

                        Spacer(Modifier.weight(1f))

                        val confirmingGroup = confirmRemoveGroup == group.name
                        VesselTextButton(
                            onClick = {
                                if (!confirmingGroup) {
                                    confirmRemoveGroup = group.name
                                } else {
                                    confirmRemoveGroup = null
                                    val ids = group.items.map { it.manifest.id }.toSet()
                                    scope.launch {
                                        for (id in ids) {
                                            repo.uninstallPlugin(id)
                                        }
                                        if (prefs.home.selectedPlugins != null) {
                                            saveHomeSelection(selectedHomeSet - ids)
                                        }
                                        repo.notify(VesselI18n.trf("extensions.removed_group", "name" to group.name))
                                    }
                                }
                            },
                            height = 30.dp,
                            contentColor = palette.danger,
                        ) {
                            Icon(Icons.Default.Delete, contentDescription = null, tint = palette.danger, modifier = Modifier.size(14.dp))
                            Spacer(Modifier.width(3.dp))
                            Text(
                                text = if (confirmingGroup) VesselI18n.tr("common.confirm") else VesselI18n.tr("common.remove"),
                                style = MaterialTheme.typography.labelSmall,
                                color = palette.danger,
                            )
                        }
                    }

                    // Expanded Sublist of Plugins inside this Group (smooth expand/collapse)
                    AnimatedVisibility(
                        visible = expanded,
                        enter = expandVertically(tween(220)) + fadeIn(tween(180)),
                        exit = shrinkVertically(tween(180)) + fadeOut(tween(140)),
                    ) {
                        HorizontalDivider(color = palette.border.copy(alpha = 0.45f))
                        Column(
                            modifier = Modifier
                                .fillMaxWidth()
                                .background(palette.bgBase.copy(alpha = 0.35f))
                                .padding(horizontal = 14.dp, vertical = 8.dp),
                            verticalArrangement = Arrangement.spacedBy(10.dp),
                        ) {
                            group.items.forEachIndexed { idx, plugin ->
                                if (idx > 0) {
                                    HorizontalDivider(color = palette.border.copy(alpha = 0.3f))
                                }
                                Column(verticalArrangement = Arrangement.spacedBy(6.dp)) {
                                    Row(
                                        modifier = Modifier.fillMaxWidth(),
                                        verticalAlignment = Alignment.CenterVertically,
                                        horizontalArrangement = Arrangement.SpaceBetween,
                                    ) {
                                        Row(
                                            modifier = Modifier.weight(1f).padding(end = 8.dp),
                                            verticalAlignment = Alignment.CenterVertically,
                                            horizontalArrangement = Arrangement.spacedBy(8.dp),
                                        ) {
                                            if (!plugin.manifest.icon.isNullOrBlank()) {
                                                AsyncImage(
                                                    model = plugin.manifest.icon,
                                                    contentDescription = null,
                                                    contentScale = ContentScale.Crop,
                                                    modifier = Modifier
                                                        .size(20.dp)
                                                        .clip(RoundedCornerShape(4.dp)),
                                                )
                                            }
                                            Box(
                                                modifier = Modifier
                                                    .size(8.dp)
                                                    .clip(CircleShape)
                                                    .background(if (plugin.enabled) palette.success else palette.textMuted.copy(alpha = 0.45f)),
                                            )
                                            Column {
                                                Row(
                                                    verticalAlignment = Alignment.CenterVertically,
                                                    horizontalArrangement = Arrangement.spacedBy(6.dp),
                                                ) {
                                                    Text(
                                                        text = plugin.manifest.name,
                                                        fontWeight = FontWeight.SemiBold,
                                                        maxLines = 1,
                                                        overflow = TextOverflow.Ellipsis,
                                                    )
                                                    Text(
                                                        text = "v${plugin.manifest.version}",
                                                        style = MaterialTheme.typography.labelSmall,
                                                        color = palette.textMuted,
                                                    )
                                                }
                                                if (plugin.allowedDomains.isNotEmpty()) {
                                                    Row(
                                                        verticalAlignment = Alignment.CenterVertically,
                                                        horizontalArrangement = Arrangement.spacedBy(3.dp),
                                                    ) {
                                                        Icon(Icons.Default.Language, contentDescription = null, tint = palette.textMuted, modifier = Modifier.size(11.dp))
                                                        Text(
                                                            text = VesselI18n.trf("extensions.domains_count", "count" to plugin.allowedDomains.size),
                                                            style = MaterialTheme.typography.labelSmall,
                                                            color = palette.textMuted,
                                                        )
                                                    }
                                                }
                                            }
                                        }

                                        // Per-plugin Update + Home toggle + Enabled Switch + Uninstall
                                        Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(4.dp)) {
                                            val upd = updateMap[plugin.manifest.id]
                                            val hasUpdate = upd?.updateAvailable == true
                                            val isUpdating = updatingId == plugin.manifest.id
                                            VesselCompactIconButton(
                                                enabled = !isUpdating && !updatingAll,
                                                size = 30.dp,
                                                containerColor = if (hasUpdate) palette.accent.copy(alpha = 0.18f) else palette.bgSurface,
                                                onClick = {
                                                    updatingId = plugin.manifest.id
                                                    scope.launch {
                                                        runCatching { repo.updatePlugin(plugin.manifest.id) }
                                                            .onSuccess { res ->
                                                                updateMap = updateMap - plugin.manifest.id
                                                                repo.notify(
                                                                    VesselI18n.trf(
                                                                        "extensions.updated_msg",
                                                                        "name" to plugin.manifest.name,
                                                                        "version" to res.version,
                                                                    ),
                                                                )
                                                            }
                                                            .onFailure { repo.notifyError(it, VesselI18n.tr("extensions.update_failed")) }
                                                        updatingId = null
                                                    }
                                                },
                                            ) {
                                                if (isUpdating) {
                                                    CircularProgressIndicator(modifier = Modifier.size(14.dp), strokeWidth = 2.dp)
                                                } else {
                                                    Icon(
                                                        imageVector = if (hasUpdate) Icons.Default.SystemUpdateAlt else Icons.Default.Refresh,
                                                        contentDescription = VesselI18n.tr("common.update"),
                                                        tint = if (hasUpdate) palette.accent else palette.textMuted,
                                                        modifier = Modifier.size(16.dp),
                                                    )
                                                }
                                            }

                                            val onHome = plugin.enabled && plugin.manifest.id in selectedHomeSet
                                            VesselCompactIconButton(
                                                enabled = plugin.enabled,
                                                size = 30.dp,
                                                onClick = {
                                                    val next = selectedHomeSet.toMutableSet()
                                                    if (onHome) next.remove(plugin.manifest.id)
                                                    else next.add(plugin.manifest.id)
                                                    scope.launch { saveHomeSelection(next) }
                                                },
                                            ) {
                                                Icon(
                                                    imageVector = Icons.Default.Home,
                                                    contentDescription = VesselI18n.tr("extensions.home_label"),
                                                    tint = if (onHome) palette.accent else palette.textMuted.copy(alpha = 0.45f),
                                                    modifier = Modifier.size(16.dp),
                                                )
                                            }

                                            Switch(
                                                checked = plugin.enabled,
                                                onCheckedChange = { enabled ->
                                                    scope.launch { repo.setPluginEnabled(plugin.manifest.id, enabled) }
                                                },
                                            )

                                            val confirming = confirmUninstallId == plugin.manifest.id
                                            VesselCompactIconButton(
                                                size = 30.dp,
                                                onClick = {
                                                    if (!confirming) {
                                                        confirmUninstallId = plugin.manifest.id
                                                    } else {
                                                        confirmUninstallId = null
                                                        scope.launch { repo.uninstallPlugin(plugin.manifest.id) }
                                                    }
                                                },
                                            ) {
                                                Icon(
                                                    imageVector = Icons.Default.Delete,
                                                    contentDescription = VesselI18n.tr("common.uninstall"),
                                                    tint = if (confirming) palette.warning else palette.danger,
                                                    modifier = Modifier.size(16.dp),
                                                )
                                            }
                                        }
                                    }

                                    plugin.manifest.description?.takeIf { it.isNotBlank() }?.let { desc ->
                                        Text(
                                            text = desc,
                                            style = MaterialTheme.typography.bodySmall,
                                            color = palette.textMuted,
                                            maxLines = 2,
                                            overflow = TextOverflow.Ellipsis,
                                        )
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
