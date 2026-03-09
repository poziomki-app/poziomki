@file:Suppress("MagicNumber")

package com.poziomki.app.ui.designsystem

import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.PathFillType
import androidx.compose.ui.graphics.SolidColor
import androidx.compose.ui.graphics.StrokeCap
import androidx.compose.ui.graphics.StrokeJoin
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.graphics.vector.path
import androidx.compose.ui.unit.dp

object StrawberryIcon {

    val Filled: ImageVector by lazy {
        ImageVector.Builder(
            name = "StrawberryFilled",
            defaultWidth = 24.dp,
            defaultHeight = 24.dp,
            viewportWidth = 24f,
            viewportHeight = 24f,
        ).apply {
            path(
                fill = SolidColor(Color.Black),
                pathFillType = PathFillType.EvenOdd,
            ) {
                // Body
                moveTo(9.36f, 2.03f)
                curveTo(6.94f, 2.84f, 5.22f, 4.58f, 3.68f, 7.78f)
                curveTo(2.28f, 10.67f, 1.53f, 13.67f, 1.51f, 16.50f)
                curveTo(1.50f, 19.81f, 2.22f, 21.16f, 4.47f, 22.04f)
                curveTo(5.62f, 22.50f, 8.57f, 22.50f, 10.63f, 22.04f)
                curveTo(14.83f, 21.11f, 18.50f, 19.22f, 20.32f, 17.08f)
                curveTo(22.01f, 15.04f, 22.50f, 12.21f, 21.59f, 9.61f)
                curveTo(21.12f, 8.28f, 19.79f, 6.29f, 18.61f, 5.13f)
                curveTo(17.61f, 4.15f, 15.58f, 2.73f, 14.61f, 2.33f)
                curveTo(12.91f, 1.61f, 10.92f, 1.50f, 9.36f, 2.03f)
                close()
                // Seed dot 1 (upper-right)
                moveTo(10.6f, 5.1f)
                curveTo(10.6f, 5.98f, 9.88f, 6.7f, 9.0f, 6.7f)
                curveTo(8.12f, 6.7f, 7.4f, 5.98f, 7.4f, 5.1f)
                curveTo(7.4f, 4.22f, 8.12f, 3.5f, 9.0f, 3.5f)
                curveTo(9.88f, 3.5f, 10.6f, 4.22f, 10.6f, 5.1f)
                close()
                // Seed dot 2 (lower-left)
                moveTo(7.8f, 9.85f)
                curveTo(7.8f, 10.73f, 7.08f, 11.45f, 6.2f, 11.45f)
                curveTo(5.32f, 11.45f, 4.6f, 10.73f, 4.6f, 9.85f)
                curveTo(4.6f, 8.97f, 5.32f, 8.25f, 6.2f, 8.25f)
                curveTo(7.08f, 8.25f, 7.8f, 8.97f, 7.8f, 9.85f)
                close()
            }
        }.build()
    }

    val Outline: ImageVector by lazy {
        ImageVector.Builder(
            name = "StrawberryOutline",
            defaultWidth = 24.dp,
            defaultHeight = 24.dp,
            viewportWidth = 24f,
            viewportHeight = 24f,
        ).apply {
            // Body outline only
            path(
                fill = null,
                stroke = SolidColor(Color.Black),
                strokeLineWidth = 1.5f,
                strokeLineCap = StrokeCap.Round,
                strokeLineJoin = StrokeJoin.Round,
            ) {
                moveTo(9.36f, 2.03f)
                curveTo(6.94f, 2.84f, 5.22f, 4.58f, 3.68f, 7.78f)
                curveTo(2.28f, 10.67f, 1.53f, 13.67f, 1.51f, 16.50f)
                curveTo(1.50f, 19.81f, 2.22f, 21.16f, 4.47f, 22.04f)
                curveTo(5.62f, 22.50f, 8.57f, 22.50f, 10.63f, 22.04f)
                curveTo(14.83f, 21.11f, 18.50f, 19.22f, 20.32f, 17.08f)
                curveTo(22.01f, 15.04f, 22.50f, 12.21f, 21.59f, 9.61f)
                curveTo(21.12f, 8.28f, 19.79f, 6.29f, 18.61f, 5.13f)
                curveTo(17.61f, 4.15f, 15.58f, 2.73f, 14.61f, 2.33f)
                curveTo(12.91f, 1.61f, 10.92f, 1.50f, 9.36f, 2.03f)
                close()
            }
            // Seed dot 1 (upper-right) - filled
            path(fill = SolidColor(Color.Black)) {
                moveTo(10.6f, 5.1f)
                curveTo(10.6f, 5.98f, 9.88f, 6.7f, 9.0f, 6.7f)
                curveTo(8.12f, 6.7f, 7.4f, 5.98f, 7.4f, 5.1f)
                curveTo(7.4f, 4.22f, 8.12f, 3.5f, 9.0f, 3.5f)
                curveTo(9.88f, 3.5f, 10.6f, 4.22f, 10.6f, 5.1f)
                close()
            }
            // Seed dot 2 (lower-left) - filled
            path(fill = SolidColor(Color.Black)) {
                moveTo(7.8f, 9.85f)
                curveTo(7.8f, 10.73f, 7.08f, 11.45f, 6.2f, 11.45f)
                curveTo(5.32f, 11.45f, 4.6f, 10.73f, 4.6f, 9.85f)
                curveTo(4.6f, 8.97f, 5.32f, 8.25f, 6.2f, 8.25f)
                curveTo(7.08f, 8.25f, 7.8f, 8.97f, 7.8f, 9.85f)
                close()
            }
        }.build()
    }
}
