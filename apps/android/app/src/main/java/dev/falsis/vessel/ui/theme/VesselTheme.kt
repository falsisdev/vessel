package dev.falsis.vessel.ui.theme

import android.app.Activity
import android.app.UiModeManager
import android.content.Context
import android.content.pm.PackageManager
import android.content.res.Configuration
import android.os.Build
import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.ColorScheme
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Shapes
import androidx.compose.material3.Typography
import androidx.compose.material3.darkColorScheme
import androidx.compose.material3.dynamicDarkColorScheme
import androidx.compose.material3.dynamicLightColorScheme
import androidx.compose.material3.lightColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.Immutable
import androidx.compose.runtime.SideEffect
import androidx.compose.runtime.remember
import androidx.compose.runtime.staticCompositionLocalOf
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalConfiguration
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalView
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.core.view.WindowCompat
import dev.falsis.vessel.data.ShellPreferences
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

@Immutable
data class VesselUiTokens(
    val scale: Float = 1f,
    val densityKey: String = "comfortable",
    val cornerRadius: Dp = 11.dp,
    val buttonHeight: Dp = 40.dp,
    val compactButtonHeight: Dp = 35.dp,
    val textButtonHeight: Dp = 34.dp,
    val iconButtonSize: Dp = 38.dp,
    val smallIconButtonSize: Dp = 32.dp,
    val chipHeight: Dp = 33.dp,
    val searchFieldHeight: Dp = 44.dp,
    val badgeHorizontalPadding: Dp = 7.dp,
    val badgeVerticalPadding: Dp = 2.5.dp,
    val badgeIconSize: Dp = 12.dp,
)

private val DefaultPalette = ThemeSpec().toPalette()
private val DefaultUiTokens = VesselUiTokens()

val LocalVesselPalette = staticCompositionLocalOf { DefaultPalette }
val LocalVesselUiTokens = staticCompositionLocalOf { DefaultUiTokens }
val LocalVesselFloatingNavInset = staticCompositionLocalOf<Dp> { 0.dp }
val LocalIsTvDevice = staticCompositionLocalOf { false }
val LocalIsTvLayout = staticCompositionLocalOf { false }

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
    shell: ShellPreferences = ShellPreferences(),
    content: @Composable () -> Unit,
) {
    val context = LocalContext.current
    val configuration = LocalConfiguration.current
    val isHardwareTv = remember(context) {
        val uiModeManager = context.getSystemService(Context.UI_MODE_SERVICE) as? UiModeManager
        uiModeManager?.currentModeType == Configuration.UI_MODE_TYPE_TELEVISION ||
            context.packageManager.hasSystemFeature(PackageManager.FEATURE_TELEVISION) ||
            context.packageManager.hasSystemFeature(PackageManager.FEATURE_LEANBACK)
    }
    val isHardwareTablet = configuration.smallestScreenWidthDp >= 600
    val (isTvDevice, isTvLayout) = when (shell.androidDeviceMode) {
        "tv" -> true to true
        "tablet" -> false to true
        "phone" -> false to false
        else -> isHardwareTv to (isHardwareTv || isHardwareTablet)
    }
    val systemDark = isSystemInDarkTheme()
    val fontScale = (shell.fontScalePercent.coerceIn(85, 130) / 100f)
    val densityScale = when (shell.density) {
        "compact" -> 0.90f
        "spacious" -> 1.12f
        else -> 1.00f
    }
    val sizeFactor = (fontScale * densityScale).coerceIn(0.82f, 1.38f)
    val baseCorner = when (shell.cornerStyle) {
        "sharp" -> 4.dp
        "pill" -> 18.dp
        else -> 11.dp
    }
    val uiTokens = VesselUiTokens(
        scale = sizeFactor,
        densityKey = shell.density,
        cornerRadius = baseCorner,
        buttonHeight = (40f * sizeFactor).dp,
        compactButtonHeight = (35f * sizeFactor).dp,
        textButtonHeight = (34f * sizeFactor).dp,
        iconButtonSize = (38f * sizeFactor).dp,
        smallIconButtonSize = (32f * sizeFactor).dp,
        chipHeight = (33f * sizeFactor).dp,
        searchFieldHeight = (44f * sizeFactor).dp,
        badgeHorizontalPadding = (7f * sizeFactor).dp,
        badgeVerticalPadding = (2.5f * sizeFactor).dp,
        badgeIconSize = (12f * sizeFactor).dp,
    )

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

    val view = LocalView.current
    if (!view.isInEditMode) {
        SideEffect {
            val window = (view.context as? Activity)?.window
            if (window != null) {
                val controller = WindowCompat.getInsetsController(window, view)
                controller.isAppearanceLightStatusBars = !palette.isDark
                controller.isAppearanceLightNavigationBars = !palette.isDark
            }
        }
    }

    val VesselCompactTypography = Typography(
        headlineMedium = TextStyle(fontSize = (24f * fontScale).sp, lineHeight = (29f * fontScale).sp, fontWeight = FontWeight.ExtraBold),
        headlineSmall = TextStyle(fontSize = (20f * fontScale).sp, lineHeight = (25f * fontScale).sp, fontWeight = FontWeight.Bold),
        titleLarge = TextStyle(fontSize = (18f * fontScale).sp, lineHeight = (23f * fontScale).sp, fontWeight = FontWeight.Bold),
        titleMedium = TextStyle(fontSize = (15.5f * fontScale).sp, lineHeight = (21f * fontScale).sp, fontWeight = FontWeight.SemiBold),
        titleSmall = TextStyle(fontSize = (14f * fontScale).sp, lineHeight = (19f * fontScale).sp, fontWeight = FontWeight.SemiBold),
        bodyLarge = TextStyle(fontSize = (15f * fontScale).sp, lineHeight = (21f * fontScale).sp),
        bodyMedium = TextStyle(fontSize = (14f * fontScale).sp, lineHeight = (19.5f * fontScale).sp),
        bodySmall = TextStyle(fontSize = (12.5f * fontScale).sp, lineHeight = (17f * fontScale).sp),
        labelLarge = TextStyle(fontSize = (13.5f * fontScale).sp, lineHeight = (17.5f * fontScale).sp, fontWeight = FontWeight.SemiBold),
        labelMedium = TextStyle(fontSize = (12f * fontScale).sp, lineHeight = (16f * fontScale).sp, fontWeight = FontWeight.Medium),
        labelSmall = TextStyle(fontSize = (11f * fontScale).sp, lineHeight = (14.5f * fontScale).sp, fontWeight = FontWeight.Medium),
    )

    val VesselShapes = Shapes(
        extraSmall = RoundedCornerShape(if (shell.cornerStyle == "sharp") 3.dp else 6.dp),
        small = RoundedCornerShape(if (shell.cornerStyle == "sharp") 4.dp else 9.dp),
        medium = RoundedCornerShape(baseCorner),
        large = RoundedCornerShape(if (shell.cornerStyle == "sharp") 6.dp else if (shell.cornerStyle == "pill") 22.dp else 16.dp),
        extraLarge = RoundedCornerShape(if (shell.cornerStyle == "sharp") 8.dp else 24.dp),
    )

    CompositionLocalProvider(
        LocalVesselPalette provides palette,
        LocalVesselUiTokens provides uiTokens,
        LocalIsTvDevice provides isTvDevice,
        LocalIsTvLayout provides isTvLayout,
    ) {
        MaterialTheme(
            colorScheme = colorScheme,
            typography = VesselCompactTypography,
            shapes = VesselShapes,
            content = content,
        )
    }
}
