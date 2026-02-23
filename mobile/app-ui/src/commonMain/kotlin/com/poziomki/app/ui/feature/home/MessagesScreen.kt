package com.poziomki.app.ui.feature.home

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
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
import androidx.compose.ui.unit.dp
import com.poziomki.app.ui.designsystem.components.EmptyView
import com.poziomki.app.ui.designsystem.components.FilterTabs
import com.poziomki.app.ui.designsystem.components.LoadingView
import com.poziomki.app.ui.designsystem.components.PoziomkiSearchBar
import com.poziomki.app.ui.designsystem.components.ScreenHeader
import com.poziomki.app.ui.designsystem.theme.Background
import com.poziomki.app.ui.designsystem.theme.PoziomkiTheme
import com.poziomki.app.ui.designsystem.theme.TextPrimary
import com.poziomki.app.ui.designsystem.theme.TextSecondary
import com.poziomki.app.ui.feature.home.messages.MessagesRoomFilter
import com.poziomki.app.ui.feature.home.messages.RoomRow
import com.poziomki.app.ui.feature.home.messages.filterMessagesRooms
import com.poziomki.app.ui.feature.home.messages.resolveRoomProfilePicture
import com.poziomki.app.ui.feature.home.messages.roomFilterTabs
import org.koin.compose.viewmodel.koinViewModel

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
    var selectedFilter by remember { mutableStateOf(MessagesRoomFilter.Direct) }

    val unreadTotal = state.rooms.sumOf { it.unreadCount }
    val filteredRooms = state.rooms.filterMessagesRooms(selectedFilter, searchQuery, state.eventRoomIds)
    val roomFilterTabs = roomFilterTabs()

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
                                        resolveRoomProfilePicture(
                                            room = room,
                                            profilePictures = state.profilePictures,
                                            profilePicturesByName = state.profilePicturesByName,
                                        )
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
