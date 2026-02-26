package com.poziomki.app.ui.feature.home

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import com.poziomki.app.chat.matrix.api.MatrixClient
import com.poziomki.app.chat.matrix.api.MatrixClientState
import com.poziomki.app.chat.matrix.api.MatrixRoomSummary
import com.poziomki.app.data.repository.EventRepository
import com.poziomki.app.data.repository.MatchProfileRepository
import com.poziomki.app.ui.feature.home.messages.MessagesUiState
import com.poziomki.app.ui.feature.home.messages.buildDisplayNameOverrides
import com.poziomki.app.ui.feature.home.messages.buildProfilePicturesByName
import com.poziomki.app.ui.feature.home.messages.buildProfilePicturesByUserId
import kotlinx.coroutines.Job
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.isActive
import kotlinx.coroutines.launch

class MessagesViewModel(
    private val matrixClient: MatrixClient,
    private val matchProfileRepository: MatchProfileRepository,
    private val eventRepository: EventRepository,
) : ViewModel() {
    private companion object {
        const val PROFILE_PICTURE_REFRESH_INTERVAL_MS = 30 * 60 * 1000L
        const val EMPTY_ROOMS_FALLBACK_MS = 15_000L
    }

    private var emptyRoomsFallbackJob: Job? = null

    private val _state = MutableStateFlow(MessagesUiState(isLoading = true))
    val state: StateFlow<MessagesUiState> = _state.asStateFlow()

    private val roomSortComparator: Comparator<MatrixRoomSummary> =
        compareByDescending<MatrixRoomSummary> { it.latestTimestampMillis ?: Long.MIN_VALUE }
            .thenByDescending { it.unreadCount }
            .thenBy { stableRoomKey(it) }
            .thenBy { it.roomId }

    init {
        observeClientState()
        observeRooms()
        observeEventRooms()
        observeProfilePictures()
        observeProfilePicturesByName()
        observeDisplayNameOverrides()
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

            _state.update { current ->
                if (current.rooms.isNotEmpty()) {
                    current.copy(isLoading = false)
                } else {
                    current // keep isLoading = true, let observeRooms() handle it
                }
            }
            scheduleEmptyRoomsFallback()
        }
    }

    private fun scheduleEmptyRoomsFallback() {
        emptyRoomsFallbackJob?.cancel()
        emptyRoomsFallbackJob = viewModelScope.launch {
            delay(EMPTY_ROOMS_FALLBACK_MS)
            _state.update { current ->
                if (current.isLoading && current.rooms.isEmpty()) {
                    current.copy(isLoading = false)
                } else {
                    current
                }
            }
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
                val deduplicatedRooms = deduplicateAndSortRooms(rooms)
                _state.update { current ->
                    if (current.rooms == deduplicatedRooms && (!current.isLoading || deduplicatedRooms.isEmpty())) {
                        return@update current
                    }
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

    private fun observeDisplayNameOverrides() {
        viewModelScope.launch {
            matchProfileRepository.observeProfiles().collect { profiles ->
                _state.update { it.copy(displayNameOverrides = buildDisplayNameOverrides(profiles)) }
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

    private fun deduplicateAndSortRooms(rooms: List<MatrixRoomSummary>): List<MatrixRoomSummary> {
        val deduplicated = LinkedHashMap<String, MatrixRoomSummary>()
        rooms.forEach { room ->
            val key = stableRoomKey(room)
            val existing = deduplicated[key]
            if (existing == null) {
                deduplicated[key] = room
            } else {
                if (isPreferredRoomCandidate(candidate = room, current = existing)) {
                    deduplicated[key] = room
                }
            }
        }
        return deduplicated.values.sortedWith(roomSortComparator)
    }

    private fun stableRoomKey(room: MatrixRoomSummary): String =
        if (room.isDirect) {
            room.directUserId
                ?.trim()
                ?.lowercase()
                ?.ifBlank { null }
                ?: room.roomId
        } else {
            room.roomId
        }

    private fun isPreferredRoomCandidate(
        candidate: MatrixRoomSummary,
        current: MatrixRoomSummary,
    ): Boolean {
        val candidateTs = candidate.latestTimestampMillis ?: Long.MIN_VALUE
        val currentTs = current.latestTimestampMillis ?: Long.MIN_VALUE
        if (candidateTs != currentTs) return candidateTs > currentTs
        if (candidate.unreadCount != current.unreadCount) return candidate.unreadCount > current.unreadCount

        val candidateHasMessage = !candidate.latestMessage.isNullOrBlank()
        val currentHasMessage = !current.latestMessage.isNullOrBlank()
        if (candidateHasMessage != currentHasMessage) return candidateHasMessage

        val candidateHasAvatar = !candidate.avatarUrl.isNullOrBlank()
        val currentHasAvatar = !current.avatarUrl.isNullOrBlank()
        if (candidateHasAvatar != currentHasAvatar) return candidateHasAvatar

        return candidate.roomId < current.roomId
    }
}
