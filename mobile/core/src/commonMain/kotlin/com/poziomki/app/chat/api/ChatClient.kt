/*
 * NOTICE: Portions of this interface are adapted from Element X Android Matrix API.
 * Copyright (c) 2025 Element Creations Ltd.
 * Copyright 2023-2025 New Vector Ltd.
 * SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-Element-Commercial.
 */
package com.poziomki.app.chat.api

import kotlinx.coroutines.flow.StateFlow

sealed interface ChatClientState {
    data object Idle : ChatClientState

    data object Connecting : ChatClientState

    data class Ready(
        val userId: String,
        val deviceId: String,
    ) : ChatClientState

    data class Error(
        val message: String,
    ) : ChatClientState
}

data class RoomSummary(
    val roomId: String,
    val displayName: String,
    val avatarUrl: String?,
    val isDirect: Boolean,
    val directUserId: String? = null,
    val unreadCount: Int,
    val latestMessage: String?,
    val latestTimestampMillis: Long?,
    val latestMessageIsMine: Boolean = false,
    val latestMessageSendStatus: EventSendStatus? = null,
    val latestMessageReadByCount: Int = 0,
    val isBlocked: Boolean = false,
    val isMuted: Boolean = false,
    /** Bielik-Guard verdict for the latest message body, or null when unscanned / mine. */
    val latestModerationVerdict: String? = null,
    /** Categories that exceeded the flag threshold for the latest message. */
    val latestModerationCategories: List<String> = emptyList(),
)

data class RoomTimelineCacheSnapshot(
    val items: List<TimelineItem>,
    val isHydrated: Boolean,
    val cachedItemCount: Int,
    val updatedAtMillis: Long,
)

@Suppress("TooManyFunctions")
interface ChatClient {
    val state: StateFlow<ChatClientState>
    val rooms: StateFlow<List<RoomSummary>>

    suspend fun ensureStarted(): Result<Unit>

    suspend fun refreshRooms(): Result<Unit>

    suspend fun getJoinedRoom(roomId: String): JoinedRoom?

    suspend fun getRoomTimelineCache(
        roomId: String,
        limit: Int = 500,
    ): RoomTimelineCacheSnapshot

    suspend fun requestRoomTimelineBackfill(roomId: String): Result<Unit>

    suspend fun createDM(
        userId: String,
        displayName: String? = null,
    ): Result<String>

    suspend fun createRoom(
        name: String,
        invitedUserIds: List<String>,
    ): Result<String>

    suspend fun registerPusher(
        fcmToken: String,
        platform: String,
    ): Result<Unit>

    suspend fun unregisterPusher(): Result<Unit>

    suspend fun hideConversation(roomId: String)

    /**
     * Optimistically clears `unreadCount` for [roomId] in the local rooms
     * StateFlow so the UI un-bolds the row immediately. Server confirmation
     * (via Read → ReadReceipt round-trip) still happens, but the user sees
     * the room as "read" the moment they open it, not after the network
     * round-trip completes.
     */
    suspend fun markRoomReadLocally(roomId: String)

    /**
     * Marks [roomId] as the currently focused room. Inbound messages for
     * this room won't bump `unreadCount` while it's focused — the user is
     * actively reading. Pass `null` when the user leaves the chat screen.
     */
    suspend fun setActiveRoom(roomId: String?)

    suspend fun stop()
}
