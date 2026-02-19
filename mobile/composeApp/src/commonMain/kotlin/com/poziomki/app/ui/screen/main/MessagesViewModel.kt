package com.poziomki.app.ui.screen.main

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import com.poziomki.app.chat.matrix.api.MatrixClient
import com.poziomki.app.chat.matrix.api.MatrixClientState
import com.poziomki.app.chat.matrix.api.MatrixRoomSummary
import com.poziomki.app.data.repository.MatchProfileRepository
import com.poziomki.app.ui.screen.main.messages.MessagesUiState
import com.poziomki.app.util.matrixLocalpartFromUserId
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch

class MessagesViewModel(
    private val matrixClient: MatrixClient,
    private val matchProfileRepository: MatchProfileRepository,
) : ViewModel() {
    private val _state = MutableStateFlow(MessagesUiState(isLoading = true))
    val state: StateFlow<MessagesUiState> = _state.asStateFlow()

    init {
        observeClientState()
        observeRooms()
        observeProfilePictures()
        refresh()
        refreshProfilePictures()
    }

    fun refresh() {
        viewModelScope.launch {
            if (_state.value.rooms.isEmpty()) {
                _state.update { it.copy(isLoading = true, error = null) }
            }

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

    fun pullToRefresh() {
        viewModelScope.launch {
            _state.update { it.copy(isRefreshing = true) }

            matrixClient.ensureStarted().onFailure { throwable ->
                _state.update {
                    it.copy(
                        isRefreshing = false,
                        refreshError = throwable.message ?: "Failed to initialize Matrix",
                    )
                }
                return@launch
            }

            matrixClient.refreshRooms().onFailure { throwable ->
                _state.update {
                    it.copy(
                        isRefreshing = false,
                        refreshError = throwable.message ?: "Failed to refresh Matrix room list",
                    )
                }
                return@launch
            }

            matchProfileRepository.refreshProfiles(forceRefresh = true)
            _state.update { it.copy(isRefreshing = false) }
        }
    }

    fun clearError() {
        _state.update { it.copy(error = null) }
    }

    fun clearRefreshError() {
        _state.update { it.copy(refreshError = null) }
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
                val deduplicatedRooms = deduplicateRooms(rooms)
                _state.update { current ->
                    current.copy(
                        rooms = deduplicatedRooms,
                        isLoading = if (deduplicatedRooms.isNotEmpty()) false else current.isLoading,
                    )
                }
            }
        }
    }

    private fun observeProfilePictures() {
        viewModelScope.launch {
            matchProfileRepository.observeProfilePicturesByUserId().collect { userIdToPic ->
                val pictureMap = mutableMapOf<String, String>()
                userIdToPic.forEach { (userId, pic) ->
                    val localpart = matrixLocalpartFromUserId(userId)
                    pictureMap[userId] = pic
                    pictureMap[userId.lowercase()] = pic
                    pictureMap[localpart] = pic
                    pictureMap["@$localpart"] = pic
                }
                _state.update { it.copy(profilePictures = pictureMap) }
            }
        }
    }

    private fun refreshProfilePictures() {
        viewModelScope.launch {
            matchProfileRepository.refreshProfiles()
        }
    }

    private fun deduplicateRooms(rooms: List<MatrixRoomSummary>): List<MatrixRoomSummary> {
        val deduplicated = LinkedHashMap<String, MatrixRoomSummary>()
        rooms.forEach { room ->
            val key =
                if (room.isDirect) {
                    room.directUserId
                        ?.trim()
                        ?.lowercase()
                        ?.ifBlank { null }
                        ?: room.roomId
                } else {
                    room.roomId
                }
            val existing = deduplicated[key]
            if (existing == null) {
                deduplicated[key] = room
            } else {
                val roomTs = room.latestTimestampMillis ?: Long.MIN_VALUE
                val existingTs = existing.latestTimestampMillis ?: Long.MIN_VALUE
                if (roomTs > existingTs) {
                    deduplicated[key] = room
                }
            }
        }
        return deduplicated.values.toList()
    }
}
