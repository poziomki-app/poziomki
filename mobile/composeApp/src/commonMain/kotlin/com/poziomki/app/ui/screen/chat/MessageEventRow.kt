package com.poziomki.app.ui.screen.chat

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.combinedClickable
import androidx.compose.foundation.gestures.detectHorizontalDragGestures
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.IntrinsicSize
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.Reply
import androidx.compose.material.icons.filled.Check
import androidx.compose.material.icons.filled.CheckCircle
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.IntOffset
import androidx.compose.ui.unit.dp
import com.poziomki.app.chat.matrix.api.MatrixReplyDetails
import com.poziomki.app.chat.matrix.api.MatrixTimelineItem
import com.poziomki.app.ui.component.UserAvatar
import com.poziomki.app.ui.theme.Background
import com.poziomki.app.ui.theme.Border
import com.poziomki.app.ui.theme.Primary
import com.poziomki.app.ui.theme.TextPrimary
import com.poziomki.app.ui.theme.TextSecondary
import kotlinx.datetime.Instant
import kotlinx.datetime.TimeZone
import kotlinx.datetime.toLocalDateTime
import kotlin.math.roundToInt
import com.poziomki.app.ui.theme.Surface as SurfaceColor

private val AvatarSize = 44.dp
private val AvatarSpacing = 8.dp

@Composable
internal fun MessageEventRow(
    event: MatrixTimelineItem.Event,
    groupedWithPrevious: Boolean,
    onToggleReaction: (String) -> Unit,
    onReactionsClick: () -> Unit,
    onFocusOnReply: () -> Unit,
    onSenderClick: () -> Unit,
    onActionsLongPress: () -> Unit,
    onSwipeReply: () -> Unit,
    avatarOverride: String? = null,
) {
    val horizontalAlignment = if (event.isMine) Alignment.End else Alignment.Start
    val bubbleColor = if (event.isMine) Primary.copy(alpha = 0.68f) else SurfaceColor
    val canSwipeReply = event.canReply && event.eventId != null
    val density = LocalDensity.current
    val maxSwipePx = with(density) { 84.dp.toPx() }
    val triggerSwipePx = with(density) { 52.dp.toPx() }
    var dragOffsetPx by remember(event.eventOrTransactionId) { mutableFloatStateOf(0f) }
    val swipeProgress = (dragOffsetPx / triggerSwipePx).coerceIn(0f, 1f)
    val bubbleShape =
        if (event.isMine) {
            RoundedCornerShape(
                topStart = 18.dp,
                topEnd = if (groupedWithPrevious) 18.dp else 8.dp,
                bottomEnd = 18.dp,
                bottomStart = 18.dp,
            )
        } else {
            RoundedCornerShape(
                topStart = if (groupedWithPrevious) 18.dp else 8.dp,
                topEnd = 18.dp,
                bottomEnd = 18.dp,
                bottomStart = 18.dp,
            )
        }

    val showAvatar = !event.isMine && !groupedWithPrevious

    Column(
        modifier =
            Modifier
                .fillMaxWidth()
                .padding(top = if (groupedWithPrevious) 2.dp else 10.dp),
        horizontalAlignment = horizontalAlignment,
    ) {
        if (!event.isMine && !groupedWithPrevious) {
            Text(
                text = event.senderDisplayName ?: event.senderId,
                style = MaterialTheme.typography.labelSmall,
                color = TextSecondary,
                maxLines = 1,
                overflow = TextOverflow.Ellipsis,
                modifier =
                    Modifier
                        .clickable(onClick = onSenderClick)
                        .padding(
                            start = if (!event.isMine) AvatarSize + AvatarSpacing + 10.dp else 10.dp,
                            end = 10.dp,
                            top = 2.dp,
                            bottom = 2.dp,
                        ),
            )
        }

        Box(modifier = Modifier.fillMaxWidth()) {
            if (canSwipeReply) {
                Surface(
                    shape = CircleShape,
                    color = Primary.copy(alpha = 0.2f),
                    modifier =
                        Modifier
                            .align(Alignment.CenterStart)
                            .padding(start = 8.dp)
                            .size(30.dp)
                            .graphicsLayer {
                                alpha = swipeProgress
                            },
                ) {
                    Box(contentAlignment = Alignment.Center) {
                        Icon(
                            imageVector = Icons.AutoMirrored.Filled.Reply,
                            contentDescription = null,
                            tint = Primary,
                            modifier = Modifier.size(16.dp),
                        )
                    }
                }
            }

            BoxWithConstraints(
                modifier =
                    Modifier
                        .fillMaxWidth()
                        .offset { IntOffset(dragOffsetPx.roundToInt(), 0) }
                        .pointerInput(canSwipeReply) {
                            if (!canSwipeReply) return@pointerInput
                            detectHorizontalDragGestures(
                                onHorizontalDrag = { change, dragAmount ->
                                    val nextOffset = (dragOffsetPx + dragAmount).coerceIn(0f, maxSwipePx)
                                    if (nextOffset != dragOffsetPx) {
                                        dragOffsetPx = nextOffset
                                        change.consume()
                                    }
                                },
                                onDragEnd = {
                                    if (dragOffsetPx >= triggerSwipePx) {
                                        onSwipeReply()
                                    }
                                    dragOffsetPx = 0f
                                },
                                onDragCancel = {
                                    dragOffsetPx = 0f
                                },
                            )
                        },
            ) {
                val maxBubbleWidth = maxWidth * 0.86f

                Column(
                    horizontalAlignment = horizontalAlignment,
                    modifier = Modifier.fillMaxWidth(),
                ) {
                    if (event.isMine) {
                        // Mine: bubble + reactions, right-aligned
                        Column(horizontalAlignment = Alignment.End) {
                            Surface(
                                color = bubbleColor,
                                shape = bubbleShape,
                                modifier =
                                    Modifier
                                        .widthIn(max = maxBubbleWidth)
                                        .combinedClickable(
                                            onClick = {},
                                            onLongClick = onActionsLongPress,
                                        ),
                            ) {
                                BubbleContent(event = event, onFocusOnReply = onFocusOnReply)
                            }
                            if (event.reactions.isNotEmpty()) {
                                Row(
                                    modifier =
                                        Modifier
                                            .align(Alignment.Start)
                                            .offset(y = (-6).dp)
                                            .padding(start = 4.dp),
                                    horizontalArrangement = Arrangement.spacedBy(4.dp),
                                ) {
                                    event.reactions.forEach { reaction ->
                                        Surface(
                                            shape = RoundedCornerShape(12.dp),
                                            color = SurfaceColor,
                                            modifier = Modifier.clickable { onReactionsClick() },
                                        ) {
                                            Text(
                                                text = reaction.emoji,
                                                style = MaterialTheme.typography.bodyMedium,
                                                modifier = Modifier.padding(horizontal = 6.dp, vertical = 3.dp),
                                            )
                                        }
                                    }
                                }
                            }
                        }
                    } else {
                        // Not mine: avatar + bubble row
                        Row(verticalAlignment = Alignment.Bottom) {
                            if (showAvatar) {
                                UserAvatar(
                                    picture = avatarOverride,
                                    fallbackPicture = event.senderAvatarUrl,
                                    displayName = event.senderDisplayName,
                                    size = AvatarSize,
                                    modifier = Modifier.clickable(onClick = onSenderClick),
                                )
                            } else {
                                Spacer(modifier = Modifier.width(AvatarSize))
                            }
                            Spacer(modifier = Modifier.width(AvatarSpacing))
                            Column {
                                Surface(
                                    color = bubbleColor,
                                    shape = bubbleShape,
                                    modifier =
                                        Modifier
                                            .widthIn(max = maxBubbleWidth - AvatarSize - AvatarSpacing)
                                            .combinedClickable(
                                                onClick = {},
                                                onLongClick = onActionsLongPress,
                                            ),
                                ) {
                                    BubbleContent(event = event, onFocusOnReply = onFocusOnReply)
                                }
                                if (event.reactions.isNotEmpty()) {
                                    Row(
                                        modifier =
                                            Modifier
                                                .offset(y = (-6).dp)
                                                .padding(start = 4.dp),
                                        horizontalArrangement = Arrangement.spacedBy(4.dp),
                                    ) {
                                        event.reactions.forEach { reaction ->
                                            Surface(
                                                shape = RoundedCornerShape(12.dp),
                                                color = SurfaceColor,
                                                modifier = Modifier.clickable { onReactionsClick() },
                                            ) {
                                                Text(
                                                    text = reaction.emoji,
                                                    style = MaterialTheme.typography.bodyMedium,
                                                    modifier = Modifier.padding(horizontal = 6.dp, vertical = 3.dp),
                                                )
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

@Composable
private fun BubbleContent(
    event: MatrixTimelineItem.Event,
    onFocusOnReply: () -> Unit,
) {
    Column(
        modifier =
            Modifier
                .width(IntrinsicSize.Max)
                .padding(horizontal = 12.dp, vertical = 8.dp),
    ) {
        event.inReplyTo?.let { reply ->
            ReplyReference(
                reply = reply,
                onClick = onFocusOnReply,
                isMine = event.isMine,
            )
            Spacer(modifier = Modifier.height(6.dp))
        }
        Text(
            text = event.body,
            style = MaterialTheme.typography.bodyLarge,
            color = TextPrimary,
        )
        Row(
            modifier = Modifier.align(Alignment.End).padding(top = 4.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Text(
                text = formatTime(event.timestampMillis),
                style = MaterialTheme.typography.labelSmall,
                color = TextSecondary,
            )
            if (event.isMine) {
                Spacer(modifier = Modifier.width(4.dp))
                Icon(
                    imageVector =
                        if (event.readByCount > 0) {
                            Icons.Filled.CheckCircle
                        } else {
                            Icons.Filled.Check
                        },
                    contentDescription = null,
                    tint = if (event.readByCount > 0) Primary else TextSecondary,
                    modifier = Modifier.size(14.dp),
                )
            }
        }
    }
}

@Composable
private fun ReplyReference(
    reply: MatrixReplyDetails,
    onClick: () -> Unit,
    isMine: Boolean,
) {
    Surface(
        color = Background.copy(alpha = if (isMine) 0.22f else 0.55f),
        shape = RoundedCornerShape(10.dp),
        modifier = Modifier.fillMaxWidth().clickable(onClick = onClick),
    ) {
        Row(modifier = Modifier.fillMaxWidth().padding(8.dp)) {
            Box(
                modifier =
                    Modifier
                        .width(3.dp)
                        .height(30.dp)
                        .background(if (isMine) Primary else Border, CircleShape),
            )
            Spacer(modifier = Modifier.width(8.dp))
            Column {
                Text(
                    text = reply.senderDisplayName ?: "wiadomość",
                    style = MaterialTheme.typography.labelSmall,
                    color = TextSecondary,
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis,
                )
                Text(
                    text = reply.body ?: "odpowiedź",
                    style = MaterialTheme.typography.bodySmall,
                    color = TextPrimary,
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis,
                )
            }
        }
    }
}

private fun formatTime(timestampMillis: Long): String {
    val localDateTime = Instant.fromEpochMilliseconds(timestampMillis).toLocalDateTime(TimeZone.currentSystemDefault())
    val hour = localDateTime.hour.toString().padStart(2, '0')
    val minute = localDateTime.minute.toString().padStart(2, '0')
    return "$hour:$minute"
}
