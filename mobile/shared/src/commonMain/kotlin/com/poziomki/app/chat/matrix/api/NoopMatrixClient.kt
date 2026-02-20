package com.poziomki.app.chat.matrix.api

import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow

class NoopMatrixClient : MatrixClient {
    private val _state = MutableStateFlow<MatrixClientState>(MatrixClientState.Idle)
    override val state: StateFlow<MatrixClientState> = _state

    override val rooms: StateFlow<List<MatrixRoomSummary>> = MutableStateFlow(emptyList())

    override suspend fun ensureStarted(): Result<Unit> = Result.success(Unit)

    override suspend fun refreshRooms(): Result<Unit> = Result.success(Unit)

    override suspend fun getJoinedRoom(roomId: String): JoinedRoom? = null

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
        gatewayUrl: String,
    ): Result<Unit> = Result.success(Unit)

    override suspend fun unregisterPusher(ntfyEndpoint: String): Result<Unit> = Result.success(Unit)

    override suspend fun getMediaThumbnail(
        mxcUrl: String,
        width: Long,
        height: Long,
    ): ByteArray? = null

    override suspend fun getMediaContent(mxcUrl: String): ByteArray? = null

    override suspend fun stop() {
        _state.value = MatrixClientState.Idle
    }
}
