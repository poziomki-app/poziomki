package com.poziomki.app.ui.screen.chat

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import com.poziomki.app.chat.matrix.api.MatrixClient
import com.poziomki.app.ui.screen.chat.model.NewChatUiState
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch

class NewChatViewModel(
    private val matrixClient: MatrixClient,
) : ViewModel() {
    private val _uiState = MutableStateFlow(NewChatUiState())
    val uiState: StateFlow<NewChatUiState> = _uiState.asStateFlow()

    fun onDmUserIdChanged(value: String) {
        _uiState.update { it.copy(dmUserId = value) }
    }

    fun onRoomNameChanged(value: String) {
        _uiState.update { it.copy(roomName = value) }
    }

    fun onInviteUserIdsRawChanged(value: String) {
        _uiState.update { it.copy(inviteUserIdsRaw = value) }
    }

    fun clearError() {
        _uiState.update { it.copy(error = null) }
    }

    fun createDm(onChatCreated: (String) -> Unit) {
        val userId = uiState.value.dmUserId.trim()
        if (userId.isEmpty()) {
            _uiState.update { it.copy(error = "Enter Matrix user id for DM") }
            return
        }

        viewModelScope.launch {
            _uiState.update { it.copy(isSubmitting = true, error = null) }

            matrixClient.ensureStarted().onFailure { throwable ->
                _uiState.update {
                    it.copy(
                        isSubmitting = false,
                        error = throwable.message ?: "Failed to initialize Matrix",
                    )
                }
                return@launch
            }

            matrixClient
                .createDM(userId)
                .onSuccess { roomId ->
                    _uiState.update { it.copy(isSubmitting = false, dmUserId = "") }
                    onChatCreated(roomId)
                }.onFailure { throwable ->
                    _uiState.update {
                        it.copy(
                            isSubmitting = false,
                            error = throwable.message ?: "Failed to create DM room",
                        )
                    }
                }
        }
    }

    fun createRoom(onChatCreated: (String) -> Unit) {
        val name = uiState.value.roomName.trim()
        val invitedUsers =
            uiState.value.inviteUserIdsRaw
                .split(",", "\n", " ")
                .map { it.trim() }
                .filter { it.isNotBlank() }
                .distinct()

        if (name.isEmpty() && invitedUsers.isEmpty()) {
            _uiState.update { it.copy(error = "Enter room name or at least one invited user") }
            return
        }

        viewModelScope.launch {
            _uiState.update { it.copy(isSubmitting = true, error = null) }

            matrixClient.ensureStarted().onFailure { throwable ->
                _uiState.update {
                    it.copy(
                        isSubmitting = false,
                        error = throwable.message ?: "Failed to initialize Matrix",
                    )
                }
                return@launch
            }

            matrixClient
                .createRoom(name = name, invitedUserIds = invitedUsers)
                .onSuccess { roomId ->
                    _uiState.update {
                        it.copy(
                            roomName = "",
                            inviteUserIdsRaw = "",
                            isSubmitting = false,
                        )
                    }
                    onChatCreated(roomId)
                }.onFailure { throwable ->
                    _uiState.update {
                        it.copy(
                            isSubmitting = false,
                            error = throwable.message ?: "Failed to create room",
                        )
                    }
                }
        }
    }
}
