package com.poziomki.app.ui.feature.home

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material3.Badge
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.FloatingActionButton
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
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.unit.dp
import com.poziomki.app.ui.designsystem.components.EmptyView
import com.poziomki.app.ui.designsystem.components.FilterTabs
import com.poziomki.app.ui.designsystem.components.LoadingView
import com.poziomki.app.ui.designsystem.components.PoziomkiSearchBar
import com.poziomki.app.ui.designsystem.components.ScreenHeader
import com.poziomki.app.ui.navigation.LocalNavBarPadding
import com.poziomki.app.ui.designsystem.theme.Background
import com.poziomki.app.ui.designsystem.theme.PoziomkiTheme
import com.poziomki.app.ui.designsystem.theme.Primary
import com.poziomki.app.ui.designsystem.theme.TextPrimary
import com.poziomki.app.ui.designsystem.theme.TextSecondary
import com.poziomki.app.ui.feature.home.messages.MessagesRoomFilter
import com.poziomki.app.ui.feature.home.messages.RoomRow
import com.poziomki.app.ui.feature.home.messages.filterMessagesRooms
import com.poziomki.app.ui.feature.home.messages.resolveRoomDisplayName
import com.poziomki.app.ui.feature.home.messages.resolveRoomProfilePicture
import com.poziomki.app.ui.feature.home.messages.roomFilterTabs
import org.koin.compose.viewmodel.koinViewModel
import com.adamglin.PhosphorIcons
import com.adamglin.phosphoricons.Bold
import com.adamglin.phosphoricons.bold.Bell
import com.adamglin.phosphoricons.bold.PencilSimple

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun MessagesScreen(
    onNavigateToChat: (String) -> Unit,
    onNavigateToNewChat: () -> Unit,
    onNavigateToProfile: (String) -> Unit = {},
    viewModel: MessagesViewModel = koinViewModel(),
) {
    val state by viewModel.state.collectAsState()
    var selectedFilter by remember { mutableStateOf(MessagesRoomFilter.All) }

    val unreadTotal = state.rooms.sumOf { it.unreadCount }
    val filteredRooms = state.rooms.filterMessagesRooms(
        selectedFilter,
        state.searchQuery,
        state.eventRoomIds,
        state.searchMatchingRoomIds,
    )
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
                            imageVector = PhosphorIcons.Bold.Bell,
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
                        imageVector = PhosphorIcons.Bold.PencilSimple,
                        contentDescription = "Nowa wiadomość",
                        tint = TextSecondary,
                        modifier = Modifier.size(22.dp),
                    )
                }
            }
            PoziomkiSearchBar(
                query = state.searchQuery,
                onQueryChange = { viewModel.onSearchQueryChanged(it) },
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
                                contentPadding = PaddingValues(bottom = LocalNavBarPadding.current),
                            ) {
                                items(filteredRooms, key = { it.roomId }) { room ->
                                    val profilePicture =
                                        resolveRoomProfilePicture(
                                            room = room,
                                            profilePictures = state.profilePictures,
                                            profilePicturesByName = state.profilePicturesByName,
                                            eventRoomAvatars = state.eventRoomAvatars,
                                        )
                                    val displayNameOverride =
                                        if (room.roomId in state.eventRoomIds) {
                                            null
                                        } else {
                                            resolveRoomDisplayName(
                                                room = room,
                                                displayNameOverrides = state.displayNameOverrides,
                                            )
                                        }
                                    RoomRow(
                                        room = room,
                                        profilePictureUrl = profilePicture,
                                        displayNameOverride = displayNameOverride,
                                        onClick = { onNavigateToChat(room.roomId) },
                                        onAvatarClick =
                                            room.directUserId?.let { userId ->
                                                { onNavigateToProfile(userId) }
                                            },
                                    )
                                }
                            }
                        }
                    }
                }
            }
        }

        // FAB: new message
        FloatingActionButton(
            onClick = onNavigateToNewChat,
            containerColor = Primary,
            contentColor = Color.White,
            shape = CircleShape,
            modifier = Modifier
                .align(Alignment.BottomEnd)
                .padding(
                    end = PoziomkiTheme.spacing.lg,
                    bottom = LocalNavBarPadding.current + 24.dp,
                ),
        ) {
            Icon(
                PhosphorIcons.Bold.PencilSimple,
                contentDescription = "Nowa wiadomo\u015b\u0107",
                modifier = Modifier.size(24.dp),
            )
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
