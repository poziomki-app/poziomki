package com.poziomki.app.chat.push

import android.app.Service
import android.content.Intent
import android.os.IBinder
import com.poziomki.app.chat.ActiveChat
import io.ktor.client.HttpClient
import io.ktor.client.engine.okhttp.OkHttp
import io.ktor.client.request.prepareGet
import io.ktor.client.statement.bodyAsChannel
import io.ktor.utils.io.readUTF8Line
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.delay
import kotlinx.coroutines.isActive
import kotlinx.coroutines.launch
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.jsonPrimitive
import org.koin.core.component.KoinComponent
import org.koin.core.component.inject

class NtfyPushService :
    Service(),
    KoinComponent {
    private val notificationHelper: NotificationHelper by inject()
    private val serviceScope = CoroutineScope(SupervisorJob() + Dispatchers.IO)
    private var sseJob: Job? = null
    private var sseUrl: String? = null

    private val json = Json { ignoreUnknownKeys = true }

    private val httpClient =
        HttpClient(OkHttp) {
            engine {
                config {
                    readTimeout(0, java.util.concurrent.TimeUnit.MILLISECONDS)
                }
            }
        }

    override fun onCreate() {
        super.onCreate()
        startForeground(
            NotificationHelper.SERVICE_NOTIFICATION_ID,
            notificationHelper.buildServiceNotification(),
        )
    }

    override fun onStartCommand(
        intent: Intent?,
        flags: Int,
        startId: Int,
    ): Int {
        val url = intent?.getStringExtra(EXTRA_SSE_URL)
        if (url != null && url != sseUrl) {
            sseUrl = url
            sseJob?.cancel()
            sseJob = serviceScope.launch { connectWithBackoff(url) }
        }
        return START_STICKY
    }

    override fun onDestroy() {
        sseJob?.cancel()
        httpClient.close()
        super.onDestroy()
    }

    override fun onBind(intent: Intent?): IBinder? = null

    private suspend fun connectWithBackoff(url: String) {
        var backoffMs = INITIAL_BACKOFF_MS
        while (serviceScope.isActive) {
            try {
                connectSse(url)
            } catch (_: Exception) {
                // Connection failed or dropped — reconnect after backoff
            }
            if (!serviceScope.isActive) break
            delay(backoffMs)
            backoffMs = (backoffMs * 2).coerceAtMost(MAX_BACKOFF_MS)
        }
    }

    @Suppress("DEPRECATION")
    private suspend fun connectSse(url: String) {
        httpClient.prepareGet(url).execute { response ->
            val channel = response.bodyAsChannel()
            // Reset backoff on successful connection
            while (!channel.isClosedForRead) {
                val line = channel.readUTF8Line() ?: break
                if (line.isBlank() || line.startsWith("event:") || line.startsWith("id:")) continue
                handleSseEvent(line)
            }
        }
    }

    private fun handleSseEvent(data: String) {
        val jsonStr = if (data.startsWith("data:")) data.substringAfter("data:").trim() else data
        val parsed =
            runCatching {
                json.decodeFromString<JsonObject>(jsonStr)
            }.getOrNull() ?: return

        val event = parsed["event"]?.jsonPrimitive?.content
        if (event != "message") return

        val message = parsed["message"]?.jsonPrimitive?.content ?: return
        val pushData =
            runCatching {
                json.decodeFromString<JsonObject>(message)
            }.getOrNull()

        val sender =
            parsed["title"]?.jsonPrimitive?.content
                ?: pushData?.get("sender")?.jsonPrimitive?.content
        val roomId = pushData?.get("room_id")?.jsonPrimitive?.content
        val body = pushData?.get("body")?.jsonPrimitive?.content
        val avatar = pushData?.get("avatar")?.jsonPrimitive?.content

        // Suppress notification if the user is viewing this chat
        if (roomId != null && roomId == ActiveChat.roomId) return

        notificationHelper.showMessageNotification(sender, roomId, body, avatar)
    }

    companion object {
        const val EXTRA_SSE_URL = "sse_url"
        private const val INITIAL_BACKOFF_MS = 1_000L
        private const val MAX_BACKOFF_MS = 60_000L
    }
}
