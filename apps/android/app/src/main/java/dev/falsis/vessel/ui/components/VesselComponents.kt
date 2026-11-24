package dev.falsis.vessel.ui.components

import androidx.compose.foundation.Canvas
import androidx.compose.foundation.ExperimentalFoundationApi
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.combinedClickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.aspectRatio
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.BookmarkAdd
import androidx.compose.material.icons.filled.PlayArrow
import androidx.compose.material.icons.filled.Star
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.LinearProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.geometry.CornerRadius
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.Path
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import coil3.compose.AsyncImage
import dev.falsis.vessel.data.CardPreferences
import dev.falsis.vessel.data.ContinueWatchingEntry
import dev.falsis.vessel.data.DomainPermissionPrompt
import dev.falsis.vessel.data.MediaSummary
import dev.falsis.vessel.ui.i18n.VesselI18n
import dev.falsis.vessel.ui.theme.LocalVesselPalette

@Composable
fun VesselLogoMark(modifier: Modifier = Modifier) {
    val palette = LocalVesselPalette.current
    Canvas(modifier = modifier) {
        // Exact 24x24 Bow "V" Monolith cargo ship mark from desktop VesselLogo.vue
        val s = minOf(size.width, size.height) / 24f
        drawRoundRect(
            color = palette.accent,
            topLeft = Offset(8.5f * s, 3.5f * s),
            size = Size(7f * s, 3f * s),
            cornerRadius = CornerRadius(0.9f * s, 0.9f * s),
        )
        drawRoundRect(
            color = palette.accentSecondary,
            topLeft = Offset(6f * s, 7.5f * s),
            size = Size(5.5f * s, 3.2f * s),
            cornerRadius = CornerRadius(0.9f * s, 0.9f * s),
        )
        drawRoundRect(
            color = palette.textPrimary.copy(alpha = 0.92f),
            topLeft = Offset(12.5f * s, 7.5f * s),
            size = Size(5.5f * s, 3.2f * s),
            cornerRadius = CornerRadius(0.9f * s, 0.9f * s),
        )
        val hull = Path().apply {
            moveTo(3.5f * s, 12f * s)
            lineTo(12f * s, 20.8f * s)
            lineTo(20.5f * s, 12f * s)
            lineTo(16.2f * s, 12f * s)
            lineTo(12f * s, 16.4f * s)
            lineTo(7.8f * s, 12f * s)
            close()
        }
        drawPath(path = hull, color = palette.textPrimary)
        drawLine(
            color = palette.bgBase,
            start = Offset(12f * s, 12f * s),
            end = Offset(12f * s, 20.5f * s),
            strokeWidth = 1.2f * s,
        )
    }
}

@Composable
fun MediaPosterCard(
    item: MediaSummary,
    onClick: () -> Unit,
    onQuickStatus: ((String) -> Unit)? = null,
    cards: CardPreferences = CardPreferences(),
    modifier: Modifier = Modifier,
) {
    val palette = LocalVesselPalette.current
    var showMenu by remember { mutableStateOf(false) }
    val posterAspect = when (cards.aspectRatio) {
        "wide_16x9" -> 16f / 9f
        "square" -> 1f
        else -> 2f / 3f
    }

    Column(
        modifier = modifier
            .width(if (cards.aspectRatio == "wide_16x9") 210.dp else 136.dp)
            .clip(RoundedCornerShape(14.dp))
            .clickable(onClick = onClick),
    ) {
        Box(
            modifier = Modifier
                .fillMaxWidth()
                .aspectRatio(posterAspect)
                .clip(RoundedCornerShape(14.dp))
                .background(palette.bgElevated)
                .border(1.dp, palette.border.copy(alpha = 0.5f), RoundedCornerShape(14.dp)),
        ) {
            if (!item.poster.isNullOrBlank()) {
                AsyncImage(
                    model = item.poster,
                    contentDescription = item.title,
                    contentScale = ContentScale.Crop,
                    modifier = Modifier.fillMaxSize(),
                )
            }

            // Bottom subtle vignette for legibility
            Box(
                modifier = Modifier
                    .fillMaxWidth()
                    .height(48.dp)
                    .align(Alignment.BottomCenter)
                    .background(
                        Brush.verticalGradient(
                            listOf(Color.Transparent, Color.Black.copy(alpha = 0.75f)),
                        ),
                    ),
            )

            // Rating chip top-left
            if (cards.showRating) {
                item.rating?.takeIf { it > 0f }?.let { score ->
                    Surface(
                        shape = RoundedCornerShape(8.dp),
                        color = Color.Black.copy(alpha = 0.68f),
                        modifier = Modifier
                            .align(Alignment.TopStart)
                            .padding(6.dp),
                    ) {
                        Row(
                            verticalAlignment = Alignment.CenterVertically,
                            modifier = Modifier.padding(horizontal = 6.dp, vertical = 3.dp),
                        ) {
                            Icon(
                                imageVector = Icons.Default.Star,
                                contentDescription = null,
                                tint = palette.warning,
                                modifier = Modifier.size(11.dp),
                            )
                            Spacer(modifier = Modifier.width(3.dp))
                            Text(
                                text = String.format("%.1f", score),
                                style = MaterialTheme.typography.labelSmall,
                                color = Color.White,
                                fontSize = 10.sp,
                            )
                        }
                    }
                }
            }

            if (cards.showContentType) {
                Surface(
                    shape = RoundedCornerShape(6.dp),
                    color = Color.Black.copy(alpha = 0.68f),
                    modifier = Modifier
                        .align(Alignment.BottomStart)
                        .padding(6.dp),
                ) {
                    Text(
                        text = if (item.isAnime) "ANIME" else item.type.uppercase(),
                        style = MaterialTheme.typography.labelSmall,
                        color = Color.White,
                        fontSize = 9.sp,
                        modifier = Modifier.padding(horizontal = 5.dp, vertical = 2.dp),
                    )
                }
            }

            if (onQuickStatus != null) {
                Box(
                    modifier = Modifier
                        .align(Alignment.TopEnd)
                        .padding(4.dp),
                ) {
                    IconButton(
                        onClick = { showMenu = true },
                        modifier = Modifier
                            .size(28.dp)
                            .clip(CircleShape)
                            .background(Color.Black.copy(alpha = 0.62f)),
                    ) {
                        Icon(
                            imageVector = Icons.Default.BookmarkAdd,
                            contentDescription = VesselI18n.tr("media.quick_status"),
                            tint = Color.White,
                            modifier = Modifier.size(15.dp),
                        )
                    }
                    DropdownMenu(expanded = showMenu, onDismissRequest = { showMenu = false }) {
                        listOf(
                            "watching" to VesselI18n.tr("library.watching"),
                            "plan_to_watch" to VesselI18n.tr("library.plan_to_watch"),
                            "completed" to VesselI18n.tr("library.completed"),
                            "on_hold" to VesselI18n.tr("library.on_hold"),
                            "dropped" to VesselI18n.tr("library.dropped"),
                        ).forEach { (statusKey, label) ->
                            DropdownMenuItem(
                                text = { Text(label) },
                                onClick = {
                                    showMenu = false
                                    onQuickStatus(statusKey)
                                },
                            )
                        }
                    }
                }
            }
        }

        Spacer(modifier = Modifier.height(6.dp))
        Text(
            text = item.title,
            style = MaterialTheme.typography.bodyMedium,
            fontWeight = FontWeight.SemiBold,
            maxLines = 2,
            overflow = TextOverflow.Ellipsis,
            color = palette.textPrimary,
        )
        if (cards.showYear) {
            item.year?.let { y ->
                Text(
                    text = y.toString(),
                    style = MaterialTheme.typography.labelSmall,
                    color = palette.textMuted,
                )
            }
        }
    }
}

@OptIn(ExperimentalFoundationApi::class)
@Composable
fun ContinueWatchingCard(
    entry: ContinueWatchingEntry,
    onResume: () -> Unit,
    onGoToDetails: () -> Unit,
    onEdit: () -> Unit,
    onRemove: () -> Unit,
    modifier: Modifier = Modifier,
) {
    val palette = LocalVesselPalette.current
    var showMenu by remember { mutableStateOf(false) }
    val p = entry.progress
    val ratio = if (p.durationMs > 0) (p.positionMs.toFloat() / p.durationMs).coerceIn(0.04f, 0.99f) else 0.15f
    val remainingMin = ((p.durationMs - p.positionMs).coerceAtLeast(0L) / 60_000L).toInt()
    val subtitleText = if (entry.mediaType == "movie") {
        p.title ?: p.media.nativeId
    } else {
        val prefix = "S${p.season} · E${p.episode}"
        val ep = p.episodeTitle?.takeIf { it.isNotBlank() }
        if (ep != null) "$prefix · $ep" else prefix
    }

    Box(modifier = modifier.width(248.dp)) {
        Column(
            modifier = Modifier
                .fillMaxWidth()
                .clip(RoundedCornerShape(14.dp))
                .combinedClickable(
                    onClick = onResume,
                    onLongClick = { showMenu = true },
                ),
        ) {
            Box(
                modifier = Modifier
                    .fillMaxWidth()
                    .aspectRatio(16f / 9f)
                    .clip(RoundedCornerShape(14.dp))
                    .background(palette.bgElevated)
                    .border(1.dp, palette.border.copy(alpha = 0.5f), RoundedCornerShape(14.dp)),
            ) {
                val thumb = p.backdropUrl ?: p.posterUrl
                if (!thumb.isNullOrBlank()) {
                    AsyncImage(
                        model = thumb,
                        contentDescription = p.title,
                        contentScale = ContentScale.Crop,
                        modifier = Modifier.fillMaxSize(),
                    )
                }

                Box(
                    modifier = Modifier
                        .size(42.dp)
                        .clip(CircleShape)
                        .background(Color.Black.copy(alpha = 0.6f))
                        .align(Alignment.Center),
                    contentAlignment = Alignment.Center,
                ) {
                    Icon(
                        imageVector = Icons.Default.PlayArrow,
                        contentDescription = VesselI18n.tr("details.resume"),
                        tint = Color.White,
                    )
                }

                if (remainingMin > 0) {
                    Surface(
                        color = Color.Black.copy(alpha = 0.72f),
                        shape = RoundedCornerShape(8.dp),
                        modifier = Modifier
                            .align(Alignment.BottomStart)
                            .padding(8.dp),
                    ) {
                        Text(
                            text = "${remainingMin}m",
                            style = MaterialTheme.typography.labelSmall,
                            color = Color.White,
                            modifier = Modifier.padding(horizontal = 6.dp, vertical = 2.dp),
                        )
                    }
                }

                LinearProgressIndicator(
                    progress = { ratio },
                    color = palette.accent,
                    trackColor = Color.White.copy(alpha = 0.25f),
                    modifier = Modifier
                        .fillMaxWidth()
                        .height(4.dp)
                        .align(Alignment.BottomCenter),
                )
            }

            Spacer(modifier = Modifier.height(6.dp))
            Text(
                text = p.title ?: p.media.nativeId,
                style = MaterialTheme.typography.bodyMedium,
                fontWeight = FontWeight.SemiBold,
                maxLines = 1,
                overflow = TextOverflow.Ellipsis,
                color = palette.textPrimary,
            )
            Text(
                text = subtitleText,
                style = MaterialTheme.typography.bodySmall,
                maxLines = 1,
                overflow = TextOverflow.Ellipsis,
                color = palette.textMuted,
            )
        }

        DropdownMenu(expanded = showMenu, onDismissRequest = { showMenu = false }) {
            DropdownMenuItem(
                text = { Text(VesselI18n.tr("home.go_to_details")) },
                onClick = { showMenu = false; onGoToDetails() },
            )
            DropdownMenuItem(
                text = { Text(VesselI18n.tr("home.edit_cw")) },
                onClick = { showMenu = false; onEdit() },
            )
            DropdownMenuItem(
                text = { Text(VesselI18n.tr("common.remove")) },
                onClick = { showMenu = false; onRemove() },
            )
        }
    }
}

@Composable
fun DomainPromptDialog(
    prompt: DomainPermissionPrompt,
    onRespond: (allow: Boolean, permanent: Boolean) -> Unit,
) {
    AlertDialog(
        onDismissRequest = { onRespond(false, false) },
        title = { Text(VesselI18n.tr("overlay.domain_title")) },
        text = {
            Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                Text("${prompt.pluginId} → ${prompt.domain}", fontWeight = FontWeight.Bold)
                prompt.url?.let {
                    Text(it, style = MaterialTheme.typography.bodySmall, maxLines = 2, overflow = TextOverflow.Ellipsis)
                }
            }
        },
        confirmButton = {
            Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                OutlinedButton(onClick = { onRespond(true, false) }) {
                    Text(VesselI18n.tr("overlay.allow_once"))
                }
                Button(onClick = { onRespond(true, true) }) {
                    Text(VesselI18n.tr("overlay.allow_always"))
                }
            }
        },
        dismissButton = {
            TextButton(onClick = { onRespond(false, false) }) {
                Text(VesselI18n.tr("overlay.reject"))
            }
        },
    )
}
