package com.poziomki.app.chat.ws

import io.ktor.client.HttpClient
import io.ktor.client.engine.HttpClientEngine
import io.ktor.client.plugins.websocket.DefaultClientWebSocketSession
import io.ktor.client.plugins.websocket.WebSockets
import io.ktor.client.plugins.websocket.webSocketSession
import io.ktor.http.URLProtocol
import io.ktor.websocket.Frame
import io.ktor.websocket.readText
import kotlinx.coroutines.cancel

actual class WsTransport actual constructor(
    engine: HttpClientEngine,
) {
    private val client = HttpClient(engine) { install(WebSockets) }
    private var session: DefaultClientWebSocketSession? = null

    actual suspend fun connect(
        host: String,
        port: Int,
        useTls: Boolean,
        path: String,
    ) {
        session =
            client.webSocketSession(host = host, port = port, path = path) {
                url.protocol = if (useTls) URLProtocol.WSS else URLProtocol.WS
                headers.append("Origin", "${if (useTls) "https" else "http"}://$host")
            }
    }

    actual suspend fun send(text: String) {
        session?.send(Frame.Text(text))
    }

    actual suspend fun receive(): String {
        val s = checkNotNull(session) { "WebSocket not connected" }
        while (true) {
            val frame = s.incoming.receive()
            if (frame is Frame.Text) return frame.readText()
        }
    }

    actual fun close() {
        session?.cancel()
        session = null
    }
}
