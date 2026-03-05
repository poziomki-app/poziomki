package com.poziomki.app.ui.feature.home

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import com.poziomki.app.chat.matrix.api.MatrixClient
import com.poziomki.app.chat.matrix.api.MatrixClientState
import com.poziomki.app.chat.matrix.api.MatrixRoomSummary
import com.poziomki.app.connectivity.ConnectivityMonitor
import com.poziomki.app.data.repository.EventRepository
import com.poziomki.app.data.repository.MatchProfileRepository
import com.poziomki.app.network.ApiResult
import com.poziomki.app.network.ApiService
import com.poziomki.app.ui.feature.home.messages.MessagesUiState
import com.poziomki.app.ui.feature.home.messages.buildDisplayNameOverrides
import com.poziomki.app.ui.feature.home.messages.buildProfilePicturesByName
import com.poziomki.app.ui.feature.home.messages.buildProfilePicturesByUserId
import kotlinx.coroutines.Job
import kotlinx.coroutines.async
import kotlinx.coroutines.awaitAll
import kotlinx.coroutines.coroutineScope
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
    private val apiService: ApiService,
    private val connectivityMonitor: ConnectivityMonitor,
) : ViewModel() {
    private companion object {
        const val PROFILE_PICTURE_REFRESH_INTERVAL_MS = 30 * 60 * 1000L
        const val EMPTY_ROOMS_FALLBACK_MS = 15_000L
        const val PREVIEW_WARMUP_BATCH_SIZE = 4
    }

    private var emptyRoomsFallbackJob: Job? = null
    private var searchJob: Job? = null
    private var warmupPreviewsJob: Job? = null
    private val queuedWarmupRoomIds = ArrayDeque<String>()
    private val queuedWarmupRoomIdSet = mutableSetOf<String>()
    private val warmedPreviewRoomIds = mutableSetOf<String>()
    private var pendingConnectivityRetry = false

    private val _state = MutableStateFlow(MessagesUiState(isLoading = true))
    val state: StateFlow<MessagesUiState> = _state.asStateFlow()

    private val roomSortComparator: Comparator<MatrixRoomSummary> =
        compareByDescending<MatrixRoomSummary> { it.latestTimestampMillis ?: Long.MIN_VALUE }
            .thenByDescending { it.unreadCount }
            .thenBy { stableRoomKey(it) }
            .thenBy { it.roomId }

    init {
        val cachedRooms = deduplicateAndSortRooms(matrixClient.rooms.value)
        if (cachedRooms.isNotEmpty()) {
            _state.value = _state.value.copy(rooms = cachedRooms, isLoading = false)
        }
        observeConnectivity()
        observeClientState()
        observeRooms()
        observeEventRooms()
        observeEventRoomAvatars()
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
                _state.update { it.copy(isLoading = true) }
            }

            matrixClient.ensureStarted().onFailure {
                pendingConnectivityRetry = true
                _state.update { current -> current.copy(isLoading = false) }
                return@launch
            }

            matrixClient.refreshRooms().onFailure {
                pendingConnectivityRetry = true
                _state.update { current -> current.copy(isLoading = false) }
                return@launch
            }

            pendingConnectivityRetry = false
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

            matrixClient.ensureStarted().onFailure {
                pendingConnectivityRetry = true
                _state.update { it.copy(isRefreshing = false) }
                return@launch
            }

            matrixClient.refreshRooms().onFailure {
                pendingConnectivityRetry = true
                _state.update { it.copy(isRefreshing = false) }
                return@launch
            }

            pendingConnectivityRetry = false
            matchProfileRepository.refreshProfiles(forceRefresh = true)
            _state.update { it.copy(isRefreshing = false) }
        }
    }

    fun onSearchQueryChanged(query: String) {
        _state.update { it.copy(searchQuery = query) }
        searchJob?.cancel()

        if (query.length < 2) {
            _state.update { it.copy(searchMatchingRoomIds = null) }
            return
        }

        searchJob = viewModelScope.launch {
            delay(300)
            when (val result = apiService.searchMessageRooms(query)) {
                is ApiResult.Success -> {
                    _state.update { it.copy(searchMatchingRoomIds = result.data.roomIds.toSet()) }
                }
                is ApiResult.Error -> {
                    _state.update { it.copy(searchMatchingRoomIds = null) }
                }
            }
        }
    }

    private fun observeConnectivity() {
        viewModelScope.launch {
            connectivityMonitor.isOnline.collect { isOnline ->
                val shouldRetry = isOnline && pendingConnectivityRetry
                if (shouldRetry) {
                    pendingConnectivityRetry = false
                    refresh()
                }
            }
        }
    }

    private fun observeClientState() {
        viewModelScope.launch {
            matrixClient.state.collect { matrixState ->
                _state.update { current -> current.copy(matrixState = matrixState) }
            }
        }
    }

    private fun observeRooms() {
        viewModelScope.launch {
            matrixClient.rooms.collect { rooms ->
                val deduplicatedRooms = deduplicateAndSortRooms(rooms)
                warmupMissingRoomPreviews(deduplicatedRooms)
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

    private fun warmupMissingRoomPreviews(rooms: List<MatrixRoomSummary>) {
        rooms.forEach { room ->
            if (!room.latestMessage.isNullOrBlank()) return@forEach
            val roomId = room.roomId
            if (roomId in warmedPreviewRoomIds || roomId in queuedWarmupRoomIdSet) return@forEach
            queuedWarmupRoomIds.addLast(roomId)
            queuedWarmupRoomIdSet += roomId
        }
        if (queuedWarmupRoomIds.isEmpty()) return
        if (warmupPreviewsJob?.isActive == true) return

        warmupPreviewsJob =
            viewModelScope.launch {
                while (queuedWarmupRoomIds.isNotEmpty()) {
                    val batch = mutableListOf<String>()
                    repeat(PREVIEW_WARMUP_BATCH_SIZE) {
                        val roomId = queuedWarmupRoomIds.removeFirstOrNull() ?: return@repeat
                        queuedWarmupRoomIdSet -= roomId
                        batch += roomId
                    }
                    if (batch.isEmpty()) continue

                    coroutineScope {
                        batch
                            .map { roomId ->
                                async {
                                    runCatching { matrixClient.getJoinedRoom(roomId) }
                                    warmedPreviewRoomIds += roomId
                                }
                            }.awaitAll()
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


    private fun observeEventRoomAvatars() {
        viewModelScope.launch {
            eventRepository.observeEvents().collect { events ->
                val avatars =
                    events
                        .filter { it.isAttending && it.conversationId != null && it.coverImage != null }
                        .associate { it.conversationId!! to it.coverImage!! }
                _state.update { it.copy(eventRoomAvatars = avatars) }
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
        if (room.isDirect && room.roomId !in _state.value.eventRoomIds) {
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
