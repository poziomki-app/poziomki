package com.poziomki.app.chat.ws

import com.poziomki.app.chat.cache.RoomTimelineCacheStore
import com.poziomki.app.chat.matrix.api.JoinedRoom
import com.poziomki.app.chat.matrix.api.Timeline
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow

@Suppress("TooManyFunctions")
class WsJoinedRoom(
    override val roomId: String,
    initialDisplayName: String,
    private val wsConnection: WsConnection,
    private val roomTimelineCacheStore: RoomTimelineCacheStore,
) : JoinedRoom {
    private val _displayName = MutableStateFlow(initialDisplayName)
    override val displayName: StateFlow<String> = _displayName

    private val _typingUserIds = MutableStateFlow<List<String>>(emptyList())
    override val typingUserIds: StateFlow<List<String>> = _typingUserIds

    override val liveTimeline: WsTimeline = WsTimeline(
        conversationId = roomId,
        wsConnection = wsConnection,
        roomTimelineCacheStore = roomTimelineCacheStore,
    )

    internal fun onMessage(msg: WsServerMessage.Message) {
        liveTimeline.onMessage(msg)
    }

    internal fun onEdited(msg: WsServerMessage.Edited) {
        liveTimeline.onEdited(msg)
    }

    internal fun onDeleted(msg: WsServerMessage.Deleted) {
        liveTimeline.onDeleted(msg)
    }

    internal fun onReaction(msg: WsServerMessage.Reaction) {
        liveTimeline.onReaction(msg)
    }

    internal fun onReadReceipt(msg: WsServerMessage.ReadReceipt) {
        liveTimeline.onReadReceipt(msg)
    }

    internal fun onTyping(msg: WsServerMessage.Typing) {
        val current = _typingUserIds.value.toMutableList()
        val userId = msg.userId.toString()
        if (msg.isTyping) {
            if (userId !in current && userId != wsConnection.userId.value) {
                current.add(userId)
            }
        } else {
            current.remove(userId)
        }
        _typingUserIds.value = current
    }

    internal fun onHistoryResponse(msg: WsServerMessage.HistoryResponse) {
        liveTimeline.onHistoryResponse(msg)
    }

    override suspend fun createFocusedTimeline(eventId: String): Result<Timeline> {
        val timeline = WsTimeline(
            conversationId = roomId,
            wsConnection = wsConnection,
            roomTimelineCacheStore = roomTimelineCacheStore,
            mode = com.poziomki.app.chat.matrix.api.MatrixTimelineMode.FocusedOnEvent(eventId),
        )
        return Result.success(timeline)
    }

    override suspend fun typingNotice(isTyping: Boolean): Result<Unit> {
        wsConnection.send(WsClientMessage.Typing(conversationId = roomId, isTyping = isTyping))
        return Result.success(Unit)
    }

    override suspend fun markAsRead(): Result<Unit> = liveTimeline.markAsRead()

    override suspend fun inviteUserById(userId: String): Result<Unit> =
        Result.success(Unit) // Event rooms auto-manage membership

    override suspend fun getMemberDisplayName(userId: String): String? = null

    override suspend fun getMemberAvatarUrl(userId: String): String? = null
}
