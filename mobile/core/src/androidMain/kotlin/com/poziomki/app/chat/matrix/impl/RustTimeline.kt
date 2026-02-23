/*
 * NOTICE: Portions of this implementation are adapted from Element X Android Matrix timeline code.
 * Copyright (c) 2025 Element Creations Ltd.
 * Copyright 2024-2025 New Vector Ltd.
 * SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-Element-Commercial.
 */
package com.poziomki.app.chat.matrix.impl

import com.poziomki.app.chat.matrix.api.MatrixReaction
import com.poziomki.app.chat.matrix.api.MatrixReactionSender
import com.poziomki.app.chat.matrix.api.MatrixReplyDetails
import com.poziomki.app.chat.matrix.api.MatrixTimelineItem
import com.poziomki.app.chat.matrix.api.MatrixTimelineMode
import com.poziomki.app.chat.matrix.api.Timeline
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.launch
import org.matrix.rustcomponents.sdk.EditedContent
import org.matrix.rustcomponents.sdk.EmbeddedEventDetails
import org.matrix.rustcomponents.sdk.EventOrTransactionId
import org.matrix.rustcomponents.sdk.FileInfo
import org.matrix.rustcomponents.sdk.ImageInfo
import org.matrix.rustcomponents.sdk.MessageType
import org.matrix.rustcomponents.sdk.MsgLikeKind
import org.matrix.rustcomponents.sdk.PaginationStatusListener
import org.matrix.rustcomponents.sdk.ProfileDetails
import org.matrix.rustcomponents.sdk.ReceiptType
import org.matrix.rustcomponents.sdk.TimelineDiff
import org.matrix.rustcomponents.sdk.TimelineItem
import org.matrix.rustcomponents.sdk.TimelineItemContent
import org.matrix.rustcomponents.sdk.TimelineListener
import org.matrix.rustcomponents.sdk.UploadParameters
import org.matrix.rustcomponents.sdk.UploadSource
import org.matrix.rustcomponents.sdk.VirtualTimelineItem
import org.matrix.rustcomponents.sdk.messageEventContentFromMarkdown
import uniffi.matrix_sdk.RoomPaginationStatus

class RustTimeline(
    private val inner: org.matrix.rustcomponents.sdk.Timeline,
    override val mode: MatrixTimelineMode,
    private val ownUserId: String,
    private val coroutineScope: CoroutineScope,
) : Timeline {
    private val rawItems = mutableListOf<TimelineItem>()

    private val _items = MutableStateFlow<List<MatrixTimelineItem>>(emptyList())
    override val items: StateFlow<List<MatrixTimelineItem>> = _items

    private val _isPaginatingBackwards = MutableStateFlow(false)
    override val isPaginatingBackwards: StateFlow<Boolean> = _isPaginatingBackwards

    private val _hasMoreBackwards = MutableStateFlow(true)
    override val hasMoreBackwards: StateFlow<Boolean> = _hasMoreBackwards

    private var listenerHandle: org.matrix.rustcomponents.sdk.TaskHandle? = null
    private var paginationStatusHandle: org.matrix.rustcomponents.sdk.TaskHandle? = null

    init {
        coroutineScope.launch(Dispatchers.Default) {
            runCatching {
                inner.addListener(
                    object : TimelineListener {
                        override fun onUpdate(diff: List<TimelineDiff>) {
                            applyDiffs(diff)
                        }
                    },
                )
            }.onSuccess { handle ->
                listenerHandle = handle
            }
        }

        // Fetch room members so senderProfile resolves to ProfileDetails.Ready (with avatarUrl)
        coroutineScope.launch(Dispatchers.Default) {
            runCatching { inner.fetchMembers() }
        }

        // For live timelines, subscribe to back-pagination status so the SDK can
        // auto-paginate and we get accurate hasMoreBackwards / isPaginatingBackwards.
        if (mode == MatrixTimelineMode.Live) {
            coroutineScope.launch(Dispatchers.Default) {
                runCatching {
                    inner.subscribeToBackPaginationStatus(
                        object : PaginationStatusListener {
                            override fun onUpdate(status: RoomPaginationStatus) {
                                when (status) {
                                    is RoomPaginationStatus.Idle -> {
                                        _isPaginatingBackwards.value = false
                                        _hasMoreBackwards.value = !status.hitTimelineStart
                                    }

                                    is RoomPaginationStatus.Paginating -> {
                                        _isPaginatingBackwards.value = true
                                        _hasMoreBackwards.value = true
                                    }
                                }
                            }
                        },
                    )
                }.onSuccess { handle ->
                    paginationStatusHandle = handle
                }
            }
        }
    }

    private fun applyDiffs(diff: List<TimelineDiff>) {
        synchronized(rawItems) {
            diff.forEach { update ->
                when (update) {
                    is TimelineDiff.Append -> {
                        rawItems.addAll(update.values)
                    }

                    is TimelineDiff.Clear -> {
                        rawItems.clear()
                    }

                    is TimelineDiff.PushFront -> {
                        rawItems.add(0, update.value)
                    }

                    is TimelineDiff.PushBack -> {
                        rawItems.add(update.value)
                    }

                    TimelineDiff.PopFront -> {
                        if (rawItems.isNotEmpty()) rawItems.removeAt(0)
                    }

                    TimelineDiff.PopBack -> {
                        if (rawItems.isNotEmpty()) rawItems.removeAt(rawItems.lastIndex)
                    }

                    is TimelineDiff.Insert -> {
                        val index = update.index.toInt().coerceIn(0, rawItems.size)
                        rawItems.add(index, update.value)
                    }

                    is TimelineDiff.Set -> {
                        val index = update.index.toInt()
                        if (index in rawItems.indices) {
                            rawItems[index] = update.value
                        }
                    }

                    is TimelineDiff.Remove -> {
                        val index = update.index.toInt()
                        if (index in rawItems.indices) {
                            rawItems.removeAt(index)
                        }
                    }

                    is TimelineDiff.Truncate -> {
                        val size = update.length.toInt().coerceAtLeast(0)
                        if (size < rawItems.size) {
                            rawItems.subList(size, rawItems.size).clear()
                        }
                    }

                    is TimelineDiff.Reset -> {
                        rawItems.clear()
                        rawItems.addAll(update.values)
                    }
                }
            }

            _items.value = rawItems.mapNotNull { item -> item.toUiTimelineItem(ownUserId) }
        }
    }

    override suspend fun paginateBackwards(count: UShort): Result<Boolean> {
        _isPaginatingBackwards.value = true
        return runCatching {
            val hitStart = inner.paginateBackwards(count)
            val hasMore = !hitStart
            _hasMoreBackwards.value = hasMore
            hasMore
        }.also {
            _isPaginatingBackwards.value = false
        }
    }

    override suspend fun sendMessage(body: String): Result<Unit> =
        runCatching {
            inner.send(messageEventContentFromMarkdown(body))
            Unit
        }

    override suspend fun sendReply(
        repliedToEventId: String,
        body: String,
    ): Result<Unit> =
        runCatching {
            inner.sendReply(messageEventContentFromMarkdown(body), repliedToEventId)
            Unit
        }

    override suspend fun sendImage(
        data: ByteArray,
        fileName: String,
        mimeType: String?,
        caption: String?,
        inReplyToEventId: String?,
    ): Result<Unit> =
        runCatching {
            val uploadSource = UploadSource.Data(bytes = data, filename = fileName)
            val resolvedMimeType = mimeType ?: "image/jpeg"
            val uploadParameters =
                UploadParameters(
                    source = uploadSource,
                    caption = caption,
                    formattedCaption = null,
                    mentions = null,
                    inReplyTo = inReplyToEventId,
                )
            val imageInfo =
                ImageInfo(
                    height = 0uL,
                    width = 0uL,
                    mimetype = resolvedMimeType,
                    size = data.size.toULong(),
                    thumbnailInfo = null,
                    thumbnailSource = null,
                    blurhash = null,
                    isAnimated = null,
                )
            val joinHandle = inner.sendImage(uploadParameters, uploadSource, imageInfo)
            try {
                joinHandle.join()
            } finally {
                joinHandle.close()
                imageInfo.destroy()
            }
            Unit
        }

    override suspend fun sendFile(
        data: ByteArray,
        fileName: String,
        mimeType: String?,
        caption: String?,
        inReplyToEventId: String?,
    ): Result<Unit> =
        runCatching {
            val uploadSource = UploadSource.Data(bytes = data, filename = fileName)
            val resolvedMimeType = mimeType ?: "application/octet-stream"
            val uploadParameters =
                UploadParameters(
                    source = uploadSource,
                    caption = caption,
                    formattedCaption = null,
                    mentions = null,
                    inReplyTo = inReplyToEventId,
                )
            val fileInfo =
                FileInfo(
                    mimetype = resolvedMimeType,
                    size = data.size.toULong(),
                    thumbnailInfo = null,
                    thumbnailSource = null,
                )
            val joinHandle = inner.sendFile(uploadParameters, fileInfo)
            try {
                joinHandle.join()
            } finally {
                joinHandle.close()
                fileInfo.destroy()
            }
            Unit
        }

    override suspend fun edit(
        eventOrTransactionId: String,
        body: String,
    ): Result<Unit> =
        runCatching {
            inner.edit(
                eventOrTransactionId = eventOrTransactionId.toEventOrTransactionId(),
                newContent = EditedContent.RoomMessage(messageEventContentFromMarkdown(body)),
            )
            Unit
        }

    override suspend fun redact(
        eventOrTransactionId: String,
        reason: String?,
    ): Result<Unit> =
        runCatching {
            inner.redactEvent(eventOrTransactionId.toEventOrTransactionId(), reason)
            Unit
        }

    override suspend fun toggleReaction(
        eventOrTransactionId: String,
        emoji: String,
    ): Result<Boolean> = runCatching { inner.toggleReaction(eventOrTransactionId.toEventOrTransactionId(), emoji) }

    override suspend fun markAsRead(): Result<Unit> =
        runCatching {
            inner.markAsRead(ReceiptType.READ)
            Unit
        }

    override suspend fun sendReadReceipt(eventId: String): Result<Unit> =
        runCatching {
            inner.sendReadReceipt(ReceiptType.READ, eventId)
            Unit
        }

    override fun close() {
        paginationStatusHandle?.cancel()
        listenerHandle?.cancel()
        inner.close()
    }
}

private fun String.toEventOrTransactionId(): EventOrTransactionId =
    if (startsWith("$")) {
        EventOrTransactionId.EventId(this)
    } else {
        EventOrTransactionId.TransactionId(this)
    }

private fun TimelineItem.toUiTimelineItem(ownUserId: String): MatrixTimelineItem? {
    val virtual = asVirtual()
    if (virtual != null) {
        return when (virtual) {
            is VirtualTimelineItem.DateDivider -> MatrixTimelineItem.DateDivider(timestampMillis = virtual.ts.toLong())
            VirtualTimelineItem.ReadMarker -> MatrixTimelineItem.ReadMarker
            VirtualTimelineItem.TimelineStart -> MatrixTimelineItem.TimelineStart
        }
    }

    val event = asEvent() ?: return null

    // Filter out state events — only show message-like content in the timeline
    when (event.content) {
        is TimelineItemContent.ProfileChange,
        is TimelineItemContent.RoomMembership,
        is TimelineItemContent.State,
        TimelineItemContent.CallInvite,
        TimelineItemContent.RtcNotification,
        is TimelineItemContent.FailedToParseState,
        -> {
            return null
        }

        else -> {} // continue processing
    }

    val senderDisplayName =
        when (val profile = event.senderProfile) {
            is ProfileDetails.Ready -> profile.displayName
            else -> null
        }
    val senderAvatarUrl =
        when (val profile = event.senderProfile) {
            is ProfileDetails.Ready -> profile.avatarUrl
            else -> null
        }

    val messageBody = timelineContentToText(event.content)
    val reactions =
        when (val content = event.content) {
            is TimelineItemContent.MsgLike -> {
                content.content.reactions.map { reaction ->
                    MatrixReaction(
                        emoji = reaction.key,
                        count = reaction.senders.size,
                        reactedByMe = reaction.senders.any { sender -> sender.senderId.sameMatrixUser(ownUserId) },
                        senders =
                            reaction.senders.map { sender ->
                                MatrixReactionSender(
                                    senderId = sender.senderId,
                                    displayName = null,
                                )
                            },
                    )
                }
            }

            else -> {
                emptyList()
            }
        }
    val inReplyTo = extractReplyDetails(event.content)

    val eventId =
        when (val eventOrTransactionId = event.eventOrTransactionId) {
            is EventOrTransactionId.EventId -> eventOrTransactionId.eventId
            is EventOrTransactionId.TransactionId -> null
        }

    val rawId =
        when (val eventOrTransactionId = event.eventOrTransactionId) {
            is EventOrTransactionId.EventId -> eventOrTransactionId.eventId
            is EventOrTransactionId.TransactionId -> eventOrTransactionId.transactionId
        }

    return MatrixTimelineItem.Event(
        eventOrTransactionId = rawId,
        eventId = eventId,
        senderId = event.sender,
        senderDisplayName = senderDisplayName,
        senderAvatarUrl = senderAvatarUrl,
        // Prefer sender id matching over SDK ownership flags to avoid wrong-side bubbles
        // when the local SDK mislabels remote events in encrypted/direct timelines.
        isMine = event.sender.sameMatrixUser(ownUserId) || (event.sender.isBlank() && event.isOwn),
        body = messageBody,
        timestampMillis = event.timestamp.toLong(),
        inReplyTo = inReplyTo,
        reactions = reactions,
        isEditable = event.isEditable,
        readByCount = event.readReceipts.keys.count { userId -> !userId.sameMatrixUser(ownUserId) },
        canReply = event.canBeRepliedTo,
    )
}

private fun String.sameMatrixUser(other: String): Boolean = trim().equals(other.trim(), ignoreCase = true)

private fun extractReplyDetails(content: TimelineItemContent): MatrixReplyDetails? {
    val msgLike = content as? TimelineItemContent.MsgLike ?: return null
    val inReplyTo = msgLike.content.inReplyTo ?: return null
    val embedded = inReplyTo.event()
    return when (embedded) {
        is EmbeddedEventDetails.Ready -> {
            val senderDisplayName =
                when (val profile = embedded.senderProfile) {
                    is ProfileDetails.Ready -> profile.displayName
                    else -> null
                }
            MatrixReplyDetails(
                eventId = inReplyTo.eventId(),
                senderDisplayName = senderDisplayName,
                body = timelineContentToText(embedded.content),
            )
        }

        else -> {
            MatrixReplyDetails(
                eventId = inReplyTo.eventId(),
                senderDisplayName = null,
                body = null,
            )
        }
    }
}

private fun timelineContentToText(content: TimelineItemContent): String =
    when (content) {
        is TimelineItemContent.MsgLike -> {
            when (val kind = content.content.kind) {
                is MsgLikeKind.Message -> {
                    when (val msgType = kind.content.msgType) {
                        is MessageType.Text -> msgType.content.body
                        is MessageType.Notice -> msgType.content.body
                        is MessageType.Emote -> msgType.content.body
                        is MessageType.Image -> "Image"
                        is MessageType.Video -> "Video"
                        is MessageType.Audio -> "Audio"
                        is MessageType.File -> "File"
                        is MessageType.Location -> msgType.content.body
                        is MessageType.Gallery -> "Gallery"
                        is MessageType.Other -> msgType.body
                    }
                }

                MsgLikeKind.Redacted -> {
                    "Message removed"
                }

                is MsgLikeKind.Poll -> {
                    "Poll: ${kind.question}"
                }

                is MsgLikeKind.Sticker -> {
                    kind.body
                }

                is MsgLikeKind.UnableToDecrypt -> {
                    "Encrypted message"
                }

                is MsgLikeKind.Other -> {
                    "Unsupported message"
                }
            }
        }

        TimelineItemContent.CallInvite -> {
            "Call invite"
        }

        TimelineItemContent.RtcNotification -> {
            "RTC notification"
        }

        is TimelineItemContent.ProfileChange -> {
            "Profile updated"
        }

        is TimelineItemContent.RoomMembership -> {
            "Membership updated"
        }

        is TimelineItemContent.State -> {
            "State updated"
        }

        is TimelineItemContent.FailedToParseMessageLike -> {
            "Unsupported event"
        }

        is TimelineItemContent.FailedToParseState -> {
            "Unsupported state event"
        }
    }
