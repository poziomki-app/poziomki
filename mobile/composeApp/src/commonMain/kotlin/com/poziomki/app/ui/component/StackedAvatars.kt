package com.poziomki.app.ui.component

import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import com.poziomki.app.ui.theme.Border

@Composable
fun StackedAvatars(
    imageUrls: List<String?>,
    modifier: Modifier = Modifier,
    avatarSize: Dp = 28.dp,
    overlapOffset: Dp = (-8).dp,
    maxAvatars: Int = 5,
) {
    val visible = imageUrls.take(maxAvatars)
    val step = avatarSize + overlapOffset
    val totalWidth =
        when {
            visible.isEmpty() -> 0.dp
            else -> avatarSize + (step * (visible.size - 1))
        }

    Box(
        modifier =
            modifier
                .size(width = totalWidth, height = avatarSize),
    ) {
        visible.forEachIndexed { index, url ->
            val xOffset = step * index
            UserAvatar(
                picture = url,
                displayName = null,
                size = avatarSize,
                modifier =
                    Modifier
                        .offset(x = xOffset)
                        .border(1.5.dp, Border, CircleShape),
            )
        }
    }
}
