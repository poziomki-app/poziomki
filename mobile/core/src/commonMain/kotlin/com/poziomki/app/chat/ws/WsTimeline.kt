package com.poziomki.app.chat.ws

import com.poziomki.app.chat.api.EventSendStatus
import com.poziomki.app.chat.api.Reaction
import com.poziomki.app.chat.api.ReactionSender
import com.poziomki.app.chat.api.ReplyDetails
import com.poziomki.app.chat.api.Timeline
import com.poziomki.app.chat.api.TimelineItem
import com.poziomki.app.chat.api.TimelineMode
import com.poziomki.app.chat.cache.RoomTimelineCacheStore
import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.launch
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import kotlinx.coroutines.withTimeoutOrNull
import kotlinx.datetime.Clock
import kotlin.random.Random

@Suppress("TooManyFunctions")
class WsTimeline(
    private val conversationId: String,
    private val wsConnection: WsConnection,
    private val roomTimelineCacheStore: RoomTimelineCacheStore,
    override val mode: TimelineMode = TimelineMode.Live,
    private val memberCache: MemberCache? = null,
) : Timeline {
    private val scopeJob = SupervisorJob()
    private val scope = CoroutineScope(scopeJob + Dispatchers.Default)
    private val itemsMutex = Mutex()

    /** Tracks which users have sent read receipts per message to prevent duplicate counting. */
    private val readReceiptUsers = mutableMapOf<String, MutableSet<String>>()

    private val _items = MutableStateFlow<List<TimelineItem>>(emptyList())
    override val items: StateFlow<List<TimelineItem>> = _items

    private val _isPaginatingBackwards = MutableStateFlow(false)
    override val isPaginatingBackwards: StateFlow<Boolean> = _isPaginatingBackwards

    private val _hasMoreBackwards = MutableStateFlow(true)
    override val hasMoreBackwards: StateFlow<Boolean> = _hasMoreBackwards

    private var pendingHistoryDeferred: CompletableDeferred<Boolean>? = null
    private var persistJob: Job? = null

    init {
        // Load cached items on start, request history if empty
        scope.launch {
            val cached = roomTimelineCacheStore.loadSnapshot(conversationId)
            if (cached.items.isNotEmpty()) {
                val uid = wsConnection.userId.value
                val corrected =
                    if (uid != null) {
                        cached.items.map { item ->
                            if (item is TimelineItem.Event) {
                                item.copy(isMine = item.senderId == uid, isEditable = item.senderId == uid)
                            } else {
                                item
                            }
                        }
                    } else {
                        cached.items
                    }
                _items.value = corrected
                _hasMoreBackwards.value = !cached.isHydrated
            } else {
                // No cache — request initial history from server
                requestInitialHistory()
            }
        }
    }

    private suspend fun requestInitialHistory() {
        if (!wsConnection.isConnected.value) return
        wsConnection.send(
            WsClientMessage.History(
                conversationId = conversationId,
                before = null,
                limit = 50,
            ),
        )
    }

    internal fun backfillOnReconnect() {
        scope.launch {
            itemsMutex.withLock {
                _items.value = emptyList()
                _hasMoreBackwards.value = true
            }
            requestInitialHistory()
        }
    }

    internal fun onMessage(msg: WsServerMessage.Message) {
        cacheMembers(msg)
        scope.launch {
            itemsMutex.withLock {
                val item = msg.toTimelineItem(wsConnection.userId.value)
                val current = _items.value.toMutableList()

                // Check for optimistic update by clientId
                val clientId = msg.clientId
                if (clientId != null) {
                    val idx =
                        current.indexOfFirst {
                            it is TimelineItem.Event && it.eventOrTransactionId == clientId
                        }
                    if (idx >= 0) {
                        current[idx] = item
                        emitAndPersist(current)
                        return@withLock
                    }
                }

                // Check for duplicate by id
                val exists =
                    current.any {
                        it is TimelineItem.Event && it.eventId == msg.id
                    }
                if (!exists) {
                    current.add(item)
                    emitAndPersist(current)
                }
            }
        }
    }

    internal fun onEdited(msg: WsServerMessage.Edited) {
        scope.launch {
            itemsMutex.withLock {
                val current = _items.value.toMutableList()
                val idx =
                    current.indexOfFirst {
                        it is TimelineItem.Event && it.eventId == msg.messageId
                    }
                if (idx >= 0) {
                    val event = current[idx] as TimelineItem.Event
                    current[idx] = event.copy(body = msg.body)
                    emitAndPersist(current)
                }
            }
        }
    }

    internal fun onDeleted(msg: WsServerMessage.Deleted) {
        scope.launch {
            itemsMutex.withLock {
                val current = _items.value.toMutableList()
                current.removeAll {
                    it is TimelineItem.Event && it.eventId == msg.messageId
                }
                readReceiptUsers.remove(msg.messageId)
                emitAndPersist(current)
            }
        }
    }

    internal fun onReaction(msg: WsServerMessage.Reaction) {
        val senderId = msg.userId.toString()
        memberCache?.put(senderId, msg.senderName, msg.senderAvatar)
        scope.launch {
            itemsMutex.withLock {
                val current = _items.value.toMutableList()
                val idx =
                    current.indexOfFirst {
                        it is TimelineItem.Event && it.eventId == msg.messageId
                    }
                if (idx >= 0) {
                    val event = current[idx] as TimelineItem.Event
                    val reactions = event.reactions.toMutableList()
                    val existingIdx = reactions.indexOfFirst { it.emoji == msg.emoji }
                    val isMe = senderId == wsConnection.userId.value
                    if (msg.added) {
                        val sender =
                            ReactionSender(
                                senderId = senderId,
                                displayName = msg.senderName,
                            )
                        if (existingIdx >= 0) {
                            val r = reactions[existingIdx]
                            reactions[existingIdx] =
                                r.copy(
                                    count = r.count + 1,
                                    reactedByMe = r.reactedByMe || isMe,
                                    senders = r.senders + sender,
                                )
                        } else {
                            reactions.add(
                                Reaction(
                                    emoji = msg.emoji,
                                    count = 1,
                                    reactedByMe = isMe,
                                    senders = listOf(sender),
                                ),
                            )
                        }
                    } else if (existingIdx >= 0) {
                        val r = reactions[existingIdx]
                        if (r.count <= 1) {
                            reactions.removeAt(existingIdx)
                        } else {
                            reactions[existingIdx] =
                                r.copy(
                                    count = r.count - 1,
                                    reactedByMe = if (isMe) false else r.reactedByMe,
                                    senders = r.senders.filter { it.senderId != senderId },
                                )
                        }
                    }
                    current[idx] = event.copy(reactions = reactions)
                    emitAndPersist(current)
                }
            }
        }
    }

    internal fun onReadReceipt(msg: WsServerMessage.ReadReceipt) {
        if (msg.userId.toString() == wsConnection.userId.value) return
        scope.launch {
            applyReadReceipt(msg.messageId, msg.userId, msg.readAt)
        }
    }

    private suspend fun applyReadReceipt(
        messageId: String,
        userId: Int,
        readAtIso: String?,
    ) {
        itemsMutex.withLock {
            val readAt = readAtIso?.let { parseTimestamp(it) } ?: Clock.System.now().toEpochMilliseconds()
            val seenUsers = readReceiptUsers.getOrPut(messageId) { mutableSetOf() }
            seenUsers.add(userId.toString())

            val current = _items.value.toMutableList()
            val idx =
                current.indexOfFirst {
                    it is TimelineItem.Event && it.eventId == messageId
                }
            if (idx >= 0) {
                val event = current[idx] as TimelineItem.Event
                if (event.readBy[userId] == readAt) return@withLock
                val newReadBy = event.readBy + (userId to readAt)
                // A read implies delivery; flip ✓ → ✓✓ → ✓✓ blue.
                val newStatus =
                    if (event.isMine) EventSendStatus.Read else event.sendStatus
                current[idx] =
                    event.copy(
                        readBy = newReadBy,
                        sendStatus = newStatus,
                    )
                emitAndPersist(current)
            }
        }
    }

    internal fun onDelivered(msg: WsServerMessage.Delivered) {
        if (msg.userId.toString() == wsConnection.userId.value) return
        scope.launch {
            itemsMutex.withLock {
                val deliveredAt = parseTimestamp(msg.deliveredAt)
                val current = _items.value.toMutableList()
                val idx =
                    current.indexOfFirst {
                        it is TimelineItem.Event && it.eventId == msg.messageId
                    }
                if (idx < 0) return@withLock
                val event = current[idx] as TimelineItem.Event
                if (event.deliveredTo[msg.userId] == deliveredAt) return@withLock
                val newDeliveredTo = event.deliveredTo + (msg.userId to deliveredAt)
                // Don't downgrade Read → Delivered.
                val newStatus =
                    if (event.isMine && event.sendStatus != EventSendStatus.Read) {
                        EventSendStatus.Delivered
                    } else {
                        event.sendStatus
                    }
                current[idx] =
                    event.copy(
                        deliveredTo = newDeliveredTo,
                        sendStatus = newStatus,
                    )
                emitAndPersist(current)
            }
        }
    }

    private fun hydrateHistoryItem(
        payload: WsMessagePayload,
        uid: String?,
        readsByMessage: Map<String, Map<Int, Long>>,
        deliveriesByMessage: Map<String, Map<Int, Long>>,
    ): TimelineItem.Event {
        val event = payload.toTimelineItem(uid)
        val rb = readsByMessage[payload.id].orEmpty()
        val dt = deliveriesByMessage[payload.id].orEmpty()
        if (rb.isNotEmpty()) {
            val seen = readReceiptUsers.getOrPut(payload.id) { mutableSetOf() }
            rb.keys.forEach { seen.add(it.toString()) }
        }
        val status =
            when {
                !event.isMine -> event.sendStatus
                rb.any { it.key.toString() != uid } -> EventSendStatus.Read
                dt.any { it.key.toString() != uid } -> EventSendStatus.Delivered
                else -> event.sendStatus
            }
        return event.copy(readBy = rb, deliveredTo = dt, sendStatus = status)
    }

    internal fun onHistoryResponse(msg: WsServerMessage.HistoryResponse) {
        cacheHistoryMembers(msg.messages)
        scope.launch {
            itemsMutex.withLock {
                val uid = wsConnection.userId.value
                val readsByMessage: Map<String, Map<Int, Long>> =
                    msg.readReceipts
                        .groupBy { it.messageId }
                        .mapValues { (_, list) ->
                            list.associate { it.userId to parseTimestamp(it.readAt) }
                        }
                val deliveriesByMessage: Map<String, Map<Int, Long>> =
                    msg.deliveries
                        .groupBy { it.messageId }
                        .mapValues { (_, list) ->
                            list.associate { it.userId to parseTimestamp(it.deliveredAt) }
                        }
                val historyItems =
                    msg.messages.map { payload ->
                        hydrateHistoryItem(payload, uid, readsByMessage, deliveriesByMessage)
                    }
                val current = _items.value.toMutableList()

                // Filter duplicates
                val existingIds =
                    current
                        .filterIsInstance<TimelineItem.Event>()
                        .mapNotNull { it.eventId }
                        .toSet()
                val newItems =
                    historyItems.filter { item ->
                        item.eventId !in existingIds
                    }

                // Prepend history
                current.addAll(0, newItems)
                if (!msg.hasMore) {
                    current.add(0, TimelineItem.TimelineStart)
                }

                _hasMoreBackwards.value = msg.hasMore
                _isPaginatingBackwards.value = false
                emitAndPersist(current)
            }
            pendingHistoryDeferred?.complete(msg.hasMore)
            pendingHistoryDeferred = null
        }
    }

    private fun emitAndPersist(items: List<TimelineItem>) {
        _items.value = items
        schedulePersist(items)
    }

    private fun schedulePersist(items: List<TimelineItem>) {
        persistJob?.cancel()
        persistJob =
            scope.launch {
                delay(350)
                roomTimelineCacheStore.saveSnapshot(
                    roomId = conversationId,
                    items = items.takeLast(500),
                    isHydrated = !_hasMoreBackwards.value,
                )
            }
    }

    override suspend fun paginateBackwards(count: UShort): Result<Boolean> {
        if (_isPaginatingBackwards.value) return Result.success(_hasMoreBackwards.value)
        if (!_hasMoreBackwards.value) return Result.success(false)

        _isPaginatingBackwards.value = true
        val deferred = CompletableDeferred<Boolean>()
        pendingHistoryDeferred = deferred

        val oldestId =
            _items.value
                .filterIsInstance<TimelineItem.Event>()
                .firstOrNull()
                ?.eventId

        val sent =
            wsConnection.send(
                WsClientMessage.History(
                    conversationId = conversationId,
                    before = oldestId,
                    limit = count.toInt(),
                ),
            )

        if (!sent) {
            _isPaginatingBackwards.value = false
            pendingHistoryDeferred = null
            return Result.failure(IllegalStateException("Not connected"))
        }

        return try {
            val hasMore =
                withTimeoutOrNull(10_000L) { deferred.await() }
                    ?: run {
                        _isPaginatingBackwards.value = false
                        pendingHistoryDeferred = null
                        return Result.failure(IllegalStateException("History request timed out"))
                    }
            Result.success(hasMore)
        } catch (
            @Suppress("TooGenericExceptionCaught") e: Exception,
        ) {
            _isPaginatingBackwards.value = false
            Result.failure(e)
        }
    }

    override suspend fun sendMessage(body: String): Result<Unit> {
        val clientId = "local_${Clock.System.now().toEpochMilliseconds()}_${Random.nextLong()}"
        addOptimisticItem(body, clientId)
        val sent =
            wsConnection.send(
                WsClientMessage.Send(
                    conversationId = conversationId,
                    body = body,
                    clientId = clientId,
                ),
            )
        if (!sent) {
            removeOptimisticItem(clientId)
            return Result.failure(IllegalStateException("Not connected"))
        }
        return Result.success(Unit)
    }

    override suspend fun sendReply(
        repliedToEventId: String,
        body: String,
    ): Result<Unit> {
        val clientId = "local_${Clock.System.now().toEpochMilliseconds()}_${Random.nextLong()}"
        addOptimisticItem(body, clientId)
        val sent =
            wsConnection.send(
                WsClientMessage.Send(
                    conversationId = conversationId,
                    body = body,
                    replyToId = repliedToEventId,
                    clientId = clientId,
                ),
            )
        if (!sent) {
            removeOptimisticItem(clientId)
            return Result.failure(IllegalStateException("Not connected"))
        }
        return Result.success(Unit)
    }

    override suspend fun sendImage(
        data: ByteArray,
        fileName: String,
        mimeType: String?,
        caption: String?,
        inReplyToEventId: String?,
    ): Result<Unit> = Result.failure(UnsupportedOperationException("Image sending not yet supported"))

    override suspend fun sendFile(
        data: ByteArray,
        fileName: String,
        mimeType: String?,
        caption: String?,
        inReplyToEventId: String?,
    ): Result<Unit> = Result.failure(UnsupportedOperationException("File sending not yet supported"))

    override suspend fun edit(
        eventOrTransactionId: String,
        body: String,
    ): Result<Unit> = sendOrFail(WsClientMessage.Edit(messageId = eventOrTransactionId, body = body), Unit)

    override suspend fun redact(
        eventOrTransactionId: String,
        reason: String?,
    ): Result<Unit> = sendOrFail(WsClientMessage.Delete(messageId = eventOrTransactionId), Unit)

    override suspend fun toggleReaction(
        eventOrTransactionId: String,
        emoji: String,
    ): Result<Boolean> = sendOrFail(WsClientMessage.React(messageId = eventOrTransactionId, emoji = emoji), true)

    override suspend fun markAsRead(): Result<Unit> {
        val lastEvent =
            _items.value
                .filterIsInstance<TimelineItem.Event>()
                .lastOrNull()
                ?: return Result.success(Unit)
        return sendReadReceipt(lastEvent.eventId ?: lastEvent.eventOrTransactionId)
    }

    override suspend fun sendReadReceipt(eventId: String): Result<Unit> =
        sendOrFail(WsClientMessage.Read(conversationId = conversationId, messageId = eventId), Unit)

    override suspend fun markModerationRevealed(eventId: String): Result<Unit> {
        itemsMutex.withLock {
            val current = _items.value.toMutableList()
            val idx =
                current.indexOfFirst {
                    it is TimelineItem.Event && it.eventId == eventId
                }
            if (idx >= 0) {
                val event = current[idx] as TimelineItem.Event
                if (!event.locallyRevealed) {
                    current[idx] = event.copy(locallyRevealed = true)
                    emitAndPersist(current)
                }
            }
        }
        return Result.success(Unit)
    }

    override suspend fun markModerationReported(eventId: String): Result<Unit> {
        itemsMutex.withLock {
            val current = _items.value.toMutableList()
            val idx =
                current.indexOfFirst {
                    it is TimelineItem.Event && it.eventId == eventId
                }
            if (idx >= 0) {
                val event = current[idx] as TimelineItem.Event
                if (!event.locallyReported) {
                    current[idx] = event.copy(locallyReported = true)
                    emitAndPersist(current)
                }
            }
        }
        return Result.success(Unit)
    }

    private suspend fun <T> sendOrFail(
        msg: WsClientMessage,
        value: T,
    ): Result<T> =
        if (wsConnection.send(msg)) {
            Result.success(value)
        } else {
            Result.failure(IllegalStateException("Not connected"))
        }

    override fun close() {
        scopeJob.cancel()
        pendingHistoryDeferred?.cancel()
    }

    private suspend fun removeOptimisticItem(clientId: String) {
        itemsMutex.withLock {
            val current = _items.value.toMutableList()
            current.removeAll {
                it is TimelineItem.Event && it.eventOrTransactionId == clientId
            }
            _items.value = current
        }
    }

    private suspend fun addOptimisticItem(
        body: String,
        clientId: String,
    ) {
        itemsMutex.withLock {
            val current = _items.value.toMutableList()
            current.add(
                TimelineItem.Event(
                    eventOrTransactionId = clientId,
                    eventId = null,
                    senderId = wsConnection.userId.value ?: "",
                    senderDisplayName = null,
                    isMine = true,
                    body = body,
                    timestampMillis = Clock.System.now().toEpochMilliseconds(),
                    inReplyTo = null,
                    reactions = emptyList(),
                    isEditable = true,
                    sendStatus = EventSendStatus.Sending,
                    canReply = true,
                ),
            )
            _items.value = current
        }
    }

    private fun cacheMembers(msg: WsServerMessage.Message) {
        memberCache?.put(msg.senderId.toString(), msg.senderName, msg.senderAvatar)
    }

    private fun cacheHistoryMembers(messages: List<WsMessagePayload>) {
        val cache = memberCache ?: return
        messages.forEach { msg ->
            cache.put(msg.senderId.toString(), msg.senderName, msg.senderAvatar)
        }
    }
}

@Suppress("LongParameterList")
private fun toTimelineEvent(
    id: String,
    senderId: Int,
    senderPid: String?,
    senderName: String?,
    senderAvatar: String?,
    isMine: Boolean,
    body: String,
    createdAt: String,
    replyTo: WsReplyPayload?,
    reactions: List<WsReactionPayload>,
    moderationVerdict: String? = null,
    moderationCategories: List<String> = emptyList(),
): TimelineItem.Event {
    val replyDetails =
        replyTo?.let {
            ReplyDetails(
                eventId = it.messageId,
                senderDisplayName = it.senderName,
                body = it.body,
            )
        }
    return TimelineItem.Event(
        eventOrTransactionId = id,
        eventId = id,
        senderId = senderId.toString(),
        senderPid = senderPid,
        senderDisplayName = senderName,
        senderAvatarUrl = senderAvatar,
        isMine = isMine,
        body = body,
        timestampMillis = parseTimestamp(createdAt),
        inReplyTo = replyDetails,
        reactions = reactions.map { it.toReaction() },
        isEditable = isMine,
        sendStatus = EventSendStatus.Sent,
        canReply = true,
        moderationVerdict = moderationVerdict,
        moderationCategories = moderationCategories,
    )
}

private fun WsServerMessage.Message.toTimelineItem(currentUserId: String?): TimelineItem.Event {
    val mine = currentUserId != null && senderId.toString() == currentUserId
    return toTimelineEvent(
        id,
        senderId,
        senderPid,
        senderName,
        senderAvatar,
        mine,
        body,
        createdAt,
        replyTo,
        reactions,
        moderationVerdict,
        moderationCategories,
    )
}

private fun WsReactionPayload.toReaction(): Reaction {
    val reactionSenders =
        userIds.zip(senderNames).map { (uid, name) ->
            ReactionSender(senderId = uid.toString(), displayName = name)
        }
    return Reaction(
        emoji = emoji,
        count = count,
        reactedByMe = reactedByMe,
        senders = reactionSenders,
    )
}

internal fun WsMessagePayload.toTimelineItem(currentUserId: String? = null): TimelineItem.Event {
    val mine = if (currentUserId != null) senderId.toString() == currentUserId else isMine
    return toTimelineEvent(
        id,
        senderId,
        senderPid,
        senderName,
        senderAvatar,
        mine,
        body,
        createdAt,
        replyTo,
        reactions,
        moderationVerdict,
        moderationCategories,
    )
}
