/*
 * NOTICE: Portions of this interface are adapted from Element X Android Matrix API.
 * Copyright (c) 2025 Element Creations Ltd.
 * Copyright 2024-2025 New Vector Ltd.
 * SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-Element-Commercial.
 */
package com.poziomki.app.chat.api

import kotlinx.coroutines.flow.StateFlow

sealed interface TimelineMode {
    data object Live : TimelineMode

    data class FocusedOnEvent(
        val eventId: String,
    ) : TimelineMode
}

data class ReactionSender(
    val senderId: String,
    val displayName: String?,
)

data class Reaction(
    val emoji: String,
    val count: Int,
    val reactedByMe: Boolean,
    val senders: List<ReactionSender> = emptyList(),
)

data class ReplyDetails(
    val eventId: String,
    val senderDisplayName: String?,
    val body: String?,
)

enum class EventSendStatus {
    Sending,
    Sent,
    Failed,
}

sealed interface TimelineItem {
    data class Event(
        val eventOrTransactionId: String,
        val eventId: String?,
        val senderId: String,
        val senderPid: String? = null,
        val senderDisplayName: String?,
        val senderAvatarUrl: String? = null,
        val isMine: Boolean,
        val body: String,
        val timestampMillis: Long,
        val inReplyTo: ReplyDetails?,
        val reactions: List<Reaction>,
        val isEditable: Boolean,
        val sendStatus: EventSendStatus?,
        val readByCount: Int,
        val canReply: Boolean,
        /**
         * Bielik-Guard verdict for this message body. `null` until the
         * async scan completes (clients render as allow). `"allow"` /
         * `"flag"` / `"block"` once scanned.
         */
        val moderationVerdict: String? = null,
        /** Categories that exceeded the flag threshold, e.g. `["vulgar"]`. */
        val moderationCategories: List<String> = emptyList(),
        /**
         * `true` once the local viewer has tapped to reveal a
         * flagged/blocked message. Not persisted server-side per
         * device — re-blurs on app restart unless the cache layer
         * later carries the reveal state.
         */
        val locallyRevealed: Boolean = false,
        /**
         * `true` after the local viewer has filed a moderation
         * report against this message. Hides the floating flag so
         * a single message can only be reported once per device.
         * Persisted via the timeline cache so reinstall-style
         * resets don't allow stacking reports on the same row.
         */
        val locallyReported: Boolean = false,
    ) : TimelineItem

    data class DateDivider(
        val timestampMillis: Long,
    ) : TimelineItem

    data object ReadMarker : TimelineItem

    data object TimelineStart : TimelineItem
}

interface Timeline : AutoCloseable {
    val mode: TimelineMode
    val items: StateFlow<List<TimelineItem>>
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

    /**
     * Locally unblur a moderation-flagged message. The audit-trail
     * POST to `/chat/messages/:id/reveal` is the caller's
     * responsibility — keeping it out of the timeline lets a
     * higher-level view-model fail loudly on network errors without
     * blocking the UI from updating. Idempotent.
     */
    suspend fun markModerationRevealed(eventId: String): Result<Unit>

    /**
     * Mark this message as reported locally so the floating flag
     * is hidden. Idempotent.
     */
    suspend fun markModerationReported(eventId: String): Result<Unit>

    override fun close()
}
