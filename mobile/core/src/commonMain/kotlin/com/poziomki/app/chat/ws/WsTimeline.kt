package com.poziomki.app.chat.ws

import com.poziomki.app.chat.cache.RoomTimelineCacheStore
import com.poziomki.app.chat.matrix.api.MatrixEventSendStatus
import com.poziomki.app.chat.matrix.api.MatrixReaction
import com.poziomki.app.chat.matrix.api.MatrixReactionSender
import com.poziomki.app.chat.matrix.api.MatrixReplyDetails
import com.poziomki.app.chat.matrix.api.MatrixTimelineItem
import com.poziomki.app.chat.matrix.api.MatrixTimelineMode
import com.poziomki.app.chat.matrix.api.Timeline
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
import kotlinx.datetime.Clock

@Suppress("TooManyFunctions")
class WsTimeline(
    private val conversationId: String,
    private val wsConnection: WsConnection,
    private val roomTimelineCacheStore: RoomTimelineCacheStore,
    private val apiService: ApiService? = null,
    override val mode: MatrixTimelineMode = MatrixTimelineMode.Live,
) : Timeline {
    private val scope = CoroutineScope(SupervisorJob() + Dispatchers.Default)
    private val itemsMutex = Mutex()

    private val _items = MutableStateFlow<List<MatrixTimelineItem>>(emptyList())
    override val items: StateFlow<List<MatrixTimelineItem>> = _items

    private val _isPaginatingBackwards = MutableStateFlow(false)
    override val isPaginatingBackwards: StateFlow<Boolean> = _isPaginatingBackwards

    private val _hasMoreBackwards = MutableStateFlow(true)
    override val hasMoreBackwards: StateFlow<Boolean> = _hasMoreBackwards

    private var pendingHistoryDeferred: CompletableDeferred<Boolean>? = null
    private var debounceJob: Job? = null
    private var persistJob: Job? = null

    init {
        // Load cached items on start, request history if empty
        scope.launch {
            val cached = roomTimelineCacheStore.loadSnapshot(conversationId)
            if (cached.items.isNotEmpty()) {
                _items.value = cached.items
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

    internal fun onMessage(msg: WsServerMessage.Message) {
        scope.launch {
            itemsMutex.withLock {
                val item = msg.toTimelineItem()
                val current = _items.value.toMutableList()

                // Check for optimistic update by clientId
                val clientId = msg.clientId
                if (clientId != null) {
                    val idx = current.indexOfFirst {
                        it is MatrixTimelineItem.Event && it.eventOrTransactionId == clientId
                    }
                    if (idx >= 0) {
                        current[idx] = item
                        scheduleEmit(current)
                        return@withLock
                    }
                }

                // Check for duplicate by id
                val exists = current.any {
                    it is MatrixTimelineItem.Event && it.eventId == msg.id
                }
                if (!exists) {
                    current.add(item)
                    scheduleEmit(current)
                }
            }
        }
    }

    internal fun onEdited(msg: WsServerMessage.Edited) {
        scope.launch {
            itemsMutex.withLock {
                val current = _items.value.toMutableList()
                val idx = current.indexOfFirst {
                    it is MatrixTimelineItem.Event && it.eventId == msg.messageId
                }
                if (idx >= 0) {
                    val event = current[idx] as MatrixTimelineItem.Event
                    current[idx] = event.copy(body = msg.body)
                    scheduleEmit(current)
                }
            }
        }
    }

    internal fun onDeleted(msg: WsServerMessage.Deleted) {
        scope.launch {
            itemsMutex.withLock {
                val current = _items.value.toMutableList()
                current.removeAll {
                    it is MatrixTimelineItem.Event && it.eventId == msg.messageId
                }
                scheduleEmit(current)
            }
        }
    }

    internal fun onReaction(msg: WsServerMessage.Reaction) {
        scope.launch {
            itemsMutex.withLock {
                val current = _items.value.toMutableList()
                val idx = current.indexOfFirst {
                    it is MatrixTimelineItem.Event && it.eventId == msg.messageId
                }
                if (idx >= 0) {
                    val event = current[idx] as MatrixTimelineItem.Event
                    val reactions = event.reactions.toMutableList()
                    val existingIdx = reactions.indexOfFirst { it.emoji == msg.emoji }
                    if (msg.added) {
                        if (existingIdx >= 0) {
                            val r = reactions[existingIdx]
                            reactions[existingIdx] = r.copy(
                                count = r.count + 1,
                                reactedByMe = r.reactedByMe || (msg.userId.toString() == wsConnection.userId.value),
                            )
                        } else {
                            reactions.add(
                                MatrixReaction(
                                    emoji = msg.emoji,
                                    count = 1,
                                    reactedByMe = msg.userId.toString() == wsConnection.userId.value,
                                ),
                            )
                        }
                    } else if (existingIdx >= 0) {
                        val r = reactions[existingIdx]
                        if (r.count <= 1) {
                            reactions.removeAt(existingIdx)
                        } else {
                            val wasMe = msg.userId.toString() == wsConnection.userId.value
                            reactions[existingIdx] = r.copy(
                                count = r.count - 1,
                                reactedByMe = if (wasMe) false else r.reactedByMe,
                            )
                        }
                    }
                    current[idx] = event.copy(reactions = reactions)
                    scheduleEmit(current)
                }
            }
        }
    }

    @Suppress("UnusedParameter")
    internal fun onReadReceipt(msg: WsServerMessage.ReadReceipt) {
        // Could update readByCount on messages - simplified for now
    }

    internal fun onHistoryResponse(msg: WsServerMessage.HistoryResponse) {
        scope.launch {
            itemsMutex.withLock {
                val historyItems = msg.messages.map { it.toTimelineItem() }
                val current = _items.value.toMutableList()

                // Filter duplicates
                val existingIds = current
                    .filterIsInstance<MatrixTimelineItem.Event>()
                    .mapNotNull { it.eventId }
                    .toSet()
                val newItems = historyItems.filter { item ->
                    item.eventId !in existingIds
                }

                // Prepend history
                current.addAll(0, newItems)
                if (!msg.hasMore) {
                    current.add(0, MatrixTimelineItem.TimelineStart)
                }

                _hasMoreBackwards.value = msg.hasMore
                _isPaginatingBackwards.value = false
                scheduleEmit(current)
            }
            pendingHistoryDeferred?.complete(msg.hasMore)
            pendingHistoryDeferred = null
        }
    }

    private fun scheduleEmit(items: List<MatrixTimelineItem>) {
        debounceJob?.cancel()
        debounceJob = scope.launch {
            delay(120)
            _items.value = items
            schedulePersist(items)
        }
    }

    private fun schedulePersist(items: List<MatrixTimelineItem>) {
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
            .filterIsInstance<MatrixTimelineItem.Event>()
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
            val hasMore = deferred.await()
            Result.success(hasMore)
        } catch (@Suppress("TooGenericExceptionCaught") e: Exception) {
            _isPaginatingBackwards.value = false
            Result.failure(e)
        }
    }

    override suspend fun sendMessage(body: String): Result<Unit> {
        val clientId = "local_${Clock.System.now().toEpochMilliseconds()}"
        addOptimisticItem(body, clientId)
        wsConnection.send(
            WsClientMessage.Send(
                conversationId = conversationId,
                body = body,
                clientId = clientId,
            ),
        )
        return Result.success(Unit)
    }

    override suspend fun sendReply(repliedToEventId: String, body: String): Result<Unit> {
        val clientId = "local_${Clock.System.now().toEpochMilliseconds()}"
        wsConnection.send(
            WsClientMessage.Send(
                conversationId = conversationId,
                body = body,
                replyToId = repliedToEventId,
                clientId = clientId,
            ),
        )
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
            .filterIsInstance<MatrixTimelineItem.Event>()
            .lastOrNull()
            ?: return Result.success(Unit)
        return sendReadReceipt(lastEvent.eventId ?: lastEvent.eventOrTransactionId)
    }

    override suspend fun sendReadReceipt(eventId: String): Result<Unit> {
        wsConnection.send(WsClientMessage.Read(conversationId = conversationId, messageId = eventId))
        return Result.success(Unit)
    }

    override fun close() {
        debounceJob?.cancel()
        persistJob?.cancel()
        pendingHistoryDeferred?.cancel()
    }

    private suspend fun addOptimisticItem(body: String, clientId: String) {
        itemsMutex.withLock {
            val current = _items.value.toMutableList()
            current.add(
                MatrixTimelineItem.Event(
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
                    sendStatus = MatrixEventSendStatus.Sending,
                    readByCount = 0,
                    canReply = true,
                ),
            )
            _items.value = current
        }
    }
}

private fun WsServerMessage.Message.toTimelineItem(): MatrixTimelineItem.Event =
    MatrixTimelineItem.Event(
        eventOrTransactionId = id,
        eventId = id,
        senderId = senderId.toString(),
        senderDisplayName = senderName,
        senderAvatarUrl = senderAvatar,
        isMine = isMine,
        body = body,
        timestampMillis = parseTimestamp(createdAt),
        inReplyTo = replyTo?.let {
            MatrixReplyDetails(
                eventId = it.messageId,
                senderDisplayName = it.senderName,
                body = it.body,
            )
        },
        reactions = reactions.map { it.toMatrixReaction() },
        isEditable = isMine,
        sendStatus = MatrixEventSendStatus.Sent,
        readByCount = 0,
        canReply = true,
    )

private fun WsReactionPayload.toMatrixReaction(): MatrixReaction =
    MatrixReaction(
        emoji = emoji,
        count = count,
        reactedByMe = reactedByMe,
        senders = userIds.zip(senderNames).map { (uid, name) ->
            MatrixReactionSender(senderId = uid.toString(), displayName = name)
        },
    )

internal fun WsMessagePayload.toTimelineItem(): MatrixTimelineItem.Event =
    MatrixTimelineItem.Event(
        eventOrTransactionId = id,
        eventId = id,
        senderId = senderId.toString(),
        senderDisplayName = senderName,
        senderAvatarUrl = senderAvatar,
        isMine = isMine,
        body = body,
        timestampMillis = parseTimestamp(createdAt),
        inReplyTo = replyTo?.let {
            MatrixReplyDetails(
                eventId = it.messageId,
                senderDisplayName = it.senderName,
                body = it.body,
            )
        },
        reactions = reactions.map { it.toMatrixReaction() },
        isEditable = isMine,
        sendStatus = MatrixEventSendStatus.Sent,
        readByCount = 0,
        canReply = true,
    )
