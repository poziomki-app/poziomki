package com.poziomki.app.ui.screen.chat

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import com.poziomki.app.chat.matrix.api.JoinedRoom
import com.poziomki.app.chat.matrix.api.MatrixClient
import com.poziomki.app.chat.matrix.api.MatrixTimelineMode
import com.poziomki.app.chat.matrix.api.Timeline
import com.poziomki.app.chat.timeline.TimelineController
import com.poziomki.app.ui.screen.chat.model.ChatUiState
import com.poziomki.app.ui.screen.chat.model.ComposerMode
import kotlinx.coroutines.Job
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.collectLatest
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch

class ChatViewModel(
    private val matrixClient: MatrixClient,
) : ViewModel() {
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
        val shouldType = value.isNotBlank()
        if (typingState != shouldType) {
            typingState = shouldType
            viewModelScope.launch {
                activeRoom?.typingNotice(shouldType)
            }
        }
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

        _uiState.update {
            it.copy(
                messageDraft = "",
                composerMode = ComposerMode.NewMessage,
                error = null,
            )
        }

        typingState = false

        viewModelScope.launch {
            activeRoom?.typingNotice(false)
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
            }
        }
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

    override fun onCleared() {
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
        typingState = false

        _uiState.value =
            ChatUiState(
                roomId = roomId,
                isLoading = true,
            )

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
        _uiState.update {
            it.copy(
                roomId = room.roomId,
                roomDisplayName = "",
                timelineItems = emptyList(),
                typingUserIds = emptyList(),
                messageDraft = "",
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
                    _uiState.update { current ->
                        current.copy(
                            timelineItems = items,
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
}
