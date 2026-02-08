package com.poziomki.app.ui.screen.main

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import com.poziomki.app.chat.matrix.api.MatrixClient
import com.poziomki.app.chat.matrix.api.MatrixClientState
import com.poziomki.app.ui.screen.main.messages.MessagesUiState
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch

class MessagesViewModel(
    private val matrixClient: MatrixClient,
) : ViewModel() {
    private val _state = MutableStateFlow(MessagesUiState(isLoading = true))
    val state: StateFlow<MessagesUiState> = _state.asStateFlow()

    init {
        observeClientState()
        observeRooms()
        refresh()
    }

    fun refresh() {
        viewModelScope.launch {
            _state.update { it.copy(isLoading = true, error = null) }

            matrixClient.ensureStarted().onFailure { throwable ->
                _state.update {
                    it.copy(
                        isLoading = false,
                        error = throwable.message ?: "Failed to initialize Matrix",
                    )
                }
                return@launch
            }

            matrixClient.refreshRooms().onFailure { throwable ->
                _state.update {
                    it.copy(
                        isLoading = false,
                        error = throwable.message ?: "Failed to refresh Matrix room list",
                    )
                }
                return@launch
            }

            _state.update { it.copy(isLoading = false) }
        }
    }

    fun clearError() {
        _state.update { it.copy(error = null) }
    }

    private fun observeClientState() {
        viewModelScope.launch {
            matrixClient.state.collect { matrixState ->
                _state.update { current ->
                    current.copy(
                        matrixState = matrixState,
                        error =
                            when (matrixState) {
                                is MatrixClientState.Error -> matrixState.message
                                else -> current.error
                            },
                    )
                }
            }
        }
    }

    private fun observeRooms() {
        viewModelScope.launch {
            matrixClient.rooms.collect { rooms ->
                _state.update { current ->
                    current.copy(
                        rooms = rooms,
                        isLoading = false,
                    )
                }
            }
        }
    }
}
