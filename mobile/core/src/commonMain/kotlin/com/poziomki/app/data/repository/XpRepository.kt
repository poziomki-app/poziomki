package com.poziomki.app.data.repository

import com.poziomki.app.network.ApiResult
import com.poziomki.app.network.ApiService
import com.poziomki.app.network.ClaimTaskResponse
import com.poziomki.app.network.XpScanResponse
import com.poziomki.app.network.XpTokenResponse
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.IO
import kotlinx.coroutines.withContext

class XpRepository(
    private val api: ApiService,
    private val profileRepository: ProfileRepository,
) {
    suspend fun generateToken(): ApiResult<XpTokenResponse> = withContext(Dispatchers.IO) { api.getXpToken() }

    suspend fun scan(token: String): ApiResult<XpScanResponse> =
        withContext(Dispatchers.IO) {
            val result = api.scanXpToken(token)
            if (result is ApiResult.Success && result.data.xpGained > 0) {
                profileRepository.refreshOwnProfile(forceRefresh = true)
            }
            result
        }

    suspend fun claimTask(taskId: String): ApiResult<ClaimTaskResponse> =
        withContext(Dispatchers.IO) {
            val result = api.claimTask(taskId)
            if (result is ApiResult.Success && result.data.xpGained > 0) {
                profileRepository.refreshOwnProfile(forceRefresh = true)
            }
            result
        }
}
