package dev.falsis.vessel.ui.screens

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.filled.Delete
import androidx.compose.material.icons.filled.Download
import androidx.compose.material3.AssistChip
import androidx.compose.material3.Button
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
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
import androidx.compose.foundation.layout.size
import dev.falsis.vessel.data.ResolvedSource
import dev.falsis.vessel.data.VesselRepository
import dev.falsis.vessel.ui.i18n.VesselI18n
import dev.falsis.vessel.ui.theme.LocalVesselPalette
import kotlinx.coroutines.launch

@Composable
fun ExtensionsScreen(
    repo: VesselRepository,
    initialSourceInput: String? = null,
    onBack: (() -> Unit)? = null,
) {
    val palette = LocalVesselPalette.current
    val scope = rememberCoroutineScope()
    val plugins by repo.plugins.collectAsState()

    var sourceInput by remember(initialSourceInput) { mutableStateOf(initialSourceInput ?: "") }
    var resolving by remember { mutableStateOf(false) }
    var resolved by remember { mutableStateOf<ResolvedSource?>(null) }
    var installingTarget by remember { mutableStateOf<String?>(null) }
    var batchInstalling by remember { mutableStateOf(false) }
    var batchProgress by remember { mutableStateOf<String?>(null) }

    val installedIds = remember(plugins) { plugins.map { it.manifest.id }.toSet() }

    fun inspectSource() {
        val raw = sourceInput.trim()
        if (raw.isEmpty()) return
        resolving = true
        scope.launch {
            runCatching { repo.resolveSource(raw) }
                .onSuccess { resolved = it }
                .onFailure { repo.notify(it.message ?: "Failed to inspect source") }
            resolving = false
        }
    }

    fun installAllFromRepo() {
        val res = resolved ?: return
        val repoObj = res.repo ?: return
        if (batchInstalling) return
        val targets = repoObj.plugins.filter { entry ->
            val target = entry.downloadUrl ?: entry.sourcePath
            !target.isNullOrBlank() && !(entry.description ?: "").startsWith("[Disabled in repo]")
        }
        if (targets.isEmpty()) return
        batchInstalling = true
        scope.launch {
            var okCount = 0
            var failCount = 0
            targets.forEachIndexed { idx, entry ->
                val target = entry.downloadUrl ?: entry.sourcePath ?: res.resolvedUrl
                installingTarget = target
                batchProgress = "${idx + 1}/${targets.size}"
                runCatching { repo.installSource(target, repoObj.id) }
                    .onSuccess { okCount++ }
                    .onFailure { failCount++ }
            }
            installingTarget = null
            batchProgress = null
            batchInstalling = false
            val msg = if (failCount > 0) {
                "${repoObj.name}: $okCount installed ($failCount failed)"
            } else {
                "${repoObj.name}: $okCount installed"
            }
            repo.notify(msg)
        }
    }

    LazyColumn(
        modifier = Modifier
            .fillMaxSize()
            .background(palette.bgBase)
            .padding(16.dp),
        contentPadding = PaddingValues(bottom = 24.dp),
        verticalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        item {
            Row(verticalAlignment = Alignment.CenterVertically) {
                if (onBack != null) {
                    IconButton(onClick = onBack) {
                        Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = VesselI18n.tr("common.back", "Back"))
                    }
                }
                Text(VesselI18n.tr("extensions.title"), style = MaterialTheme.typography.headlineSmall, fontWeight = FontWeight.Bold)
            }
        }

        item {
            Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                OutlinedTextField(
                    value = sourceInput,
                    onValueChange = { sourceInput = it },
                    placeholder = { Text(VesselI18n.tr("extensions.source_placeholder")) },
                    singleLine = true,
                    modifier = Modifier.fillMaxWidth(),
                )
                Row(horizontalArrangement = Arrangement.spacedBy(8.dp), verticalAlignment = Alignment.CenterVertically) {
                    Button(onClick = { inspectSource() }, enabled = !resolving) {
                        Text(VesselI18n.tr("extensions.resolve"))
                    }
                    listOf("anthology", "tr").forEach { code ->
                        AssistChip(onClick = { sourceInput = code; inspectSource() }, label = { Text(code) })
                    }
                    if (resolving) CircularProgressIndicator()
                }
            }
        }

        resolved?.let { res ->
            item {
                Surface(shape = RoundedCornerShape(14.dp), color = palette.bgSurface) {
                    Column(modifier = Modifier.fillMaxWidth().padding(14.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
                        val repoObj = res.repo
                        val pkgManifest = res.manifest
                        if (repoObj != null) {
                            val enabledRepoTargets = repoObj.plugins.count {
                                val target = it.downloadUrl ?: it.sourcePath
                                !target.isNullOrBlank() && !(it.description ?: "").startsWith("[Disabled in repo]")
                            }
                            Row(
                                modifier = Modifier.fillMaxWidth(),
                                horizontalArrangement = Arrangement.SpaceBetween,
                                verticalAlignment = Alignment.CenterVertically,
                            ) {
                                Text(
                                    text = "${repoObj.name} (${res.badge})",
                                    fontWeight = FontWeight.Bold,
                                    modifier = Modifier.weight(1f).padding(end = 8.dp),
                                )
                                Button(
                                    onClick = { installAllFromRepo() },
                                    enabled = !batchInstalling && enabledRepoTargets > 0,
                                ) {
                                    Icon(Icons.Default.Download, contentDescription = null, modifier = Modifier.size(16.dp))
                                    Spacer(Modifier.width(6.dp))
                                    Text(
                                        if (batchInstalling && batchProgress != null) {
                                            "${VesselI18n.tr("common.loading", "Installing")} ($batchProgress)"
                                        } else {
                                            VesselI18n.tr("extensions.install_all", "Install All")
                                        },
                                    )
                                }
                            }
                            repoObj.plugins.forEach { entry ->
                                val installTarget = entry.downloadUrl ?: entry.sourcePath ?: res.resolvedUrl
                                val isInstalled = entry.id in installedIds
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
                                            Text("${entry.name} v${entry.version}", fontWeight = FontWeight.SemiBold)
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
                                            Text(it, style = MaterialTheme.typography.bodySmall, color = palette.textMuted)
                                        }
                                    }
                                    Button(
                                        enabled = !isBusy,
                                        onClick = {
                                            scope.launch {
                                                installingTarget = installTarget
                                                runCatching { repo.installSource(installTarget, repoObj.id) }
                                                    .onSuccess { repo.notify("${entry.name} installed") }
                                                    .onFailure { repo.notify(it.message ?: "Install failed") }
                                                installingTarget = null
                                            }
                                        },
                                    ) {
                                        Text(
                                            if (installingTarget == installTarget) {
                                                batchProgress ?: "..."
                                            } else if (isInstalled) {
                                                VesselI18n.tr("common.reinstall", "Reinstall")
                                            } else {
                                                VesselI18n.tr("extensions.install_now")
                                            },
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
                                Column(modifier = Modifier.weight(1f)) {
                                    Text("${pkgManifest.name} v${pkgManifest.version} (${res.badge})", fontWeight = FontWeight.Bold)
                                    pkgManifest.description?.let {
                                        Text(it, style = MaterialTheme.typography.bodySmall, color = palette.textMuted)
                                    }
                                }
                                Button(onClick = {
                                    scope.launch {
                                        runCatching { repo.installSource(sourceInput.trim(), null) }
                                            .onSuccess { repo.notify("${pkgManifest.name} installed"); resolved = null }
                                            .onFailure { repo.notify(it.message ?: "Install failed") }
                                    }
                                }) {
                                    Text(VesselI18n.tr("extensions.install_now"))
                                }
                            }
                        }
                    }
                }
            }
        }

        items(plugins, key = { it.manifest.id }) { plugin ->
            Surface(shape = RoundedCornerShape(12.dp), color = palette.bgSurface) {
                Row(
                    modifier = Modifier.fillMaxWidth().padding(14.dp),
                    verticalAlignment = Alignment.CenterVertically,
                    horizontalArrangement = Arrangement.SpaceBetween,
                ) {
                    Column(modifier = Modifier.weight(1f)) {
                        Text("${plugin.manifest.name} · v${plugin.manifest.version}", fontWeight = FontWeight.Bold)
                        Spacer(Modifier.height(2.dp))
                        Text(
                            plugin.manifest.capabilities.joinToString(", "),
                            style = MaterialTheme.typography.labelSmall,
                            color = palette.textMuted,
                        )
                    }
                    Row(verticalAlignment = Alignment.CenterVertically) {
                        Switch(
                            checked = plugin.enabled,
                            onCheckedChange = { enabled ->
                                scope.launch { repo.setPluginEnabled(plugin.manifest.id, enabled) }
                            },
                        )
                        IconButton(onClick = { scope.launch { repo.uninstallPlugin(plugin.manifest.id) } }) {
                            Icon(Icons.Default.Delete, contentDescription = VesselI18n.tr("common.uninstall"), tint = palette.danger)
                        }
                    }
                }
            }
        }
    }
}
