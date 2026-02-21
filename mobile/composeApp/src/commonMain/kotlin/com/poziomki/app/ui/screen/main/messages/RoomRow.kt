package com.poziomki.app.ui.screen.main.messages

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.material3.Badge
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import com.poziomki.app.chat.matrix.api.MatrixRoomSummary
import com.poziomki.app.ui.component.UserAvatar
import com.poziomki.app.ui.theme.Background
import com.poziomki.app.ui.theme.Primary
import com.poziomki.app.ui.theme.TextPrimary
import com.poziomki.app.ui.theme.TextSecondary
import kotlinx.datetime.Clock
import kotlinx.datetime.Instant
import kotlinx.datetime.TimeZone
import kotlinx.datetime.toLocalDateTime
import kotlin.math.absoluteValue

@Composable
fun RoomRow(
    room: MatrixRoomSummary,
    profilePictureUrl: String? = null,
    onClick: () -> Unit,
    onAvatarClick: (() -> Unit)? = null,
) {
    Row(
        modifier =
            Modifier
                .fillMaxWidth()
                .clickable(onClick = onClick)
                .padding(vertical = 10.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Box(
            modifier =
                if (onAvatarClick != null) {
                    Modifier.clickable(onClick = onAvatarClick)
                } else {
                    Modifier
                },
        ) {
            UserAvatar(
                picture = profilePictureUrl,
                fallbackPicture = room.avatarUrl,
                displayName = room.displayName,
            )
            if (room.unreadCount > 0) {
                Badge(
                    containerColor = Primary,
                    contentColor = Background,
                    modifier = Modifier.align(Alignment.TopEnd),
                ) {
                    Text(
                        text = room.unreadCount.toString(),
                        style = MaterialTheme.typography.labelSmall,
                        fontWeight = FontWeight.Bold,
                    )
                }
            }
        }

        Spacer(modifier = Modifier.width(12.dp))

        Column(modifier = Modifier.weight(1f)) {
            Row(verticalAlignment = Alignment.CenterVertically) {
                Text(
                    text = room.displayName,
                    style = MaterialTheme.typography.titleMedium,
                    color = TextPrimary,
                    fontWeight = if (room.unreadCount > 0) FontWeight.Bold else FontWeight.SemiBold,
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis,
                    modifier = Modifier.weight(1f),
                )
                Spacer(modifier = Modifier.width(8.dp))
                room.latestTimestampMillis?.let {
                    Text(
                        text = formatRoomTimestamp(it),
                        style = MaterialTheme.typography.labelSmall,
                        color = if (room.unreadCount > 0) Primary else TextSecondary,
                    )
                }
            }
            Spacer(modifier = Modifier.height(2.dp))
            Text(
                text = room.latestMessage ?: "Brak wiadomości",
                style = MaterialTheme.typography.bodyMedium,
                color = if (room.unreadCount > 0) TextPrimary else TextSecondary,
                maxLines = 1,
                overflow = TextOverflow.Ellipsis,
            )
        }
    }
}

private fun formatRoomTimestamp(timestampMillis: Long): String {
    val nowMillis = Clock.System.now().toEpochMilliseconds()
    val diffMillis = (nowMillis - timestampMillis).absoluteValue
    if (diffMillis < 60_000L) return "teraz"

    val now = Instant.fromEpochMilliseconds(nowMillis).toLocalDateTime(TimeZone.currentSystemDefault())
    val dateTime = Instant.fromEpochMilliseconds(timestampMillis).toLocalDateTime(TimeZone.currentSystemDefault())
    return if (
        now.year == dateTime.year &&
        now.monthNumber == dateTime.monthNumber &&
        now.dayOfMonth == dateTime.dayOfMonth
    ) {
        val hour = dateTime.hour.toString().padStart(2, '0')
        val minute = dateTime.minute.toString().padStart(2, '0')
        "$hour:$minute"
    } else {
        "${dateTime.dayOfMonth}.${dateTime.monthNumber.toString().padStart(2, '0')}"
    }
}
