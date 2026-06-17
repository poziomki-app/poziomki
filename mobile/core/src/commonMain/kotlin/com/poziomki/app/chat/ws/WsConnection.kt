package com.poziomki.app.chat.ws

import io.ktor.client.engine.HttpClientEngine
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
    private val engine: HttpClientEngine,
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
    private var sendChannel: (suspend (String) -> Unit)? = null

    suspend fun send(msg: WsClientMessage): Boolean {
        val channel = sendChannel ?: return false
        val text = wsJson.encodeToString(msg)
        return try {
            channel.invoke(text)
            true
        } catch (
            @Suppress("TooGenericExceptionCaught") _: Exception,
        ) {
            false
        }
    }

    fun connect() {
        if (connectionJob?.isActive == true) return
        connectionJob =
            scope.launch {
                var backoffMs = 1_000L
                while (isActive) {
                    try {
                        connectOnce()
                        backoffMs = 1_000L
                    } catch (_: CancellationException) {
                        break
                    } catch (
                        @Suppress("TooGenericExceptionCaught", "SwallowedException") _: Exception,
                    ) {
                        // connection error, will reconnect
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
        val transport = WsTransport(engine)
        transport.connect(host = host, port = port, useTls = useTls, path = "/api/v1/chat/ws")

        val pongReceived = MutableStateFlow(true)
        try {
            val token = checkNotNull(tokenProvider()) { "No auth token" }
            transport.send(wsJson.encodeToString<WsClientMessage>(WsClientMessage.Auth(token)))
            authenticate(transport)
            _isConnected.value = true
            sendChannel = { outgoing -> transport.send(outgoing) }
            heartbeatJob = scope.launch { heartbeatLoop(transport, pongReceived) }
            // `receive()` throws when the socket closes, which bubbles up to the
            // reconnect loop in connect().
            readLoop(transport, pongReceived)
        } finally {
            heartbeatJob?.cancel()
            _isConnected.value = false
            sendChannel = null
            transport.close()
        }
    }

    /** Reads frames until the auth response. Throws on auth failure. */
    private suspend fun authenticate(transport: WsTransport) {
        while (true) {
            val msg = decodeFrame(transport.receive()) ?: continue
            when (msg) {
                is WsServerMessage.AuthOk -> {
                    _userId.value = msg.userId
                    return
                }

                is WsServerMessage.AuthError -> {
                    error("Auth failed: ${msg.message}")
                }

                else -> {
                    error("Unexpected auth response: $msg")
                }
            }
        }
    }

    /** Steady-state frame loop. Throws when the socket closes. */
    private suspend fun readLoop(
        transport: WsTransport,
        pongReceived: MutableStateFlow<Boolean>,
    ) {
        while (true) {
            val msg = decodeFrame(transport.receive()) ?: continue
            if (msg is WsServerMessage.Pong) {
                pongReceived.value = true
            } else {
                _incoming.emit(msg)
            }
        }
    }

    private fun decodeFrame(text: String): WsServerMessage? =
        try {
            wsJson.decodeFromString<WsServerMessage>(text)
        } catch (
            @Suppress("TooGenericExceptionCaught", "SwallowedException") _: Exception,
        ) {
            null
        }

    private suspend fun heartbeatLoop(
        transport: WsTransport,
        pongReceived: MutableStateFlow<Boolean>,
    ) {
        while (true) {
            delay(30_000L)
            pongReceived.value = false
            try {
                transport.send(wsJson.encodeToString<WsClientMessage>(WsClientMessage.Ping))
            } catch (
                @Suppress("TooGenericExceptionCaught", "SwallowedException") _: Exception,
            ) {
                break
            }
            delay(10_000L)
            if (!pongReceived.value) {
                // Pong timeout — drop the socket so the read loop unblocks and reconnects.
                transport.close()
                break
            }
        }
    }
}

private data class HostConfig(
    val host: String,
    val port: Int,
    val useTls: Boolean,
)

private fun parseBaseUrl(baseUrl: String): HostConfig {
    val stripped = baseUrl.removePrefix("https://").removePrefix("http://").trimEnd('/')
    val useTls = baseUrl.startsWith("https://")
    val parts = stripped.split(":", limit = 2)
    val host = parts[0]
    val defaultPort = if (useTls) 443 else 80
    val port = parts.getOrNull(1)?.trimEnd('/')?.toIntOrNull() ?: defaultPort
    return HostConfig(host, port, useTls)
}
