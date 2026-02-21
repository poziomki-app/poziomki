package com.poziomki.app.data.repository

import com.poziomki.app.api.ApiResult
import com.poziomki.app.api.ApiService
import com.poziomki.app.api.resolveRoomId
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.IO
import kotlinx.coroutines.withContext

class ChatRoomRepository(
    private val api: ApiService,
) {
    suspend fun resolveDirectRoom(
        targetUserId: String,
    ): Result<String> =
        withContext(Dispatchers.IO) {
            val normalizedTarget = targetUserId.trim()
            if (normalizedTarget.isBlank()) {
                return@withContext Result.failure(IllegalArgumentException("Target user id cannot be blank"))
            }

            resolveViaBackend(normalizedTarget)
        }

    private suspend fun resolveViaBackend(
        targetUserId: String,
    ): Result<String> =
        when (val result = api.resolveMatrixDirectRoom(targetUserId)) {
            is ApiResult.Success -> {
                val roomId = result.data.resolveRoomId()
                if (roomId == null) {
                    Result.failure(IllegalStateException("Backend returned empty direct room id"))
                } else {
                    Result.success(roomId)
                }
            }
            is ApiResult.Error -> {
                Result.failure(IllegalStateException(result.message))
            }
        }
}
