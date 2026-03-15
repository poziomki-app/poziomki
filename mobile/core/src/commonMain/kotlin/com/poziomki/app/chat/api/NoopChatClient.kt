package com.poziomki.app.chat.api

import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow

class NoopChatClient : ChatClient {
    private val _state = MutableStateFlow<ChatClientState>(ChatClientState.Idle)
    override val state: StateFlow<ChatClientState> = _state

    override val rooms: StateFlow<List<RoomSummary>> = MutableStateFlow(emptyList())

    override suspend fun ensureStarted(): Result<Unit> = Result.success(Unit)

    override suspend fun refreshRooms(): Result<Unit> = Result.success(Unit)

    override suspend fun getJoinedRoom(roomId: String): JoinedRoom? = null

    override suspend fun getRoomTimelineCache(
        roomId: String,
        limit: Int,
    ): RoomTimelineCacheSnapshot =
        RoomTimelineCacheSnapshot(
            items = emptyList(),
            isHydrated = false,
            cachedItemCount = 0,
            updatedAtMillis = 0L,
        )

    override suspend fun requestRoomTimelineBackfill(roomId: String): Result<Unit> = Result.success(Unit)

    override suspend fun createDM(
        userId: String,
        displayName: String?,
    ): Result<String> = Result.failure(IllegalStateException("Chat is not available yet"))

    override suspend fun createRoom(
        name: String,
        invitedUserIds: List<String>,
    ): Result<String> = Result.failure(IllegalStateException("Chat is not available yet"))

    override suspend fun registerPusher(
        ntfyEndpoint: String,
    ): Result<Unit> = Result.success(Unit)

    override suspend fun unregisterPusher(ntfyEndpoint: String): Result<Unit> = Result.success(Unit)

    override suspend fun stop() {
        _state.value = ChatClientState.Idle
    }
}
