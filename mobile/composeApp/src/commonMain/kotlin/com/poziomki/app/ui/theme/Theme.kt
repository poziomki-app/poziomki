package com.poziomki.app.ui.theme

import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.darkColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider

private val DarkColorScheme =
    darkColorScheme(
        primary = Primary,
        onPrimary = Black,
        primaryContainer = PrimaryLight,
        onPrimaryContainer = PrimaryMuted,
        secondary = Secondary,
        onSecondary = Black,
        secondaryContainer = SecondaryLight,
        onSecondaryContainer = SecondaryMuted,
        tertiary = Accent,
        onTertiary = Black,
        background = Background,
        onBackground = TextPrimary,
        surface = Surface,
        onSurface = TextPrimary,
        surfaceVariant = SurfaceElevated,
        onSurfaceVariant = TextSecondary,
        error = Error,
        onError = White,
        errorContainer = ErrorLight,
        onErrorContainer = Error,
        outline = Border,
        outlineVariant = BorderLight,
    )

@Composable
fun PoziomkiTheme(content: @Composable () -> Unit) {
    val typography = poziomkiTypography()

    CompositionLocalProvider(
        LocalSpacing provides Spacing(),
        LocalRadius provides Radius(),
        LocalComponentSizes provides ComponentSizes(),
    ) {
        MaterialTheme(
            colorScheme = DarkColorScheme,
            typography = typography,
            content = content,
        )
    }
}

object PoziomkiTheme {
    val spacing: Spacing
        @Composable get() = LocalSpacing.current
    val radius: Radius
        @Composable get() = LocalRadius.current
    val componentSizes: ComponentSizes
        @Composable get() = LocalComponentSizes.current
}
