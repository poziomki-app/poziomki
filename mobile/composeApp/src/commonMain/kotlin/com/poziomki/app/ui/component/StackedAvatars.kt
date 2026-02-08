package com.poziomki.app.ui.component

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Person
import androidx.compose.material3.Icon
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import coil3.compose.AsyncImage
import com.poziomki.app.ui.theme.Border
import com.poziomki.app.ui.theme.SurfaceElevated
import com.poziomki.app.ui.theme.TextMuted
import com.poziomki.app.util.resolveImageUrl

@Composable
fun StackedAvatars(
    imageUrls: List<String?>,
    modifier: Modifier = Modifier,
    avatarSize: Dp = 28.dp,
    overlapOffset: Dp = (-8).dp,
    maxAvatars: Int = 5,
) {
    val visible = imageUrls.take(maxAvatars)
    Box(modifier = modifier) {
        visible.forEachIndexed { index, url ->
            val xOffset = (avatarSize + overlapOffset) * index
            if (url != null) {
                AsyncImage(
                    model = resolveImageUrl(url),
                    contentDescription = null,
                    modifier =
                        Modifier
                            .offset(x = xOffset)
                            .size(avatarSize)
                            .clip(CircleShape)
                            .border(1.5.dp, Border, CircleShape),
                    contentScale = ContentScale.Crop,
                )
            } else {
                Box(
                    modifier =
                        Modifier
                            .offset(x = xOffset)
                            .size(avatarSize)
                            .clip(CircleShape)
                            .background(SurfaceElevated)
                            .border(1.5.dp, Border, CircleShape),
                    contentAlignment = Alignment.Center,
                ) {
                    Icon(
                        Icons.Filled.Person,
                        contentDescription = null,
                        modifier = Modifier.size(avatarSize * 0.6f),
                        tint = TextMuted,
                    )
                }
            }
        }
    }
}
