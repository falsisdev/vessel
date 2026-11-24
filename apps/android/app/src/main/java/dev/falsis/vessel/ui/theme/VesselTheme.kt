package dev.falsis.vessel.ui.theme

import android.os.Build
import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.material3.ColorScheme
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.darkColorScheme
import androidx.compose.material3.dynamicDarkColorScheme
import androidx.compose.material3.dynamicLightColorScheme
import androidx.compose.material3.lightColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.Immutable
import androidx.compose.runtime.staticCompositionLocalOf
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalContext
import dev.falsis.vessel.data.ThemeColors
import dev.falsis.vessel.data.ThemeSpec

const val DYNAMIC_MATERIAL_THEME_ID = "dynamic-material"

@Immutable
data class VesselPalette(
    val bgBase: Color,
    val bgSurface: Color,
    val bgElevated: Color,
    val textPrimary: Color,
    val textMuted: Color,
    val accent: Color,
    val accentSecondary: Color,
    val border: Color,
    val success: Color,
    val warning: Color,
    val danger: Color,
    val isDark: Boolean,
)

private val DefaultPalette = ThemeSpec().toPalette()

val LocalVesselPalette = staticCompositionLocalOf { DefaultPalette }

fun parseHexColor(raw: String, fallback: Color): Color {
    val hex = raw.trim().removePrefix("#")
    return try {
        when (hex.length) {
            6 -> Color(("FF$hex").toLong(16))
            8 -> Color(hex.toLong(16))
            else -> fallback
        }
    } catch (_: Throwable) {
        fallback
    }
}

fun ThemeSpec.toPalette(): VesselPalette {
    val c: ThemeColors = colors
    return VesselPalette(
        bgBase = parseHexColor(c.bgBase, Color(0xFF1E1E2E)),
        bgSurface = parseHexColor(c.bgSurface, Color(0xFF181825)),
        bgElevated = parseHexColor(c.bgElevated, Color(0xFF313244)),
        textPrimary = parseHexColor(c.textPrimary, Color(0xFFCDD6F4)),
        textMuted = parseHexColor(c.textMuted, Color(0xFFA6ADC8)),
        accent = parseHexColor(c.accent, Color(0xFFCBA6F7)),
        accentSecondary = parseHexColor(c.accentSecondary, Color(0xFF89B4FA)),
        border = parseHexColor(c.border, Color(0xFF45475A)),
        success = parseHexColor(c.success, Color(0xFFA6E3A1)),
        warning = parseHexColor(c.warning, Color(0xFFF9E2AF)),
        danger = parseHexColor(c.danger, Color(0xFFF38BA8)),
        isDark = isDark,
    )
}

fun ColorScheme.toVesselPalette(isDark: Boolean): VesselPalette = VesselPalette(
    bgBase = background,
    bgSurface = surfaceContainerLow,
    bgElevated = surfaceContainerHigh,
    textPrimary = onBackground,
    textMuted = onSurfaceVariant,
    accent = primary,
    accentSecondary = secondary,
    border = outlineVariant,
    success = Color(0xFFA6E3A1),
    warning = Color(0xFFF9E2AF),
    danger = error,
    isDark = isDark,
)

fun VesselPalette.toColorScheme(): ColorScheme {
    val onAccent = if (isDark) Color(0xFF11111B) else Color.White
    return if (isDark) {
        darkColorScheme(
            primary = accent,
            onPrimary = onAccent,
            secondary = accentSecondary,
            onSecondary = onAccent,
            background = bgBase,
            onBackground = textPrimary,
            surface = bgSurface,
            onSurface = textPrimary,
            surfaceVariant = bgElevated,
            onSurfaceVariant = textMuted,
            outline = border,
            error = danger,
        )
    } else {
        lightColorScheme(
            primary = accent,
            onPrimary = onAccent,
            secondary = accentSecondary,
            onSecondary = onAccent,
            background = bgBase,
            onBackground = textPrimary,
            surface = bgSurface,
            onSurface = textPrimary,
            surfaceVariant = bgElevated,
            onSurfaceVariant = textMuted,
            outline = border,
            error = danger,
        )
    }
}

@Composable
fun VesselTheme(
    themeSpec: ThemeSpec = ThemeSpec(),
    content: @Composable () -> Unit,
) {
    val context = LocalContext.current
    val systemDark = isSystemInDarkTheme()

    val (colorScheme, palette) = if (themeSpec.id == DYNAMIC_MATERIAL_THEME_ID) {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.S) {
            val dynamicScheme = if (systemDark) {
                dynamicDarkColorScheme(context)
            } else {
                dynamicLightColorScheme(context)
            }
            dynamicScheme to dynamicScheme.toVesselPalette(systemDark)
        } else {
            val fallbackSpec = if (systemDark) {
                ThemeSpec()
            } else {
                ThemeSpec(
                    id = "catppuccin-latte",
                    name = "Catppuccin Latte",
                    isDark = false,
                    colors = ThemeColors(
                        bgBase = "#eff1f5",
                        bgSurface = "#e6e9ef",
                        bgElevated = "#ccd0da",
                        textPrimary = "#4c4f69",
                        textMuted = "#6c6f85",
                        accent = "#8839ef",
                        accentSecondary = "#1e66f5",
                        border = "#bcc0cc",
                    ),
                )
            }
            val p = fallbackSpec.toPalette()
            p.toColorScheme() to p
        }
    } else {
        val p = themeSpec.toPalette()
        p.toColorScheme() to p
    }

    CompositionLocalProvider(LocalVesselPalette provides palette) {
        MaterialTheme(
            colorScheme = colorScheme,
            content = content,
        )
    }
}
