package dev.falsis.vessel.ui.screens

import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.input.key.Key
import androidx.compose.ui.input.key.KeyEventType
import androidx.compose.ui.input.key.key
import androidx.compose.ui.input.key.onPreviewKeyEvent
import androidx.compose.ui.input.key.type
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import dev.falsis.vessel.ui.components.vesselFocusBorder

internal enum class PlayerDrawer {
    EPISODES,
    SOURCES,
    AUDIO,
    SUBTITLES,
    SETTINGS,
}

internal fun Modifier.tvStepSliderKeys(
    current: Int,
    step: Int,
    range: IntRange,
    onStep: (Int) -> Unit,
): Modifier = this
    .vesselFocusBorder(RoundedCornerShape(8.dp))
    .onPreviewKeyEvent { ev ->
        if (ev.type != KeyEventType.KeyDown) return@onPreviewKeyEvent false
        when (ev.key) {
            Key.DirectionLeft -> {
                onStep((current - step).coerceIn(range.first, range.last))
                true
            }
            Key.DirectionRight -> {
                onStep((current + step).coerceIn(range.first, range.last))
                true
            }
            else -> false
        }
    }

@Composable
internal fun DrawerItemCard(
    active: Boolean,
    accentColor: Color,
    modifier: Modifier = Modifier,
    onClick: () -> Unit,
    content: @Composable () -> Unit,
) {
    Surface(
        shape = RoundedCornerShape(10.dp),
        color = if (active) accentColor.copy(alpha = 0.18f) else Color.White.copy(alpha = 0.04f),
        border = BorderStroke(
            width = 1.dp,
            color = if (active) accentColor.copy(alpha = 0.55f) else Color.White.copy(alpha = 0.08f),
        ),
        modifier = modifier
            .fillMaxWidth()
            .clip(RoundedCornerShape(10.dp))
            .vesselFocusBorder(RoundedCornerShape(10.dp))
            .clickable(onClick = onClick),
    ) {
        Box(modifier = Modifier.padding(horizontal = 10.dp, vertical = 7.dp)) {
            content()
        }
    }
}

@Composable
internal fun DrawerTagBadge(
    text: String,
    accent: Boolean,
    accentColor: Color,
) {
    Surface(
        shape = RoundedCornerShape(6.dp),
        color = if (accent) accentColor.copy(alpha = 0.26f) else Color.White.copy(alpha = 0.10f),
    ) {
        Text(
            text = text,
            color = if (accent) accentColor.copy(alpha = 0.95f) else Color.White.copy(alpha = 0.85f),
            style = MaterialTheme.typography.labelSmall,
            fontWeight = FontWeight.Bold,
            modifier = Modifier.padding(horizontal = 6.dp, vertical = 2.5.dp),
        )
    }
}

@Composable
internal fun DrawerSettingBlock(
    title: String,
    content: @Composable () -> Unit,
) {
    Surface(
        shape = RoundedCornerShape(10.dp),
        color = Color.White.copy(alpha = 0.04f),
        border = BorderStroke(1.dp, Color.White.copy(alpha = 0.08f)),
        modifier = Modifier.fillMaxWidth(),
    ) {
        Column(
            modifier = Modifier.padding(9.dp),
            verticalArrangement = Arrangement.spacedBy(5.dp),
        ) {
            Text(
                text = title,
                color = Color.White.copy(alpha = 0.78f),
                style = MaterialTheme.typography.labelSmall,
                fontWeight = FontWeight.Bold,
            )
            content()
        }
    }
}

@Composable
internal fun DrawerChipButton(
    label: String,
    active: Boolean,
    accentColor: Color,
    modifier: Modifier = Modifier,
    onClick: () -> Unit,
) {
    Surface(
        shape = RoundedCornerShape(999.dp),
        color = if (active) accentColor else Color.White.copy(alpha = 0.06f),
        border = BorderStroke(
            width = 1.dp,
            color = if (active) Color.Transparent else Color.White.copy(alpha = 0.14f),
        ),
        modifier = modifier
            .clip(RoundedCornerShape(999.dp))
            .vesselFocusBorder(RoundedCornerShape(999.dp))
            .clickable(onClick = onClick),
    ) {
        Text(
            text = label,
            color = if (active) Color(0xFF0B0B12) else Color.White,
            style = MaterialTheme.typography.labelMedium,
            fontWeight = FontWeight.SemiBold,
            modifier = Modifier.padding(horizontal = 11.dp, vertical = 5.5.dp),
        )
    }
}
