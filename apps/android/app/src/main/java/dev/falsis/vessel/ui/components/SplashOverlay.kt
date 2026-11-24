package dev.falsis.vessel.ui.components

import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.LinearEasing
import androidx.compose.animation.core.tween
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.size
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.CornerRadius
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.Path
import androidx.compose.ui.graphics.StrokeCap
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.graphics.drawscope.withTransform
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.unit.dp
import dev.falsis.vessel.ui.theme.LocalVesselPalette
import kotlin.math.PI
import kotlin.math.max
import kotlin.math.pow
import kotlin.math.sin

private fun clamp01(v: Float): Float = v.coerceIn(0f, 1f)
private fun lerpF(a: Float, b: Float, t: Float): Float = a + (b - a) * t
private fun easeOutCubic(t: Float): Float = 1f - (1f - t).pow(3)
private fun easeInOutCubic(t: Float): Float =
    if (t < 0.5f) 4f * t * t * t else 1f - (-2f * t + 2f).pow(3) / 2f

@Composable
fun VesselSplashOverlay(
    onDone: () -> Unit,
) {
    val palette = LocalVesselPalette.current
    val progress = remember { Animatable(0f) }

    LaunchedEffect(Unit) {
        progress.animateTo(
            targetValue = 1f,
            animationSpec = tween(durationMillis = 2850, easing = LinearEasing),
        )
        onDone()
    }

    val p = progress.value
    val sailP = easeOutCubic(clamp01(p / 0.34f))
    val turnP = easeInOutCubic(clamp01((p - 0.31f) / 0.18f))
    val settleP = easeOutCubic(clamp01((p - 0.43f) / 0.13f))
    val revealP = easeInOutCubic(clamp01((p - 0.80f) / 0.20f))

    // Fully opaque solid base layer + subtle accent glow so underlying UI is never visible during splash
    Box(
        modifier = Modifier
            .fillMaxSize()
            .background(palette.bgBase)
            .background(
                Brush.radialGradient(
                    colors = listOf(
                        palette.accent.copy(alpha = 0.10f),
                        Color.Transparent,
                    ),
                ),
            )
            .pointerInput(Unit) {},
        contentAlignment = Alignment.Center,
    ) {
        Canvas(modifier = Modifier.size(340.dp, 210.dp)) {
            // Map Desktop's 360x220 stage coordinate space directly onto Android Canvas
            val unit = minOf(size.width / 360f, size.height / 220f)
            val cx = size.width / 2f
            val cy = size.height / 2f

            // Phase 3+ radial glow behind settled bow logo
            val glowAlpha = clamp01((p - 0.44f) / 0.18f) * 0.24f
            if (glowAlpha > 0f) {
                drawCircle(
                    brush = Brush.radialGradient(
                        colors = listOf(palette.accent.copy(alpha = glowAlpha), Color.Transparent),
                        center = Offset(cx, cy),
                        radius = 64f * unit,
                    ),
                    radius = 64f * unit,
                    center = Offset(cx, cy),
                )
            }

            // Phase 1 waves
            val waveOpacity = clamp01(1f - clamp01((p - 0.35f) / 0.22f))
            if (waveOpacity > 0.01f) {
                val waveShift = -26f * p * unit
                val waveY = (sin(p * PI * 3.5).toFloat() * 1.4f) * unit
                withTransform({ translate(cx + waveShift, cy + waveY) }) {
                    val wave1 = Path().apply {
                        moveTo(-125f * unit, 22f * unit)
                        quadraticTo(-90f * unit, 15f * unit, -55f * unit, 22f * unit)
                        quadraticTo(-20f * unit, 29f * unit, 15f * unit, 22f * unit)
                        quadraticTo(50f * unit, 15f * unit, 85f * unit, 22f * unit)
                        quadraticTo(120f * unit, 29f * unit, 155f * unit, 22f * unit)
                    }
                    drawPath(
                        path = wave1,
                        color = palette.accentSecondary.copy(alpha = 0.58f * waveOpacity),
                        style = Stroke(width = 2f * unit, cap = StrokeCap.Round),
                    )
                    val wave2 = Path().apply {
                        moveTo(-110f * unit, 31f * unit)
                        quadraticTo(-75f * unit, 25f * unit, -40f * unit, 31f * unit)
                        quadraticTo(-5f * unit, 37f * unit, 30f * unit, 31f * unit)
                        quadraticTo(65f * unit, 25f * unit, 100f * unit, 31f * unit)
                        quadraticTo(135f * unit, 37f * unit, 170f * unit, 31f * unit)
                    }
                    drawPath(
                        path = wave2,
                        color = palette.accent.copy(alpha = 0.38f * waveOpacity),
                        style = Stroke(width = 1.5f * unit, cap = StrokeCap.Round),
                    )
                }
            }

            // Phase 1 -> 2 side-profile cargo ship sailing into center
            val sideAlpha = if (p < 0.08f) p / 0.08f else clamp01(1f - clamp01((turnP - 0.3f) / 0.4f))
            if (sideAlpha > 0.01f) {
                val sideX = lerpF(-92f, 0f, sailP) * unit
                val sideY = (sin(p * PI * 5.5).toFloat() * 1.7f * (1f - turnP)) * unit
                val sideScaleX = max(0.04f, 1f - turnP)
                withTransform({
                    translate(cx + sideX, cy + sideY)
                    scale(sideScaleX, 1f, pivot = Offset.Zero)
                }) {
                    drawRoundRect(
                        color = palette.accent.copy(alpha = sideAlpha),
                        topLeft = Offset(-28f * unit, -18f * unit),
                        size = Size(22f * unit, 10f * unit),
                        cornerRadius = CornerRadius(2f * unit, 2f * unit),
                    )
                    drawRoundRect(
                        color = palette.accentSecondary.copy(alpha = sideAlpha),
                        topLeft = Offset(-3f * unit, -18f * unit),
                        size = Size(18f * unit, 10f * unit),
                        cornerRadius = CornerRadius(2f * unit, 2f * unit),
                    )
                    drawRoundRect(
                        color = palette.textPrimary.copy(alpha = sideAlpha),
                        topLeft = Offset(18f * unit, -24f * unit),
                        size = Size(11f * unit, 16f * unit),
                        cornerRadius = CornerRadius(2f * unit, 2f * unit),
                    )
                    val hullSide = Path().apply {
                        moveTo(-42f * unit, -5f * unit)
                        lineTo(40f * unit, -5f * unit)
                        lineTo(31f * unit, 12f * unit)
                        lineTo(-33f * unit, 12f * unit)
                        close()
                    }
                    drawPath(path = hullSide, color = palette.textPrimary.copy(alpha = sideAlpha))
                }
            }

            // Phase 2 -> 4 Bow "V" Monolith Vessel Logo reveal & settle hold
            val bowScaleX = clamp01((turnP - 0.32f) / 0.68f)
            if (bowScaleX > 0.01f) {
                val settleScale = lerpF(0.92f, 1.0f, settleP)
                val revealZoom = lerpF(1.0f, 1.14f, revealP)
                val finalScaleX = bowScaleX * settleScale * revealZoom
                val finalScaleY = settleScale * revealZoom
                val logoScale = 3.2f * unit

                withTransform({
                    translate(cx, cy)
                    scale(finalScaleX, finalScaleY, pivot = Offset.Zero)
                    translate(-12f * logoScale, -12f * logoScale)
                }) {
                    drawRoundRect(
                        color = palette.accent.copy(alpha = bowScaleX),
                        topLeft = Offset(8.5f * logoScale, 3.5f * logoScale),
                        size = Size(7f * logoScale, 3f * logoScale),
                        cornerRadius = CornerRadius(0.9f * logoScale, 0.9f * logoScale),
                    )
                    drawRoundRect(
                        color = palette.accentSecondary.copy(alpha = bowScaleX),
                        topLeft = Offset(6f * logoScale, 7.5f * logoScale),
                        size = Size(5.5f * logoScale, 3.2f * logoScale),
                        cornerRadius = CornerRadius(0.9f * logoScale, 0.9f * logoScale),
                    )
                    drawRoundRect(
                        color = palette.textPrimary.copy(alpha = 0.92f * bowScaleX),
                        topLeft = Offset(12.5f * logoScale, 7.5f * logoScale),
                        size = Size(5.5f * logoScale, 3.2f * logoScale),
                        cornerRadius = CornerRadius(0.9f * logoScale, 0.9f * logoScale),
                    )
                    val bowPath = Path().apply {
                        moveTo(3.5f * logoScale, 12f * logoScale)
                        lineTo(12f * logoScale, 20.8f * logoScale)
                        lineTo(20.5f * logoScale, 12f * logoScale)
                        lineTo(16.2f * logoScale, 12f * logoScale)
                        lineTo(12f * logoScale, 16.4f * logoScale)
                        lineTo(7.8f * logoScale, 12f * logoScale)
                        close()
                    }
                    drawPath(path = bowPath, color = palette.textPrimary.copy(alpha = bowScaleX))
                }
            }
        }
    }
}
