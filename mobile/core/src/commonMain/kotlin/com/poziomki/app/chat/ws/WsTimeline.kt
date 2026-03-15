package com.poziomki.app.chat.ws

import com.poziomki.app.chat.api.EventSendStatus
import com.poziomki.app.chat.api.Reaction
import com.poziomki.app.chat.api.ReactionSender
import com.poziomki.app.chat.api.ReplyDetails
import com.poziomki.app.chat.api.Timeline
import com.poziomki.app.chat.api.TimelineItem
import com.poziomki.app.chat.api.TimelineMode
import com.poziomki.app.chat.cache.RoomTimelineCacheStore
import com.poziomki.app.network.ApiResult
import com.poziomki.app.network.ApiService
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

@Suppress("TooManyFunctions")
class WsTimeline(
    private val conversationId: String,
    private val wsConnection: WsConnection,
    private val roomTimelineCacheStore: RoomTimelineCacheStore,
    private val apiService: ApiService? = null,
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
                val corrected = if (uid != null) {
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
                    val idx = current.indexOfFirst {
                        it is TimelineItem.Event && it.eventOrTransactionId == clientId
                    }
                    if (idx >= 0) {
                        current[idx] = item
                        emitAndPersist(current)
                        return@withLock
                    }
                }

                // Check for duplicate by id
                val exists = current.any {
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
                val idx = current.indexOfFirst {
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
                val idx = current.indexOfFirst {
                    it is TimelineItem.Event && it.eventId == msg.messageId
                }
                if (idx >= 0) {
                    val event = current[idx] as TimelineItem.Event
                    val reactions = event.reactions.toMutableList()
                    val existingIdx = reactions.indexOfFirst { it.emoji == msg.emoji }
                    val isMe = senderId == wsConnection.userId.value
                    if (msg.added) {
                        val sender = ReactionSender(
                            senderId = senderId,
                            displayName = msg.senderName,
                        )
                        if (existingIdx >= 0) {
                            val r = reactions[existingIdx]
                            reactions[existingIdx] = r.copy(
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
                            reactions[existingIdx] = r.copy(
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
        val receiptUserId = msg.userId.toString()
        if (receiptUserId == wsConnection.userId.value) return
        scope.launch {
            itemsMutex.withLock {
                val messageId = msg.messageId
                val seenUsers = readReceiptUsers.getOrPut(messageId) { mutableSetOf() }
                if (!seenUsers.add(receiptUserId)) return@withLock // duplicate

                val current = _items.value.toMutableList()
                val idx = current.indexOfFirst {
                    it is TimelineItem.Event && it.eventId == messageId
                }
                if (idx >= 0) {
                    val event = current[idx] as TimelineItem.Event
                    current[idx] = event.copy(readByCount = seenUsers.size)
                    emitAndPersist(current)
                }
            }
        }
    }

    internal fun onHistoryResponse(msg: WsServerMessage.HistoryResponse) {
        cacheHistoryMembers(msg.messages)
        scope.launch {
            itemsMutex.withLock {
                val uid = wsConnection.userId.value
                val historyItems = msg.messages.map { it.toTimelineItem(uid) }
                val current = _items.value.toMutableList()

                // Filter duplicates
                val existingIds = current
                    .filterIsInstance<TimelineItem.Event>()
                    .mapNotNull { it.eventId }
                    .toSet()
                val newItems = historyItems.filter { item ->
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
        persistJob = scope.launch {
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

        val oldestId = _items.value
            .filterIsInstance<TimelineItem.Event>()
            .firstOrNull()
            ?.eventId

        wsConnection.send(
            WsClientMessage.History(
                conversationId = conversationId,
                before = oldestId,
                limit = count.toInt(),
            ),
        )

        return try {
            val hasMore = withTimeoutOrNull(10_000L) { deferred.await() }
                ?: run {
                    _isPaginatingBackwards.value = false
                    return Result.failure(IllegalStateException("History request timed out"))
                }
            Result.success(hasMore)
        } catch (@Suppress("TooGenericExceptionCaught") e: Exception) {
            _isPaginatingBackwards.value = false
            Result.failure(e)
        }
    }

    override suspend fun sendMessage(body: String): Result<Unit> {
        val clientId = "local_${Clock.System.now().toEpochMilliseconds()}"
        addOptimisticItem(body, clientId)
        val sent = wsConnection.send(
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

    override suspend fun sendReply(repliedToEventId: String, body: String): Result<Unit> {
        val clientId = "local_${Clock.System.now().toEpochMilliseconds()}"
        addOptimisticItem(body, clientId)
        val sent = wsConnection.send(
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
    ): Result<Unit> {
        val api = apiService ?: return Result.failure(IllegalStateException("No ApiService"))
        val upload = when (val result = api.uploadImage(data, fileName, "chat_attachment")) {
            is ApiResult.Success -> result.data
            is ApiResult.Error -> return Result.failure(IllegalStateException(result.message))
        }
        wsConnection.send(
            WsClientMessage.Send(
                conversationId = conversationId,
                body = caption ?: fileName,
                attachmentUploadId = upload.filename,
                replyToId = inReplyToEventId,
                clientId = "local_${Clock.System.now().toEpochMilliseconds()}",
            ),
        )
        return Result.success(Unit)
    }

    override suspend fun sendFile(
        data: ByteArray,
        fileName: String,
        mimeType: String?,
        caption: String?,
        inReplyToEventId: String?,
    ): Result<Unit> = sendImage(data, fileName, mimeType, caption, inReplyToEventId)

    override suspend fun edit(eventOrTransactionId: String, body: String): Result<Unit> {
        wsConnection.send(WsClientMessage.Edit(messageId = eventOrTransactionId, body = body))
        return Result.success(Unit)
    }

    override suspend fun redact(eventOrTransactionId: String, reason: String?): Result<Unit> {
        wsConnection.send(WsClientMessage.Delete(messageId = eventOrTransactionId))
        return Result.success(Unit)
    }

    override suspend fun toggleReaction(eventOrTransactionId: String, emoji: String): Result<Boolean> {
        wsConnection.send(WsClientMessage.React(messageId = eventOrTransactionId, emoji = emoji))
        return Result.success(true)
    }

    override suspend fun markAsRead(): Result<Unit> {
        val lastEvent = _items.value
            .filterIsInstance<TimelineItem.Event>()
            .lastOrNull()
            ?: return Result.success(Unit)
        return sendReadReceipt(lastEvent.eventId ?: lastEvent.eventOrTransactionId)
    }

    override suspend fun sendReadReceipt(eventId: String): Result<Unit> {
        wsConnection.send(WsClientMessage.Read(conversationId = conversationId, messageId = eventId))
        return Result.success(Unit)
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

    private suspend fun addOptimisticItem(body: String, clientId: String) {
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
                    readByCount = 0,
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

private fun WsServerMessage.Message.toTimelineItem(currentUserId: String?): TimelineItem.Event {
    val mine = currentUserId != null && senderId.toString() == currentUserId
    return TimelineItem.Event(
        eventOrTransactionId = id,
        eventId = id,
        senderId = senderId.toString(),
        senderPid = senderPid,
        senderDisplayName = senderName,
        senderAvatarUrl = senderAvatar,
        isMine = mine,
        body = body,
        timestampMillis = parseTimestamp(createdAt),
        inReplyTo = replyTo?.let {
            ReplyDetails(
                eventId = it.messageId,
                senderDisplayName = it.senderName,
                body = it.body,
            )
        },
        reactions = reactions.map { it.toReaction() },
        isEditable = mine,
        sendStatus = EventSendStatus.Sent,
        readByCount = 0,
        canReply = true,
    )
}

private fun WsReactionPayload.toReaction(): Reaction =
    Reaction(
        emoji = emoji,
        count = count,
        reactedByMe = reactedByMe,
        senders = userIds.zip(senderNames).map { (uid, name) ->
            ReactionSender(senderId = uid.toString(), displayName = name)
        },
    )

internal fun WsMessagePayload.toTimelineItem(currentUserId: String? = null): TimelineItem.Event {
    val mine = if (currentUserId != null) senderId.toString() == currentUserId else isMine
    return TimelineItem.Event(
        eventOrTransactionId = id,
        eventId = id,
        senderId = senderId.toString(),
        senderPid = senderPid,
        senderDisplayName = senderName,
        senderAvatarUrl = senderAvatar,
        isMine = mine,
        body = body,
        timestampMillis = parseTimestamp(createdAt),
        inReplyTo = replyTo?.let {
            ReplyDetails(
                eventId = it.messageId,
                senderDisplayName = it.senderName,
                body = it.body,
            )
        },
        reactions = reactions.map { it.toReaction() },
        isEditable = mine,
        sendStatus = EventSendStatus.Sent,
        readByCount = 0,
        canReply = true,
    )
}
