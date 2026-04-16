package com.poziomki.app.chat.push

import android.content.Context
import android.content.Intent
import com.poziomki.app.chat.api.ChatClient
import com.poziomki.app.chat.api.ChatClientState
import com.poziomki.app.network.ApiResult
import com.poziomki.app.network.ApiService
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.flow.collectLatest
import kotlinx.coroutines.launch

class PushManager(
    private val chatClient: ChatClient,
    private val apiService: ApiService,
    private val appContext: Context,
) {
    private val scope = CoroutineScope(SupervisorJob() + Dispatchers.Default)

    fun startObserving() {
        scope.launch {
            chatClient.state.collectLatest { state ->
                when (state) {
                    is ChatClientState.Ready -> {
                        startPushService(state.deviceId)
                    }

                    is ChatClientState.Idle -> {
                        stopPushService()
                    }

                    else -> {}
                }
            }
        }
    }

    private suspend fun startPushService(deviceId: String) {
        val config =
            when (val result = apiService.getChatConfig()) {
                is ApiResult.Success -> result.data
                is ApiResult.Error -> return
            }

        val ntfyServer = config.ntfyServer ?: return
        if (!isAllowedNtfyServer(ntfyServer)) return
        val ntfyTopic = "poz_$deviceId"

        // Register push subscription with backend
        apiService.registerChatPush(deviceId, ntfyTopic)

        val sseUrl = "$ntfyServer/$ntfyTopic/sse"
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

    companion object {
        private val ALLOWED_NTFY_HOSTS = setOf("ntfy.poziomki.app")

        private fun isAllowedNtfyServer(url: String): Boolean =
            url.startsWith("https://") &&
                ALLOWED_NTFY_HOSTS.any { host ->
                    url == "https://$host" || url.startsWith("https://$host/")
                }
    }
}
