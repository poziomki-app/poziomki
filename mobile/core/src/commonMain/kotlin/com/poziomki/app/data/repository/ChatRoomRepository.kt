package com.poziomki.app.data.repository

import com.poziomki.app.network.ApiResult
import com.poziomki.app.network.ApiService
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.IO
import kotlinx.coroutines.withContext

class ChatRoomRepository(
    private val api: ApiService,
) {
    suspend fun resolveDirectRoom(targetUserId: String): Result<String> =
        withContext(Dispatchers.IO) {
            val normalizedTarget = targetUserId.trim()
            if (normalizedTarget.isBlank()) {
                return@withContext Result.failure(IllegalArgumentException("Target user id cannot be blank"))
            }

            when (val result = api.resolveChatDm(normalizedTarget)) {
                is ApiResult.Success -> Result.success(result.data.conversationId)
                is ApiResult.Error -> Result.failure(IllegalStateException(result.message))
            }
        }
}
