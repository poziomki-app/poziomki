package com.poziomki.app.ui.screen.chat

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.combinedClickable
import androidx.compose.foundation.gestures.detectHorizontalDragGestures
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.Reply
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
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.IntOffset
import androidx.compose.ui.unit.dp
import com.poziomki.app.chat.matrix.api.MatrixReplyDetails
import com.poziomki.app.chat.matrix.api.MatrixTimelineItem
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
                modifier = Modifier.padding(horizontal = 10.dp, vertical = 2.dp).clickable(onClick = onSenderClick),
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
                                alpha = 0.35f + (0.65f * swipeProgress)
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

            Column(
                horizontalAlignment = horizontalAlignment,
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
                Surface(
                    color = bubbleColor,
                    shape = bubbleShape,
                    modifier =
                        Modifier
                            .fillMaxWidth(0.86f)
                            .combinedClickable(
                                onClick = {},
                                onLongClick = onActionsLongPress,
                            ),
                ) {
                    Column(modifier = Modifier.padding(horizontal = 12.dp, vertical = 8.dp)) {
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
                            modifier = Modifier.fillMaxWidth().padding(top = 4.dp),
                            horizontalArrangement = Arrangement.End,
                            verticalAlignment = Alignment.CenterVertically,
                        ) {
                            Text(
                                text = formatTime(event.timestampMillis),
                                style = MaterialTheme.typography.labelSmall,
                                color = TextSecondary,
                            )
                            if (event.isMine && event.readByCount > 0) {
                                Spacer(modifier = Modifier.width(4.dp))
                                Text(
                                    text = "✓",
                                    style = MaterialTheme.typography.labelSmall,
                                    color = Primary,
                                    fontWeight = FontWeight.Bold,
                                )
                            }
                        }
                    }
                }

                if (event.reactions.isNotEmpty()) {
                    Row(
                        modifier = Modifier.padding(top = 4.dp, start = 4.dp, end = 4.dp),
                        horizontalArrangement = Arrangement.spacedBy(6.dp),
                    ) {
                        event.reactions.forEach { reaction ->
                            Surface(
                                shape = RoundedCornerShape(999.dp),
                                color = if (reaction.reactedByMe) Primary.copy(alpha = 0.22f) else SurfaceColor,
                                modifier = Modifier.clickable { onReactionsClick() },
                            ) {
                                Text(
                                    text = "${reaction.emoji} ${reaction.count}",
                                    style = MaterialTheme.typography.labelSmall,
                                    fontWeight = FontWeight.SemiBold,
                                    color = TextPrimary,
                                    modifier = Modifier.padding(horizontal = 8.dp, vertical = 4.dp),
                                )
                            }
                        }
                    }
                }
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
            Column(modifier = Modifier.weight(1f)) {
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
