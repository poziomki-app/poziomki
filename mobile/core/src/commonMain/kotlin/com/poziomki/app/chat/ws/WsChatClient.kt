package com.poziomki.app.chat.ws

import com.poziomki.app.chat.api.ChatClient
import com.poziomki.app.chat.api.ChatClientState
import com.poziomki.app.chat.api.EventSendStatus
import com.poziomki.app.chat.api.JoinedRoom
import com.poziomki.app.chat.api.RoomSummary
import com.poziomki.app.chat.api.RoomTimelineCacheSnapshot
import com.poziomki.app.chat.cache.RoomTimelineCacheStore
import com.poziomki.app.network.ApiResult
import com.poziomki.app.network.ApiService
import com.poziomki.app.session.SessionManager
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.launch
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import kotlinx.coroutines.withTimeoutOrNull

@Suppress("TooManyFunctions")
class WsChatClient(
    private val apiService: ApiService,
    private val sessionManager: SessionManager,
    private val wsConnection: WsConnection,
    private val roomTimelineCacheStore: RoomTimelineCacheStore,
) : ChatClient {
    private val scopeJob = SupervisorJob()
    private val scope = CoroutineScope(scopeJob + Dispatchers.Default)

    private val _state = MutableStateFlow<ChatClientState>(ChatClientState.Idle)
    override val state: StateFlow<ChatClientState> = _state

    private val _rooms = MutableStateFlow<List<RoomSummary>>(emptyList())
    override val rooms: StateFlow<List<RoomSummary>> = _rooms

    private val openedRoomsMutex = Mutex()
    private val openedRooms = mutableMapOf<String, WsJoinedRoom>()

    /** RoomId currently focused on the chat screen — incoming messages skip the unread bump. */
    private var activeRoomId: String? = null

    /** Cache conversation metadata for populating member caches on room open */
    private var latestConversations: List<WsConversationPayload> = emptyList()

    /** Debounce job for refreshing conversation list when an unknown room appears */
    private var refreshJob: Job? = null

    init {
        // Observe connection state
        scope.launch {
            var wasReady = false
            wsConnection.isConnected.collect { connected ->
                if (connected) {
                    val isReconnect = wasReady
                    val userId = wsConnection.userId.value ?: ""
                    _state.value =
                        ChatClientState.Ready(
                            userId = userId,
                            deviceId = deviceId(),
                        )
                    wasReady = true
                    if (isReconnect) {
                        // Backfill all opened rooms after reconnect
                        val rooms = openedRoomsMutex.withLock { openedRooms.values.toList() }
                        rooms.forEach { room ->
                            room.liveTimeline.backfillOnReconnect()
                        }
                    }
                } else if (_state.value is ChatClientState.Ready) {
                    _state.value = ChatClientState.Connecting
                }
            }
        }

        // Observe incoming messages and dispatch
        scope.launch {
            wsConnection.incoming.collect { msg ->
                handleServerMessage(msg)
            }
        }
    }

    private suspend fun handleServerMessage(msg: WsServerMessage) {
        if (_state.value is ChatClientState.Idle) return
        when (msg) {
            is WsServerMessage.Conversations -> {
                latestConversations = msg.conversations
                _rooms.value = msg.conversations.map { it.toRoomSummary() }
            }

            is WsServerMessage.Message -> {
                // Update room list (latest message)
                updateRoomLatestMessage(msg)
                // Dispatch to opened room timeline
                openedRoomsMutex.withLock { openedRooms[msg.conversationId] }?.onMessage(msg)
            }

            is WsServerMessage.Edited -> {
                openedRoomsMutex.withLock { openedRooms[msg.conversationId] }?.onEdited(msg)
            }

            is WsServerMessage.Deleted -> {
                openedRoomsMutex.withLock { openedRooms[msg.conversationId] }?.onDeleted(msg)
            }

            is WsServerMessage.Reaction -> {
                openedRoomsMutex.withLock { openedRooms[msg.conversationId] }?.onReaction(msg)
            }

            is WsServerMessage.ReadReceipt -> {
                openedRoomsMutex.withLock { openedRooms[msg.conversationId] }?.onReadReceipt(msg)
                // Only own-device receipts clear the room unread badge; peer
                // receipts no longer drive any room-list UI here, so we ignore
                // them rather than risk a stale or duplicate receipt inflating
                // a per-room read counter that is not scoped to a specific
                // message.
                if (msg.userId.toString() == wsConnection.userId.value) {
                    clearRoomUnreadCount(msg.conversationId)
                }
            }

            is WsServerMessage.Typing -> {
                openedRoomsMutex.withLock { openedRooms[msg.conversationId] }?.onTyping(msg)
            }

            is WsServerMessage.HistoryResponse -> {
                openedRoomsMutex.withLock { openedRooms[msg.conversationId] }?.onHistoryResponse(msg)
            }

            else -> {}
        }
    }

    private fun updateRoomLatestMessage(msg: WsServerMessage.Message) {
        val current = _rooms.value.toMutableList()
        val idx = current.indexOfFirst { it.roomId == msg.conversationId }
        if (idx >= 0) {
            val isMine = msg.senderId.toString() == wsConnection.userId.value
            val isFocusedRoom = msg.conversationId == activeRoomId
            val nextUnread =
                if (isMine || isFocusedRoom) {
                    current[idx].unreadCount
                } else {
                    current[idx].unreadCount + 1
                }
            current[idx] =
                current[idx].copy(
                    latestMessage = msg.body,
                    latestTimestampMillis = parseTimestamp(msg.createdAt),
                    latestMessageIsMine = isMine,
                    latestMessageSendStatus = EventSendStatus.Sent,
                    unreadCount = nextUnread,
                    latestMessageReadByCount = 0,
                    // Live broadcasts pre-date the worker scan (verdict
                    // is null at this point). Carry whatever the WS
                    // payload says — typically null until the next
                    // refreshRooms cycle, when the server returns the
                    // verdict that the worker has since written.
                    latestModerationVerdict = if (isMine) null else msg.moderationVerdict,
                    latestModerationCategories = if (isMine) emptyList() else msg.moderationCategories,
                )
            current.sortByDescending { it.latestTimestampMillis ?: 0L }
            _rooms.value = current
        } else {
            // Unknown room — debounce a conversation list refresh
            refreshJob?.cancel()
            refreshJob =
                scope.launch {
                    delay(500L)
                    wsConnection.send(WsClientMessage.ListConversations)
                }
        }
    }

    private fun clearRoomUnreadCount(roomId: String) {
        val current = _rooms.value.toMutableList()
        val idx = current.indexOfFirst { it.roomId == roomId }
        if (idx >= 0) {
            current[idx] = current[idx].copy(unreadCount = 0)
            _rooms.value = current
        }
    }

    override suspend fun ensureStarted(): Result<Unit> {
        if (_state.value is ChatClientState.Ready) return Result.success(Unit)

        _state.value = ChatClientState.Connecting

        openedRoomsMutex.withLock {
            openedRooms.values.forEach {
                it.close()
                it.liveTimeline.close()
            }
            openedRooms.clear()
        }
        latestConversations = emptyList()
        _rooms.value = emptyList()

        wsConnection.connect()

        // Wait for connected state
        val connected =
            withTimeoutOrNull(15_000L) {
                wsConnection.isConnected.first { it }
            }

        return if (connected == true) {
            // Don't clear the persistent timeline cache here — server
            // snapshots overwrite stale entries as they arrive, and
            // wiping on every cold start defeats the offline-open path
            // (UI reads the cache first in ChatViewModel.openRoom).
            Result.success(Unit)
        } else {
            _state.value = ChatClientState.Error("Connection timeout")
            Result.failure(IllegalStateException("WebSocket connection timeout"))
        }
    }

    override suspend fun refreshRooms(): Result<Unit> {
        ensureStarted().getOrElse { return Result.failure(it) }
        wsConnection.send(WsClientMessage.ListConversations)
        return Result.success(Unit)
    }

    override suspend fun getJoinedRoom(roomId: String): JoinedRoom? {
        openedRoomsMutex.withLock { openedRooms[roomId] }?.let { return it }

        val summary = _rooms.value.find { it.roomId == roomId }
        val displayName = summary?.displayName ?: ""

        val conversation = latestConversations.find { it.id == roomId }
        val room =
            WsJoinedRoom(
                roomId = roomId,
                initialDisplayName = displayName,
                wsConnection = wsConnection,
                roomTimelineCacheStore = roomTimelineCacheStore,
                directUserId = conversation?.directUserId ?: summary?.directUserId,
                directUserName = conversation?.directUserName,
                directUserAvatar = conversation?.directUserAvatar,
            )
        openedRoomsMutex.withLock { openedRooms[roomId] = room }
        return room
    }

    override suspend fun getRoomTimelineCache(
        roomId: String,
        limit: Int,
    ): RoomTimelineCacheSnapshot {
        val snapshot = roomTimelineCacheStore.loadSnapshot(roomId, limit)
        return RoomTimelineCacheSnapshot(
            items = snapshot.items,
            isHydrated = snapshot.isHydrated,
            cachedItemCount = snapshot.cachedItemCount,
            updatedAtMillis = snapshot.updatedAtMillis,
        )
    }

    override suspend fun requestRoomTimelineBackfill(roomId: String): Result<Unit> {
        val room =
            openedRoomsMutex.withLock { openedRooms[roomId] }
                ?: return Result.failure(IllegalStateException("Room not opened"))
        room.liveTimeline.paginateBackwards(200u)
        return Result.success(Unit)
    }

    override suspend fun createDM(
        userId: String,
        displayName: String?,
    ): Result<String> =
        when (val result = apiService.resolveChatDm(userId)) {
            is ApiResult.Success -> Result.success(result.data.conversationId)
            is ApiResult.Error -> Result.failure(IllegalStateException(result.message))
        }

    override suspend fun createRoom(
        name: String,
        invitedUserIds: List<String>,
    ): Result<String> = Result.failure(UnsupportedOperationException("Event rooms are created server-side"))

    override suspend fun registerPusher(ntfyEndpoint: String): Result<Unit> {
        val deviceId = deviceId()
        val ntfyTopic = "poz_$deviceId"
        return when (val result = apiService.registerChatPushAndroid(deviceId, ntfyTopic)) {
            is ApiResult.Success -> Result.success(Unit)
            is ApiResult.Error -> Result.failure(IllegalStateException(result.message))
        }
    }

    override suspend fun unregisterPusher(ntfyEndpoint: String): Result<Unit> {
        val deviceId = deviceId()
        return when (val result = apiService.unregisterChatPush(deviceId)) {
            is ApiResult.Success -> Result.success(Unit)
            is ApiResult.Error -> Result.failure(IllegalStateException(result.message))
        }
    }

    override suspend fun markRoomReadLocally(roomId: String) {
        clearRoomUnreadCount(roomId)
    }

    override suspend fun setActiveRoom(roomId: String?) {
        activeRoomId = roomId
        if (roomId != null) clearRoomUnreadCount(roomId)
    }

    override suspend fun hideConversation(roomId: String) {
        val current = _rooms.value.toMutableList()
        current.removeAll { it.roomId == roomId }
        _rooms.value = current
        openedRoomsMutex.withLock {
            openedRooms.remove(roomId)?.let {
                it.close()
                it.liveTimeline.close()
            }
        }
        roomTimelineCacheStore.clear(roomId)
    }

    override suspend fun stop() {
        _state.value = ChatClientState.Idle
        wsConnection.disconnect()
        openedRoomsMutex.withLock {
            openedRooms.values.forEach {
                it.close()
                it.liveTimeline.close()
            }
            openedRooms.clear()
        }
        latestConversations = emptyList()
        _rooms.value = emptyList()
        roomTimelineCacheStore.clearAll()
    }

    private suspend fun deviceId(): String = sessionManager.getOrCreateDeviceId()
}

private fun WsConversationPayload.toRoomSummary(): RoomSummary =
    RoomSummary(
        roomId = id,
        displayName = if (isDirect) directUserName ?: title ?: "" else title ?: "",
        // Only direct (1:1) rooms get an avatar from the server payload.
        // For event/group rooms the cover comes from EventRepository — using
        // directUserAvatar here would leak the event creator's face as the
        // room avatar in previews and chat header.
        avatarUrl = if (isDirect) directUserAvatar else null,
        isDirect = isDirect,
        directUserId = directUserPid ?: directUserId,
        unreadCount = unreadCount.toInt(),
        latestMessage = latestMessage,
        latestTimestampMillis = latestTimestamp?.let { parseTimestamp(it) },
        latestMessageIsMine = latestMessageIsMine,
        latestMessageSendStatus = if (latestMessage != null) EventSendStatus.Sent else null,
        isBlocked = isBlocked,
        latestModerationVerdict = latestModerationVerdict,
        latestModerationCategories = latestModerationCategories,
    )

internal fun parseTimestamp(iso8601: String): Long =
    try {
        kotlin.time.Instant
            .parse(iso8601)
            .toEpochMilliseconds()
    } catch (_: Exception) {
        0L
    }
