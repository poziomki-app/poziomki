/*
 * NOTICE: Portions of this interface are adapted from Element X Android Matrix API.
 * Copyright (c) 2025 Element Creations Ltd.
 * Copyright 2025 New Vector Ltd.
 * SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-Element-Commercial.
 */
package com.poziomki.app.chat.matrix.api

import kotlinx.coroutines.flow.StateFlow

interface JoinedRoom {
    val roomId: String
    val displayName: StateFlow<String>
    val typingUserIds: StateFlow<List<String>>
    val liveTimeline: Timeline

    suspend fun createFocusedTimeline(eventId: String): Result<Timeline>

    suspend fun typingNotice(isTyping: Boolean): Result<Unit>

    suspend fun markAsRead(): Result<Unit>

    suspend fun inviteUserById(userId: String): Result<Unit>

    suspend fun getMemberDisplayName(userId: String): String?

    suspend fun getMemberAvatarUrl(userId: String): String?
}
