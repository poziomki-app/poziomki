package com.poziomki.app.ui.component

import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material3.Icon
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.TextUnit
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import coil3.compose.AsyncImage
import com.adamglin.PhosphorIcons
import com.adamglin.phosphoricons.Regular
import com.adamglin.phosphoricons.regular.User
import com.poziomki.app.ui.theme.Border
import com.poziomki.app.ui.theme.TextMuted
import com.poziomki.app.util.isImageUrl
import com.poziomki.app.util.resolveImageUrl

/**
 * Returns true when the string looks like an emoji (no ASCII letters or digits).
 */
private fun isEmoji(value: String): Boolean =
    value.length <= 6 && value.none { it in 'A'..'Z' || it in 'a'..'z' || it in '0'..'9' || it == '.' }

@Composable
fun UserAvatar(
    picture: String?,
    displayName: String?,
    modifier: Modifier = Modifier,
    size: Dp = 52.dp,
    backgroundColor: Color = Border,
    iconTint: Color = TextMuted,
) {
    val emojiSize: TextUnit = (size.value * 0.45f).sp
    val iconSize: Dp = size * 0.5f

    Surface(
        modifier = modifier.size(size),
        shape = CircleShape,
        color = backgroundColor,
    ) {
        when {
            picture != null && isImageUrl(picture) -> {
                AsyncImage(
                    model = resolveImageUrl(picture),
                    contentDescription = displayName,
                    modifier =
                        Modifier
                            .size(size)
                            .clip(CircleShape),
                    contentScale = ContentScale.Crop,
                )
            }

            picture != null && isEmoji(picture) -> {
                Box(
                    modifier = Modifier.size(size),
                    contentAlignment = Alignment.Center,
                ) {
                    Text(text = picture, fontSize = emojiSize)
                }
            }

            else -> {
                Box(
                    modifier = Modifier.size(size),
                    contentAlignment = Alignment.Center,
                ) {
                    Icon(
                        PhosphorIcons.Regular.User,
                        contentDescription = displayName,
                        modifier = Modifier.size(iconSize),
                        tint = iconTint,
                    )
                }
            }
        }
    }
}
