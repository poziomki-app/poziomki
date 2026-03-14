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
        val homeserver: String,
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
)

data class RoomTimelineCacheSnapshot(
    val items: List<TimelineItem>,
    val isHydrated: Boolean,
    val cachedItemCount: Int,
    val updatedAtMillis: Long,
)

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
        ntfyEndpoint: String,
        gatewayUrl: String,
    ): Result<Unit>

    suspend fun unregisterPusher(ntfyEndpoint: String): Result<Unit>

    suspend fun getMediaThumbnail(
        mxcUrl: String,
        width: Long,
        height: Long,
    ): ByteArray?

    suspend fun getMediaContent(mxcUrl: String): ByteArray?

    suspend fun stop()
}
