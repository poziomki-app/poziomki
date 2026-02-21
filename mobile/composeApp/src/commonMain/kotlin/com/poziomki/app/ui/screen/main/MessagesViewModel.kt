package com.poziomki.app.ui.screen.main

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import com.poziomki.app.chat.matrix.api.MatrixClient
import com.poziomki.app.chat.matrix.api.MatrixClientState
import com.poziomki.app.data.repository.EventRepository
import com.poziomki.app.data.repository.MatchProfileRepository
import com.poziomki.app.ui.screen.main.messages.buildProfilePicturesByName
import com.poziomki.app.ui.screen.main.messages.buildProfilePicturesByUserId
import com.poziomki.app.ui.screen.main.messages.deduplicateRooms
import com.poziomki.app.ui.screen.main.messages.MessagesUiState
import kotlinx.coroutines.delay
import kotlinx.coroutines.isActive
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch

class MessagesViewModel(
    private val matrixClient: MatrixClient,
    private val matchProfileRepository: MatchProfileRepository,
    private val eventRepository: EventRepository,
) : ViewModel() {
    private companion object {
        const val PROFILE_PICTURE_REFRESH_INTERVAL_MS = 30 * 60 * 1000L
    }

    private val _state = MutableStateFlow(MessagesUiState(isLoading = true))
    val state: StateFlow<MessagesUiState> = _state.asStateFlow()

    init {
        observeClientState()
        observeRooms()
        observeEventRooms()
        observeProfilePictures()
        observeProfilePicturesByName()
        refresh()
        refreshProfilePictures()
        refreshProfilePicturesPeriodically()
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
                _state.update { it.copy(profilePictures = buildProfilePicturesByUserId(userIdToPic)) }
            }
        }
    }

    private fun observeProfilePicturesByName() {
        viewModelScope.launch {
            matchProfileRepository.observeProfiles().collect { profiles ->
                _state.update { it.copy(profilePicturesByName = buildProfilePicturesByName(profiles)) }
            }
        }
    }

    private fun observeEventRooms() {
        viewModelScope.launch {
            eventRepository.observeEventConversationIds().collect { eventRoomIds ->
                _state.update { it.copy(eventRoomIds = eventRoomIds) }
            }
        }
    }

    private fun refreshProfilePictures() {
        viewModelScope.launch {
            matchProfileRepository.refreshProfiles()
        }
    }

    private fun refreshProfilePicturesPeriodically() {
        viewModelScope.launch {
            while (isActive) {
                delay(PROFILE_PICTURE_REFRESH_INTERVAL_MS)
                matchProfileRepository.refreshProfiles(forceRefresh = true)
            }
        }
    }
}
