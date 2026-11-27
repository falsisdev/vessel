package dev.falsis.vessel.ui.components

import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.OpenInNew
import androidx.compose.material.icons.filled.ArrowDownward
import androidx.compose.material.icons.filled.ArrowUpward
import androidx.compose.material.icons.filled.CloudDownload
import androidx.compose.material.icons.filled.ContentCopy
import androidx.compose.material.icons.filled.Delete
import androidx.compose.material.icons.filled.Key
import androidx.compose.material.icons.filled.PlayArrow
import androidx.compose.material.icons.filled.Refresh
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.LinearProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Switch
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.key
import androidx.compose.runtime.mutableStateMapOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.focus.FocusRequester
import androidx.compose.ui.focus.focusRequester
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalClipboardManager
import androidx.compose.ui.platform.LocalUriHandler
import androidx.compose.ui.text.AnnotatedString
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import dev.falsis.vessel.data.DebridAccountStatus
import dev.falsis.vessel.data.DebridCloudItem
import dev.falsis.vessel.data.DebridOverview
import dev.falsis.vessel.data.DebridPreferences
import dev.falsis.vessel.data.RealDebridDeviceAuthStart
import dev.falsis.vessel.data.VesselRepository
import dev.falsis.vessel.ui.i18n.VesselI18n
import dev.falsis.vessel.ui.theme.LocalVesselPalette
import dev.falsis.vessel.ui.theme.LocalVesselUiTokens
import java.util.Locale
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch

@OptIn(ExperimentalLayoutApi::class)
@Composable
fun DebridSettingsSection(repo: VesselRepository) {
    val palette = LocalVesselPalette.current
    val tokens = LocalVesselUiTokens.current
    val scope = rememberCoroutineScope()
    val clipboard = LocalClipboardManager.current
    val uriHandler = LocalUriHandler.current

    var overview by remember { mutableStateOf<DebridOverview?>(null) }
    var loading by remember { mutableStateOf(true) }
    var saving by remember { mutableStateOf(false) }
    var cloudItems by remember { mutableStateOf<List<DebridCloudItem>>(emptyList()) }
    var loadingCloud by remember { mutableStateOf(false) }
    var cloudAutoPoll by remember { mutableStateOf(true) }
    var rdDeviceAuth by remember { mutableStateOf<RealDebridDeviceAuthStart?>(null) }
    val apiKeyInputs = remember { mutableStateMapOf<String, String>() }
    val providerFocusRequesters = remember { mutableMapOf<String, Pair<FocusRequester, FocusRequester>>() }
    var pendingProviderFocus by remember { mutableStateOf<Pair<String, Boolean>?>(null) }

    LaunchedEffect(pendingProviderFocus, overview?.accounts) {
        val target = pendingProviderFocus ?: return@LaunchedEffect
        delay(16)
        val pair = providerFocusRequesters[target.first]
        if (pair != null) {
            runCatching { (if (target.second) pair.first else pair.second).requestFocus() }
        }
        pendingProviderFocus = null
    }

    suspend fun refreshOverview() {
        runCatching { repo.getDebridOverview() }
            .onSuccess { overview = it }
            .onFailure { repo.notifyError(it, "Failed to load Debrid settings") }
        loading = false
    }

    suspend fun refreshCloud(silent: Boolean = false) {
        if (!silent) loadingCloud = true
        runCatching { repo.listDebridCloud() }
            .onSuccess { cloudItems = it }
            .onFailure { if (!silent) repo.notifyError(it, "Failed to load Debrid cloud") }
        if (!silent) loadingCloud = false
    }

    LaunchedEffect(Unit) {
        refreshOverview()
        refreshCloud(silent = true)
    }

    LaunchedEffect(cloudAutoPoll) {
        if (!cloudAutoPoll) return@LaunchedEffect
        while (true) {
            delay(2000)
            refreshCloud(silent = true)
        }
    }

    LaunchedEffect(rdDeviceAuth?.deviceCode) {
        val auth = rdDeviceAuth ?: return@LaunchedEffect
        val intervalMs = (auth.intervalSeconds.coerceAtLeast(3L)) * 1000L
        val maxAttempts = ((auth.expiresInSeconds.coerceAtLeast(60L)) / (auth.intervalSeconds.coerceAtLeast(3L))).toInt()
        for (i in 0 until maxAttempts) {
            delay(intervalMs)
            if (rdDeviceAuth?.deviceCode != auth.deviceCode) break
            val poll = runCatching { repo.pollRealDebridDeviceAuth(auth.deviceCode) }.getOrNull() ?: continue
            if (poll.authorized) {
                rdDeviceAuth = null
                repo.notify("Real-Debrid connected${poll.username?.let { " ($it)" }.orEmpty()}")
                refreshOverview()
                refreshCloud(silent = true)
                break
            }
            if (!poll.pending && poll.error != null) {
                rdDeviceAuth = null
                repo.notify(poll.error)
                break
            }
        }
    }

    fun updatePrefs(next: DebridPreferences) {
        val prev = overview
        if (prev != null) overview = prev.copy(preferences = next)
        saving = true
        scope.launch {
            runCatching { repo.setDebridPreferences(next) }
                .onSuccess { saved ->
                    overview = overview?.copy(preferences = saved)
                }
                .onFailure { repo.notifyError(it, "Failed to save Debrid preferences") }
            saving = false
        }
    }

    fun moveProvider(accounts: List<DebridAccountStatus>, index: Int, delta: Int) {
        val targetIdx = index + delta
        if (targetIdx !in accounts.indices) return
        val order = accounts.map { it.target }.toMutableList()
        val item = order.removeAt(index)
        order.add(targetIdx, item)
        val cur = overview?.preferences ?: DebridPreferences()
        updatePrefs(cur.copy(priorityOrder = order))
        overview = overview?.copy(
            accounts = order.mapNotNull { t -> accounts.firstOrNull { it.target == t } },
        )
    }

    Surface(shape = RoundedCornerShape(14.dp), color = palette.bgSurface) {
        Column(
            modifier = Modifier.fillMaxWidth().padding(14.dp),
            verticalArrangement = Arrangement.spacedBy(12.dp),
        ) {
            Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                Icon(Icons.Default.CloudDownload, contentDescription = null, tint = palette.accent, modifier = Modifier.size(18.dp))
                Column(modifier = Modifier.weight(1f)) {
                    Text(VesselI18n.tr("debrid.title"), fontWeight = FontWeight.Bold)
                    Text(VesselI18n.tr("debrid.subtitle"), style = MaterialTheme.typography.labelSmall, color = palette.textMuted)
                }
            }

            if (loading) {
                CircularProgressIndicator(modifier = Modifier.size(20.dp), strokeWidth = 2.dp)
            } else {
                val ov = overview
                val prefs = ov?.preferences ?: DebridPreferences()
                val accounts = ov?.accounts ?: emptyList()

                Row(
                    modifier = Modifier.fillMaxWidth(),
                    verticalAlignment = Alignment.CenterVertically,
                    horizontalArrangement = Arrangement.SpaceBetween,
                ) {
                    Column(modifier = Modifier.weight(1f).padding(end = 8.dp)) {
                        Text(VesselI18n.tr("debrid.only_cached"), style = MaterialTheme.typography.bodyMedium)
                        Text(VesselI18n.tr("debrid.only_cached_desc"), style = MaterialTheme.typography.labelSmall, color = palette.textMuted)
                    }
                    Switch(
                        checked = prefs.onlyCached,
                        enabled = !saving,
                        onCheckedChange = { updatePrefs(prefs.copy(onlyCached = it)) },
                    )
                }

                Row(
                    modifier = Modifier.fillMaxWidth(),
                    verticalAlignment = Alignment.CenterVertically,
                    horizontalArrangement = Arrangement.SpaceBetween,
                ) {
                    Column(modifier = Modifier.weight(1f).padding(end = 8.dp)) {
                        Text(VesselI18n.tr("debrid.auto_fallback"), style = MaterialTheme.typography.bodyMedium)
                        Text(VesselI18n.tr("debrid.auto_fallback_desc"), style = MaterialTheme.typography.labelSmall, color = palette.textMuted)
                    }
                    Switch(
                        checked = prefs.autoFallback,
                        enabled = !saving,
                        onCheckedChange = { updatePrefs(prefs.copy(autoFallback = it)) },
                    )
                }

                HorizontalDivider(color = palette.border.copy(alpha = 0.5f))
                Text(VesselI18n.tr("debrid.priority_hint"), style = MaterialTheme.typography.labelSmall, color = palette.textMuted)

                accounts.forEachIndexed { idx, acc ->
                    key(acc.target) {
                    val (upReq, downReq) = providerFocusRequesters.getOrPut(acc.target) { FocusRequester() to FocusRequester() }
                    Surface(
                        shape = RoundedCornerShape(10.dp),
                        color = palette.bgElevated,
                        border = BorderStroke(
                            1.dp,
                            if (acc.authenticated) palette.accent.copy(alpha = 0.45f) else palette.border.copy(alpha = 0.45f),
                        ),
                    ) {
                        Column(
                            modifier = Modifier.fillMaxWidth().padding(10.dp),
                            verticalArrangement = Arrangement.spacedBy(8.dp),
                        ) {
                            Row(
                                modifier = Modifier.fillMaxWidth(),
                                verticalAlignment = Alignment.CenterVertically,
                                horizontalArrangement = Arrangement.spacedBy(8.dp),
                            ) {
                                Column(modifier = Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(2.dp)) {
                                    Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(6.dp)) {
                                        Text("${idx + 1}. ${acc.label}", fontWeight = FontWeight.SemiBold, maxLines = 1, overflow = TextOverflow.Ellipsis)
                                        Surface(
                                            shape = RoundedCornerShape(999.dp),
                                            color = if (acc.authenticated) Color(0x2822C55E) else palette.bgSurface,
                                        ) {
                                            Text(
                                                text = if (acc.authenticated) {
                                                    if (acc.isPremium) "${VesselI18n.tr("debrid.connected")} · ${VesselI18n.tr("debrid.premium")}"
                                                    else VesselI18n.tr("debrid.connected")
                                                } else {
                                                    VesselI18n.tr("debrid.not_configured")
                                                },
                                                style = MaterialTheme.typography.labelSmall,
                                                color = if (acc.authenticated) Color(0xFF4ADE80) else palette.textMuted,
                                                modifier = Modifier.padding(horizontal = 7.dp, vertical = 2.dp),
                                            )
                                        }
                                        if (!acc.isBuiltin) {
                                            Surface(shape = RoundedCornerShape(999.dp), color = palette.accent.copy(alpha = 0.16f)) {
                                                Text(
                                                    text = "Plugin",
                                                    style = MaterialTheme.typography.labelSmall,
                                                    color = palette.accent,
                                                    modifier = Modifier.padding(horizontal = 6.dp, vertical = 2.dp),
                                                )
                                            }
                                        }
                                    }
                                    val subline = listOfNotNull(
                                        acc.username ?: acc.email,
                                        acc.pointsOrQuota,
                                        acc.expiresAt?.take(10),
                                    ).joinToString(" · ")
                                    if (subline.isNotBlank()) {
                                        Text(subline, style = MaterialTheme.typography.labelSmall, color = palette.textMuted)
                                    }
                                    acc.error?.takeIf { it.isNotBlank() }?.let { err ->
                                        Text(err, style = MaterialTheme.typography.labelSmall, color = palette.danger)
                                    }
                                }

                                VesselCompactIconButton(
                                    enabled = idx > 0,
                                    size = 28.dp,
                                    modifier = Modifier.focusRequester(upReq),
                                    onClick = {
                                        val newIdx = idx - 1
                                        moveProvider(accounts, idx, -1)
                                        pendingProviderFocus = acc.target to (newIdx > 0)
                                    },
                                ) {
                                    Icon(Icons.Default.ArrowUpward, contentDescription = null, modifier = Modifier.size(14.dp))
                                }
                                VesselCompactIconButton(
                                    enabled = idx < accounts.lastIndex,
                                    size = 28.dp,
                                    modifier = Modifier.focusRequester(downReq),
                                    onClick = {
                                        val newIdx = idx + 1
                                        moveProvider(accounts, idx, 1)
                                        pendingProviderFocus = acc.target to (newIdx >= accounts.lastIndex)
                                    },
                                ) {
                                    Icon(Icons.Default.ArrowDownward, contentDescription = null, modifier = Modifier.size(14.dp))
                                }
                            }

                            if (acc.isBuiltin) {
                                val slug = acc.target.removePrefix("builtin:")
                                val keyInput = apiKeyInputs[acc.target].orEmpty()

                                Row(
                                    modifier = Modifier.fillMaxWidth(),
                                    verticalAlignment = Alignment.CenterVertically,
                                    horizontalArrangement = Arrangement.spacedBy(8.dp),
                                ) {
                                    VesselOutlinedTextField(
                                        value = keyInput,
                                        onValueChange = { apiKeyInputs[acc.target] = it },
                                        placeholder = { Text(VesselI18n.tr("debrid.api_key_placeholder")) },
                                        singleLine = true,
                                        modifier = Modifier.weight(1f).vesselDpadInputEscape(isTextEmpty = keyInput.isEmpty()),
                                    )
                                    VesselButton(
                                        height = tokens.compactButtonHeight,
                                        enabled = keyInput.isNotBlank(),
                                        onClick = {
                                            val raw = keyInput.trim()
                                            scope.launch {
                                                runCatching { repo.saveDebridApiKey(slug, raw) }
                                                    .onSuccess {
                                                        apiKeyInputs[acc.target] = ""
                                                        repo.notify("${acc.label}: ${VesselI18n.tr("debrid.connected")}")
                                                        refreshOverview()
                                                        refreshCloud(silent = true)
                                                    }
                                                    .onFailure { repo.notifyError(it, "Failed to save Debrid key") }
                                            }
                                        },
                                    ) {
                                        Text(VesselI18n.tr("debrid.save_key"), style = MaterialTheme.typography.labelSmall)
                                    }
                                }

                                FlowRow(
                                    horizontalArrangement = Arrangement.spacedBy(8.dp),
                                    verticalArrangement = Arrangement.spacedBy(6.dp),
                                ) {
                                    if (acc.target == "builtin:real_debrid") {
                                        VesselOutlinedButton(
                                            height = tokens.compactButtonHeight,
                                            onClick = {
                                                scope.launch {
                                                    runCatching { repo.startRealDebridDeviceAuth() }
                                                        .onSuccess { rdDeviceAuth = it }
                                                        .onFailure { repo.notifyError(it, "Failed to start device auth") }
                                                }
                                            },
                                        ) {
                                            Icon(Icons.Default.Key, contentDescription = null, modifier = Modifier.size(14.dp))
                                            Spacer(Modifier.width(4.dp))
                                            Text(VesselI18n.tr("debrid.connect_device"), style = MaterialTheme.typography.labelSmall)
                                        }
                                    }
                                    if (acc.authenticated) {
                                        VesselOutlinedButton(
                                            height = tokens.compactButtonHeight,
                                            onClick = {
                                                scope.launch {
                                                    runCatching { repo.removeDebridCredentials(slug) }
                                                        .onSuccess {
                                                            refreshOverview()
                                                            refreshCloud(silent = true)
                                                        }
                                                        .onFailure { repo.notifyError(it, "Failed to disconnect") }
                                                }
                                            },
                                        ) {
                                            Text(VesselI18n.tr("debrid.disconnect"), style = MaterialTheme.typography.labelSmall, color = palette.danger)
                                        }
                                    }
                                }

                                if (acc.target == "builtin:real_debrid" && rdDeviceAuth != null) {
                                    val auth = rdDeviceAuth!!
                                    Surface(
                                        shape = RoundedCornerShape(8.dp),
                                        color = palette.bgSurface,
                                        border = BorderStroke(1.dp, palette.accent.copy(alpha = 0.5f)),
                                    ) {
                                        Column(
                                            modifier = Modifier.fillMaxWidth().padding(10.dp),
                                            verticalArrangement = Arrangement.spacedBy(6.dp),
                                        ) {
                                            Text(
                                                VesselI18n.trf("debrid.device_auth_prompt", "url" to auth.verificationUrl),
                                                style = MaterialTheme.typography.labelSmall,
                                            )
                                            Row(
                                                verticalAlignment = Alignment.CenterVertically,
                                                horizontalArrangement = Arrangement.spacedBy(10.dp),
                                            ) {
                                                Text(
                                                    text = auth.userCode,
                                                    style = MaterialTheme.typography.titleMedium,
                                                    fontFamily = FontFamily.Monospace,
                                                    fontWeight = FontWeight.Bold,
                                                    color = palette.accent,
                                                )
                                                VesselOutlinedButton(
                                                    height = 28.dp,
                                                    onClick = {
                                                        clipboard.setText(AnnotatedString(auth.userCode))
                                                        repo.notify(VesselI18n.tr("common.copied", "Copied"))
                                                    },
                                                ) {
                                                    Icon(Icons.Default.ContentCopy, contentDescription = null, modifier = Modifier.size(13.dp))
                                                    Spacer(Modifier.width(4.dp))
                                                    Text(VesselI18n.tr("common.copy", "Copy"), style = MaterialTheme.typography.labelSmall)
                                                }
                                                VesselOutlinedButton(
                                                    height = 28.dp,
                                                    onClick = {
                                                        runCatching {
                                                            uriHandler.openUri(auth.directVerificationUrl ?: auth.verificationUrl)
                                                        }
                                                    },
                                                ) {
                                                    Icon(Icons.AutoMirrored.Filled.OpenInNew, contentDescription = null, modifier = Modifier.size(13.dp))
                                                    Spacer(Modifier.width(4.dp))
                                                    Text("Open", style = MaterialTheme.typography.labelSmall)
                                                }
                                            }
                                            Text(VesselI18n.tr("debrid.device_auth_waiting"), style = MaterialTheme.typography.labelSmall, color = palette.textMuted)
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                HorizontalDivider(color = palette.border.copy(alpha = 0.5f))

                Row(
                    modifier = Modifier.fillMaxWidth(),
                    verticalAlignment = Alignment.CenterVertically,
                    horizontalArrangement = Arrangement.SpaceBetween,
                ) {
                    Text(VesselI18n.tr("debrid.cloud_title"), fontWeight = FontWeight.SemiBold)
                    Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                        Text(VesselI18n.tr("debrid.cloud_auto_poll"), style = MaterialTheme.typography.labelSmall, color = palette.textMuted)
                        Switch(checked = cloudAutoPoll, onCheckedChange = { cloudAutoPoll = it })
                        VesselOutlinedButton(
                            height = 28.dp,
                            enabled = !loadingCloud,
                            onClick = { scope.launch { refreshCloud(silent = false) } },
                        ) {
                            Icon(Icons.Default.Refresh, contentDescription = null, modifier = Modifier.size(13.dp))
                        }
                    }
                }

                if (cloudItems.isEmpty()) {
                    Text(VesselI18n.tr("debrid.cloud_empty"), style = MaterialTheme.typography.labelSmall, color = palette.textMuted)
                } else {
                    cloudItems.take(25).forEach { item ->
                        Surface(
                            shape = RoundedCornerShape(10.dp),
                            color = palette.bgElevated,
                        ) {
                            Column(
                                modifier = Modifier.fillMaxWidth().padding(10.dp),
                                verticalArrangement = Arrangement.spacedBy(5.dp),
                            ) {
                                Row(
                                    modifier = Modifier.fillMaxWidth(),
                                    verticalAlignment = Alignment.CenterVertically,
                                    horizontalArrangement = Arrangement.spacedBy(8.dp),
                                ) {
                                    Column(modifier = Modifier.weight(1f)) {
                                        Text(item.filename, style = MaterialTheme.typography.bodySmall, fontWeight = FontWeight.SemiBold, maxLines = 1, overflow = TextOverflow.Ellipsis)
                                        val meta = listOfNotNull(
                                            item.providerLabel,
                                            item.status.uppercase(Locale.US),
                                            "${item.progressPct.toInt()}%",
                                            item.sizeBytes?.let(::formatSizeBytes),
                                            item.speedBps?.takeIf { it > 0L }?.let { "${formatSizeBytes(it)}/s" },
                                            item.seeders?.let { "$it seeders" },
                                            item.etaSeconds?.takeIf { it > 0L }?.let(::formatEtaSeconds),
                                        ).joinToString(" · ")
                                        Text(meta, style = MaterialTheme.typography.labelSmall, color = palette.textMuted)
                                    }
                                    if (item.status == "ready" && item.links.isNotEmpty()) {
                                        VesselCompactIconButton(
                                            size = 28.dp,
                                            onClick = {
                                                scope.launch {
                                                    runCatching { repo.unrestrictDebridCloudLink(item.target, item.links.first()) }
                                                        .onSuccess { resolved ->
                                                            clipboard.setText(AnnotatedString(resolved.url))
                                                            repo.notify("Resolved & copied stream URL")
                                                        }
                                                        .onFailure { repo.notifyError(it, "Failed to unrestrict link") }
                                                }
                                            },
                                        ) {
                                            Icon(Icons.Default.PlayArrow, contentDescription = null, modifier = Modifier.size(14.dp), tint = palette.accent)
                                        }
                                    }
                                    VesselCompactIconButton(
                                        size = 28.dp,
                                        onClick = {
                                            scope.launch {
                                                runCatching { repo.deleteDebridCloud(item.target, item.id) }
                                                    .onSuccess { refreshCloud(silent = true) }
                                                    .onFailure { repo.notifyError(it, "Failed to delete cloud item") }
                                            }
                                        },
                                    ) {
                                        Icon(Icons.Default.Delete, contentDescription = null, modifier = Modifier.size(14.dp), tint = palette.danger)
                                    }
                                }
                                LinearProgressIndicator(
                                    progress = { (item.progressPct / 100f).coerceIn(0f, 1f) },
                                    color = if (item.status == "error") palette.danger else palette.accent,
                                    modifier = Modifier.fillMaxWidth().height(4.dp).clip(RoundedCornerShape(999.dp)),
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

internal fun formatSizeBytes(bytes: Long): String {
    if (bytes <= 0L) return "0 B"
    val units = arrayOf("B", "KB", "MB", "GB", "TB")
    var value = bytes.toDouble()
    var idx = 0
    while (value >= 1024.0 && idx < units.lastIndex) {
        value /= 1024.0
        idx++
    }
    return if (idx == 0) "$bytes B" else String.format(Locale.US, "%.1f %s", value, units[idx])
}

internal fun formatEtaSeconds(seconds: Long): String {
    val h = seconds / 3600
    val m = (seconds % 3600) / 60
    val s = seconds % 60
    return if (h > 0) String.format(Locale.US, "%dh %02dm", h, m) else String.format(Locale.US, "%dm %02ds", m, s)
}
