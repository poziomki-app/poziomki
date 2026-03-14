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
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.launch
import kotlinx.coroutines.withTimeoutOrNull

@Suppress("TooManyFunctions")
class WsChatClient(
    private val apiService: ApiService,
    @Suppress("UnusedPrivateProperty") private val sessionManager: SessionManager,
    private val wsConnection: WsConnection,
    private val roomTimelineCacheStore: RoomTimelineCacheStore,
) : ChatClient {
    private val scope = CoroutineScope(SupervisorJob() + Dispatchers.Default)

    private val _state = MutableStateFlow<ChatClientState>(ChatClientState.Idle)
    override val state: StateFlow<ChatClientState> = _state

    private val _rooms = MutableStateFlow<List<RoomSummary>>(emptyList())
    override val rooms: StateFlow<List<RoomSummary>> = _rooms

    private val openedRooms = mutableMapOf<String, WsJoinedRoom>()

    /** Cache conversation metadata for populating member caches on room open */
    private var latestConversations: List<WsConversationPayload> = emptyList()

    init {
        // Observe connection state
        scope.launch {
            wsConnection.isConnected.collect { connected ->
                if (connected) {
                    val userId = wsConnection.userId.value ?: ""
                    _state.value = ChatClientState.Ready(
                        userId = userId,
                        homeserver = "",
                        deviceId = deviceId(),
                    )
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

    private fun handleServerMessage(msg: WsServerMessage) {
        when (msg) {
            is WsServerMessage.Conversations -> {
                latestConversations = msg.conversations
                _rooms.value = msg.conversations.map { it.toRoomSummary() }
            }
            is WsServerMessage.Message -> {
                // Update room list (latest message)
                updateRoomLatestMessage(msg)
                // Dispatch to opened room timeline
                openedRooms[msg.conversationId]?.onMessage(msg)
            }
            is WsServerMessage.Edited -> {
                openedRooms[msg.conversationId]?.onEdited(msg)
            }
            is WsServerMessage.Deleted -> {
                openedRooms[msg.conversationId]?.onDeleted(msg)
            }
            is WsServerMessage.Reaction -> {
                openedRooms[msg.conversationId]?.onReaction(msg)
            }
            is WsServerMessage.ReadReceipt -> {
                openedRooms[msg.conversationId]?.onReadReceipt(msg)
            }
            is WsServerMessage.Typing -> {
                openedRooms[msg.conversationId]?.onTyping(msg)
            }
            is WsServerMessage.HistoryResponse -> {
                openedRooms[msg.conversationId]?.onHistoryResponse(msg)
            }
            else -> {}
        }
    }

    private fun updateRoomLatestMessage(msg: WsServerMessage.Message) {
        val current = _rooms.value.toMutableList()
        val idx = current.indexOfFirst { it.roomId == msg.conversationId }
        if (idx >= 0) {
            current[idx] = current[idx].copy(
                latestMessage = msg.body,
                latestTimestampMillis = parseTimestamp(msg.createdAt),
                latestMessageIsMine = msg.isMine,
                latestMessageSendStatus = EventSendStatus.Sent,
                unreadCount = if (msg.isMine) current[idx].unreadCount else current[idx].unreadCount + 1,
            )
            current.sortByDescending { it.latestTimestampMillis ?: 0L }
            _rooms.value = current
        }
    }

    override suspend fun ensureStarted(): Result<Unit> {
        if (_state.value is ChatClientState.Ready) return Result.success(Unit)

        _state.value = ChatClientState.Connecting
        wsConnection.connect()

        // Wait for connected state
        val connected = withTimeoutOrNull(15_000L) {
            wsConnection.isConnected.first { it }
        }

        return if (connected == true) {
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
        openedRooms[roomId]?.let { return it }

        val summary = _rooms.value.find { it.roomId == roomId }
        val displayName = summary?.displayName ?: ""

        val conversation = latestConversations.find { it.id == roomId }
        val room = WsJoinedRoom(
            roomId = roomId,
            initialDisplayName = displayName,
            wsConnection = wsConnection,
            roomTimelineCacheStore = roomTimelineCacheStore,
            directUserId = conversation?.directUserId ?: summary?.directUserId,
            directUserName = conversation?.directUserName,
            directUserAvatar = conversation?.directUserAvatar,
        )
        openedRooms[roomId] = room
        return room
    }

    override suspend fun getRoomTimelineCache(roomId: String, limit: Int): RoomTimelineCacheSnapshot {
        val snapshot = roomTimelineCacheStore.loadSnapshot(roomId, limit)
        return RoomTimelineCacheSnapshot(
            items = snapshot.items,
            isHydrated = snapshot.isHydrated,
            cachedItemCount = snapshot.cachedItemCount,
            updatedAtMillis = snapshot.updatedAtMillis,
        )
    }

    override suspend fun requestRoomTimelineBackfill(roomId: String): Result<Unit> {
        val room = openedRooms[roomId] ?: return Result.failure(IllegalStateException("Room not opened"))
        room.liveTimeline.paginateBackwards(200u)
        return Result.success(Unit)
    }

    override suspend fun createDM(userId: String, displayName: String?): Result<String> {
        return when (val result = apiService.resolveChatDm(userId)) {
            is ApiResult.Success -> Result.success(result.data.conversationId)
            is ApiResult.Error -> Result.failure(IllegalStateException(result.message))
        }
    }

    override suspend fun createRoom(name: String, invitedUserIds: List<String>): Result<String> {
        return Result.failure(UnsupportedOperationException("Event rooms are created server-side"))
    }

    override suspend fun registerPusher(ntfyEndpoint: String, gatewayUrl: String): Result<Unit> {
        val deviceId = deviceId()
        val ntfyTopic = "poz_$deviceId"
        return when (val result = apiService.registerChatPush(deviceId, ntfyTopic)) {
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

    override suspend fun getMediaThumbnail(mxcUrl: String, width: Long, height: Long): ByteArray? = null

    override suspend fun getMediaContent(mxcUrl: String): ByteArray? = null

    override suspend fun stop() {
        wsConnection.disconnect()
        openedRooms.values.forEach { it.liveTimeline.close() }
        openedRooms.clear()
        latestConversations = emptyList()
        _rooms.value = emptyList()
        roomTimelineCacheStore.clearAll()
        _state.value = ChatClientState.Idle
    }

    private fun deviceId(): String {
        // Use a stable device identifier derived from session
        return "android_${wsConnection.userId.value?.hashCode()?.toString(16) ?: "unknown"}"
    }
}

private fun WsConversationPayload.toRoomSummary(): RoomSummary =
    RoomSummary(
        roomId = id,
        displayName = if (isDirect) directUserName ?: title ?: "" else title ?: "",
        avatarUrl = directUserAvatar,
        isDirect = isDirect,
        directUserId = directUserId,
        unreadCount = unreadCount.toInt(),
        latestMessage = latestMessage,
        latestTimestampMillis = latestTimestamp?.let { parseTimestamp(it) },
        latestMessageIsMine = latestMessageIsMine,
        latestMessageSendStatus = if (latestMessage != null) EventSendStatus.Sent else null,
    )

internal fun parseTimestamp(iso8601: String): Long =
    try {
        kotlinx.datetime.Instant.parse(iso8601).toEpochMilliseconds()
    } catch (_: Exception) {
        0L
    }
