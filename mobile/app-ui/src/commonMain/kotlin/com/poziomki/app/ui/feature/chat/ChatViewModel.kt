package com.poziomki.app.ui.feature.chat

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import com.poziomki.app.chat.api.ChatClient
import com.poziomki.app.chat.api.JoinedRoom
import com.poziomki.app.chat.api.RoomSummary
import com.poziomki.app.chat.api.Timeline
import com.poziomki.app.chat.api.TimelineItem
import com.poziomki.app.chat.api.TimelineMode
import com.poziomki.app.chat.draft.RoomComposerDraftStore
import com.poziomki.app.chat.timeline.TimelineController
import com.poziomki.app.data.repository.EventRepository
import com.poziomki.app.data.repository.MatchProfileRepository
import com.poziomki.app.data.repository.XpRepository
import com.poziomki.app.network.ApiResult
import com.poziomki.app.network.ApiService
import com.poziomki.app.ui.feature.chat.model.ChatUiState
import com.poziomki.app.ui.feature.chat.model.ComposerMode
import kotlinx.coroutines.Job
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.collectLatest
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock

class ChatViewModel(
    private val chatClient: ChatClient,
    private val roomComposerDraftStore: RoomComposerDraftStore,
    private val matchProfileRepository: MatchProfileRepository,
    private val eventRepository: EventRepository,
    private val apiService: ApiService,
    private val xpRepository: XpRepository,
) : ViewModel() {
    private companion object {
        const val TYPING_START_DEBOUNCE_MS = 300L
        const val TYPING_STOP_IDLE_MS = 5_000L
        const val TYPING_INDICATOR_TIMEOUT_MS = 8_000L
    }

    private val _uiState = MutableStateFlow(ChatUiState())
    val uiState: StateFlow<ChatUiState> = _uiState.asStateFlow()

    private val timelineController = TimelineController()

    private var boundRoomId: String? = null
    private var activeRoom: JoinedRoom? = null
    private var activeTimeline: Timeline? = null
    private var focusedTimeline: Timeline? = null
    private val bindMutex = Mutex()
    private var bindJob: Job? = null
    private var pendingRoomRetryJob: Job? = null
    private var bindingRoomId: String? = null
    private val roomJobs = mutableListOf<Job>()
    private val timelineJobs = mutableListOf<Job>()
    private var typingState = false
    private var typingStartJob: Job? = null
    private var typingStopJob: Job? = null
    private var typingIndicatorTimeoutJob: Job? = null
    private var lastVisibleTimelineIndex: Int? = null
    private var totalTimelineItemCount: Int = 0
    private var latestRoomSummaries: List<RoomSummary> = emptyList()
    private var activeDirectUserId: String? = null
    private var activeDirectProfileId: String? = null
    private var latestAvatarByName: Map<String, String> = emptyMap()
    private var eventCoverByRoomId: Map<String, String> = emptyMap()

    init {
        observeAvatarOverrides()
        observeEventCovers()
    }

    fun loadRoom(
        roomId: String,
        fallbackDisplayName: String? = null,
        fallbackDirectUserId: String? = null,
        fallbackProfileId: String? = null,
    ) {
        if (roomId.isBlank()) return
        if (roomId.length < 2) {
            _uiState.value = ChatUiState(error = "Invalid chat room id")
            return
        }
        val isAlreadyBound = boundRoomId == roomId && activeRoom != null
        val isBindingSameRoom = bindingRoomId == roomId
        val hasPendingBindForSameRoom = boundRoomId == roomId && bindJob?.isActive == true
        if (isAlreadyBound || isBindingSameRoom || hasPendingBindForSameRoom) return

        boundRoomId = roomId
        bindJob?.cancel()
        pendingRoomRetryJob?.cancel()
        bindJob =
            viewModelScope.launch {
                bindMutex.withLock {
                    if (boundRoomId != roomId) return@withLock
                    bindingRoomId = roomId
                    try {
                        activeDirectProfileId = fallbackProfileId
                        bindRoom(
                            roomId = roomId,
                            fallbackDisplayName = fallbackDisplayName,
                            fallbackDirectUserId = fallbackDirectUserId,
                        )
                    } finally {
                        if (bindingRoomId == roomId) {
                            bindingRoomId = null
                        }
                    }
                }
            }
    }

    private suspend fun awaitJoinedRoom(
        roomId: String,
        attempts: Int = 20,
        initialDelayMs: Long = 250L,
    ): JoinedRoom? {
        var currentDelay = initialDelayMs
        repeat(attempts) { attempt ->
            chatClient.getJoinedRoom(roomId)?.let { return it }
            if (attempt == 0 || attempt % 3 == 2) {
                runCatching { chatClient.refreshRooms() }
            }
            if (attempt < attempts - 1) {
                delay(currentDelay)
                currentDelay = (currentDelay * 3 / 2).coerceAtMost(2_000L)
            }
        }
        return null
    }

    fun onDraftChanged(value: String) {
        _uiState.update { it.copy(messageDraft = value) }
        currentDraftRoomId()?.let { roomId ->
            roomComposerDraftStore.saveDraft(roomId = roomId, draft = value)
        }
        if (value.isBlank()) {
            stopTyping(notifyRoom = true)
            return
        }

        if (typingState) {
            scheduleTypingStop()
            return
        }

        scheduleTypingStart()
    }

    fun focusOnEvent(eventId: String) {
        if (eventId.isBlank()) return
        timelineController.focusOnEvent(eventId)
    }

    fun enterLiveTimeline() {
        timelineController.enterLive()
    }

    fun sendMessage() {
        val body = _uiState.value.messageDraft.trim()
        if (body.isEmpty()) return
        val composerMode = _uiState.value.composerMode
        val roomId = currentDraftRoomId()
        viewModelScope.launch {
            val timeline = awaitTimelineForSend()
            if (timeline == null) {
                _uiState.update {
                    it.copy(error = "Conversation is still connecting. Please wait a few seconds.")
                }
                return@launch
            }
            _uiState.update {
                it.copy(
                    messageDraft = "",
                    composerMode = ComposerMode.NewMessage,
                    error = null,
                )
            }

            roomId?.let(roomComposerDraftStore::clearDraft)
            stopTyping(notifyRoom = true)
            val result =
                when (composerMode) {
                    ComposerMode.NewMessage -> timeline.sendMessage(body)
                    is ComposerMode.Reply -> timeline.sendReply(composerMode.eventId, body)
                    is ComposerMode.Edit -> timeline.edit(composerMode.eventOrTransactionId, body)
                }
            result.onSuccess {
                // Award say_hi XP — backend idempotent per day, safe to fire every send.
                runCatching { xpRepository.claimTask("say_hi") }
            }
            result.onFailure { throwable ->
                _uiState.update {
                    it.copy(
                        messageDraft = body,
                        composerMode = composerMode,
                        error = throwable.message ?: "Failed to send message",
                    )
                }
                roomId?.let { failedRoomId ->
                    roomComposerDraftStore.saveDraft(roomId = failedRoomId, draft = body)
                }
            }
        }
    }

    fun toggleReaction(
        eventOrTransactionId: String,
        emoji: String,
    ) {
        viewModelScope.launch {
            val timeline = activeTimeline ?: return@launch

            // Enforce one reaction per user: remove existing different reaction first
            val event =
                _uiState.value.timelineItems
                    .filterIsInstance<TimelineItem.Event>()
                    .find { it.eventOrTransactionId == eventOrTransactionId }
            event
                ?.reactions
                ?.filter { it.reactedByMe && it.emoji != emoji }
                ?.forEach { existing ->
                    timeline.toggleReaction(eventOrTransactionId, existing.emoji)
                }

            timeline.toggleReaction(eventOrTransactionId, emoji).onFailure { throwable ->
                _uiState.update {
                    it.copy(error = throwable.message ?: "Failed to toggle reaction")
                }
            }
        }
    }

    fun paginateBackwards() {
        viewModelScope.launch {
            val timeline = activeTimeline ?: return@launch
            timeline.paginateBackwards().onFailure { throwable ->
                _uiState.update {
                    it.copy(error = throwable.message ?: "Failed to load older messages")
                }
            }
        }
    }

    fun markAsRead() {
        viewModelScope.launch {
            runCatching { activeTimeline?.markAsRead() }
        }
    }

    fun onTimelineViewportChanged(firstVisibleItemIndex: Int?) {
        lastVisibleTimelineIndex = firstVisibleItemIndex
        _uiState.update { current ->
            current.copy(
                isAwayFromLatest = isAwayFromLatest(firstVisibleItemIndex = firstVisibleItemIndex, totalItems = totalTimelineItemCount),
            )
        }
    }

    fun jumpToLatestHandled() {
        _uiState.update {
            it.copy(
                isAwayFromLatest = false,
                unreadBelowCount = 0,
            )
        }
        markAsRead()
    }

    fun startReply(event: TimelineItem.Event) {
        val eventId = event.eventId ?: return
        _uiState.update {
            it.copy(
                composerMode =
                    ComposerMode.Reply(
                        eventId = eventId,
                        senderDisplayName = event.senderDisplayName,
                        bodyPreview = event.body,
                    ),
            )
        }
    }

    fun startEdit(event: TimelineItem.Event) {
        if (!event.isEditable) return
        _uiState.update {
            it.copy(
                messageDraft = event.body,
                composerMode =
                    ComposerMode.Edit(
                        eventOrTransactionId = event.eventOrTransactionId,
                        originalBody = event.body,
                    ),
            )
        }
    }

    fun cancelComposerMode() {
        _uiState.update {
            it.copy(
                composerMode = ComposerMode.NewMessage,
            )
        }
    }

    fun redactEvent(eventOrTransactionId: String) {
        viewModelScope.launch {
            val timeline = activeTimeline ?: return@launch
            timeline.redact(eventOrTransactionId).onFailure { throwable ->
                _uiState.update {
                    it.copy(error = throwable.message ?: "Failed to delete message")
                }
            }
        }
    }

    fun clearError() {
        _uiState.update { it.copy(error = null) }
    }

    fun blockUser() {
        val profileId = activeDirectProfileId ?: activeDirectUserId ?: return
        viewModelScope.launch {
            when (apiService.blockProfile(profileId)) {
                is ApiResult.Success -> _uiState.update { it.copy(isBlocked = true) }
                is ApiResult.Error -> _uiState.update { it.copy(error = "Nie udało się zablokować") }
            }
        }
    }

    fun unblockUser() {
        val profileId = activeDirectProfileId ?: activeDirectUserId ?: return
        viewModelScope.launch {
            when (apiService.unblockProfile(profileId)) {
                is ApiResult.Success -> _uiState.update { it.copy(isBlocked = false) }
                is ApiResult.Error -> _uiState.update { it.copy(error = "Nie udało się odblokować") }
            }
        }
    }

    fun reportConversation(
        reason: String,
        description: String?,
    ) {
        val roomId = boundRoomId ?: return
        viewModelScope.launch {
            when (apiService.reportConversation(roomId, reason, description)) {
                is ApiResult.Success -> { /* success — UI will dismiss the dialog */ }

                is ApiResult.Error -> {
                    _uiState.update { it.copy(error = "Nie udało się zgłosić") }
                }
            }
        }
    }

    fun removeConversation() {
        val roomId = boundRoomId ?: return
        viewModelScope.launch {
            chatClient.hideConversation(roomId)
        }
    }

    fun toggleSearch() {
        _uiState.update {
            if (it.isSearchActive) {
                it.copy(
                    isSearchActive = false,
                    searchQuery = "",
                    searchMatchIndices = emptyList(),
                    currentSearchMatchIndex = -1,
                )
            } else {
                it.copy(isSearchActive = true)
            }
        }
    }

    fun onSearchQueryChanged(query: String) {
        val items = _uiState.value.timelineItems
        val matchIndices =
            if (query.length >= 2) {
                items.mapIndexedNotNull { index, item ->
                    if (item is TimelineItem.Event && item.body.contains(query, ignoreCase = true)) index else null
                }
            } else {
                emptyList()
            }
        _uiState.update {
            it.copy(
                searchQuery = query,
                searchMatchIndices = matchIndices,
                currentSearchMatchIndex = if (matchIndices.isNotEmpty()) 0 else -1,
            )
        }
    }

    fun nextSearchMatch() {
        _uiState.update {
            if (it.searchMatchIndices.isEmpty()) return@update it
            val next = (it.currentSearchMatchIndex + 1) % it.searchMatchIndices.size
            it.copy(currentSearchMatchIndex = next)
        }
    }

    fun prevSearchMatch() {
        _uiState.update {
            if (it.searchMatchIndices.isEmpty()) return@update it
            val prev = (it.currentSearchMatchIndex - 1 + it.searchMatchIndices.size) % it.searchMatchIndices.size
            it.copy(currentSearchMatchIndex = prev)
        }
    }

    suspend fun resolveDisplayNames(userIds: List<String>): Map<String, String> {
        val room = activeRoom ?: return emptyMap()
        return userIds.associateWith { userId ->
            room.getMemberDisplayName(userId) ?: userId
        }
    }

    suspend fun resolveAvatarUrls(userIds: List<String>): Map<String, String> {
        val room = activeRoom ?: return emptyMap()
        return userIds
            .mapNotNull { userId ->
                room.getMemberAvatarUrl(userId)?.takeIf { avatar -> avatar.isNotBlank() }?.let { avatar ->
                    userId to avatar
                }
            }.toMap()
    }

    override fun onCleared() {
        bindJob?.cancel()
        pendingRoomRetryJob?.cancel()
        clearTypingTimers()
        typingIndicatorTimeoutJob?.cancel()
        focusedTimeline?.close()
        focusedTimeline = null
        timelineJobs.forEach { it.cancel() }
        timelineJobs.clear()
        roomJobs.forEach { it.cancel() }
        roomJobs.clear()
        super.onCleared()
    }

    private suspend fun bindRoom(
        roomId: String,
        fallbackDisplayName: String?,
        fallbackDirectUserId: String?,
    ) {
        focusedTimeline?.close()
        focusedTimeline = null
        activeTimeline = null
        pendingRoomRetryJob?.cancel()
        pendingRoomRetryJob = null
        timelineJobs.forEach { it.cancel() }
        timelineJobs.clear()
        roomJobs.forEach { it.cancel() }
        roomJobs.clear()
        activeRoom = null
        clearTypingTimers()
        typingState = false
        activeDirectUserId = fallbackDirectUserId?.takeIf { it.isNotBlank() }
        if (latestRoomSummaries.isEmpty()) {
            latestRoomSummaries = chatClient.rooms.value
        }
        val cachedTimeline =
            runCatching {
                chatClient.getRoomTimelineCache(roomId = roomId, limit = 500)
            }.getOrNull()
        val knownSummary = latestRoomSummaries.firstOrNull { it.roomId == roomId }
        val inferredDirectUserId = fallbackDirectUserId?.takeIf { it.isNotBlank() } ?: knownSummary?.directUserId
        val inferredIsDirect = knownSummary?.isDirect ?: (inferredDirectUserId != null)
        activeDirectUserId = inferredDirectUserId

        val avatarOverrides = _uiState.value.avatarOverrides
        val seededDisplayName =
            fallbackDisplayName
                ?.trim()
                ?.takeIf { it.isNotBlank() }
                ?: knownSummary?.displayName.orEmpty()
        _uiState.value =
            ChatUiState(
                roomId = roomId,
                roomDisplayName = seededDisplayName,
                roomAvatarUrl =
                    knownSummary?.avatarUrl
                        ?: inferredDirectUserId?.let { resolveAvatarOverride(it, avatarOverrides) },
                isDirectRoom = inferredIsDirect,
                directProfileId = activeDirectProfileId ?: activeDirectUserId,
                avatarOverrides = avatarOverrides,
                timelineItems = cachedTimeline?.items ?: emptyList(),
                isLoading = cachedTimeline?.items.isNullOrEmpty(),
            )
        lastVisibleTimelineIndex = null
        totalTimelineItemCount = 0

        chatClient.ensureStarted().getOrElse { throwable ->
            _uiState.update { current ->
                if (current.timelineItems.isNotEmpty()) {
                    current.copy(
                        isLoading = false,
                        error = null,
                    )
                } else {
                    current.copy(
                        isLoading = false,
                        error = throwable.message ?: "Failed to initialize chat",
                    )
                }
            }
            if (seededDisplayName.isNotBlank() || activeDirectUserId != null) {
                schedulePendingRoomRetry(
                    roomId = roomId,
                    fallbackDisplayName = fallbackDisplayName,
                    fallbackDirectUserId = activeDirectUserId,
                )
            }
            return
        }
        runCatching { chatClient.refreshRooms() }

        val room =
            awaitJoinedRoom(roomId) ?: run {
                val currentOverrides = _uiState.value.avatarOverrides
                if (!fallbackDirectUserId.isNullOrBlank() || seededDisplayName.isNotBlank()) {
                    _uiState.value =
                        ChatUiState(
                            roomId = roomId,
                            roomDisplayName = seededDisplayName,
                            roomAvatarUrl =
                                fallbackDirectUserId?.let { resolveAvatarOverride(it, currentOverrides) },
                            directProfileId = activeDirectProfileId ?: activeDirectUserId,
                            avatarOverrides = currentOverrides,
                            isLoading = false,
                            error = null,
                        )
                    schedulePendingRoomRetry(
                        roomId = roomId,
                        fallbackDisplayName = fallbackDisplayName,
                        fallbackDirectUserId = fallbackDirectUserId,
                    )
                } else {
                    _uiState.value =
                        ChatUiState(
                            roomId = roomId,
                            avatarOverrides = currentOverrides,
                            isLoading = false,
                            error = "Nie można jeszcze otworzyć tej rozmowy.",
                        )
                }
                return
            }

        activeRoom = room
        val restoredDraft = roomComposerDraftStore.getDraft(room.roomId)
        val initialSummary = latestRoomSummaries.firstOrNull { it.roomId == room.roomId }
        val initialDisplayName =
            resolvePreferredRoomDisplayName(
                roomId = room.roomId,
                summary = initialSummary,
                liveName = null,
                currentName = seededDisplayName,
                fallbackDisplayName = fallbackDisplayName,
            )
        val initialAvatar =
            resolveRoomAvatar(
                summary = initialSummary,
                overrides = _uiState.value.avatarOverrides,
                roomDisplayName = initialDisplayName,
                currentAvatar = null,
                directUserIdFallback = activeDirectUserId,
            )
        val initialIsDirect = initialSummary?.isDirect ?: (activeDirectUserId != null)
        _uiState.update {
            it.copy(
                roomId = room.roomId,
                roomDisplayName = initialDisplayName,
                roomAvatarUrl = initialAvatar,
                isDirectRoom = initialIsDirect,
                isBlocked = initialSummary?.isBlocked ?: false,
                timelineItems = cachedTimeline?.items ?: emptyList(),
                isAwayFromLatest = false,
                unreadBelowCount = 0,
                typingUserIds = emptyList(),
                typingDisplayNames = emptyList(),
                typingAvatarUrls = emptyList(),
                messageDraft = restoredDraft,
                composerMode = ComposerMode.NewMessage,
                isLoading = cachedTimeline?.items.isNullOrEmpty(),
                timelineMode = TimelineMode.Live,
                error = null,
            )
        }

        roomJobs +=
            viewModelScope.launch {
                room.displayName.collectLatest { name ->
                    _uiState.update { current ->
                        val summary = latestRoomSummaries.firstOrNull { it.roomId == room.roomId }
                        val resolvedName =
                            when {
                                summary?.isDirect == true && !summary.displayName.isNullOrBlank() -> {
                                    summary.displayName
                                }

                                else -> {
                                    resolvePreferredRoomDisplayName(
                                        roomId = room.roomId,
                                        summary = summary,
                                        liveName = name,
                                        currentName = current.roomDisplayName,
                                        fallbackDisplayName = fallbackDisplayName,
                                    )
                                }
                            }
                        current.copy(
                            roomDisplayName = resolvedName,
                            isDirectRoom = summary?.isDirect ?: current.isDirectRoom,
                            roomAvatarUrl =
                                resolveRoomAvatar(
                                    summary = summary,
                                    overrides = current.avatarOverrides,
                                    roomDisplayName = resolvedName,
                                    currentAvatar = current.roomAvatarUrl,
                                ),
                        )
                    }
                }
            }

        roomJobs +=
            viewModelScope.launch {
                chatClient.rooms.collectLatest { summaries ->
                    latestRoomSummaries = summaries
                    val summary = summaries.firstOrNull { it.roomId == room.roomId }
                    activeDirectUserId = summary?.directUserId ?: activeDirectUserId
                    _uiState.update { current ->
                        val resolvedName =
                            when {
                                summary?.isDirect == true && !summary.displayName.isNullOrBlank() -> {
                                    summary.displayName
                                }

                                else -> {
                                    resolvePreferredRoomDisplayName(
                                        roomId = room.roomId,
                                        summary = summary,
                                        liveName = null,
                                        currentName = current.roomDisplayName,
                                        fallbackDisplayName = fallbackDisplayName,
                                    )
                                }
                            }
                        current.copy(
                            roomDisplayName = resolvedName,
                            isDirectRoom = summary?.isDirect ?: current.isDirectRoom,
                            directProfileId = current.directProfileId ?: activeDirectProfileId ?: activeDirectUserId,
                            roomAvatarUrl =
                                resolveRoomAvatar(
                                    summary = summary,
                                    overrides = current.avatarOverrides,
                                    roomDisplayName = resolvedName,
                                    currentAvatar = current.roomAvatarUrl,
                                ),
                        )
                    }
                }
            }

        roomJobs +=
            viewModelScope.launch {
                room.typingUserIds.collect { typingUsers ->
                    typingIndicatorTimeoutJob?.cancel()
                    // Update immediately with IDs, then resolve names/avatars
                    _uiState.update { current ->
                        current.copy(
                            typingUserIds = typingUsers,
                            typingDisplayNames = typingUsers,
                            typingAvatarUrls = typingUsers.map { null },
                        )
                    }
                    if (typingUsers.isNotEmpty()) {
                        // Resolve display names and avatars in background
                        viewModelScope.launch {
                            val names = typingUsers.map { room.getMemberDisplayName(it) ?: it }
                            val avatars = typingUsers.map { room.getMemberAvatarUrl(it) }
                            _uiState.update { current ->
                                if (current.typingUserIds == typingUsers) {
                                    current.copy(
                                        typingDisplayNames = names,
                                        typingAvatarUrls = avatars,
                                    )
                                } else {
                                    current
                                }
                            }
                        }
                        typingIndicatorTimeoutJob =
                            viewModelScope.launch {
                                delay(TYPING_INDICATOR_TIMEOUT_MS)
                                _uiState.update { current ->
                                    current.copy(
                                        typingUserIds = emptyList(),
                                        typingDisplayNames = emptyList(),
                                        typingAvatarUrls = emptyList(),
                                    )
                                }
                            }
                    }
                }
            }

        roomJobs +=
            viewModelScope.launch {
                timelineController.mode.collectLatest { mode ->
                    activateTimeline(room = room, mode = mode)
                }
            }

        timelineController.enterLive()
        runCatching { room.markAsRead() }
    }

    private fun scheduleTypingStart() {
        typingStartJob?.cancel()
        typingStartJob =
            viewModelScope.launch {
                delay(TYPING_START_DEBOUNCE_MS)
                if (_uiState.value.messageDraft.isBlank() || typingState) return@launch
                setTypingState(isTyping = true)
                scheduleTypingStop()
            }
    }

    private fun scheduleTypingStop() {
        typingStopJob?.cancel()
        typingStopJob =
            viewModelScope.launch {
                delay(TYPING_STOP_IDLE_MS)
                if (!typingState) return@launch
                setTypingState(isTyping = false)
            }
    }

    private fun stopTyping(notifyRoom: Boolean) {
        clearTypingTimers()
        if (notifyRoom && typingState) {
            setTypingState(isTyping = false)
            return
        }
        typingState = false
    }

    private fun clearTypingTimers() {
        typingStartJob?.cancel()
        typingStartJob = null
        typingStopJob?.cancel()
        typingStopJob = null
    }

    private fun setTypingState(isTyping: Boolean) {
        if (typingState == isTyping) return
        typingState = isTyping
        viewModelScope.launch {
            activeRoom?.typingNotice(isTyping)
        }
    }

    private fun currentDraftRoomId(): String? = activeRoom?.roomId?.takeIf { it.isNotBlank() } ?: boundRoomId

    private suspend fun awaitTimelineForSend(
        attempts: Int = 120,
        delayMs: Long = 500L,
    ): Timeline? {
        repeat(attempts) { attempt ->
            activeTimeline?.let { return it }

            val roomId = boundRoomId
            if (!roomId.isNullOrBlank()) {
                if (attempt == 0 || attempt % 3 == 2) {
                    runCatching { chatClient.refreshRooms() }
                }
                val shouldKickRebind =
                    activeRoom == null &&
                        bindingRoomId != roomId &&
                        (bindJob?.isActive != true)
                if (shouldKickRebind) {
                    val current = _uiState.value
                    loadRoom(
                        roomId = roomId,
                        fallbackDisplayName = current.roomDisplayName.takeIf { it.isNotBlank() },
                        fallbackDirectUserId = activeDirectUserId,
                    )
                }
                if (bindJob?.isActive == true) {
                    // Let the in-flight bind finish an iteration before we retry.
                    bindJob?.join()
                    activeTimeline?.let { return it }
                }
            }

            if (attempt < attempts - 1) {
                delay(delayMs)
            }
        }
        return activeTimeline
    }

    private suspend fun activateTimeline(
        room: JoinedRoom,
        mode: TimelineMode,
    ) {
        when (mode) {
            TimelineMode.Live -> {
                focusedTimeline?.close()
                focusedTimeline = null
                bindActiveTimeline(room.liveTimeline)
            }

            is TimelineMode.FocusedOnEvent -> {
                room
                    .createFocusedTimeline(mode.eventId)
                    .onSuccess { focused ->
                        focusedTimeline?.close()
                        focusedTimeline = focused
                        bindActiveTimeline(focused)
                    }.onFailure { throwable ->
                        _uiState.update {
                            it.copy(error = throwable.message ?: "Failed to open focused timeline")
                        }
                        timelineController.enterLive()
                    }
            }
        }
    }

    private fun bindActiveTimeline(timeline: Timeline) {
        if (activeTimeline === timeline) return

        activeTimeline = timeline
        timelineJobs.forEach { it.cancel() }
        timelineJobs.clear()
        val initialSnapshot = timeline.items.value
        if (initialSnapshot.isNotEmpty()) {
            applyTimelineItems(timeline = timeline, items = initialSnapshot)
        }

        timelineJobs +=
            viewModelScope.launch {
                timeline.items.collectLatest { items ->
                    applyTimelineItems(timeline = timeline, items = items)
                }
            }

        timelineJobs +=
            viewModelScope.launch {
                timeline.isPaginatingBackwards.collectLatest { isPaginating ->
                    _uiState.update { current ->
                        current.copy(isPaginatingBackwards = isPaginating)
                    }
                }
            }

        timelineJobs +=
            viewModelScope.launch {
                timeline.hasMoreBackwards.collectLatest { hasMore ->
                    _uiState.update { current ->
                        current.copy(hasMoreBackwards = hasMore)
                    }
                }
            }
    }

    private fun applyTimelineItems(
        timeline: Timeline,
        items: List<TimelineItem>,
    ) {
        totalTimelineItemCount = items.size
        val unreadBelowCount = computeUnreadBelowCount(items)
        _uiState.update { current ->
            val timelineAvatar =
                items
                    .asSequence()
                    .filterIsInstance<TimelineItem.Event>()
                    .filter { !it.isMine }
                    .mapNotNull { event ->
                        resolveAvatarOverride(event.senderId, current.avatarOverrides)
                            ?: event.senderAvatarUrl
                    }.firstOrNull()
            val summary = latestRoomSummaries.firstOrNull { it.roomId == current.roomId }
            val updated =
                current.copy(
                    timelineItems = items,
                    roomAvatarUrl =
                        resolveRoomAvatar(
                            summary = summary,
                            overrides = current.avatarOverrides,
                            roomDisplayName = current.roomDisplayName,
                            currentAvatar = current.roomAvatarUrl,
                            timelineAvatar = timelineAvatar,
                        ),
                    isAwayFromLatest =
                        isAwayFromLatest(
                            firstVisibleItemIndex = lastVisibleTimelineIndex,
                            totalItems = items.size,
                        ),
                    unreadBelowCount = unreadBelowCount,
                    timelineMode = timeline.mode,
                    isLoading = false,
                )
            if (updated == current) current else updated
        }
    }

    private fun observeAvatarOverrides() {
        viewModelScope.launch {
            matchProfileRepository.observeProfiles().collect { profiles ->
                val overrides = mutableMapOf<String, String>()
                profiles.forEach { profile ->
                    val userId = profile.userId
                    val pic = profile.profilePicture ?: return@forEach
                    if (pic.isBlank()) return@forEach
                    overrides[userId] = pic
                    overrides[userId.lowercase()] = pic
                }
                val byName =
                    profiles
                        .asSequence()
                        .filter { !it.name.isBlank() }
                        .groupBy { it.name.trim().lowercase() }
                        .mapNotNull { (name, sameNameProfiles) ->
                            val allPictures = sameNameProfiles.map { it.profilePicture?.takeIf { p -> p.isNotBlank() } }
                            if (allPictures.any { it == null }) return@mapNotNull null
                            val uniquePictures = allPictures.filterNotNull().distinct()
                            if (uniquePictures.size == 1) {
                                name to uniquePictures.first()
                            } else {
                                null
                            }
                        }.toMap()
                latestAvatarByName = byName
                _uiState.update { current ->
                    val summary = latestRoomSummaries.firstOrNull { it.roomId == current.roomId }
                    current.copy(
                        avatarOverrides = overrides,
                        roomAvatarUrl =
                            resolveRoomAvatar(
                                summary = summary,
                                overrides = overrides,
                                roomDisplayName = current.roomDisplayName,
                                currentAvatar = current.roomAvatarUrl,
                                directUserIdFallback = activeDirectUserId,
                            ),
                    )
                }
            }
        }
    }

    private fun observeEventCovers() {
        viewModelScope.launch {
            eventRepository.observeEvents().collect { events ->
                val covers =
                    events
                        .filter { it.isAttending && it.conversationId != null && it.coverImage != null }
                        .associate { it.conversationId!! to it.coverImage!! }
                eventCoverByRoomId = covers
                _uiState.update { current ->
                    val rid = current.roomId
                    val eventCover = covers[rid]
                    if (eventCover != null && current.roomAvatarUrl != eventCover) {
                        current.copy(roomAvatarUrl = eventCover)
                    } else {
                        current
                    }
                }
            }
        }
    }

    private fun resolveRoomAvatar(
        summary: RoomSummary?,
        overrides: Map<String, String>,
        roomDisplayName: String,
        currentAvatar: String?,
        timelineAvatar: String? = null,
        directUserIdFallback: String? = null,
    ): String? {
        val eventCover = summary?.roomId?.let { eventCoverByRoomId[it] }
        if (eventCover != null) return eventCover
        val directUserId = summary?.directUserId ?: directUserIdFallback
        val summaryAvatar =
            summary?.avatarUrl
                ?: directUserId?.let { resolveAvatarOverride(it, overrides) }
        val byNameAvatar = latestAvatarByName[roomDisplayName.trim().lowercase()]
        return summaryAvatar ?: currentAvatar ?: timelineAvatar ?: byNameAvatar
    }

    private fun schedulePendingRoomRetry(
        roomId: String,
        fallbackDisplayName: String?,
        fallbackDirectUserId: String?,
    ) {
        pendingRoomRetryJob?.cancel()
        pendingRoomRetryJob =
            viewModelScope.launch {
                repeat(12) { attempt ->
                    delay((1_000L + attempt * 500L).coerceAtMost(5_000L))
                    if (boundRoomId != roomId || activeRoom != null) return@launch
                    runCatching { chatClient.refreshRooms() }
                    if (chatClient.getJoinedRoom(roomId) != null) {
                        loadRoom(
                            roomId = roomId,
                            fallbackDisplayName = fallbackDisplayName,
                            fallbackDirectUserId = fallbackDirectUserId,
                        )
                        return@launch
                    }
                }
            }
    }

    private fun resolvePreferredRoomDisplayName(
        roomId: String,
        summary: RoomSummary?,
        liveName: String?,
        currentName: String,
        fallbackDisplayName: String?,
    ): String {
        val summaryName = summary?.displayName?.trim().orEmpty()
        val sdkName = liveName?.trim().orEmpty()
        val fallback = fallbackDisplayName?.trim().orEmpty()

        if (summary?.isDirect == true && summaryName.isNotBlank() && !isGenericDmTitle(summaryName, roomId)) {
            return summaryName
        }
        if (fallback.isNotBlank() && (sdkName.isBlank() || isGenericDmTitle(sdkName, roomId))) {
            return fallback
        }
        if (sdkName.isNotBlank() && !isGenericDmTitle(sdkName, roomId)) {
            return sdkName
        }
        if (summaryName.isNotBlank()) {
            return summaryName
        }
        if (currentName.isNotBlank()) {
            return currentName
        }
        return fallback
    }

    private fun isGenericDmTitle(
        value: String,
        roomId: String,
    ): Boolean {
        val trimmed = value.trim()
        if (trimmed.isBlank()) return true
        if (trimmed == roomId) return true
        if (isMemberCountName(trimmed)) return true
        return when (trimmed.lowercase()) {
            "chat", "dm", "direct message", "wiadomość", "wiadomosc" -> true
            else -> false
        }
    }

    private fun computeUnreadBelowCount(items: List<TimelineItem>): Int {
        val readMarkerIndex = items.indexOfLast { it == TimelineItem.ReadMarker }
        if (readMarkerIndex < 0) return 0
        return items
            .drop(readMarkerIndex + 1)
            .count { it is TimelineItem.Event }
    }

    // In reversed layout, index 0 = newest message at the bottom.
    // "Away from latest" means the first visible item index is > 0.
    private fun isAwayFromLatest(
        firstVisibleItemIndex: Int?,
        @Suppress("UNUSED_PARAMETER") totalItems: Int,
    ): Boolean {
        val index = firstVisibleItemIndex ?: return false
        return index > 0
    }
}

private val MEMBER_COUNT_PATTERN = Regex("^\\d+\\s+(people|person|members?|users?)$", RegexOption.IGNORE_CASE)

private fun isMemberCountName(value: String): Boolean = MEMBER_COUNT_PATTERN.matches(value.trim())
