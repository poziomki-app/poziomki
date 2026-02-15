package com.poziomki.app.ui.screen.main

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
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
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Edit
import androidx.compose.material.icons.filled.Notifications
import androidx.compose.material.icons.filled.Person
import androidx.compose.material.icons.filled.Search
import androidx.compose.material.icons.filled.Tune
import androidx.compose.material3.Badge
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.VerticalDivider
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import coil3.compose.AsyncImage
import com.poziomki.app.chat.matrix.api.MatrixRoomSummary
import com.poziomki.app.ui.theme.Background
import com.poziomki.app.ui.theme.Border
import com.poziomki.app.ui.theme.NunitoFamily
import com.poziomki.app.ui.theme.PoziomkiTheme
import com.poziomki.app.ui.theme.Primary
import com.poziomki.app.ui.theme.TextMuted
import com.poziomki.app.ui.theme.TextPrimary
import com.poziomki.app.ui.theme.TextSecondary
import com.poziomki.app.util.isImageUrl
import com.poziomki.app.util.resolveImageUrl
import kotlinx.datetime.Clock
import kotlinx.datetime.Instant
import kotlinx.datetime.TimeZone
import kotlinx.datetime.toLocalDateTime
import org.koin.compose.viewmodel.koinViewModel
import kotlin.math.absoluteValue
import com.poziomki.app.ui.theme.Surface as SurfaceColor

private enum class RoomFilter {
    Direct,
    Groups,
    Events,
}

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

    Column(
        modifier =
            Modifier
                .fillMaxSize()
                .background(Background)
                .padding(horizontal = PoziomkiTheme.spacing.lg),
    ) {
        Header(
            unreadTotal = unreadTotal,
            onNewChat = onNavigateToNewChat,
        )
        SearchBar(
            query = searchQuery,
            onQueryChange = { searchQuery = it },
        )
        FilterTabs(
            selectedFilter = selectedFilter,
            onSelect = { selectedFilter = it },
        )

        when {
            state.isLoading && state.rooms.isEmpty() -> {
                Box(modifier = Modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
                    CircularProgressIndicator(color = Primary)
                }
            }

            filteredRooms.isEmpty() -> {
                Box(modifier = Modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
                    Text(
                        text = state.error ?: "Brak rozmów",
                        style = MaterialTheme.typography.bodyMedium,
                        color = TextSecondary,
                    )
                }
            }

            else -> {
                LazyColumn(
                    modifier = Modifier.fillMaxSize(),
                ) {
                    items(filteredRooms, key = { it.roomId }) { room ->
                        val profilePicture = room.directUserId
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

@Composable
private fun Header(
    unreadTotal: Int,
    onNewChat: () -> Unit,
) {
    Row(
        modifier =
            Modifier
                .fillMaxWidth()
                .padding(top = PoziomkiTheme.spacing.md, bottom = PoziomkiTheme.spacing.md),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Text(
            text = "wiadomości",
            style = MaterialTheme.typography.headlineMedium,
            color = TextPrimary,
            modifier = Modifier.weight(1f),
        )
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
        IconButton(onClick = onNewChat) {
            Icon(
                imageVector = Icons.Filled.Edit,
                contentDescription = "Nowa wiadomość",
                tint = TextSecondary,
                modifier = Modifier.size(22.dp),
            )
        }
    }
}

@Composable
private fun SearchBar(
    query: String,
    onQueryChange: (String) -> Unit,
) {
    Surface(
        shape = RoundedCornerShape(28.dp),
        color = SurfaceColor,
        border = androidx.compose.foundation.BorderStroke(1.dp, Border),
        modifier = Modifier.fillMaxWidth(),
    ) {
        Row(
            modifier =
                Modifier
                    .fillMaxWidth()
                    .padding(horizontal = 16.dp, vertical = 10.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Icon(
                imageVector = Icons.Filled.Search,
                contentDescription = null,
                tint = TextMuted,
                modifier = Modifier.size(20.dp),
            )
            Spacer(modifier = Modifier.width(8.dp))
            androidx.compose.foundation.text.BasicTextField(
                value = query,
                onValueChange = onQueryChange,
                textStyle = MaterialTheme.typography.bodyMedium.copy(color = TextPrimary),
                singleLine = true,
                modifier = Modifier.weight(1f),
                decorationBox = { innerTextField ->
                    if (query.isBlank()) {
                        Text(
                            text = "szukaj wiadomości...",
                            fontFamily = NunitoFamily,
                            color = TextMuted,
                        )
                    }
                    innerTextField()
                },
            )
            VerticalDivider(
                modifier = Modifier.height(20.dp),
                thickness = 1.dp,
                color = Border,
            )
            Spacer(modifier = Modifier.width(8.dp))
            Icon(
                imageVector = Icons.Filled.Tune,
                contentDescription = "Filtruj",
                tint = TextMuted,
                modifier = Modifier.size(20.dp),
            )
        }
    }
}

@Composable
private fun FilterTabs(
    selectedFilter: RoomFilter,
    onSelect: (RoomFilter) -> Unit,
) {
    val tabs =
        listOf(
            RoomFilter.Direct to "znajomi",
            RoomFilter.Groups to "grupy",
            RoomFilter.Events to "wydarzenia",
        )
    Row(
        modifier =
            Modifier
                .fillMaxWidth()
                .padding(top = 12.dp, bottom = 16.dp),
        horizontalArrangement = Arrangement.Center,
    ) {
        tabs.forEachIndexed { index, (filter, label) ->
            val selected = filter == selectedFilter
            Row(
                modifier =
                    Modifier
                        .clickable { onSelect(filter) }
                        .padding(horizontal = 12.dp, vertical = 4.dp),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                if (selected) {
                    Box(
                        modifier =
                            Modifier
                                .size(6.dp)
                                .background(Primary, CircleShape),
                    )
                    Spacer(modifier = Modifier.width(6.dp))
                }
                Text(
                    text = label,
                    fontFamily = NunitoFamily,
                    fontWeight = if (selected) FontWeight.Bold else FontWeight.Normal,
                    fontSize = 14.sp,
                    color = if (selected) TextPrimary else TextMuted,
                )
            }
            if (index < tabs.lastIndex) {
                Spacer(modifier = Modifier.width(8.dp))
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
            RoomAvatar(
                displayName = room.displayName,
                avatarUrl = room.avatarUrl,
                profilePictureUrl = profilePictureUrl,
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

@Composable
private fun RoomAvatar(
    displayName: String,
    avatarUrl: String?,
    profilePictureUrl: String? = null,
) {
    val picture = profilePictureUrl ?: avatarUrl
    Surface(
        modifier = Modifier.size(52.dp),
        shape = CircleShape,
        color = Border,
    ) {
        when {
            picture != null && isImageUrl(picture) -> {
                AsyncImage(
                    model = resolveImageUrl(picture),
                    contentDescription = displayName,
                    modifier =
                        Modifier
                            .size(52.dp)
                            .clip(CircleShape),
                    contentScale = ContentScale.Crop,
                )
            }

            picture != null -> {
                Box(
                    modifier = Modifier.size(52.dp),
                    contentAlignment = Alignment.Center,
                ) {
                    Text(
                        text = picture,
                        fontSize = 24.sp,
                    )
                }
            }

            else -> {
                Box(
                    modifier = Modifier.size(52.dp),
                    contentAlignment = Alignment.Center,
                ) {
                    Icon(
                        imageVector = Icons.Filled.Person,
                        contentDescription = displayName,
                        modifier = Modifier.size(26.dp),
                        tint = TextMuted,
                    )
                }
            }
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
