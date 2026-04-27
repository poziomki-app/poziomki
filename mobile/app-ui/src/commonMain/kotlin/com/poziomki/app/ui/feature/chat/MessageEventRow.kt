package com.poziomki.app.ui.feature.chat

import androidx.compose.animation.core.LinearEasing
import androidx.compose.animation.core.RepeatMode
import androidx.compose.animation.core.animateFloat
import androidx.compose.animation.core.infiniteRepeatable
import androidx.compose.animation.core.rememberInfiniteTransition
import androidx.compose.animation.core.tween
import androidx.compose.foundation.background
import androidx.compose.foundation.border
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
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.IntOffset
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.adamglin.PhosphorIcons
import com.adamglin.phosphoricons.Bold
import com.adamglin.phosphoricons.bold.ArrowBendUpLeft
import com.adamglin.phosphoricons.bold.Check
import com.adamglin.phosphoricons.bold.CheckCircle
import com.adamglin.phosphoricons.bold.Clock
import com.adamglin.phosphoricons.bold.WarningCircle
import com.poziomki.app.chat.api.EventSendStatus
import com.poziomki.app.chat.api.ReplyDetails
import com.poziomki.app.chat.api.TimelineItem
import com.poziomki.app.ui.designsystem.components.UserAvatar
import com.poziomki.app.ui.designsystem.theme.Background
import com.poziomki.app.ui.designsystem.theme.Border
import com.poziomki.app.ui.designsystem.theme.ChatBubble
import com.poziomki.app.ui.designsystem.theme.ChatNameColors
import com.poziomki.app.ui.designsystem.theme.Primary
import com.poziomki.app.ui.designsystem.theme.TextPrimary
import com.poziomki.app.ui.designsystem.theme.TextSecondary
import kotlinx.datetime.Instant
import kotlinx.datetime.TimeZone
import kotlinx.datetime.toLocalDateTime
import kotlin.math.abs
import kotlin.math.roundToInt
import com.poziomki.app.ui.designsystem.theme.Surface as SurfaceColor

private val AvatarSize = 28.dp
private val AvatarSpacing = 6.dp

@Suppress("LongParameterList", "LongMethod", "CyclomaticComplexMethod")
@Composable
internal fun MessageEventRow(
    event: TimelineItem.Event,
    groupedWithPrevious: Boolean,
    showSenderMeta: Boolean,
    onToggleReaction: (String) -> Unit,
    onReactionsClick: () -> Unit,
    onFocusOnReply: () -> Unit,
    onSenderClick: () -> Unit,
    onActionsLongPress: () -> Unit,
    onSwipeReply: () -> Unit,
    onRevealModeration: () -> Unit,
    compactTimestamp: Boolean = false,
    avatarOverride: String? = null,
    isHighlighted: Boolean = false,
) {
    val horizontalAlignment = if (event.isMine) Alignment.End else Alignment.Start
    val bubbleColor =
        if (isHighlighted) {
            ChatBubble
        } else if (event.isMine) {
            Primary.copy(alpha = 0.68f)
        } else {
            ChatBubble
        }
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

    val highlightBorderModifier =
        if (isHighlighted) searchHighlightBorder(bubbleShape) else Modifier

    val showAvatar = showSenderMeta && !event.isMine && !groupedWithPrevious

    Column(
        modifier =
            Modifier
                .fillMaxWidth()
                .padding(top = if (groupedWithPrevious) 2.dp else 10.dp),
        horizontalAlignment = horizontalAlignment,
    ) {
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
                            imageVector = PhosphorIcons.Bold.ArrowBendUpLeft,
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
                                        .then(highlightBorderModifier)
                                        .widthIn(max = maxBubbleWidth)
                                        .combinedClickable(
                                            onClick = {},
                                            onLongClick = onActionsLongPress,
                                        ),
                            ) {
                                BubbleContent(
                                    event = event,
                                    onFocusOnReply = onFocusOnReply,
                                    compactTimestamp = compactTimestamp,
                                    onRevealModeration = onRevealModeration,
                                )
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
                                        val senderCount = reaction.senders.distinctBy { it.senderId }.size
                                        val reactionCount = if (senderCount > 0) senderCount else reaction.count
                                        Surface(
                                            shape = RoundedCornerShape(12.dp),
                                            color = SurfaceColor,
                                            modifier = Modifier.clickable { onReactionsClick() },
                                        ) {
                                            Row(
                                                modifier = Modifier.padding(horizontal = 6.dp, vertical = 3.dp),
                                                verticalAlignment = Alignment.CenterVertically,
                                            ) {
                                                Text(
                                                    text = reaction.emoji,
                                                    style = MaterialTheme.typography.labelSmall,
                                                )
                                                if (reactionCount > 1) {
                                                    Spacer(modifier = Modifier.width(3.dp))
                                                    Text(
                                                        text = reactionCount.toString(),
                                                        style = MaterialTheme.typography.labelSmall,
                                                        color = TextSecondary,
                                                    )
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    } else {
                        // Not mine: avatar + name on same line, bubble below
                        Row(verticalAlignment = Alignment.Top) {
                            if (showAvatar) {
                                UserAvatar(
                                    picture = avatarOverride,
                                    fallbackPicture = event.senderAvatarUrl,
                                    displayName = event.senderDisplayName,
                                    size = AvatarSize,
                                    modifier = Modifier.clickable(onClick = onSenderClick),
                                )
                                Spacer(modifier = Modifier.width(AvatarSpacing))
                            } else if (showSenderMeta) {
                                Spacer(modifier = Modifier.width(AvatarSize + AvatarSpacing))
                            }
                            Column {
                                if (showAvatar) {
                                    val senderNameColor = ChatNameColors[abs(event.senderId.hashCode()) % ChatNameColors.size]
                                    Text(
                                        text = event.senderDisplayName ?: event.senderId,
                                        style = MaterialTheme.typography.labelSmall,
                                        color = senderNameColor,
                                        maxLines = 1,
                                        overflow = TextOverflow.Ellipsis,
                                        modifier =
                                            Modifier
                                                .clickable(onClick = onSenderClick)
                                                .padding(top = 2.dp, bottom = 2.dp),
                                    )
                                }
                                Surface(
                                    color = bubbleColor,
                                    shape = bubbleShape,
                                    modifier =
                                        Modifier
                                            .then(highlightBorderModifier)
                                            .widthIn(
                                                max = if (showSenderMeta) maxBubbleWidth - AvatarSize - AvatarSpacing else maxBubbleWidth,
                                            ).combinedClickable(
                                                onClick = {},
                                                onLongClick = onActionsLongPress,
                                            ),
                                ) {
                                    BubbleContent(
                                        event = event,
                                        onFocusOnReply = onFocusOnReply,
                                        compactTimestamp = compactTimestamp,
                                        onRevealModeration = onRevealModeration,
                                    )
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
                                            val reactionCount =
                                                reaction.senders
                                                    .map { it.senderId }
                                                    .distinct()
                                                    .size
                                            Surface(
                                                shape = RoundedCornerShape(12.dp),
                                                color = SurfaceColor,
                                                modifier = Modifier.clickable { onReactionsClick() },
                                            ) {
                                                Row(
                                                    modifier = Modifier.padding(horizontal = 6.dp, vertical = 3.dp),
                                                    verticalAlignment = Alignment.CenterVertically,
                                                ) {
                                                    Text(
                                                        text = reaction.emoji,
                                                        style = MaterialTheme.typography.labelSmall,
                                                    )
                                                    if (reactionCount > 1) {
                                                        Spacer(modifier = Modifier.width(3.dp))
                                                        Text(
                                                            text = reactionCount.toString(),
                                                            style = MaterialTheme.typography.labelSmall,
                                                            color = TextSecondary,
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
    }
}

@Composable
private fun BubbleContent(
    event: TimelineItem.Event,
    onFocusOnReply: () -> Unit,
    compactTimestamp: Boolean,
    onRevealModeration: () -> Unit,
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
        // Hide flagged/blocked content from recipients until they tap
        // to reveal. Senders see their own message normally with a
        // subtle warning chip — they wrote it, so blurring it would
        // be patronising; instead we surface that the message was
        // flagged so they don't repeat the pattern.
        val isFlagged =
            !event.isMine &&
                !event.locallyRevealed &&
                event.moderationVerdict in setOf("flag", "block")
        if (isFlagged) {
            FlaggedMessagePlaceholder(
                categories = event.moderationCategories,
                onReveal = onRevealModeration,
            )
        } else {
            Text(
                text = event.body,
                style = MaterialTheme.typography.bodyLarge,
                color = TextPrimary,
            )
            if (event.isMine && event.moderationVerdict in setOf("flag", "block")) {
                Spacer(modifier = Modifier.height(4.dp))
                OwnFlaggedNote(categories = event.moderationCategories)
            }
        }
        Row(
            modifier = Modifier.align(Alignment.End).padding(top = 4.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Text(
                text = formatTime(event.timestampMillis),
                style =
                    if (compactTimestamp) {
                        MaterialTheme.typography.labelSmall.copy(fontSize = 10.sp)
                    } else {
                        MaterialTheme.typography.labelSmall
                    },
                color = TextSecondary,
            )
            if (event.isMine) {
                Spacer(modifier = Modifier.width(4.dp))
                OutgoingMessageStatusIcon(event = event)
            }
        }
    }
}

@Composable
private fun FlaggedMessagePlaceholder(
    categories: List<String>,
    onReveal: () -> Unit,
) {
    val label = polishCategoryList(categories)
    Surface(
        color = SurfaceColor,
        shape = RoundedCornerShape(8.dp),
        modifier =
            Modifier
                .clickable(onClick = onReveal),
    ) {
        Column(
            modifier = Modifier.padding(horizontal = 10.dp, vertical = 8.dp),
        ) {
            Text(
                text = "Wiadomość oznaczona jako $label",
                style = MaterialTheme.typography.bodyMedium,
                color = TextSecondary,
            )
            Spacer(modifier = Modifier.height(2.dp))
            Text(
                text = "Stuknij, aby pokazać",
                style = MaterialTheme.typography.labelSmall,
                color = Primary,
            )
        }
    }
}

@Composable
private fun OwnFlaggedNote(categories: List<String>) {
    Text(
        text = "Twoja wiadomość została oznaczona jako ${polishCategoryList(categories)}.",
        style = MaterialTheme.typography.labelSmall,
        color = MaterialTheme.colorScheme.error,
    )
}

private fun polishCategoryList(categories: List<String>): String {
    if (categories.isEmpty()) return "potencjalnie nieodpowiednia"
    return categories.joinToString(", ") { translateCategory(it) }
}

private fun translateCategory(category: String): String =
    when (category) {
        "vulgar" -> "wulgarna"
        "hate" -> "mowa nienawiści"
        "sex" -> "treść seksualna"
        "self_harm" -> "samookaleczenie"
        "crime" -> "treść przestępcza"
        else -> category
    }

@Composable
private fun OutgoingMessageStatusIcon(event: TimelineItem.Event) {
    when {
        event.sendStatus == EventSendStatus.Failed -> {
            Icon(
                imageVector = PhosphorIcons.Bold.WarningCircle,
                contentDescription = null,
                tint = MaterialTheme.colorScheme.error,
                modifier = Modifier.size(14.dp),
            )
        }

        event.sendStatus == EventSendStatus.Sending -> {
            Icon(
                imageVector = PhosphorIcons.Bold.Clock,
                contentDescription = null,
                tint = TextSecondary,
                modifier = Modifier.size(14.dp),
            )
        }

        event.readByCount > 0 -> {
            Icon(
                imageVector = PhosphorIcons.Bold.CheckCircle,
                contentDescription = null,
                tint = TextSecondary,
                modifier = Modifier.size(14.dp),
            )
        }

        else -> {
            Icon(
                imageVector = PhosphorIcons.Bold.Check,
                contentDescription = null,
                tint = TextSecondary,
                modifier = Modifier.size(14.dp),
            )
        }
    }
}

@Composable
private fun ReplyReference(
    reply: ReplyDetails,
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

private const val GLOW_DURATION = 8000
private const val GLOW_STEPS = 24

@Composable
private fun searchHighlightBorder(shape: RoundedCornerShape): Modifier {
    val transition = rememberInfiniteTransition(label = "searchBorder")
    val phase by transition.animateFloat(
        initialValue = 0f,
        targetValue = 1f,
        animationSpec =
            infiniteRepeatable(
                animation = tween(GLOW_DURATION, easing = LinearEasing),
                repeatMode = RepeatMode.Restart,
            ),
        label = "searchPhase",
    )
    val stops =
        Array(GLOW_STEPS) { i ->
            val pos = i.toFloat() / GLOW_STEPS
            val d = abs(pos - phase)
            val dist = minOf(d, 1f - d)
            val t = (dist / 0.25f).coerceIn(0f, 1f)
            val alpha = 0.08f + 0.27f * (1f + kotlin.math.cos(t * kotlin.math.PI.toFloat())) / 2f
            pos to Primary.copy(alpha = alpha)
        }
    return Modifier.border(1.5.dp, Brush.sweepGradient(colorStops = stops), shape)
}
