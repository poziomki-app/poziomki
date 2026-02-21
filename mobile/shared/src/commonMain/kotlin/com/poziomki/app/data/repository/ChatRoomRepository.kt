package com.poziomki.app.data.repository

import com.poziomki.app.api.ApiResult
import com.poziomki.app.api.ApiService
import com.poziomki.app.api.resolveRoomId
import com.poziomki.app.api.supportsLegacyMatrixFallback
import com.poziomki.app.chat.matrix.api.MatrixClient
import com.poziomki.app.util.matrixLocalpartFromUserId
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.IO
import kotlinx.coroutines.withContext

class ChatRoomRepository(
    private val api: ApiService,
    private val matrixClient: MatrixClient,
) {
    suspend fun resolveDirectRoom(
        targetUserId: String,
        targetDisplayName: String? = null,
    ): Result<String> =
        withContext(Dispatchers.IO) {
            val normalizedTarget = targetUserId.trim()
            if (normalizedTarget.isBlank()) {
                return@withContext Result.failure(IllegalArgumentException("Target user id cannot be blank"))
            }

            val backendResolution = resolveViaBackend(normalizedTarget, targetDisplayName)

            if (backendResolution != null) {
                return@withContext backendResolution
            }

            val matrixLocalpart = matrixLocalpartFromUserId(normalizedTarget)
            matrixClient.createDM(matrixLocalpart, targetDisplayName)
        }

    private suspend fun resolveViaBackend(
        targetUserId: String,
        targetDisplayName: String?,
    ): Result<String>? {
        val attempts =
            listOf(
                api.resolveMatrixDirectRoom(targetUserId, targetDisplayName),
                api.resolveMatrixDirectRoomPlural(targetUserId, targetDisplayName),
            )

        attempts.forEach { result ->
            when (result) {
                is ApiResult.Success -> {
                    result.data.resolveRoomId()?.let { roomId ->
                        return Result.success(roomId)
                    }
                }

                is ApiResult.Error -> {
                    if (!result.supportsLegacyMatrixFallback()) {
                        return Result.failure(IllegalStateException(result.message))
                    }
                }
            }
        }

        return null
    }
}
