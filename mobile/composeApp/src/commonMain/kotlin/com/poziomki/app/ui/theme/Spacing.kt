package com.poziomki.app.ui.theme

import androidx.compose.runtime.Immutable
import androidx.compose.runtime.staticCompositionLocalOf
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp

@Immutable
data class Spacing(
    val xs: Dp = 4.dp,
    val sm: Dp = 8.dp,
    val md: Dp = 16.dp,
    val lg: Dp = 24.dp,
    val xl: Dp = 32.dp,
    val xxl: Dp = 48.dp,
    val xxxl: Dp = 64.dp,
)

@Immutable
data class Radius(
    val xs: Dp = 4.dp,
    val sm: Dp = 8.dp,
    val md: Dp = 12.dp,
    val lg: Dp = 16.dp,
    val xl: Dp = 24.dp,
    val full: Dp = 9999.dp,
)

@Immutable
data class ComponentSizes(
    val buttonHeight: Dp = 52.dp,
    val inputHeight: Dp = 56.dp,
    val cardPadding: Dp = 16.dp,
    val cardRadius: Dp = 16.dp,
    val touchTargetMin: Dp = 44.dp,
)

val LocalSpacing = staticCompositionLocalOf { Spacing() }
val LocalRadius = staticCompositionLocalOf { Radius() }
val LocalComponentSizes = staticCompositionLocalOf { ComponentSizes() }
