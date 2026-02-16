package com.poziomki.app.ui.screen.main

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Edit
import androidx.compose.material.icons.filled.Notifications
import androidx.compose.material3.Badge
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Snackbar
import androidx.compose.material3.Text
import androidx.compose.material3.pulltorefresh.PullToRefreshBox
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.poziomki.app.chat.matrix.api.MatrixRoomSummary
import com.poziomki.app.ui.component.EmptyView
import com.poziomki.app.ui.component.FilterTabs
import com.poziomki.app.ui.component.LoadingView
import com.poziomki.app.ui.component.PoziomkiSearchBar
import com.poziomki.app.ui.component.ScreenHeader
import com.poziomki.app.ui.component.UserAvatar
import com.poziomki.app.ui.theme.Background
import com.poziomki.app.ui.theme.NunitoFamily
import com.poziomki.app.ui.theme.PoziomkiTheme
import com.poziomki.app.ui.theme.Primary
import com.poziomki.app.ui.theme.TextMuted
import com.poziomki.app.ui.theme.TextPrimary
import com.poziomki.app.ui.theme.TextSecondary
import kotlinx.datetime.Clock
import kotlinx.datetime.Instant
import kotlinx.datetime.TimeZone
import kotlinx.datetime.toLocalDateTime
import org.koin.compose.viewmodel.koinViewModel
import kotlin.math.absoluteValue

private enum class RoomFilter {
    Direct,
    Groups,
    Events,
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun MessagesScreen(
    onNavigateToChat: (String) -> Unit,
    onNavigateToNewChat: () -> Unit,
    onNavigateToProfile: (String) -> Unit = {},
    viewModel: MessagesViewModel = koinViewModel(),
) {
    val state by viewModel.state.collectAsState()
    var searchQuery by remember { mutableStateOf("") }
    var selectedFilter by remember { mutableStateOf(RoomFilter.Direct) }

    val unreadTotal = state.rooms.sumOf { it.unreadCount }
    val normalizedQuery = searchQuery.trim().lowercase()
    val filteredRooms =
        state.rooms
            .asSequence()
            .filter { room ->
                when (selectedFilter) {
                    RoomFilter.Direct -> room.isDirect
                    RoomFilter.Groups -> !room.isDirect
                    RoomFilter.Events -> !room.isDirect
                }
            }.filter { room ->
                if (normalizedQuery.isBlank()) {
                    true
                } else {
                    room.displayName.lowercase().contains(normalizedQuery) ||
                        (room.latestMessage?.lowercase()?.contains(normalizedQuery) == true)
                }
            }.toList()

    val roomFilterTabs =
        listOf(
            RoomFilter.Direct to "znajomi",
            RoomFilter.Groups to "grupy",
            RoomFilter.Events to "wydarzenia",
        )

    Box(modifier = Modifier.fillMaxSize()) {
        Column(
            modifier =
                Modifier
                    .fillMaxSize()
                    .background(Background),
        ) {
            ScreenHeader(title = "wiadomości") {
                if (unreadTotal > 0) {
                    Box {
                        Icon(
                            imageVector = Icons.Filled.Notifications,
                            contentDescription = "Powiadomienia",
                            tint = TextSecondary,
                            modifier = Modifier.size(24.dp),
                        )
                        Badge(
                            containerColor = MaterialTheme.colorScheme.error,
                            contentColor = TextPrimary,
                            modifier =
                                Modifier
                                    .align(Alignment.TopEnd)
                                    .size(10.dp),
                        ) {}
                    }
                    Spacer(modifier = Modifier.width(4.dp))
                }
                IconButton(onClick = onNavigateToNewChat) {
                    Icon(
                        imageVector = Icons.Filled.Edit,
                        contentDescription = "Nowa wiadomość",
                        tint = TextSecondary,
                        modifier = Modifier.size(22.dp),
                    )
                }
            }
            PoziomkiSearchBar(
                query = searchQuery,
                onQueryChange = { searchQuery = it },
                placeholder = "szukaj wiadomości...",
            )
            FilterTabs(
                tabs = roomFilterTabs,
                selected = selectedFilter,
                onSelect = { selectedFilter = it },
            )

            when {
                state.isLoading && state.rooms.isEmpty() -> {
                    LoadingView()
                }

                state.rooms.isEmpty() -> {
                    EmptyView(state.error ?: "brak rozmów")
                }

                else -> {
                    PullToRefreshBox(
                        isRefreshing = state.isRefreshing,
                        onRefresh = { viewModel.pullToRefresh() },
                    ) {
                        if (filteredRooms.isEmpty()) {
                            EmptyView("brak rozmów")
                        } else {
                            LazyColumn(
                                modifier =
                                    Modifier
                                        .fillMaxSize()
                                        .padding(horizontal = PoziomkiTheme.spacing.lg),
                            ) {
                                items(filteredRooms, key = { it.roomId }) { room ->
                                    val profilePicture =
                                        room.directUserId
                                            ?.substringAfter("@")
                                            ?.substringBefore(":")
                                            ?.let { state.profilePictures[it] }
                                    RoomRow(
                                        room = room,
                                        profilePictureUrl = profilePicture,
                                        onClick = { onNavigateToChat(room.roomId) },
                                        onAvatarClick =
                                            room.directUserId?.let { userId ->
                                                { onNavigateToProfile(userId) }
                                            },
                                    )
                                }
                                item { Spacer(modifier = Modifier.height(84.dp)) }
                            }
                        }
                    }
                }
            }
        }

        // Refresh error snackbar
        state.refreshError?.let { error ->
            Snackbar(
                modifier =
                    Modifier
                        .align(Alignment.BottomCenter)
                        .padding(PoziomkiTheme.spacing.md),
            ) {
                Text(text = error)
            }
            LaunchedEffect(error) {
                kotlinx.coroutines.delay(3000)
                viewModel.clearRefreshError()
            }
        }
    }
}

@Composable
private fun RoomRow(
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
                picture = profilePictureUrl ?: room.avatarUrl,
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
