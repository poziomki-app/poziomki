package com.poziomki.app.ui.feature.home.messages

import com.poziomki.app.chat.api.ChatClientState
import com.poziomki.app.chat.api.RoomSummary

data class MessagesUiState(
    val rooms: List<RoomSummary> = emptyList(),
    val chatState: ChatClientState = ChatClientState.Idle,
    val isLoading: Boolean = false,
    val isRefreshing: Boolean = false,
    val profilePicturesByName: Map<String, String> = emptyMap(),
    val eventRoomIds: Set<String> = emptySet(),
    val eventRoomAvatars: Map<String, String> = emptyMap(),
    val searchQuery: String = "",
    val searchMatchingRoomIds: Set<String>? = null,
)
