/*
 * NOTICE: Portions of this interface are adapted from Element X Android Matrix API.
 * Copyright (c) 2025 Element Creations Ltd.
 * Copyright 2023-2025 New Vector Ltd.
 * SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-Element-Commercial.
 */
package com.poziomki.app.chat.matrix.api

import kotlinx.coroutines.flow.StateFlow

sealed interface MatrixClientState {
    data object Idle : MatrixClientState

    data object Connecting : MatrixClientState

    data class Ready(
        val userId: String,
        val homeserver: String,
        val deviceId: String,
    ) : MatrixClientState

    data class Error(
        val message: String,
    ) : MatrixClientState
}

data class MatrixRoomSummary(
    val roomId: String,
    val displayName: String,
    val avatarUrl: String?,
    val isDirect: Boolean,
    val directUserId: String? = null,
    val unreadCount: Int,
    val latestMessage: String?,
    val latestTimestampMillis: Long?,
)

interface MatrixClient {
    val state: StateFlow<MatrixClientState>
    val rooms: StateFlow<List<MatrixRoomSummary>>

    suspend fun ensureStarted(): Result<Unit>

    suspend fun refreshRooms(): Result<Unit>

    suspend fun getJoinedRoom(roomId: String): JoinedRoom?

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

    suspend fun stop()
}
