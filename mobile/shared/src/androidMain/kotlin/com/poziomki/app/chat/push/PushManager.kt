package com.poziomki.app.chat.push

import android.content.Context
import android.content.Intent
import com.poziomki.app.api.ApiResult
import com.poziomki.app.api.ApiService
import com.poziomki.app.chat.matrix.api.MatrixClient
import com.poziomki.app.chat.matrix.api.MatrixClientState
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.flow.collectLatest
import kotlinx.coroutines.launch

class PushManager(
    private val matrixClient: MatrixClient,
    private val apiService: ApiService,
    private val appContext: Context,
) {
    private val scope = CoroutineScope(SupervisorJob() + Dispatchers.Default)

    fun startObserving() {
        scope.launch {
            matrixClient.state.collectLatest { state ->
                when (state) {
                    is MatrixClientState.Ready -> {
                        startPushService(state.deviceId)
                    }

                    is MatrixClientState.Idle -> {
                        stopPushService()
                    }

                    else -> {}
                }
            }
        }
    }

    private suspend fun startPushService(deviceId: String) {
        val config =
            when (val result = apiService.getMatrixConfig()) {
                is ApiResult.Success -> result.data
                is ApiResult.Error -> return
            }

        val ntfyServer = config.ntfyServer ?: return
        val sseUrl = "$ntfyServer/poz_$deviceId/sse"

        val intent =
            Intent(appContext, NtfyPushService::class.java).apply {
                putExtra(NtfyPushService.EXTRA_SSE_URL, sseUrl)
            }
        appContext.startForegroundService(intent)
    }

    private fun stopPushService() {
        val intent = Intent(appContext, NtfyPushService::class.java)
        appContext.stopService(intent)
    }
}
