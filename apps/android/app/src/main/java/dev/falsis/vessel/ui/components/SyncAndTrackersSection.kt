package dev.falsis.vessel.ui.components

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.OpenInNew
import androidx.compose.material.icons.filled.AutoAwesome
import androidx.compose.material.icons.filled.Cloud
import androidx.compose.material.icons.filled.Key
import androidx.compose.material.icons.filled.LinkOff
import androidx.compose.material.icons.filled.Refresh
import androidx.compose.material.icons.filled.Sync
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Switch
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateMapOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalUriHandler
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import dev.falsis.vessel.data.CloudSyncStatus
import dev.falsis.vessel.data.OAuthDevicePrompt
import dev.falsis.vessel.data.TrackerAccountStatus
import dev.falsis.vessel.data.VesselRepository
import dev.falsis.vessel.ui.i18n.VesselI18n
import dev.falsis.vessel.ui.theme.LocalIsTvLayout
import dev.falsis.vessel.ui.theme.LocalVesselPalette
import dev.falsis.vessel.ui.theme.LocalVesselUiTokens
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch

@OptIn(ExperimentalLayoutApi::class)
@Composable
fun SyncAndTrackersSection(repo: VesselRepository) {
    val palette = LocalVesselPalette.current
    val tokens = LocalVesselUiTokens.current
    val isTv = LocalIsTvLayout.current
    val scope = rememberCoroutineScope()
    val uriHandler = LocalUriHandler.current

    var cloudStatus by remember { mutableStateOf<CloudSyncStatus?>(null) }
    var trackers by remember { mutableStateOf<List<TrackerAccountStatus>>(emptyList()) }
    var loading by remember { mutableStateOf(true) }
    var syncingDrive by remember { mutableStateOf(false) }
    var flushingQueue by remember { mutableStateOf(false) }
    var busyProvider by remember { mutableStateOf<String?>(null) }
    var devicePrompt by remember { mutableStateOf<OAuthDevicePrompt?>(null) }

    val expandedConfig = remember { mutableStateMapOf<String, Boolean>() }
    val clientIds = remember { mutableStateMapOf<String, String>() }
    val clientSecrets = remember { mutableStateMapOf<String, String>() }

    suspend fun refreshAll() {
        runCatching {
            val cloud = repo.getCloudSyncStatus()
            val list = repo.listTrackerAccounts()
            cloudStatus = cloud
            trackers = list
        }.onFailure {
            repo.notifyError(it, "Failed to load sync & tracker accounts")
        }
        loading = false
    }

    LaunchedEffect(Unit) {
        refreshAll()
    }

    LaunchedEffect(repo) {
        repo.oauthEvents.collect { evt ->
            if (evt.success) {
                repo.notify(VesselI18n.tr("sync.oauth_success"))
            } else if (!evt.error.isNullOrBlank()) {
                repo.notify(evt.error)
            }
            refreshAll()
        }
    }

    LaunchedEffect(devicePrompt?.deviceCode) {
        val active = devicePrompt ?: return@LaunchedEffect
        val intervalMs = (active.intervalSecs.coerceAtLeast(3L)) * 1000L
        val maxAttempts = ((active.expiresInSecs.coerceAtLeast(60L)) / (active.intervalSecs.coerceAtLeast(3L))).toInt()
        for (i in 0 until maxAttempts) {
            delay(intervalMs)
            if (devicePrompt?.deviceCode != active.deviceCode) break
            val done = runCatching { repo.pollOAuthDeviceCode(active.provider, active.deviceCode) }
                .onFailure { err ->
                    val msg = err.message.orEmpty()
                    if (!msg.contains("pending", ignoreCase = true) && !msg.contains("slow_down", ignoreCase = true)) {
                        devicePrompt = null
                        repo.notifyError(err, "Device authorization failed")
                    }
                }
                .getOrNull() ?: continue
            if (done) {
                devicePrompt = null
                repo.notify(VesselI18n.tr("sync.oauth_success"))
                refreshAll()
                break
            }
        }
    }

    fun startOAuth(provider: String, preferDevice: Boolean) {
        busyProvider = provider
        scope.launch {
            runCatching {
                if (preferDevice) {
                    val prompt = repo.startOAuthDeviceCode(provider)
                    devicePrompt = prompt
                } else {
                    val res = repo.startOAuthBrowserFlow(provider)
                    val authUrl = res.authUrl
                    if (!authUrl.isNullOrBlank()) {
                        uriHandler.openUri(authUrl)
                        repo.notify(VesselI18n.tr("sync.browser_auth_opened"))
                    } else if (!res.deviceCode.isNullOrBlank() && !res.userCode.isNullOrBlank() && !res.verificationUri.isNullOrBlank()) {
                        devicePrompt = OAuthDevicePrompt(
                            provider = provider,
                            deviceCode = res.deviceCode,
                            userCode = res.userCode,
                            verificationUri = res.verificationUri,
                            verificationUriComplete = res.verificationUriComplete,
                            expiresInSecs = res.expiresInSecs ?: 600L,
                            intervalSecs = res.intervalSecs ?: 5L,
                        )
                    }
                }
            }.onFailure { err ->
                repo.notifyError(err, "Failed to start OAuth")
            }
            busyProvider = null
        }
    }

    fun saveClient(provider: String) {
        val cid = clientIds[provider].orEmpty().trim()
        val secret = clientSecrets[provider].orEmpty().trim()
        if (cid.isEmpty()) {
            repo.notify(VesselI18n.tr("sync.client_id_required"))
            return
        }
        busyProvider = provider
        scope.launch {
            runCatching {
                repo.saveOAuthClientConfig(provider, cid, secret.takeIf { it.isNotEmpty() })
            }.onSuccess {
                clientIds[provider] = ""
                clientSecrets[provider] = ""
                expandedConfig[provider] = false
                repo.notify(VesselI18n.tr("sync.client_config_saved"))
                refreshAll()
            }.onFailure { err ->
                repo.notifyError(err, "Failed to save OAuth client config")
            }
            busyProvider = null
        }
    }

    Column(verticalArrangement = Arrangement.spacedBy(12.dp)) {
        // Google Drive Cloud Sync Card
        Surface(shape = RoundedCornerShape(14.dp), color = palette.bgSurface) {
            Column(
                modifier = Modifier.fillMaxWidth().padding(14.dp),
                verticalArrangement = Arrangement.spacedBy(10.dp),
            ) {
                Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                    Icon(Icons.Default.Cloud, contentDescription = null, tint = palette.accent, modifier = Modifier.size(18.dp))
                    Column(modifier = Modifier.weight(1f)) {
                        Text(VesselI18n.tr("sync.drive_title"), fontWeight = FontWeight.Bold)
                        Text(VesselI18n.tr("sync.drive_subtitle"), style = MaterialTheme.typography.labelSmall, color = palette.textMuted)
                    }
                }

                if (loading && cloudStatus == null) {
                    CircularProgressIndicator(modifier = Modifier.size(20.dp), strokeWidth = 2.dp)
                } else {
                    val cloud = cloudStatus ?: CloudSyncStatus()
                    Surface(shape = RoundedCornerShape(12.dp), color = palette.bgElevated) {
                        Column(
                            modifier = Modifier.fillMaxWidth().padding(12.dp),
                            verticalArrangement = Arrangement.spacedBy(8.dp),
                        ) {
                            FlowRow(
                                horizontalArrangement = Arrangement.spacedBy(6.dp),
                                verticalArrangement = Arrangement.spacedBy(6.dp),
                            ) {
                                Text("Google Drive", fontWeight = FontWeight.SemiBold)
                                Surface(
                                    shape = RoundedCornerShape(999.dp),
                                    color = if (cloud.connected) Color(0x2822C55E) else palette.bgSurface,
                                ) {
                                    Text(
                                        text = if (cloud.connected) {
                                            cloud.accountEmail ?: VesselI18n.tr("debrid.connected")
                                        } else {
                                            VesselI18n.tr("debrid.not_configured")
                                        },
                                        style = MaterialTheme.typography.labelSmall,
                                        color = if (cloud.connected) Color(0xFF4ADE80) else palette.textMuted,
                                        modifier = Modifier.padding(horizontal = 7.dp, vertical = 2.dp),
                                    )
                                }
                                Surface(shape = RoundedCornerShape(999.dp), color = palette.bgSurface) {
                                    Text(
                                        text = VesselI18n.tr("sync.tombstones_count").replace("{count}", cloud.activeTombstones.toString()),
                                        style = MaterialTheme.typography.labelSmall,
                                        color = palette.textMuted,
                                        modifier = Modifier.padding(horizontal = 7.dp, vertical = 2.dp),
                                    )
                                }
                            }

                            val meta = buildList {
                                if (cloud.deviceId.isNotBlank()) add("Device ${cloud.deviceId.take(12)}")
                                if (!cloud.lastSyncedAt.isNullOrBlank()) {
                                    add("${VesselI18n.tr("sync.last_synced")}: ${cloud.lastSyncedAt.take(19)}")
                                }
                            }.joinToString(" · ")
                            if (meta.isNotBlank()) {
                                Text(meta, style = MaterialTheme.typography.labelSmall, color = palette.textMuted)
                            }
                            cloud.lastError?.takeIf { it.isNotBlank() }?.let { err ->
                                Text(err, style = MaterialTheme.typography.labelSmall, color = palette.danger)
                            }

                            FlowRow(
                                horizontalArrangement = Arrangement.spacedBy(8.dp),
                                verticalArrangement = Arrangement.spacedBy(6.dp),
                            ) {
                                VesselOutlinedButton(
                                    height = tokens.compactButtonHeight,
                                    onClick = {
                                        expandedConfig["google_drive"] = !(expandedConfig["google_drive"] ?: false)
                                    },
                                ) {
                                    Icon(Icons.Default.Key, contentDescription = null, modifier = Modifier.size(14.dp))
                                    Spacer(Modifier.width(4.dp))
                                    Text(VesselI18n.tr("sync.configure_oauth"), style = MaterialTheme.typography.labelSmall)
                                }

                                VesselButton(
                                    height = tokens.compactButtonHeight,
                                    enabled = busyProvider != "google_drive",
                                    onClick = { startOAuth("google_drive", preferDevice = false) },
                                ) {
                                    Icon(Icons.AutoMirrored.Filled.OpenInNew, contentDescription = null, modifier = Modifier.size(14.dp))
                                    Spacer(Modifier.width(4.dp))
                                    Text(
                                        if (cloud.connected) VesselI18n.tr("sync.reconnect") else VesselI18n.tr("sync.connect_google_drive"),
                                        style = MaterialTheme.typography.labelSmall,
                                    )
                                }

                                if (cloud.connected) {
                                    VesselOutlinedButton(
                                        height = tokens.compactButtonHeight,
                                        enabled = !syncingDrive,
                                        onClick = {
                                            syncingDrive = true
                                            scope.launch {
                                                runCatching { repo.syncGoogleDriveNow() }
                                                    .onSuccess { res ->
                                                        cloudStatus = res.status
                                                        val total = res.merge.libraryApplied + res.merge.progressApplied + res.merge.overridesApplied
                                                        repo.notify(VesselI18n.tr("sync.drive_synced").replace("{count}", total.toString()))
                                                    }
                                                    .onFailure { repo.notifyError(it, "Cloud sync failed") }
                                                syncingDrive = false
                                            }
                                        },
                                    ) {
                                        Icon(Icons.Default.Sync, contentDescription = null, modifier = Modifier.size(14.dp))
                                        Spacer(Modifier.width(4.dp))
                                        Text(VesselI18n.tr("sync.sync_now"), style = MaterialTheme.typography.labelSmall)
                                    }

                                    VesselOutlinedButton(
                                        height = tokens.compactButtonHeight,
                                        enabled = busyProvider != "google_drive",
                                        onClick = {
                                            busyProvider = "google_drive"
                                            scope.launch {
                                                runCatching { repo.disconnectGoogleDrive() }
                                                    .onSuccess {
                                                        repo.notify(VesselI18n.tr("sync.disconnected"))
                                                        refreshAll()
                                                    }
                                                    .onFailure { repo.notifyError(it, "Disconnect failed") }
                                                busyProvider = null
                                            }
                                        },
                                    ) {
                                        Icon(Icons.Default.LinkOff, contentDescription = null, modifier = Modifier.size(14.dp))
                                        Spacer(Modifier.width(4.dp))
                                        Text(VesselI18n.tr("debrid.disconnect"), style = MaterialTheme.typography.labelSmall)
                                    }
                                }
                            }

                            if (expandedConfig["google_drive"] == true) {
                                OAuthClientFields(
                                    provider = "google_drive",
                                    clientId = clientIds["google_drive"].orEmpty(),
                                    clientSecret = clientSecrets["google_drive"].orEmpty(),
                                    busy = busyProvider == "google_drive",
                                    onClientIdChange = { clientIds["google_drive"] = it },
                                    onClientSecretChange = { clientSecrets["google_drive"] = it },
                                    onSave = { saveClient("google_drive") },
                                )
                            }
                        }
                    }

                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        verticalAlignment = Alignment.CenterVertically,
                        horizontalArrangement = Arrangement.SpaceBetween,
                    ) {
                        Column(modifier = Modifier.weight(1f)) {
                            Text(VesselI18n.tr("sync.auto_sync"), style = MaterialTheme.typography.bodyMedium)
                            Text(VesselI18n.tr("sync.auto_sync_desc"), style = MaterialTheme.typography.labelSmall, color = palette.textMuted)
                        }
                        Switch(
                            checked = cloud.autoSync,
                            onCheckedChange = { next ->
                                scope.launch {
                                    runCatching { repo.setCloudSyncAuto(next) }
                                        .onSuccess { refreshAll() }
                                        .onFailure { repo.notifyError(it, "Failed to update cloud sync preference") }
                                }
                            },
                        )
                    }
                }
            }
        }

        // Watch Trackers Card
        Surface(shape = RoundedCornerShape(14.dp), color = palette.bgSurface) {
            Column(
                modifier = Modifier.fillMaxWidth().padding(14.dp),
                verticalArrangement = Arrangement.spacedBy(10.dp),
            ) {
                Row(
                    modifier = Modifier.fillMaxWidth(),
                    verticalAlignment = Alignment.CenterVertically,
                    horizontalArrangement = Arrangement.SpaceBetween,
                ) {
                    Row(
                        modifier = Modifier.weight(1f),
                        verticalAlignment = Alignment.CenterVertically,
                        horizontalArrangement = Arrangement.spacedBy(8.dp),
                    ) {
                        Icon(Icons.Default.AutoAwesome, contentDescription = null, tint = palette.accent, modifier = Modifier.size(18.dp))
                        Column {
                            Text(VesselI18n.tr("sync.trackers_title"), fontWeight = FontWeight.Bold)
                            Text(VesselI18n.tr("sync.trackers_subtitle"), style = MaterialTheme.typography.labelSmall, color = palette.textMuted)
                        }
                    }
                    val totalQueued = trackers.sumOf { it.pendingQueueCount }
                    VesselOutlinedButton(
                        height = tokens.compactButtonHeight,
                        enabled = !flushingQueue,
                        onClick = {
                            flushingQueue = true
                            scope.launch {
                                runCatching { repo.flushTrackerQueue() }
                                    .onSuccess { count ->
                                        repo.notify(VesselI18n.tr("sync.queue_flushed").replace("{count}", count.toString()))
                                        refreshAll()
                                    }
                                    .onFailure { repo.notifyError(it, "Flush queue failed") }
                                flushingQueue = false
                            }
                        },
                    ) {
                        Icon(Icons.Default.Refresh, contentDescription = null, modifier = Modifier.size(14.dp))
                        Spacer(Modifier.width(4.dp))
                        Text(
                            if (totalQueued > 0) "${VesselI18n.tr("sync.flush_queue")} ($totalQueued)"
                            else VesselI18n.tr("sync.flush_queue"),
                            style = MaterialTheme.typography.labelSmall,
                        )
                    }
                }

                devicePrompt?.let { prompt ->
                    Surface(shape = RoundedCornerShape(12.dp), color = palette.accent.copy(alpha = 0.12f)) {
                        Column(
                            modifier = Modifier.fillMaxWidth().padding(12.dp),
                            verticalArrangement = Arrangement.spacedBy(6.dp),
                        ) {
                            Text(
                                VesselI18n.tr("debrid.device_auth_prompt").replace("{url}", prompt.verificationUri),
                                style = MaterialTheme.typography.bodySmall,
                            )
                            Text(
                                prompt.userCode,
                                style = MaterialTheme.typography.headlineSmall,
                                fontFamily = FontFamily.Monospace,
                                fontWeight = FontWeight.Bold,
                                color = palette.accent,
                            )
                            Row(horizontalArrangement = Arrangement.spacedBy(8.dp), verticalAlignment = Alignment.CenterVertically) {
                                VesselButton(
                                    height = tokens.compactButtonHeight,
                                    onClick = { uriHandler.openUri(prompt.verificationUriComplete ?: prompt.verificationUri) },
                                ) {
                                    Text(prompt.verificationUri, style = MaterialTheme.typography.labelSmall)
                                }
                                VesselOutlinedButton(
                                    height = tokens.compactButtonHeight,
                                    onClick = { devicePrompt = null },
                                ) {
                                    Text(VesselI18n.tr("common.cancel"), style = MaterialTheme.typography.labelSmall)
                                }
                            }
                        }
                    }
                }

                trackers.forEach { acc ->
                    Surface(shape = RoundedCornerShape(12.dp), color = palette.bgElevated) {
                        Column(
                            modifier = Modifier.fillMaxWidth().padding(12.dp),
                            verticalArrangement = Arrangement.spacedBy(8.dp),
                        ) {
                            FlowRow(
                                horizontalArrangement = Arrangement.spacedBy(6.dp),
                                verticalArrangement = Arrangement.spacedBy(6.dp),
                            ) {
                                Text(acc.displayName, fontWeight = FontWeight.SemiBold)
                                Surface(
                                    shape = RoundedCornerShape(999.dp),
                                    color = if (acc.connected) Color(0x2822C55E) else palette.bgSurface,
                                ) {
                                    Text(
                                        text = when {
                                            acc.connected -> acc.username ?: VesselI18n.tr("debrid.connected")
                                            acc.implemented -> VesselI18n.tr("debrid.not_configured")
                                            else -> VesselI18n.tr("sync.coming_soon_slot")
                                        },
                                        style = MaterialTheme.typography.labelSmall,
                                        color = if (acc.connected) Color(0xFF4ADE80) else palette.textMuted,
                                        modifier = Modifier.padding(horizontal = 7.dp, vertical = 2.dp),
                                    )
                                }
                                if (acc.pendingQueueCount > 0) {
                                    Surface(shape = RoundedCornerShape(999.dp), color = palette.accent.copy(alpha = 0.16f)) {
                                        Text(
                                            text = VesselI18n.tr("sync.pending_count").replace("{count}", acc.pendingQueueCount.toString()),
                                            style = MaterialTheme.typography.labelSmall,
                                            color = palette.accent,
                                            modifier = Modifier.padding(horizontal = 6.dp, vertical = 2.dp),
                                        )
                                    }
                                }
                            }

                            FlowRow(
                                horizontalArrangement = Arrangement.spacedBy(8.dp),
                                verticalArrangement = Arrangement.spacedBy(6.dp),
                            ) {
                                VesselOutlinedButton(
                                    height = tokens.compactButtonHeight,
                                    onClick = {
                                        expandedConfig[acc.provider] = !(expandedConfig[acc.provider] ?: false)
                                    },
                                ) {
                                    Icon(Icons.Default.Key, contentDescription = null, modifier = Modifier.size(14.dp))
                                    Spacer(Modifier.width(4.dp))
                                    Text(VesselI18n.tr("sync.configure_oauth"), style = MaterialTheme.typography.labelSmall)
                                }

                                if (acc.implemented) {
                                    if (acc.provider == "trakt") {
                                        VesselOutlinedButton(
                                            height = tokens.compactButtonHeight,
                                            enabled = busyProvider != acc.provider,
                                            onClick = { startOAuth(acc.provider, preferDevice = true) },
                                        ) {
                                            Icon(Icons.AutoMirrored.Filled.OpenInNew, contentDescription = null, modifier = Modifier.size(14.dp))
                                            Spacer(Modifier.width(4.dp))
                                            Text(VesselI18n.tr("debrid.connect_device"), style = MaterialTheme.typography.labelSmall)
                                        }
                                    }
                                    if (!isTv || acc.provider != "trakt") {
                                        VesselButton(
                                            height = tokens.compactButtonHeight,
                                            enabled = busyProvider != acc.provider,
                                            onClick = { startOAuth(acc.provider, preferDevice = false) },
                                        ) {
                                            Icon(Icons.AutoMirrored.Filled.OpenInNew, contentDescription = null, modifier = Modifier.size(14.dp))
                                            Spacer(Modifier.width(4.dp))
                                            Text(
                                                if (acc.connected) VesselI18n.tr("sync.reconnect") else VesselI18n.tr("sync.connect_oauth"),
                                                style = MaterialTheme.typography.labelSmall,
                                            )
                                        }
                                    }
                                    if (acc.connected) {
                                        VesselOutlinedButton(
                                            height = tokens.compactButtonHeight,
                                            enabled = busyProvider != acc.provider,
                                            onClick = {
                                                busyProvider = acc.provider
                                                scope.launch {
                                                    runCatching { repo.disconnectTracker(acc.provider) }
                                                        .onSuccess {
                                                            repo.notify(VesselI18n.tr("sync.disconnected"))
                                                            refreshAll()
                                                        }
                                                        .onFailure { repo.notifyError(it, "Disconnect failed") }
                                                    busyProvider = null
                                                }
                                            },
                                        ) {
                                            Text(VesselI18n.tr("debrid.disconnect"), style = MaterialTheme.typography.labelSmall)
                                        }
                                    }
                                }
                            }

                            if (expandedConfig[acc.provider] == true) {
                                OAuthClientFields(
                                    provider = acc.provider,
                                    clientId = clientIds[acc.provider].orEmpty(),
                                    clientSecret = clientSecrets[acc.provider].orEmpty(),
                                    busy = busyProvider == acc.provider,
                                    onClientIdChange = { clientIds[acc.provider] = it },
                                    onClientSecretChange = { clientSecrets[acc.provider] = it },
                                    onSave = { saveClient(acc.provider) },
                                )
                            }

                            if (acc.implemented) {
                                HorizontalDivider(color = palette.border.copy(alpha = 0.25f))
                                Row(
                                    modifier = Modifier.fillMaxWidth(),
                                    verticalAlignment = Alignment.CenterVertically,
                                    horizontalArrangement = Arrangement.SpaceBetween,
                                ) {
                                    Text(VesselI18n.tr("sync.auto_scrobble"), style = MaterialTheme.typography.bodySmall)
                                    Switch(
                                        checked = acc.autoScrobble,
                                        onCheckedChange = { next ->
                                            scope.launch {
                                                runCatching { repo.setTrackerPreferences(acc.provider, next, acc.syncLibrary) }
                                                    .onSuccess { refreshAll() }
                                                    .onFailure { repo.notifyError(it, "Update failed") }
                                            }
                                        },
                                    )
                                }
                                Row(
                                    modifier = Modifier.fillMaxWidth(),
                                    verticalAlignment = Alignment.CenterVertically,
                                    horizontalArrangement = Arrangement.SpaceBetween,
                                ) {
                                    Text(VesselI18n.tr("sync.sync_library"), style = MaterialTheme.typography.bodySmall)
                                    Switch(
                                        checked = acc.syncLibrary,
                                        onCheckedChange = { next ->
                                            scope.launch {
                                                runCatching { repo.setTrackerPreferences(acc.provider, acc.autoScrobble, next) }
                                                    .onSuccess { refreshAll() }
                                                    .onFailure { repo.notifyError(it, "Update failed") }
                                            }
                                        },
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

@Composable
private fun OAuthClientFields(
    provider: String,
    clientId: String,
    clientSecret: String,
    busy: Boolean,
    onClientIdChange: (String) -> Unit,
    onClientSecretChange: (String) -> Unit,
    onSave: () -> Unit,
) {
    val tokens = LocalVesselUiTokens.current
    Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
        VesselOutlinedTextField(
            value = clientId,
            onValueChange = onClientIdChange,
            placeholder = { Text(VesselI18n.tr("sync.client_id_placeholder")) },
            singleLine = true,
            modifier = Modifier.fillMaxWidth().vesselDpadInputEscape(isTextEmpty = clientId.isEmpty()),
        )
        VesselOutlinedTextField(
            value = clientSecret,
            onValueChange = onClientSecretChange,
            placeholder = { Text(VesselI18n.tr("sync.client_secret_placeholder")) },
            singleLine = true,
            modifier = Modifier.fillMaxWidth().vesselDpadInputEscape(isTextEmpty = clientSecret.isEmpty()),
        )
        VesselButton(
            height = tokens.compactButtonHeight,
            enabled = !busy && provider.isNotBlank(),
            onClick = onSave,
        ) {
            Text(VesselI18n.tr("common.save"), style = MaterialTheme.typography.labelSmall)
        }
    }
}
