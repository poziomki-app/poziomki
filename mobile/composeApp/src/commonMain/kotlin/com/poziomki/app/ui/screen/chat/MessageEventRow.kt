package com.poziomki.app.ui.screen.chat

import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.MoreVert
import androidx.compose.material.icons.outlined.AddReaction
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import com.poziomki.app.chat.matrix.api.MatrixReplyDetails
import com.poziomki.app.chat.matrix.api.MatrixTimelineItem
import com.poziomki.app.ui.theme.Border
import com.poziomki.app.ui.theme.Primary
import com.poziomki.app.ui.theme.TextPrimary
import com.poziomki.app.ui.theme.TextSecondary
import kotlinx.datetime.Instant
import kotlinx.datetime.TimeZone
import kotlinx.datetime.toLocalDateTime
import com.poziomki.app.ui.theme.Surface as SurfaceColor

@Composable
internal fun MessageEventRow(
    event: MatrixTimelineItem.Event,
    groupedWithPrevious: Boolean,
    menuExpanded: Boolean,
    onToggleReaction: (String) -> Unit,
    onFocusOnEvent: () -> Unit,
    onFocusOnReply: () -> Unit,
    onSenderClick: () -> Unit,
    onMenuOpen: () -> Unit,
    onMenuDismiss: () -> Unit,
    onReply: () -> Unit,
    onEdit: () -> Unit,
    onDelete: () -> Unit,
) {
    val horizontalAlignment = if (event.isMine) Alignment.End else Alignment.Start
    val bubbleColor = if (event.isMine) Primary.copy(alpha = 0.22f) else SurfaceColor

    Column(
        modifier =
            Modifier
                .fillMaxWidth()
                .padding(top = if (groupedWithPrevious) 2.dp else 8.dp, bottom = 2.dp),
        horizontalAlignment = horizontalAlignment,
    ) {
        if (!event.isMine && !groupedWithPrevious) {
            Text(
                text = event.senderDisplayName ?: event.senderId,
                style = MaterialTheme.typography.labelSmall,
                color = TextSecondary,
                modifier = Modifier.clickable(onClick = onSenderClick),
            )
            Spacer(modifier = Modifier.height(2.dp))
        }

        Surface(
            color = bubbleColor,
            shape = RoundedCornerShape(12.dp),
            border = BorderStroke(1.dp, Border),
        ) {
            Column(modifier = Modifier.padding(horizontal = 12.dp, vertical = 8.dp)) {
                event.inReplyTo?.let { reply ->
                    ReplyReference(
                        reply = reply,
                        onClick = onFocusOnReply,
                    )
                    Spacer(modifier = Modifier.height(6.dp))
                }
                Text(
                    text = event.body,
                    style = MaterialTheme.typography.bodyMedium,
                    color = TextPrimary,
                )
            }
        }

        Spacer(modifier = Modifier.height(3.dp))

        Row(
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.spacedBy(6.dp),
        ) {
            Text(
                text = formatTime(event.timestampMillis),
                style = MaterialTheme.typography.labelSmall,
                color = TextSecondary,
            )
            if (event.isMine && event.readByCount > 0) {
                Text(
                    text = "czytalo ${event.readByCount}",
                    style = MaterialTheme.typography.labelSmall,
                    color = TextSecondary,
                )
            }
            if (event.eventId != null) {
                Text(
                    text = "kontekst",
                    style = MaterialTheme.typography.labelSmall,
                    color = TextSecondary,
                    modifier = Modifier.clickable(onClick = onFocusOnEvent),
                )
            }
            event.reactions.forEach { reaction ->
                Surface(
                    shape = RoundedCornerShape(10.dp),
                    color = if (reaction.reactedByMe) Primary.copy(alpha = 0.25f) else SurfaceColor,
                    border = BorderStroke(1.dp, Border),
                    modifier = Modifier.clickable { onToggleReaction(reaction.emoji) },
                ) {
                    Text(
                        text = "${reaction.emoji} ${reaction.count}",
                        style = MaterialTheme.typography.labelSmall,
                        fontWeight = FontWeight.SemiBold,
                        modifier = Modifier.padding(horizontal = 8.dp, vertical = 2.dp),
                    )
                }
            }
            var showEmojiPicker by remember { mutableStateOf(false) }
            Box {
                IconButton(
                    onClick = { showEmojiPicker = true },
                    modifier = Modifier.size(22.dp),
                ) {
                    Icon(
                        imageVector = Icons.Outlined.AddReaction,
                        contentDescription = "dodaj reakcje",
                        tint = TextSecondary,
                        modifier = Modifier.size(16.dp),
                    )
                }
                DropdownMenu(
                    expanded = showEmojiPicker,
                    onDismissRequest = { showEmojiPicker = false },
                ) {
                    Row(
                        modifier = Modifier.padding(horizontal = 8.dp, vertical = 4.dp),
                        horizontalArrangement = Arrangement.spacedBy(2.dp),
                    ) {
                        listOf("👍", "❤️", "😂", "😮", "😢", "🎉", "🔥", "👎").forEach { emoji ->
                            Text(
                                text = emoji,
                                modifier =
                                    Modifier
                                        .clickable {
                                            onToggleReaction(emoji)
                                            showEmojiPicker = false
                                        }.padding(4.dp),
                                style = MaterialTheme.typography.titleMedium,
                            )
                        }
                    }
                }
            }
            Box {
                IconButton(
                    onClick = onMenuOpen,
                    modifier = Modifier.size(22.dp),
                ) {
                    Icon(
                        imageVector = Icons.Filled.MoreVert,
                        contentDescription = "akcje",
                        tint = TextSecondary,
                        modifier = Modifier.size(16.dp),
                    )
                }
                DropdownMenu(
                    expanded = menuExpanded,
                    onDismissRequest = onMenuDismiss,
                ) {
                    if (event.canReply && event.eventId != null) {
                        DropdownMenuItem(
                            text = { Text("odpowiedz") },
                            onClick = onReply,
                        )
                    }
                    if (event.isEditable) {
                        DropdownMenuItem(
                            text = { Text("edytuj") },
                            onClick = onEdit,
                        )
                        DropdownMenuItem(
                            text = { Text("usun") },
                            onClick = onDelete,
                        )
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
) {
    Surface(
        color = SurfaceColor.copy(alpha = 0.8f),
        border = BorderStroke(1.dp, Border),
        shape = RoundedCornerShape(8.dp),
        modifier = Modifier.fillMaxWidth().clickable(onClick = onClick),
    ) {
        Column(modifier = Modifier.padding(horizontal = 8.dp, vertical = 6.dp)) {
            Text(
                text = reply.senderDisplayName ?: "wiadomosc",
                style = MaterialTheme.typography.labelSmall,
                color = TextSecondary,
                maxLines = 1,
            )
            Text(
                text = reply.body ?: "odpowiedz",
                style = MaterialTheme.typography.bodySmall,
                color = TextPrimary,
                maxLines = 1,
            )
        }
    }
}

private fun formatTime(timestampMillis: Long): String {
    val localDateTime = Instant.fromEpochMilliseconds(timestampMillis).toLocalDateTime(TimeZone.currentSystemDefault())
    val hour = localDateTime.hour.toString().padStart(2, '0')
    val minute = localDateTime.minute.toString().padStart(2, '0')
    return "$hour:$minute"
}
