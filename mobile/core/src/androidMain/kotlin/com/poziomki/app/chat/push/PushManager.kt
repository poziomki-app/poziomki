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
        // If the service was previously started (URL persisted), restart it immediately
        // so it survives app kills without waiting for ChatClient to become Ready.
        val prefs = appContext.getSharedPreferences(PREFS_NAME, Context.MODE_PRIVATE)
        if (prefs.getString(PREF_SSE_URL, null) != null) {
            val intent = Intent(appContext, NtfyPushService::class.java)
            appContext.startForegroundService(intent)
        }

        scope.launch {
            chatClient.state.collectLatest { state ->
                if (state is ChatClientState.Ready) {
                    startPushService(state.deviceId)
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

    companion object {
        private const val PREFS_NAME = "ntfy_push"
        private const val PREF_SSE_URL = "sse_url"
    }
}
