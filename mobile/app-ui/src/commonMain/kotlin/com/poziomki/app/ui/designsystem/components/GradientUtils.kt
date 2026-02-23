package com.poziomki.app.ui.designsystem.components

import androidx.compose.ui.graphics.Color
import com.poziomki.app.ui.designsystem.theme.Background

fun parseHexColor(hex: String?): Color? {
    if (hex.isNullOrBlank()) return null
    val clean = hex.trimStart('#')
    if (clean.length != 6) return null
    return runCatching { Color(("FF$clean").toLong(16).toInt()) }.getOrNull()
}

fun blendWithBackground(
    color: Color,
    amount: Float,
): Color {
    val bg = Background
    return Color(
        red = bg.red * (1f - amount) + color.red * amount,
        green = bg.green * (1f - amount) + color.green * amount,
        blue = bg.blue * (1f - amount) + color.blue * amount,
        alpha = 1f,
    )
}
