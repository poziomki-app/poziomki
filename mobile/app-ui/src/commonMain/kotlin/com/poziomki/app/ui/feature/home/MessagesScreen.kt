package com.poziomki.app.ui.feature.home

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.pulltorefresh.PullToRefreshBox
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import com.poziomki.app.ui.designsystem.components.EmptyView
import com.poziomki.app.ui.designsystem.components.FilterTabs
import com.poziomki.app.ui.designsystem.components.LoadingView
import com.poziomki.app.ui.designsystem.components.SearchableScreenHeader
import com.poziomki.app.ui.designsystem.theme.Background
import com.poziomki.app.ui.designsystem.theme.PoziomkiTheme
import com.poziomki.app.ui.feature.home.messages.MessagesRoomFilter
import com.poziomki.app.ui.feature.home.messages.RoomRow
import com.poziomki.app.ui.feature.home.messages.filterMessagesRooms
import com.poziomki.app.ui.feature.home.messages.resolveRoomProfilePicture
import com.poziomki.app.ui.feature.home.messages.roomFilterTabs
import com.poziomki.app.ui.navigation.LocalNavBarPadding
import org.koin.compose.viewmodel.koinViewModel

@OptIn(ExperimentalMaterial3Api::class)
@Composable
@Suppress("LongMethod")
fun MessagesScreen(
    onNavigateToChat: (String, String?) -> Unit,
    onNavigateToProfile: (String) -> Unit = {},
    profileAvatarAction: @Composable () -> Unit = {},
    viewModel: MessagesViewModel = koinViewModel(),
) {
    val state by viewModel.state.collectAsState()
    var selectedFilter by remember { mutableStateOf(MessagesRoomFilter.All) }
    var searchActive by remember { mutableStateOf(false) }

    val filteredRooms =
        state.rooms.filterMessagesRooms(
            selectedFilter,
            state.searchQuery,
            state.eventRoomIds,
            state.searchMatchingRoomIds,
        )
    val roomFilterTabs = roomFilterTabs()

    Column(
        modifier =
            Modifier
                .fillMaxSize()
                .background(Background),
    ) {
        SearchableScreenHeader(
            title = "wiadomości",
            searchQuery = state.searchQuery,
            onSearchQueryChange = { viewModel.onSearchQueryChanged(it) },
            searchActive = searchActive,
            onSearchActiveChange = { searchActive = it },
        ) {
            profileAvatarAction()
        }
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
                EmptyView("brak rozmów")
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
                                        profilePicturesByName = state.profilePicturesByName,
                                        eventRoomAvatars = state.eventRoomAvatars,
                                    )
                                RoomRow(
                                    room = room,
                                    profilePictureUrl = profilePicture,
                                    isEvent = room.roomId in state.eventRoomIds,
                                    onClick = { onNavigateToChat(room.roomId, profilePicture) },
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
}
