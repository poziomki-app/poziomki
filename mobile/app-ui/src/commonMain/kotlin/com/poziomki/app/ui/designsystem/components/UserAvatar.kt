package com.poziomki.app.ui.designsystem.components

import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material3.Icon
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
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
import com.poziomki.app.ui.designsystem.theme.Border
import com.poziomki.app.ui.designsystem.theme.TextMuted
import com.poziomki.app.ui.shared.isImageUrl
import com.poziomki.app.ui.shared.resolveImageUrl
import kotlinx.coroutines.delay

/**
 * Returns true when the string looks like an emoji (no ASCII letters or digits).
 */
private fun isEmoji(value: String): Boolean =
    value.length <= 6 && value.none { it in 'A'..'Z' || it in 'a'..'z' || it in '0'..'9' || it == '.' }

private const val AVATAR_NULL_FALLBACK_DELAY_MS = 250L

@Composable
fun UserAvatar(
    picture: String?,
    fallbackPicture: String? = null,
    displayName: String?,
    modifier: Modifier = Modifier,
    size: Dp = 52.dp,
    backgroundColor: Color = Border,
    iconTint: Color = TextMuted,
) {
    val emojiSize: TextUnit = (size.value * 0.45f).sp
    val iconSize: Dp = size * 0.5f
    val primaryImage = picture?.takeIf(::isImageUrl)
    val secondaryImage =
        fallbackPicture
            ?.takeIf(::isImageUrl)
            ?.takeUnless { it == primaryImage }
    var activeImage by remember(displayName) { mutableStateOf(primaryImage ?: secondaryImage) }
    val preferredImage = primaryImage ?: secondaryImage
    LaunchedEffect(preferredImage, displayName) {
        if (preferredImage != null) {
            activeImage = preferredImage
        } else {
            // Avoid photo -> placeholder -> photo flashes when refresh pipelines emit
            // a transient null avatar before the same URL arrives again.
            delay(AVATAR_NULL_FALLBACK_DELAY_MS)
            if (primaryImage == null && secondaryImage == null) {
                activeImage = null
            }
        }
    }
    val resolvedActiveImage = remember(activeImage) { activeImage?.let(::resolveImageUrl) }

    Surface(
        modifier = modifier.size(size),
        shape = CircleShape,
        color = backgroundColor,
    ) {
        when {
            activeImage != null -> {
                AsyncImage(
                    model = resolvedActiveImage,
                    contentDescription = displayName,
                    modifier =
                        Modifier
                            .size(size)
                            .clip(CircleShape),
                    contentScale = ContentScale.Crop,
                    onError = {
                        if (activeImage == primaryImage && secondaryImage != null) {
                            activeImage = secondaryImage
                        }
                    },
                    error = null,
                    placeholder = null,
                    fallback = null,
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
                FallbackUserIcon(iconSize, iconTint)
            }
        }
    }
}

@Composable
private fun FallbackUserIcon(
    iconSize: Dp,
    iconTint: Color,
) {
    Box(
        contentAlignment = Alignment.Center,
    ) {
        Icon(
            PhosphorIcons.Regular.User,
            contentDescription = null,
            modifier = Modifier.size(iconSize),
            tint = iconTint,
        )
    }
}
