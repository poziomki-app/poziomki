package com.poziomki.app.ui.feature.home.messages

import com.poziomki.app.chat.matrix.api.MatrixClientState
import com.poziomki.app.chat.matrix.api.MatrixRoomSummary

data class MessagesUiState(
    val rooms: List<MatrixRoomSummary> = emptyList(),
    val matrixState: MatrixClientState = MatrixClientState.Idle,
    val isLoading: Boolean = false,
    val isRefreshing: Boolean = false,
    val error: String? = null,
    val refreshError: String? = null,
    val profilePictures: Map<String, String> = emptyMap(),
    val profilePicturesByName: Map<String, String> = emptyMap(),
    val eventRoomIds: Set<String> = emptySet(),
)
