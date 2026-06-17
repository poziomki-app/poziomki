package com.poziomki.app.chat.ws

import io.ktor.client.engine.HttpClientEngine

/**
 * Platform WebSocket transport. The chat protocol (auth, heartbeat, reconnect,
 * framing) lives in [WsConnection]; this is just the raw text pipe.
 *
 * Android wraps Ktor/OkHttp. iOS uses a native `URLSessionWebSocketTask`
 * supplied from Swift — Ktor 3.4.x's Darwin engine caps WebSocket message size
 * too low (KTOR/#1894), which drops the ~6 KB conversations frame with
 * "Message too long" and resets the socket in a loop.
 */
expect class WsTransport(
    engine: HttpClientEngine,
) {
    suspend fun connect(
        host: String,
        port: Int,
        useTls: Boolean,
        path: String,
    )

    suspend fun send(text: String)

    suspend fun receive(): String

    fun close()
}
