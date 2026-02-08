package com.poziomki.app.chat.matrix.api

import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow

class NoopMatrixClient : MatrixClient {
    private val _state = MutableStateFlow<MatrixClientState>(MatrixClientState.Error("Matrix is not available on this platform yet"))
    override val state: StateFlow<MatrixClientState> = _state

    override val rooms: StateFlow<List<MatrixRoomSummary>> = MutableStateFlow(emptyList())

    override suspend fun ensureStarted(): Result<Unit> =
        Result.failure(IllegalStateException("Matrix is not available on this platform yet"))

    override suspend fun refreshRooms(): Result<Unit> =
        Result.failure(IllegalStateException("Matrix is not available on this platform yet"))

    override suspend fun getJoinedRoom(roomId: String): JoinedRoom? = null

    override suspend fun createDM(
        userId: String,
        displayName: String?,
    ): Result<String> =
        Result.failure(IllegalStateException("Matrix is not available on this platform yet"))

    override suspend fun createRoom(
        name: String,
        invitedUserIds: List<String>,
    ): Result<String> = Result.failure(IllegalStateException("Matrix is not available on this platform yet"))

    override suspend fun stop() {
        _state.value = MatrixClientState.Error("Matrix is not available on this platform yet")
    }
}
