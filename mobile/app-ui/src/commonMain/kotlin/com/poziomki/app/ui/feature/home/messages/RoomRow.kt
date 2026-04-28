package com.poziomki.app.ui.feature.home.messages

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.blur
import androidx.compose.ui.text.font.FontStyle
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import com.adamglin.PhosphorIcons
import com.adamglin.phosphoricons.Bold
import com.adamglin.phosphoricons.bold.Check
import com.adamglin.phosphoricons.bold.CheckCircle
import com.adamglin.phosphoricons.bold.Clock
import com.adamglin.phosphoricons.bold.WarningCircle
import com.poziomki.app.chat.api.EventSendStatus
import com.poziomki.app.chat.api.RoomSummary
import com.poziomki.app.ui.designsystem.components.UserAvatar
import com.poziomki.app.ui.designsystem.theme.Background
import com.poziomki.app.ui.designsystem.theme.Primary
import com.poziomki.app.ui.designsystem.theme.TextPrimary
import com.poziomki.app.ui.designsystem.theme.TextSecondary
import kotlinx.datetime.Clock
import kotlinx.datetime.DayOfWeek
import kotlinx.datetime.Instant
import kotlinx.datetime.TimeZone
import kotlinx.datetime.toLocalDateTime

@Suppress("CyclomaticComplexMethod", "LongMethod")
@Composable
fun RoomRow(
    room: RoomSummary,
    profilePictureUrl: String? = null,
    displayNameOverride: String? = null,
    onClick: () -> Unit,
    onAvatarClick: (() -> Unit)? = null,
) {
    val displayName = displayNameOverride ?: room.displayName
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
                displayName = displayName,
            )
        }

        Spacer(modifier = Modifier.width(12.dp))

        Column(modifier = Modifier.weight(1f)) {
            val statusIcon =
                if (room.unreadCount == 0 && room.latestMessageIsMine) {
                    latestRoomStatusIconSpec(
                        sendStatus = room.latestMessageSendStatus,
                        readByCount = room.latestMessageReadByCount,
                    )
                } else {
                    null
                }
            Row(verticalAlignment = Alignment.CenterVertically) {
                Text(
                    text = displayName,
                    style = MaterialTheme.typography.titleMedium,
                    color = TextPrimary,
                    fontWeight = if (room.unreadCount > 0) FontWeight.Bold else FontWeight.SemiBold,
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis,
                    modifier = Modifier.weight(1f),
                )
                if (room.latestTimestampMillis != null || room.unreadCount > 0) {
                    Spacer(modifier = Modifier.width(8.dp))
                    Column(horizontalAlignment = Alignment.End) {
                        room.latestTimestampMillis?.let {
                            Text(
                                text = formatRoomTimestamp(it),
                                style = MaterialTheme.typography.labelSmall,
                                color = if (room.unreadCount > 0) Primary else TextSecondary,
                            )
                        }
                        if (room.unreadCount > 0) {
                            Spacer(modifier = Modifier.height(4.dp))
                            Surface(
                                color = Primary,
                                contentColor = Background,
                                shape = CircleShape,
                                modifier = Modifier,
                            ) {
                                Text(
                                    text = if (room.unreadCount > 99) "99+" else room.unreadCount.toString(),
                                    style = MaterialTheme.typography.labelSmall,
                                    fontWeight = FontWeight.Bold,
                                    modifier = Modifier.padding(horizontal = 6.dp, vertical = 1.dp),
                                )
                            }
                        }
                    }
                }
            }
            Spacer(modifier = Modifier.height(2.dp))
            Row(verticalAlignment = Alignment.CenterVertically) {
                if (statusIcon != null) {
                    Icon(
                        imageVector = statusIcon.icon,
                        contentDescription = null,
                        tint = statusIcon.tint,
                        modifier = Modifier.size(14.dp),
                    )
                    Spacer(modifier = Modifier.width(4.dp))
                }
                val flagged =
                    !room.latestMessageIsMine &&
                        room.latestModerationVerdict in setOf("flag", "block")
                Text(
                    text = room.latestMessagePreview(),
                    style = MaterialTheme.typography.bodyMedium,
                    color = if (room.unreadCount > 0) TextPrimary else TextSecondary,
                    fontStyle = if (flagged) FontStyle.Italic else null,
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis,
                    modifier =
                        if (flagged) {
                            Modifier.blur(radius = 8.dp)
                        } else {
                            Modifier
                        },
                )
            }
        }
    }
}

private fun RoomSummary.latestMessagePreview(): String =
    latestMessage
        ?.trim()
        ?.takeIf { it.isNotEmpty() }
        ?: "Wiadomość"

private fun formatRoomTimestamp(timestampMillis: Long): String {
    val nowMillis = Clock.System.now().toEpochMilliseconds()
    val diffMillis = (nowMillis - timestampMillis).coerceAtLeast(0L)
    if (diffMillis < 60_000L) return "teraz"
    if (diffMillis < 60L * 60_000L) return "${diffMillis / 60_000L} min"
    if (diffMillis < 24L * 60L * 60_000L) {
        val hours = (diffMillis / (60L * 60_000L)).coerceAtLeast(1L)
        return "${hours}g"
    }

    val zone = TimeZone.currentSystemDefault()
    val nowDate = Instant.fromEpochMilliseconds(nowMillis).toLocalDateTime(zone).date
    val date = Instant.fromEpochMilliseconds(timestampMillis).toLocalDateTime(zone).date
    val daysDiff = nowDate.toEpochDays() - date.toEpochDays()

    return when {
        daysDiff in 1..6 -> weekdayShort(date.dayOfWeek)
        nowDate.year == date.year -> "${date.dayOfMonth} ${monthShort(date.monthNumber)}"
        else -> "${date.dayOfMonth} ${monthShort(date.monthNumber)} ${date.year}"
    }
}

private fun weekdayShort(dayOfWeek: DayOfWeek): String =
    when (dayOfWeek) {
        DayOfWeek.MONDAY -> "pon."
        DayOfWeek.TUESDAY -> "wt."
        DayOfWeek.WEDNESDAY -> "śr."
        DayOfWeek.THURSDAY -> "czw."
        DayOfWeek.FRIDAY -> "pt."
        DayOfWeek.SATURDAY -> "sob."
        DayOfWeek.SUNDAY -> "niedz."
    }

private fun monthShort(monthNumber: Int): String =
    when (monthNumber) {
        1 -> "sty."
        2 -> "lut."
        3 -> "mar."
        4 -> "kwi."
        5 -> "maj"
        6 -> "cze."
        7 -> "lip."
        8 -> "sie."
        9 -> "wrz."
        10 -> "paź."
        11 -> "lis."
        12 -> "gru."
        else -> "?"
    }

private data class RoomStatusIconSpec(
    val icon: androidx.compose.ui.graphics.vector.ImageVector,
    val tint: androidx.compose.ui.graphics.Color,
)

@Composable
private fun latestRoomStatusIconSpec(
    sendStatus: EventSendStatus?,
    readByCount: Int,
): RoomStatusIconSpec? =
    when {
        sendStatus == EventSendStatus.Failed -> {
            RoomStatusIconSpec(icon = PhosphorIcons.Bold.WarningCircle, tint = MaterialTheme.colorScheme.error)
        }

        sendStatus == EventSendStatus.Sending -> {
            RoomStatusIconSpec(icon = PhosphorIcons.Bold.Clock, tint = TextSecondary)
        }

        readByCount > 0 -> {
            RoomStatusIconSpec(icon = PhosphorIcons.Bold.CheckCircle, tint = Primary)
        }

        sendStatus == EventSendStatus.Sent -> {
            RoomStatusIconSpec(icon = PhosphorIcons.Bold.Check, tint = TextSecondary)
        }

        else -> {
            null
        }
    }
