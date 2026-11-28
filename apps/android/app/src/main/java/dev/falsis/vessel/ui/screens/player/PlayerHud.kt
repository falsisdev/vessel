package dev.falsis.vessel.ui.screens

import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.clickable
import androidx.compose.foundation.focusable
import androidx.compose.foundation.gestures.detectHorizontalDragGestures
import androidx.compose.foundation.gestures.detectTapGestures
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.List
import androidx.compose.material.icons.filled.Audiotrack
import androidx.compose.material.icons.filled.ClosedCaption
import androidx.compose.material.icons.filled.Info
import androidx.compose.material.icons.filled.Layers
import androidx.compose.material.icons.filled.LockOpen
import androidx.compose.material.icons.filled.ScreenRotation
import androidx.compose.material.icons.filled.Settings
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableLongStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.focus.FocusRequester
import androidx.compose.ui.focus.focusProperties
import androidx.compose.ui.focus.focusRequester
import androidx.compose.ui.geometry.CornerRadius
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.input.key.Key
import androidx.compose.ui.input.key.KeyEventType
import androidx.compose.ui.input.key.key
import androidx.compose.ui.input.key.onPreviewKeyEvent
import androidx.compose.ui.input.key.type
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import dev.falsis.vessel.data.SkipSegment
import dev.falsis.vessel.ui.components.vesselFocusBorder
import dev.falsis.vessel.ui.i18n.VesselI18n
import dev.falsis.vessel.ui.theme.LocalVesselPalette
import java.util.Locale

internal val PlayerDrawerSurfaceColor = Color(0xF010101A)
internal val PlayerHudControlBg = Color(0xA612121C)
internal val PlayerHudBorderColor = Color(0x24FFFFFF)

@Composable
internal fun TopActionPillsRow(
    pluginId: String,
    isLive: Boolean,
    isTv: Boolean,
    episodesCount: Int,
    streamsCount: Int,
    audioCount: Int,
    activeDrawer: PlayerDrawer?,
    showStatsHud: Boolean,
    isLandscape: Boolean,
    sourcesPillFocus: FocusRequester,
    backBtnFocus: FocusRequester,
    centerFocus: FocusRequester,
    onGoToDetails: () -> Unit,
    onLock: () -> Unit,
    onToggleStats: () -> Unit,
    onToggleDrawer: (PlayerDrawer) -> Unit,
    onToggleOrientation: () -> Unit,
    modifier: Modifier = Modifier,
) {
    val pillNavModifier = Modifier.focusProperties {
        up = backBtnFocus
        down = centerFocus
    }
    Row(
        modifier = modifier,
        horizontalArrangement = Arrangement.spacedBy(6.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        if (!isLive) {
            PlayerHudPill(
                icon = { Icon(Icons.Default.Info, contentDescription = null, tint = Color.White, modifier = Modifier.size(14.dp)) },
                label = VesselI18n.tr("player.go_to_details"),
                active = false,
                modifier = pillNavModifier,
                onClick = onGoToDetails,
            )
        }
        if (!isTv) {
            PlayerHudPill(
                icon = { Icon(Icons.Default.LockOpen, contentDescription = null, tint = Color.White, modifier = Modifier.size(14.dp)) },
                label = VesselI18n.tr("player.lock", "Kilitle"),
                active = false,
                modifier = pillNavModifier,
                onClick = onLock,
            )
        }
        if (!isLive || episodesCount > 1) {
            PlayerHudPill(
                icon = { Icon(Icons.AutoMirrored.Filled.List, contentDescription = null, tint = Color.White, modifier = Modifier.size(14.dp)) },
                label = if (isLive) "${VesselI18n.tr("iptv.channels", "Kanallar")} ($episodesCount)" else "${VesselI18n.tr("details.episodes", "Bölümler")} ($episodesCount)",
                active = activeDrawer == PlayerDrawer.EPISODES,
                modifier = pillNavModifier,
                onClick = { onToggleDrawer(PlayerDrawer.EPISODES) },
            )
        }
        PlayerHudPill(
            icon = { Icon(Icons.Default.Layers, contentDescription = null, tint = Color.White, modifier = Modifier.size(14.dp)) },
            label = "${VesselI18n.tr("details.sources", "Kaynaklar")} ($streamsCount)",
            active = activeDrawer == PlayerDrawer.SOURCES,
            modifier = pillNavModifier.focusRequester(sourcesPillFocus),
            onClick = { onToggleDrawer(PlayerDrawer.SOURCES) },
        )
        if (!isLive) {
            PlayerHudPill(
                icon = { Icon(Icons.Default.Audiotrack, contentDescription = null, tint = Color.White, modifier = Modifier.size(14.dp)) },
                label = if (audioCount > 1) "${VesselI18n.tr("player.audio", "Ses")} ($audioCount)" else VesselI18n.tr("player.audio", "Ses"),
                active = activeDrawer == PlayerDrawer.AUDIO,
                modifier = pillNavModifier,
                onClick = { onToggleDrawer(PlayerDrawer.AUDIO) },
            )
        }
        if (!isLive) {
            PlayerHudPill(
                icon = { Icon(Icons.Default.ClosedCaption, contentDescription = null, tint = Color.White, modifier = Modifier.size(14.dp)) },
                label = VesselI18n.tr("settings.tab_subtitles", "Altyazı"),
                active = activeDrawer == PlayerDrawer.SUBTITLES,
                modifier = pillNavModifier,
                onClick = { onToggleDrawer(PlayerDrawer.SUBTITLES) },
            )
        }
        PlayerHudPill(
            icon = { Icon(Icons.Default.Settings, contentDescription = null, tint = Color.White, modifier = Modifier.size(14.dp)) },
            label = VesselI18n.tr("nav.settings", "Ayarlar"),
            active = activeDrawer == PlayerDrawer.SETTINGS,
            modifier = pillNavModifier,
            onClick = { onToggleDrawer(PlayerDrawer.SETTINGS) },
        )
        PlayerHudPill(
            icon = null,
            label = VesselI18n.tr("stats.title", "Stats"),
            active = showStatsHud,
            modifier = pillNavModifier,
            onClick = onToggleStats,
        )
        if (!isTv) {
            PlayerHudPill(
                icon = { Icon(Icons.Default.ScreenRotation, contentDescription = null, tint = Color.White, modifier = Modifier.size(13.dp)) },
                label = if (isLandscape) VesselI18n.tr("common.portrait") else VesselI18n.tr("common.landscape"),
                active = isLandscape,
                modifier = pillNavModifier,
                onClick = onToggleOrientation,
            )
        }
    }
}

@Composable
internal fun DesktopScrubberBar(
    currentMs: Long,
    bufferedMs: Long,
    durationMs: Long,
    skipSegments: List<SkipSegment>,
    accentColor: Color,
    isScrubbing: Boolean,
    onScrubChange: (Long) -> Unit,
    onScrubCommit: (Long) -> Unit,
    onTogglePlayPause: () -> Unit = {},
    scrubberModifier: Modifier = Modifier,
    modifier: Modifier = Modifier,
) {
    val safeDur = durationMs.coerceAtLeast(1L)
    var dragMs by remember { mutableLongStateOf(currentMs) }

    BoxWithConstraints(
        modifier = modifier.height(34.dp),
        contentAlignment = Alignment.Center,
    ) {
        val density = LocalDensity.current
        val trackWidthPx = with(density) { maxWidth.toPx().coerceAtLeast(1f) }

        fun xToMs(x: Float): Long {
            val frac = (x / trackWidthPx).coerceIn(0f, 1f)
            return (frac * safeDur).toLong()
        }

        if (isScrubbing && durationMs > 0L) {
            val posFrac = (currentMs.toFloat() / safeDur.toFloat()).coerceIn(0f, 1f)
            val offsetDp = with(density) { (trackWidthPx * posFrac).toDp() }
            Surface(
                shape = RoundedCornerShape(6.dp),
                color = Color(0xEB0F0F18),
                border = BorderStroke(1.dp, Color.White.copy(alpha = 0.16f)),
                modifier = Modifier
                    .align(Alignment.TopStart)
                    .offset(
                        x = (offsetDp - 24.dp).coerceIn(0.dp, (maxWidth - 56.dp).coerceAtLeast(0.dp)),
                        y = (-26).dp,
                    ),
            ) {
                Text(
                    text = formatClockMs(currentMs),
                    color = Color.White,
                    style = MaterialTheme.typography.labelSmall,
                    modifier = Modifier.padding(horizontal = 7.dp, vertical = 2.dp),
                )
            }
        }
        Canvas(
            modifier = scrubberModifier
                .fillMaxWidth()
                .height(28.dp)
                .clip(RoundedCornerShape(8.dp))
                .vesselFocusBorder(RoundedCornerShape(8.dp))
                .focusable()
                .onPreviewKeyEvent { ev ->
                    if (ev.type != KeyEventType.KeyDown) return@onPreviewKeyEvent false
                    when (ev.key) {
                        Key.DirectionLeft -> {
                            if (safeDur > 1L) {
                                onScrubCommit((currentMs - 10_000L).coerceAtLeast(0L))
                                true
                            } else false
                        }
                        Key.DirectionRight -> {
                            if (safeDur > 1L) {
                                onScrubCommit((currentMs + 10_000L).coerceAtMost(durationMs.coerceAtLeast(currentMs)))
                                true
                            } else false
                        }
                        Key.DirectionCenter, Key.Enter, Key.NumPadEnter -> {
                            onTogglePlayPause()
                            true
                        }
                        else -> false
                    }
                }
                .pointerInput(safeDur) {
                    detectTapGestures { offset ->
                        val target = xToMs(offset.x)
                        onScrubCommit(target)
                    }
                }
                .pointerInput(safeDur) {
                    detectHorizontalDragGestures(
                        onDragStart = { offset ->
                            dragMs = xToMs(offset.x)
                            onScrubChange(dragMs)
                        },
                        onHorizontalDrag = { change, _ ->
                            change.consume()
                            dragMs = xToMs(change.position.x)
                            onScrubChange(dragMs)
                        },
                        onDragEnd = { onScrubCommit(dragMs) },
                        onDragCancel = { onScrubCommit(dragMs) },
                    )
                },
        ) {
            val barHeight = if (isScrubbing) 8.dp.toPx() else 6.dp.toPx()
            val barTop = (size.height - barHeight) / 2f
            val radius = CornerRadius(barHeight / 2f, barHeight / 2f)

            drawRoundRect(
                color = Color.White.copy(alpha = 0.22f),
                topLeft = Offset(0f, barTop),
                size = Size(size.width, barHeight),
                cornerRadius = radius,
            )

            val bufFrac = (bufferedMs.toFloat() / safeDur.toFloat()).coerceIn(0f, 1f)
            if (bufFrac > 0f) {
                drawRoundRect(
                    color = Color.White.copy(alpha = 0.32f),
                    topLeft = Offset(0f, barTop),
                    size = Size(size.width * bufFrac, barHeight),
                    cornerRadius = radius,
                )
            }

            if (durationMs > 10_000L) {
                for (seg in skipSegments) {
                    val startFrac = (seg.startMs.toFloat() / durationMs.toFloat()).coerceIn(0f, 1f)
                    val endMs = (seg.endMs ?: durationMs).coerceAtMost(durationMs)
                    val endFrac = (endMs.toFloat() / durationMs.toFloat()).coerceIn(startFrac, 1f)
                    val segColor = when (seg.type.lowercase(Locale.US)) {
                        "recap" -> Color(0xFFF9E2AF)
                        "credits" -> Color(0xFF89B4FA)
                        "preview" -> Color(0xFFA6E3A1)
                        else -> Color(0xFFCBA6F7)
                    }
                    drawRoundRect(
                        color = segColor.copy(alpha = 0.78f),
                        topLeft = Offset(size.width * startFrac, barTop),
                        size = Size((size.width * (endFrac - startFrac)).coerceAtLeast(6f), barHeight),
                        cornerRadius = radius,
                    )
                }
            }

            val posFrac = (currentMs.toFloat() / safeDur.toFloat()).coerceIn(0f, 1f)
            val fillWidth = size.width * posFrac
            if (fillWidth > 0f) {
                drawRoundRect(
                    color = accentColor,
                    topLeft = Offset(0f, barTop),
                    size = Size(fillWidth, barHeight),
                    cornerRadius = radius,
                )
            }
            val thumbRadius = if (isScrubbing) 8.dp.toPx() else 7.dp.toPx()
            drawCircle(
                color = Color.White,
                radius = thumbRadius,
                center = Offset(fillWidth.coerceIn(thumbRadius, size.width - thumbRadius), size.height / 2f),
            )
        }
    }
}

@Composable
internal fun HudCircleIconButton(
    onClick: () -> Unit,
    enabled: Boolean = true,
    size: Dp = 34.dp,
    modifier: Modifier = Modifier,
    content: @Composable () -> Unit,
) {
    Surface(
        shape = CircleShape,
        color = PlayerHudControlBg,
        border = BorderStroke(1.dp, PlayerHudBorderColor),
        modifier = modifier
            .size(size)
            .clip(CircleShape)
            .vesselFocusBorder(CircleShape)
            .clickable(enabled = enabled, onClick = onClick),
    ) {
        Box(contentAlignment = Alignment.Center) {
            content()
        }
    }
}

@Composable
internal fun PlayerHudPill(
    icon: (@Composable () -> Unit)?,
    label: String,
    active: Boolean,
    modifier: Modifier = Modifier,
    onClick: () -> Unit,
) {
    val palette = LocalVesselPalette.current
    Surface(
        shape = RoundedCornerShape(999.dp),
        color = if (active) palette.accent.copy(alpha = 0.26f) else PlayerHudControlBg,
        border = BorderStroke(
            width = 1.dp,
            color = if (active) palette.accent.copy(alpha = 0.65f) else PlayerHudBorderColor,
        ),
        modifier = modifier
            .clip(RoundedCornerShape(999.dp))
            .vesselFocusBorder(RoundedCornerShape(999.dp))
            .clickable(onClick = onClick),
    ) {
        Row(
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.spacedBy(4.dp),
            modifier = Modifier.padding(horizontal = 9.dp, vertical = 5.dp),
        ) {
            icon?.invoke()
            Text(
                text = label,
                color = Color.White,
                style = MaterialTheme.typography.labelSmall,
                fontWeight = FontWeight.SemiBold,
                maxLines = 1,
            )
        }
    }
}

internal fun formatClockMs(ms: Long): String {
    val totalSec = (ms / 1000L).coerceAtLeast(0L)
    val hrs = totalSec / 3600L
    val mins = (totalSec % 3600L) / 60L
    val secs = totalSec % 60L
    return if (hrs > 0L) {
        String.format(Locale.US, "%d:%02d:%02d", hrs, mins, secs)
    } else {
        String.format(Locale.US, "%d:%02d", mins, secs)
    }
}
