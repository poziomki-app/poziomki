package com.poziomki.app.ui.screen.main.messages

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
)
