package com.poziomki.app.chat.ws

import com.poziomki.app.chat.api.JoinedRoom
import com.poziomki.app.chat.api.Timeline
import com.poziomki.app.chat.api.TimelineMode
import com.poziomki.app.chat.cache.RoomTimelineCacheStore
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow

class MemberCache internal constructor() {
    private val names = mutableMapOf<String, String>()
    private val avatars = mutableMapOf<String, String>()

    fun put(userId: String, name: String?, avatar: String?) {
        if (!name.isNullOrBlank()) names[userId] = name
        if (!avatar.isNullOrBlank()) avatars[userId] = avatar
    }

    fun getDisplayName(userId: String): String? = names[userId]

    fun getAvatarUrl(userId: String): String? = avatars[userId]
}

@Suppress("TooManyFunctions", "LongParameterList")
class WsJoinedRoom(
    override val roomId: String,
    initialDisplayName: String,
    private val wsConnection: WsConnection,
    private val roomTimelineCacheStore: RoomTimelineCacheStore,
    directUserId: String? = null,
    directUserName: String? = null,
    directUserAvatar: String? = null,
) : JoinedRoom {
    private val _displayName = MutableStateFlow(initialDisplayName)
    override val displayName: StateFlow<String> = _displayName

    private val _typingUserIds = MutableStateFlow<List<String>>(emptyList())
    override val typingUserIds: StateFlow<List<String>> = _typingUserIds

    internal val memberCache = MemberCache()

    init {
        if (directUserId != null) {
            memberCache.put(directUserId, directUserName, directUserAvatar)
        }
    }

    override val liveTimeline: WsTimeline = WsTimeline(
        conversationId = roomId,
        wsConnection = wsConnection,
        roomTimelineCacheStore = roomTimelineCacheStore,
        memberCache = memberCache,
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
            mode = TimelineMode.FocusedOnEvent(eventId),
            memberCache = memberCache,
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

    override suspend fun getMemberDisplayName(userId: String): String? =
        memberCache.getDisplayName(userId)

    override suspend fun getMemberAvatarUrl(userId: String): String? =
        memberCache.getAvatarUrl(userId)
}
