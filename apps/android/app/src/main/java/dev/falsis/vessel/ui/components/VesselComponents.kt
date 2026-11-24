package dev.falsis.vessel.ui.components

import androidx.compose.animation.animateColorAsState
import androidx.compose.animation.core.FastOutSlowInEasing
import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.animation.core.tween
import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.ExperimentalFoundationApi
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.combinedClickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.RowScope
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.aspectRatio
import androidx.compose.foundation.layout.defaultMinSize
import androidx.compose.foundation.layout.displayCutout
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.statusBarsIgnoringVisibility
import androidx.compose.foundation.layout.union
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.windowInsetsPadding
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.BasicTextField
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Bookmark
import androidx.compose.material.icons.filled.CheckCircle
import androidx.compose.material.icons.filled.Clear
import androidx.compose.material.icons.filled.Edit
import androidx.compose.material.icons.filled.PlayArrow
import androidx.compose.material.icons.filled.Search
import androidx.compose.material.icons.filled.Star
import androidx.compose.material.icons.outlined.BookmarkAdd
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.Icon
import androidx.compose.material3.LinearProgressIndicator
import androidx.compose.material3.LocalContentColor
import androidx.compose.material3.LocalTextStyle
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.ProvideTextStyle
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
import androidx.compose.ui.graphics.SolidColor
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import coil3.compose.AsyncImage
import dev.falsis.vessel.data.CardPreferences
import dev.falsis.vessel.data.ContinueWatchingEntry
import dev.falsis.vessel.data.DomainPermissionPrompt
import dev.falsis.vessel.data.MediaSummary
import dev.falsis.vessel.ui.i18n.VesselI18n
import dev.falsis.vessel.ui.theme.LocalVesselPalette
import java.util.Locale

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
    }
}

@Composable
fun VesselButton(
    onClick: () -> Unit,
    modifier: Modifier = Modifier,
    enabled: Boolean = true,
    height: Dp = 36.dp,
    containerColor: Color = LocalVesselPalette.current.accent,
    contentColor: Color = if (LocalVesselPalette.current.isDark) Color(0xFF11111B) else Color.White,
    contentPadding: PaddingValues = PaddingValues(horizontal = 13.dp, vertical = 0.dp),
    content: @Composable RowScope.() -> Unit,
) {
    Button(
        onClick = onClick,
        enabled = enabled,
        shape = RoundedCornerShape(10.dp),
        colors = ButtonDefaults.buttonColors(
            containerColor = containerColor,
            contentColor = contentColor,
            disabledContainerColor = containerColor.copy(alpha = 0.35f),
            disabledContentColor = contentColor.copy(alpha = 0.55f),
        ),
        contentPadding = contentPadding,
        modifier = modifier.height(height).defaultMinSize(minWidth = 48.dp, minHeight = height),
    ) {
        ProvideTextStyle(
            MaterialTheme.typography.labelLarge.copy(fontWeight = FontWeight.SemiBold),
        ) {
            content()
        }
    }
}

@Composable
fun VesselOutlinedButton(
    onClick: () -> Unit,
    modifier: Modifier = Modifier,
    enabled: Boolean = true,
    height: Dp = 36.dp,
    contentPadding: PaddingValues = PaddingValues(horizontal = 12.dp, vertical = 0.dp),
    content: @Composable RowScope.() -> Unit,
) {
    val palette = LocalVesselPalette.current
    OutlinedButton(
        onClick = onClick,
        enabled = enabled,
        shape = RoundedCornerShape(10.dp),
        border = BorderStroke(1.dp, palette.border.copy(alpha = if (enabled) 0.85f else 0.35f)),
        colors = ButtonDefaults.outlinedButtonColors(
            containerColor = palette.bgSurface.copy(alpha = 0.72f),
            contentColor = palette.textPrimary,
            disabledContentColor = palette.textMuted.copy(alpha = 0.5f),
        ),
        contentPadding = contentPadding,
        modifier = modifier.height(height).defaultMinSize(minWidth = 44.dp, minHeight = height),
    ) {
        ProvideTextStyle(
            MaterialTheme.typography.labelLarge.copy(fontWeight = FontWeight.Medium),
        ) {
            content()
        }
    }
}

@Composable
fun VesselTextButton(
    onClick: () -> Unit,
    modifier: Modifier = Modifier,
    enabled: Boolean = true,
    height: Dp = 32.dp,
    contentColor: Color = LocalVesselPalette.current.accent,
    content: @Composable RowScope.() -> Unit,
) {
    TextButton(
        onClick = onClick,
        enabled = enabled,
        shape = RoundedCornerShape(9.dp),
        colors = ButtonDefaults.textButtonColors(contentColor = contentColor),
        contentPadding = PaddingValues(horizontal = 10.dp, vertical = 0.dp),
        modifier = modifier.height(height).defaultMinSize(minWidth = 36.dp, minHeight = height),
    ) {
        ProvideTextStyle(
            MaterialTheme.typography.labelMedium.copy(fontWeight = FontWeight.SemiBold),
        ) {
            content()
        }
    }
}

@Composable
fun VesselCompactIconButton(
    onClick: () -> Unit,
    modifier: Modifier = Modifier,
    enabled: Boolean = true,
    size: Dp = 34.dp,
    containerColor: Color = Color.Transparent,
    borderColor: Color? = null,
    content: @Composable () -> Unit,
) {
    val shape = CircleShape
    Box(
        contentAlignment = Alignment.Center,
        modifier = modifier
            .size(size)
            .clip(shape)
            .then(if (containerColor != Color.Transparent) Modifier.background(containerColor) else Modifier)
            .then(if (borderColor != null) Modifier.border(1.dp, borderColor, shape) else Modifier)
            .clickable(enabled = enabled, onClick = onClick),
    ) {
        content()
    }
}

@Composable
fun VesselFilterChip(
    selected: Boolean,
    onClick: () -> Unit,
    label: String,
    modifier: Modifier = Modifier,
) {
    val palette = LocalVesselPalette.current
    val bg by animateColorAsState(
        targetValue = if (selected) palette.accent.copy(alpha = 0.18f) else palette.bgSurface,
        animationSpec = tween(180),
        label = "chip_bg",
    )
    val border by animateColorAsState(
        targetValue = if (selected) palette.accent.copy(alpha = 0.72f) else palette.border.copy(alpha = 0.65f),
        animationSpec = tween(180),
        label = "chip_border",
    )
    val fg by animateColorAsState(
        targetValue = if (selected) palette.accent else palette.textMuted,
        animationSpec = tween(180),
        label = "chip_fg",
    )
    Surface(
        shape = RoundedCornerShape(999.dp),
        color = bg,
        border = BorderStroke(1.dp, border),
        modifier = modifier
            .height(30.dp)
            .clip(RoundedCornerShape(999.dp))
            .clickable(onClick = onClick),
    ) {
        Box(
            contentAlignment = Alignment.Center,
            modifier = Modifier.padding(horizontal = 12.dp),
        ) {
            Text(
                text = label,
                style = MaterialTheme.typography.labelMedium,
                fontWeight = if (selected) FontWeight.SemiBold else FontWeight.Medium,
                color = fg,
                maxLines = 1,
            )
        }
    }
}

@Composable
fun VesselSearchField(
    value: String,
    onValueChange: (String) -> Unit,
    placeholder: String,
    modifier: Modifier = Modifier,
    imeAction: ImeAction = ImeAction.Search,
    onSearch: (() -> Unit)? = null,
    onClear: (() -> Unit)? = null,
) {
    val palette = LocalVesselPalette.current
    Surface(
        shape = RoundedCornerShape(11.dp),
        color = palette.bgSurface,
        border = BorderStroke(1.dp, palette.border.copy(alpha = 0.65f)),
        modifier = modifier.height(42.dp),
    ) {
        Row(
            modifier = Modifier
                .fillMaxSize()
                .padding(horizontal = 12.dp),
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.spacedBy(8.dp),
        ) {
            Icon(
                imageVector = Icons.Default.Search,
                contentDescription = null,
                tint = palette.textMuted,
                modifier = Modifier.size(17.dp),
            )
            Box(
                modifier = Modifier.weight(1f),
                contentAlignment = Alignment.CenterStart,
            ) {
                if (value.isEmpty()) {
                    Text(
                        text = placeholder,
                        style = MaterialTheme.typography.bodyMedium,
                        color = palette.textMuted.copy(alpha = 0.72f),
                        maxLines = 1,
                        overflow = TextOverflow.Ellipsis,
                    )
                }
                BasicTextField(
                    value = value,
                    onValueChange = onValueChange,
                    singleLine = true,
                    textStyle = LocalTextStyle.current.copy(
                        color = palette.textPrimary,
                        fontSize = 13.5.sp,
                    ),
                    cursorBrush = SolidColor(palette.accent),
                    keyboardOptions = KeyboardOptions(imeAction = imeAction),
                    keyboardActions = KeyboardActions(onSearch = { onSearch?.invoke() }),
                    modifier = Modifier.fillMaxWidth(),
                )
            }
            if (value.isNotEmpty()) {
                VesselCompactIconButton(
                    onClick = {
                        onValueChange("")
                        onClear?.invoke()
                    },
                    size = 24.dp,
                ) {
                    Icon(
                        imageVector = Icons.Default.Clear,
                        contentDescription = VesselI18n.tr("common.close"),
                        tint = palette.textMuted,
                        modifier = Modifier.size(15.dp),
                    )
                }
            }
        }
    }
}

@Composable
@OptIn(ExperimentalLayoutApi::class)
fun Modifier.vesselTopSafeInset(minTop: Dp = 16.dp): Modifier =
    this.windowInsetsPadding(
        WindowInsets.statusBarsIgnoringVisibility
            .union(WindowInsets.displayCutout)
            .union(WindowInsets(top = minTop)),
    )

fun cardAspectRatioValue(aspectRatio: String): Float = when (aspectRatio) {
    "portrait_3x4" -> 3f / 4f
    "portrait_4x5" -> 4f / 5f
    "square" -> 1f
    "classic_4x3" -> 4f / 3f
    "wide_16x9", "wide16x9" -> 16f / 9f
    "cinema_21x9" -> 21f / 9f
    else -> 2f / 3f
}

fun isLandscapeCardRatio(aspectRatio: String): Boolean =
    cardAspectRatioValue(aspectRatio) >= (4f / 3f - 0.01f)

@Composable
fun MediaPosterCard(
    item: MediaSummary,
    onClick: () -> Unit,
    onQuickStatus: ((String) -> Unit)? = null,
    cards: CardPreferences = CardPreferences(),
    currentStatus: String? = null,
    fillGridWidth: Boolean = false,
    modifier: Modifier = Modifier,
) {
    val palette = LocalVesselPalette.current
    var showMenu by remember { mutableStateOf(false) }
    val targetAspect = cardAspectRatioValue(cards.aspectRatio)
    val isLandscape = targetAspect >= (4f / 3f - 0.01f)
    val rowWidth = when (cards.aspectRatio) {
        "cinema_21x9" -> 220.dp
        "wide_16x9", "wide16x9" -> 196.dp
        "classic_4x3" -> 168.dp
        "square" -> 136.dp
        else -> 124.dp
    }
    val posterAspect by animateFloatAsState(
        targetValue = targetAspect,
        animationSpec = tween(durationMillis = 220, easing = FastOutSlowInEasing),
        label = "card_aspect",
    )

    val widthModifier = if (fillGridWidth) {
        modifier.fillMaxWidth()
    } else {
        modifier.width(if (isLandscape) rowWidth else rowWidth)
    }

    // Important: never clip the outer Column, otherwise the title/metadata gets clipped or looks behind a container.
    Column(
        modifier = widthModifier
            .clickable(onClick = onClick),
    ) {
        Box(
            modifier = Modifier
                .fillMaxWidth()
                .aspectRatio(posterAspect)
                .clip(RoundedCornerShape(12.dp))
                .background(palette.bgElevated)
                .border(1.dp, palette.border.copy(alpha = 0.45f), RoundedCornerShape(12.dp)),
        ) {
            if (!item.poster.isNullOrBlank()) {
                AsyncImage(
                    model = item.poster,
                    contentDescription = item.title,
                    contentScale = ContentScale.Crop,
                    modifier = Modifier.fillMaxSize(),
                )
            }

            // Subtle top-right & bottom scrims for crisp overlay icons
            if (onQuickStatus != null || (cards.showRating && (item.rating ?: 0f) > 0f)) {
                Box(
                    modifier = Modifier
                        .fillMaxWidth()
                        .height(34.dp)
                        .align(Alignment.TopCenter)
                        .background(
                            Brush.verticalGradient(
                                listOf(Color.Black.copy(alpha = 0.36f), Color.Transparent),
                            ),
                        ),
                )
            }

            Box(
                modifier = Modifier
                    .fillMaxWidth()
                    .height(42.dp)
                    .align(Alignment.BottomCenter)
                    .background(
                        Brush.verticalGradient(
                            listOf(Color.Transparent, Color.Black.copy(alpha = 0.72f)),
                        ),
                    ),
            )

            // Rating chip top-left
            if (cards.showRating) {
                item.rating?.takeIf { it > 0f }?.let { score ->
                    Surface(
                        shape = RoundedCornerShape(7.dp),
                        color = Color.Black.copy(alpha = 0.58f),
                        modifier = Modifier
                            .align(Alignment.TopStart)
                            .padding(5.dp),
                    ) {
                        Row(
                            verticalAlignment = Alignment.CenterVertically,
                            modifier = Modifier.padding(horizontal = 5.dp, vertical = 2.dp),
                        ) {
                            Icon(
                                imageVector = Icons.Default.Star,
                                contentDescription = null,
                                tint = palette.warning,
                                modifier = Modifier.size(10.dp),
                            )
                            Spacer(modifier = Modifier.width(2.dp))
                            Text(
                                text = String.format(Locale.US, "%.1f", score),
                                style = MaterialTheme.typography.labelSmall,
                                color = Color.White,
                                fontSize = 9.5.sp,
                            )
                        }
                    }
                }
            }

            if (cards.showContentType) {
                Surface(
                    shape = RoundedCornerShape(5.dp),
                    color = Color.Black.copy(alpha = 0.56f),
                    modifier = Modifier
                        .align(Alignment.BottomStart)
                        .padding(5.dp),
                ) {
                    Text(
                        text = if (item.isAnime) "ANIME" else item.type.uppercase(Locale.US),
                        style = MaterialTheme.typography.labelSmall,
                        color = Color.White,
                        fontSize = 8.5.sp,
                        modifier = Modifier.padding(horizontal = 5.dp, vertical = 1.5.dp),
                    )
                }
            }

            // Minimal background-free quick status icon at top-right
            if (onQuickStatus != null) {
                Box(
                    modifier = Modifier
                        .align(Alignment.TopEnd)
                        .padding(3.dp),
                ) {
                    val iconTint = when (currentStatus) {
                        "completed" -> palette.success
                        "dropped" -> palette.danger
                        null -> Color.White.copy(alpha = 0.92f)
                        else -> palette.accent
                    }
                    val iconVec = when (currentStatus) {
                        "completed" -> Icons.Default.CheckCircle
                        null -> Icons.Outlined.BookmarkAdd
                        else -> Icons.Default.Bookmark
                    }
                    Box(
                        contentAlignment = Alignment.Center,
                        modifier = Modifier
                            .size(26.dp)
                            .clip(CircleShape)
                            .clickable { showMenu = true },
                    ) {
                        Icon(
                            imageVector = iconVec,
                            contentDescription = VesselI18n.tr("media.quick_status"),
                            tint = iconTint,
                            modifier = Modifier.size(17.dp),
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
                                text = {
                                    Text(
                                        text = label,
                                        fontWeight = if (currentStatus == statusKey) FontWeight.Bold else FontWeight.Normal,
                                        color = if (currentStatus == statusKey) palette.accent else LocalContentColor.current,
                                    )
                                },
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
            style = MaterialTheme.typography.bodySmall,
            fontSize = 12.5.sp,
            lineHeight = 15.5.sp,
            fontWeight = FontWeight.SemiBold,
            maxLines = 2,
            overflow = TextOverflow.Ellipsis,
            color = palette.textPrimary,
            modifier = Modifier
                .fillMaxWidth()
                .padding(horizontal = 2.dp),
        )
        if (cards.showYear && item.year != null) {
            Spacer(modifier = Modifier.height(2.dp))
            Text(
                text = item.year.toString(),
                style = MaterialTheme.typography.labelSmall,
                fontSize = 10.5.sp,
                color = palette.textMuted,
                maxLines = 1,
                modifier = Modifier.padding(horizontal = 2.dp),
            )
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

    Box(modifier = modifier.width(228.dp)) {
        Column(
            modifier = Modifier
                .fillMaxWidth()
                .combinedClickable(
                    onClick = onResume,
                    onLongClick = { showMenu = true },
                ),
        ) {
            Box(
                modifier = Modifier
                    .fillMaxWidth()
                    .aspectRatio(16f / 9f)
                    .clip(RoundedCornerShape(12.dp))
                    .background(palette.bgElevated)
                    .border(1.dp, palette.border.copy(alpha = 0.48f), RoundedCornerShape(12.dp)),
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
                        .size(38.dp)
                        .clip(CircleShape)
                        .background(Color.Black.copy(alpha = 0.56f))
                        .align(Alignment.Center),
                    contentAlignment = Alignment.Center,
                ) {
                    Icon(
                        imageVector = Icons.Default.PlayArrow,
                        contentDescription = VesselI18n.tr("details.resume"),
                        tint = Color.White,
                        modifier = Modifier.size(20.dp),
                    )
                }

                Box(
                    modifier = Modifier
                        .align(Alignment.TopEnd)
                        .padding(6.dp)
                        .size(26.dp)
                        .clip(CircleShape)
                        .background(Color.Black.copy(alpha = 0.58f))
                        .clickable { onEdit() },
                    contentAlignment = Alignment.Center,
                ) {
                    Icon(
                        imageVector = Icons.Default.Edit,
                        contentDescription = VesselI18n.tr("home.edit_cw"),
                        tint = Color.White,
                        modifier = Modifier.size(14.dp),
                    )
                }

                if (remainingMin > 0) {
                    Surface(
                        color = Color.Black.copy(alpha = 0.72f),
                        shape = RoundedCornerShape(7.dp),
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
                        .height(3.5.dp)
                        .align(Alignment.BottomCenter),
                )
            }

            Spacer(modifier = Modifier.height(6.dp))
            Text(
                text = p.title ?: p.media.nativeId,
                style = MaterialTheme.typography.bodySmall,
                fontSize = 12.5.sp,
                fontWeight = FontWeight.SemiBold,
                maxLines = 1,
                overflow = TextOverflow.Ellipsis,
                color = palette.textPrimary,
                modifier = Modifier.padding(horizontal = 2.dp),
            )
            Text(
                text = subtitleText,
                style = MaterialTheme.typography.labelSmall,
                maxLines = 1,
                overflow = TextOverflow.Ellipsis,
                color = palette.textMuted,
                modifier = Modifier.padding(horizontal = 2.dp),
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
                VesselOutlinedButton(onClick = { onRespond(true, false) }) {
                    Text(VesselI18n.tr("overlay.allow_once"))
                }
                VesselButton(onClick = { onRespond(true, true) }) {
                    Text(VesselI18n.tr("overlay.allow_always"))
                }
            }
        },
        dismissButton = {
            VesselTextButton(onClick = { onRespond(false, false) }) {
                Text(VesselI18n.tr("overlay.reject"))
            }
        },
    )
}
