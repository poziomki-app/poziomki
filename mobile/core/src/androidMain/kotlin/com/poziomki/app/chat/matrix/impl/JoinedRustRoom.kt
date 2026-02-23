/*
 * NOTICE: Portions of this implementation are adapted from Element X Android Matrix room wrappers.
 * Copyright (c) 2025 Element Creations Ltd.
 * Copyright 2025 New Vector Ltd.
 * SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-Element-Commercial.
 */
package com.poziomki.app.chat.matrix.impl

import com.poziomki.app.chat.matrix.api.JoinedRoom
import com.poziomki.app.chat.matrix.api.MatrixTimelineMode
import com.poziomki.app.chat.matrix.api.Timeline
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.launch
import org.matrix.rustcomponents.sdk.DateDividerMode
import org.matrix.rustcomponents.sdk.RoomInfo
import org.matrix.rustcomponents.sdk.RoomInfoListener
import org.matrix.rustcomponents.sdk.TimelineConfiguration
import org.matrix.rustcomponents.sdk.TimelineFilter
import org.matrix.rustcomponents.sdk.TimelineFocus
import org.matrix.rustcomponents.sdk.TypingNotificationsListener
import uniffi.matrix_sdk_ui.TimelineEventFocusThreadMode
import uniffi.matrix_sdk_ui.TimelineReadReceiptTracking

class JoinedRustRoom(
    private val innerRoom: org.matrix.rustcomponents.sdk.Room,
    override val liveTimeline: Timeline,
    private val coroutineScope: CoroutineScope,
) : JoinedRoom {
    override val roomId: String = innerRoom.id()

    private val _displayName = MutableStateFlow(innerRoom.displayName() ?: roomId)
    override val displayName: StateFlow<String> = _displayName

    private val _typingUserIds = MutableStateFlow<List<String>>(emptyList())
    override val typingUserIds: StateFlow<List<String>> = _typingUserIds

    private var typingHandle: org.matrix.rustcomponents.sdk.TaskHandle? = null
    private var roomInfoHandle: org.matrix.rustcomponents.sdk.TaskHandle? = null

    init {
        typingHandle =
            innerRoom.subscribeToTypingNotifications(
                object : TypingNotificationsListener {
                    override fun call(typingUserIds: List<String>) {
                        _typingUserIds.value = typingUserIds.filterNot { it == innerRoom.ownUserId() }
                    }
                },
            )

        roomInfoHandle =
            innerRoom.subscribeToRoomInfoUpdates(
                object : RoomInfoListener {
                    override fun call(roomInfo: RoomInfo) {
                        _displayName.value = roomInfo.displayName ?: roomId
                    }
                },
            )

        coroutineScope.launch(Dispatchers.Default) {
            runCatching {
                val roomInfo = innerRoom.roomInfo()
                _displayName.value = roomInfo.displayName ?: roomId
            }
        }
    }

    override suspend fun createFocusedTimeline(eventId: String): Result<Timeline> =
        runCatching {
            val focusedInnerTimeline =
                innerRoom.timelineWithConfiguration(
                    TimelineConfiguration(
                        focus =
                            TimelineFocus.Event(
                                eventId = eventId,
                                numContextEvents = 50u.toUShort(),
                                threadMode = TimelineEventFocusThreadMode.Automatic(false),
                            ),
                        filter = TimelineFilter.All,
                        internalIdPrefix = "focus_$eventId",
                        dateDividerMode = DateDividerMode.DAILY,
                        trackReadReceipts = TimelineReadReceiptTracking.ALL_EVENTS,
                        reportUtds = false,
                    ),
                )

            RustTimeline(
                inner = focusedInnerTimeline,
                mode = MatrixTimelineMode.FocusedOnEvent(eventId),
                ownUserId = innerRoom.ownUserId(),
                coroutineScope = coroutineScope,
            )
        }

    override suspend fun typingNotice(isTyping: Boolean): Result<Unit> =
        runCatching {
            innerRoom.typingNotice(isTyping)
            Unit
        }

    override suspend fun markAsRead(): Result<Unit> = liveTimeline.markAsRead()

    override suspend fun inviteUserById(userId: String): Result<Unit> =
        runCatching {
            innerRoom.inviteUserById(userId)
            Unit
        }

    override suspend fun getMemberDisplayName(userId: String): String? = runCatching { innerRoom.memberDisplayName(userId) }.getOrNull()

    fun close() {
        typingHandle?.cancel()
        roomInfoHandle?.cancel()
        liveTimeline.close()
    }
}
