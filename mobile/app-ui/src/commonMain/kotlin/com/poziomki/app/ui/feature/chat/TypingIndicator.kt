package com.poziomki.app.ui.feature.chat

import androidx.compose.animation.core.RepeatMode
import androidx.compose.animation.core.animateFloat
import androidx.compose.animation.core.infiniteRepeatable
import androidx.compose.animation.core.rememberInfiniteTransition
import androidx.compose.animation.core.tween
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Surface
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.unit.dp
import com.poziomki.app.ui.designsystem.components.UserAvatar
import com.poziomki.app.ui.designsystem.theme.ChatBubble
import com.poziomki.app.ui.designsystem.theme.Primary
import com.poziomki.app.ui.designsystem.theme.TextSecondary

private val DotSize = 8.dp
private val AvatarSize = 28.dp
private val AvatarSpacing = 6.dp

@Composable
fun TypingIndicator(
    avatarUrl: String?,
    displayName: String?,
    showAvatar: Boolean,
    modifier: Modifier = Modifier,
) {
    val transition = rememberInfiniteTransition()
    val dot1 by transition.animateFloat(
        initialValue = 0.3f,
        targetValue = 1.0f,
        animationSpec = infiniteRepeatable(
            animation = tween(durationMillis = 600),
            repeatMode = RepeatMode.Reverse,
        ),
        label = "dot1",
    )
    val dot2 by transition.animateFloat(
        initialValue = 0.3f,
        targetValue = 1.0f,
        animationSpec = infiniteRepeatable(
            animation = tween(durationMillis = 600, delayMillis = 150),
            repeatMode = RepeatMode.Reverse,
        ),
        label = "dot2",
    )
    val dot3 by transition.animateFloat(
        initialValue = 0.3f,
        targetValue = 1.0f,
        animationSpec = infiniteRepeatable(
            animation = tween(durationMillis = 600, delayMillis = 300),
            repeatMode = RepeatMode.Reverse,
        ),
        label = "dot3",
    )

    Row(
        verticalAlignment = Alignment.CenterVertically,
        modifier = modifier,
    ) {
        if (showAvatar) {
            UserAvatar(
                picture = avatarUrl,
                displayName = displayName,
                size = AvatarSize,
                backgroundColor = Primary.copy(alpha = 0.2f),
                iconTint = Primary,
            )
            Spacer(modifier = Modifier.width(AvatarSpacing))
        }

        Surface(
            shape = RoundedCornerShape(18.dp),
            color = ChatBubble,
        ) {
            Row(
                modifier = Modifier.padding(horizontal = 14.dp, vertical = 10.dp),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Dot(alpha = dot1)
                Spacer(modifier = Modifier.width(5.dp))
                Dot(alpha = dot2)
                Spacer(modifier = Modifier.width(5.dp))
                Dot(alpha = dot3)
            }
        }
    }
}

@Composable
private fun Dot(alpha: Float) {
    Box(
        modifier = Modifier
            .size(DotSize)
            .alpha(alpha)
            .background(color = TextSecondary, shape = CircleShape),
    )
}
