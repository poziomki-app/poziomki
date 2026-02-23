/*
 * NOTICE: Portions of this interface are adapted from Element X Android Matrix API.
 * Copyright (c) 2025 Element Creations Ltd.
 * Copyright 2024-2025 New Vector Ltd.
 * SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-Element-Commercial.
 */
package com.poziomki.app.chat.matrix.api

import kotlinx.coroutines.flow.StateFlow

sealed interface MatrixTimelineMode {
    data object Live : MatrixTimelineMode

    data class FocusedOnEvent(
        val eventId: String,
    ) : MatrixTimelineMode
}

data class MatrixReactionSender(
    val senderId: String,
    val displayName: String?,
)

data class MatrixReaction(
    val emoji: String,
    val count: Int,
    val reactedByMe: Boolean,
    val senders: List<MatrixReactionSender> = emptyList(),
)

data class MatrixReplyDetails(
    val eventId: String,
    val senderDisplayName: String?,
    val body: String?,
)

sealed interface MatrixTimelineItem {
    data class Event(
        val eventOrTransactionId: String,
        val eventId: String?,
        val senderId: String,
        val senderDisplayName: String?,
        val senderAvatarUrl: String? = null,
        val isMine: Boolean,
        val body: String,
        val timestampMillis: Long,
        val inReplyTo: MatrixReplyDetails?,
        val reactions: List<MatrixReaction>,
        val isEditable: Boolean,
        val readByCount: Int,
        val canReply: Boolean,
    ) : MatrixTimelineItem

    data class DateDivider(
        val timestampMillis: Long,
    ) : MatrixTimelineItem

    data object ReadMarker : MatrixTimelineItem

    data object TimelineStart : MatrixTimelineItem
}

interface Timeline : AutoCloseable {
    val mode: MatrixTimelineMode
    val items: StateFlow<List<MatrixTimelineItem>>
    val isPaginatingBackwards: StateFlow<Boolean>
    val hasMoreBackwards: StateFlow<Boolean>

    suspend fun paginateBackwards(count: UShort = 50u): Result<Boolean>

    suspend fun sendMessage(body: String): Result<Unit>

    suspend fun sendReply(
        repliedToEventId: String,
        body: String,
    ): Result<Unit>

    suspend fun sendImage(
        data: ByteArray,
        fileName: String,
        mimeType: String? = null,
        caption: String? = null,
        inReplyToEventId: String? = null,
    ): Result<Unit>

    suspend fun sendFile(
        data: ByteArray,
        fileName: String,
        mimeType: String? = null,
        caption: String? = null,
        inReplyToEventId: String? = null,
    ): Result<Unit>

    suspend fun edit(
        eventOrTransactionId: String,
        body: String,
    ): Result<Unit>

    suspend fun redact(
        eventOrTransactionId: String,
        reason: String? = null,
    ): Result<Unit>

    suspend fun toggleReaction(
        eventOrTransactionId: String,
        emoji: String,
    ): Result<Boolean>

    suspend fun markAsRead(): Result<Unit>

    suspend fun sendReadReceipt(eventId: String): Result<Unit>

    override fun close()
}
