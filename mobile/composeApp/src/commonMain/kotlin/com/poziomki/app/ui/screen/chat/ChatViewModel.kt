package com.poziomki.app.ui.screen.chat

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import com.poziomki.app.chat.draft.RoomComposerDraftStore
import com.poziomki.app.chat.matrix.api.JoinedRoom
import com.poziomki.app.chat.matrix.api.MatrixClient
import com.poziomki.app.chat.matrix.api.MatrixTimelineMode
import com.poziomki.app.chat.matrix.api.Timeline
import com.poziomki.app.chat.timeline.TimelineController
import com.poziomki.app.ui.screen.chat.model.ChatUiState
import com.poziomki.app.ui.screen.chat.model.ComposerMode
import com.poziomki.app.util.PickedFile
import kotlinx.coroutines.Job
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.collectLatest
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch
import kotlinx.datetime.Clock

class ChatViewModel(
    private val matrixClient: MatrixClient,
    private val roomComposerDraftStore: RoomComposerDraftStore,
) : ViewModel() {
    private companion object {
        const val TYPING_START_DEBOUNCE_MS = 300L
        const val TYPING_STOP_IDLE_MS = 5_000L
    }

    private val _uiState = MutableStateFlow(ChatUiState())
    val uiState: StateFlow<ChatUiState> = _uiState.asStateFlow()

    private val timelineController = TimelineController()

    private var boundRoomId: String? = null
    private var activeRoom: JoinedRoom? = null
    private var activeTimeline: Timeline? = null
    private var focusedTimeline: Timeline? = null
    private val roomJobs = mutableListOf<Job>()
    private val timelineJobs = mutableListOf<Job>()
    private var typingState = false
    private var typingStartJob: Job? = null
    private var typingStopJob: Job? = null
    private var lastVisibleTimelineIndex: Int? = null
    private var totalTimelineItemCount: Int = 0

    fun loadRoom(roomId: String) {
        if (roomId.isBlank()) return
        if (!roomId.startsWith("!")) {
            _uiState.value =
                ChatUiState(
                    roomId = roomId,
                    error = "Invalid chat route id. Expected Matrix room id (!...)",
                )
            return
        }
        if (boundRoomId == roomId && activeRoom != null) return

        boundRoomId = roomId
        viewModelScope.launch {
            bindRoom(roomId)
        }
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

        _uiState.update {
            it.copy(
                messageDraft = "",
                composerMode = ComposerMode.NewMessage,
                error = null,
            )
        }

        roomId?.let(roomComposerDraftStore::clearDraft)
        stopTyping(notifyRoom = true)
        viewModelScope.launch {
            val timeline = activeTimeline ?: return@launch
            val result =
                when (composerMode) {
                    ComposerMode.NewMessage -> timeline.sendMessage(body)
                    is ComposerMode.Reply -> timeline.sendReply(composerMode.eventId, body)
                    is ComposerMode.Edit -> timeline.edit(composerMode.eventOrTransactionId, body)
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

    fun sendImageAttachment(data: ByteArray) {
        if (data.isEmpty()) return
        sendAttachment(
            sendOperation = { timeline, caption, inReplyTo ->
                timeline.sendImage(
                    data = data,
                    fileName = "image_${Clock.System.now().toEpochMilliseconds()}.jpg",
                    mimeType = "image/jpeg",
                    caption = caption,
                    inReplyToEventId = inReplyTo,
                )
            },
            sendError = "Failed to send image",
        )
    }

    fun sendFileAttachment(file: PickedFile) {
        if (file.bytes.isEmpty()) return
        sendAttachment(
            sendOperation = { timeline, caption, inReplyTo ->
                timeline.sendFile(
                    data = file.bytes,
                    fileName = file.name,
                    mimeType = file.mimeType,
                    caption = caption,
                    inReplyToEventId = inReplyTo,
                )
            },
            sendError = "Failed to send attachment",
        )
    }

    fun toggleReaction(
        eventOrTransactionId: String,
        emoji: String,
    ) {
        viewModelScope.launch {
            val timeline = activeTimeline ?: return@launch
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
            activeTimeline?.markAsRead()
        }
    }

    fun onTimelineViewportChanged(lastVisibleItemIndex: Int?) {
        lastVisibleTimelineIndex = lastVisibleItemIndex
        _uiState.update { current ->
            current.copy(
                isAwayFromLatest = isAwayFromLatest(lastVisibleItemIndex = lastVisibleItemIndex, totalItems = totalTimelineItemCount),
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

    fun startReply(event: com.poziomki.app.chat.matrix.api.MatrixTimelineItem.Event) {
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

    fun startEdit(event: com.poziomki.app.chat.matrix.api.MatrixTimelineItem.Event) {
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

    suspend fun resolveDisplayNames(userIds: List<String>): Map<String, String> {
        val room = activeRoom ?: return emptyMap()
        return userIds.associateWith { userId ->
            room.getMemberDisplayName(userId) ?: userId
        }
    }

    override fun onCleared() {
        clearTypingTimers()
        focusedTimeline?.close()
        focusedTimeline = null
        timelineJobs.forEach { it.cancel() }
        timelineJobs.clear()
        roomJobs.forEach { it.cancel() }
        roomJobs.clear()
        super.onCleared()
    }

    private suspend fun bindRoom(roomId: String) {
        focusedTimeline?.close()
        focusedTimeline = null
        activeTimeline = null
        timelineJobs.forEach { it.cancel() }
        timelineJobs.clear()
        roomJobs.forEach { it.cancel() }
        roomJobs.clear()
        activeRoom = null
        clearTypingTimers()
        typingState = false

        _uiState.value =
            ChatUiState(
                roomId = roomId,
                isLoading = true,
            )
        lastVisibleTimelineIndex = null
        totalTimelineItemCount = 0

        matrixClient.ensureStarted().getOrElse { throwable ->
            _uiState.value =
                ChatUiState(
                    roomId = roomId,
                    isLoading = false,
                    error = throwable.message ?: "Failed to initialize Matrix",
                )
            return
        }

        val room =
            matrixClient.getJoinedRoom(roomId) ?: run {
                _uiState.value =
                    ChatUiState(
                        roomId = roomId,
                        isLoading = false,
                        error = "Room not found or user is not joined",
                    )
                return
            }

        activeRoom = room
        val restoredDraft = roomComposerDraftStore.getDraft(room.roomId)
        _uiState.update {
            it.copy(
                roomId = room.roomId,
                roomDisplayName = "",
                timelineItems = emptyList(),
                isAwayFromLatest = false,
                unreadBelowCount = 0,
                typingUserIds = emptyList(),
                messageDraft = restoredDraft,
                composerMode = ComposerMode.NewMessage,
                isLoading = false,
                timelineMode = MatrixTimelineMode.Live,
                error = null,
            )
        }

        roomJobs +=
            viewModelScope.launch {
                room.displayName.collectLatest { name ->
                    _uiState.update { current ->
                        current.copy(roomDisplayName = name)
                    }
                }
            }

        roomJobs +=
            viewModelScope.launch {
                room.typingUserIds.collectLatest { typingUsers ->
                    _uiState.update { current ->
                        current.copy(typingUserIds = typingUsers)
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
        room.markAsRead()
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

    private fun sendAttachment(
        sendOperation: suspend (timeline: Timeline, caption: String?, inReplyToEventId: String?) -> Result<Unit>,
        sendError: String,
    ) {
        val roomId = currentDraftRoomId()
        val uiState = _uiState.value
        val composerMode = uiState.composerMode
        val caption = uiState.messageDraft.trim().ifEmpty { null }
        val inReplyToEventId =
            when (composerMode) {
                is ComposerMode.Reply -> {
                    composerMode.eventId
                }

                is ComposerMode.Edit -> {
                    _uiState.update { current ->
                        current.copy(error = "Attachment is not supported in edit mode")
                    }
                    return
                }

                ComposerMode.NewMessage -> {
                    null
                }
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

        viewModelScope.launch {
            val timeline = activeTimeline ?: return@launch
            sendOperation(timeline, caption, inReplyToEventId).onFailure { throwable ->
                _uiState.update { current ->
                    current.copy(
                        messageDraft = caption.orEmpty(),
                        composerMode = composerMode,
                        error = throwable.message ?: sendError,
                    )
                }
                roomId?.let { failedRoomId ->
                    roomComposerDraftStore.saveDraft(roomId = failedRoomId, draft = caption.orEmpty())
                }
            }
        }
    }

    private suspend fun activateTimeline(
        room: JoinedRoom,
        mode: MatrixTimelineMode,
    ) {
        when (mode) {
            MatrixTimelineMode.Live -> {
                focusedTimeline?.close()
                focusedTimeline = null
                bindActiveTimeline(room.liveTimeline)
            }

            is MatrixTimelineMode.FocusedOnEvent -> {
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

        timelineJobs +=
            viewModelScope.launch {
                timeline.items.collectLatest { items ->
                    totalTimelineItemCount = items.size
                    val unreadBelowCount = computeUnreadBelowCount(items)
                    _uiState.update { current ->
                        current.copy(
                            timelineItems = items,
                            isAwayFromLatest =
                                isAwayFromLatest(
                                    lastVisibleItemIndex = lastVisibleTimelineIndex,
                                    totalItems = items.size,
                                ),
                            unreadBelowCount = unreadBelowCount,
                            timelineMode = timeline.mode,
                            isLoading = false,
                        )
                    }
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

    private fun computeUnreadBelowCount(items: List<com.poziomki.app.chat.matrix.api.MatrixTimelineItem>): Int {
        val readMarkerIndex = items.indexOfLast { it == com.poziomki.app.chat.matrix.api.MatrixTimelineItem.ReadMarker }
        if (readMarkerIndex < 0) return 0
        return items
            .drop(readMarkerIndex + 1)
            .count { it is com.poziomki.app.chat.matrix.api.MatrixTimelineItem.Event }
    }

    private fun isAwayFromLatest(
        lastVisibleItemIndex: Int?,
        totalItems: Int,
    ): Boolean {
        if (totalItems <= 1) return false
        val index = lastVisibleItemIndex ?: return false
        val latestIndex = totalItems - 1
        return index < (latestIndex - 1)
    }
}
