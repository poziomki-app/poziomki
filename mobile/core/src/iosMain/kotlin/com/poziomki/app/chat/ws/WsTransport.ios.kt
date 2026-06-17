package com.poziomki.app.chat.ws

import io.ktor.client.engine.HttpClientEngine
import kotlinx.coroutines.channels.Channel

private class IosWsClosed(
    message: String,
) : Exception(message)

actual class WsTransport actual constructor(
    @Suppress("UNUSED_PARAMETER") engine: HttpClientEngine,
) {
    // Native socket delivers frames via callback; bridge them to a suspend queue.
    private val incoming = Channel<String>(Channel.UNLIMITED)
    private var handle: IosWsHandle? = null

    actual suspend fun connect(
        host: String,
        port: Int,
        useTls: Boolean,
        path: String,
    ) {
        val connectFn =
            checkNotNull(IosWsTransportRegistry.connect) { "iOS WebSocket bridge not registered" }
        val scheme = if (useTls) "wss" else "ws"
        val origin = "${if (useTls) "https" else "http"}://$host"
        handle =
            connectFn(
                "$scheme://$host:$port$path",
                origin,
                { text -> incoming.trySend(text) },
                { err -> incoming.close(IosWsClosed(err)) },
            )
    }

    actual suspend fun send(text: String) {
        handle?.send?.invoke(text)
    }

    actual suspend fun receive(): String = incoming.receive()

    actual fun close() {
        handle?.close?.invoke()
        handle = null
        incoming.close()
    }
}
