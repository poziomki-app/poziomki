package com.poziomki.app.chat.ws

import io.ktor.client.HttpClient
import io.ktor.client.engine.HttpClientEngine
import io.ktor.client.plugins.defaultRequest
import io.ktor.client.plugins.websocket.WebSockets
import io.ktor.client.plugins.websocket.webSocket
import io.ktor.http.URLProtocol
import io.ktor.websocket.Frame
import io.ktor.websocket.readText
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.MutableSharedFlow
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.SharedFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.isActive
import kotlinx.coroutines.launch
import kotlinx.serialization.encodeToString

class WsConnection(
    private val baseUrl: String,
    private val tokenProvider: suspend () -> String?,
    engine: HttpClientEngine,
) {
    private val scope = CoroutineScope(SupervisorJob() + Dispatchers.Default)
    private val _incoming = MutableSharedFlow<WsServerMessage>(extraBufferCapacity = 64)
    val incoming: SharedFlow<WsServerMessage> = _incoming

    private val _isConnected = MutableStateFlow(false)
    val isConnected: StateFlow<Boolean> = _isConnected

    private val _userId = MutableStateFlow<String?>(null)
    val userId: StateFlow<String?> = _userId

    private var connectionJob: Job? = null
    private var heartbeatJob: Job? = null

    private val wsClient = HttpClient(engine) {
        install(WebSockets)
        defaultRequest {
            headers.append("Origin", baseUrl.trimEnd('/'))
        }
    }

    private var sendChannel: (suspend (String) -> Unit)? = null

    suspend fun send(msg: WsClientMessage): Boolean {
        val channel = sendChannel ?: return false
        val text = wsJson.encodeToString(msg)
        return try {
            channel.invoke(text)
            true
        } catch (@Suppress("TooGenericExceptionCaught") _: Exception) {
            false
        }
    }

    fun connect() {
        if (connectionJob?.isActive == true) return
        connectionJob = scope.launch {
            var backoffMs = 1_000L
            while (isActive) {
                try {
                    connectOnce()
                    backoffMs = 1_000L
                } catch (_: CancellationException) {
                    break
                } catch (@Suppress("TooGenericExceptionCaught") e: Exception) {
                    println("WsConnection error: ${e.message}")
                }
                _isConnected.value = false
                sendChannel = null
                heartbeatJob?.cancel()
                delay(backoffMs)
                backoffMs = (backoffMs * 2).coerceAtMost(60_000L)
            }
        }
    }

    fun disconnect() {
        connectionJob?.cancel()
        connectionJob = null
        heartbeatJob?.cancel()
        heartbeatJob = null
        _isConnected.value = false
        _userId.value = null
        sendChannel = null
    }

    private suspend fun connectOnce() {
        val (host, port, useTls) = parseBaseUrl(baseUrl)

        wsClient.webSocket(
            host = host,
            port = port,
            path = "/api/v1/chat/ws",
            request = {
                url.protocol = if (useTls) URLProtocol.WSS else URLProtocol.WS
            },
        ) {
            sendChannel = { text -> send(Frame.Text(text)) }

            // Authenticate
            val token = checkNotNull(tokenProvider()) { "No auth token" }
            val authMsg = wsJson.encodeToString<WsClientMessage>(WsClientMessage.Auth(token))
            send(Frame.Text(authMsg))

            // Wait for auth response
            val authFrame = incoming.receive()
            check(authFrame is Frame.Text) { "Expected text auth response" }
            val authResponse = wsJson.decodeFromString<WsServerMessage>(authFrame.readText())
            when (authResponse) {
                is WsServerMessage.AuthOk -> {
                    _userId.value = authResponse.userId
                    _isConnected.value = true
                }
                is WsServerMessage.AuthError -> {
                    error("Auth failed: ${authResponse.message}")
                }
                else -> error("Unexpected auth response: $authResponse")
            }

            // Start heartbeat only after successful auth
            heartbeatJob = scope.launch {
                while (isActive) {
                    delay(30_000L)
                    try {
                        val pingText = wsJson.encodeToString<WsClientMessage>(WsClientMessage.Ping)
                        send(Frame.Text(pingText))
                    } catch (_: Exception) {
                        break
                    }
                }
            }

            // Read loop
            try {
                for (frame in this.incoming) {
                    if (frame is Frame.Text) {
                        try {
                            val msg = wsJson.decodeFromString<WsServerMessage>(frame.readText())
                            if (msg !is WsServerMessage.Pong) {
                                _incoming.emit(msg)
                            }
                        } catch (_: Exception) {
                            // Skip malformed frames
                        }
                    }
                }
            } finally {
                heartbeatJob?.cancel()
                _isConnected.value = false
                sendChannel = null
            }
        }
    }
}

private data class HostConfig(val host: String, val port: Int, val useTls: Boolean)

private fun parseBaseUrl(baseUrl: String): HostConfig {
    val stripped = baseUrl.removePrefix("https://").removePrefix("http://").trimEnd('/')
    val useTls = baseUrl.startsWith("https://")
    val parts = stripped.split(":", limit = 2)
    val host = parts[0]
    val port = if (parts.size > 1) {
        parts[1].trimEnd('/').toIntOrNull() ?: if (useTls) 443 else 80
    } else {
        if (useTls) 443 else 80
    }
    return HostConfig(host, port, useTls)
}
