package com.poziomki.app.ui.ws

import com.poziomki.app.chat.ws.IosWsHandle
import com.poziomki.app.chat.ws.IosWsTransportRegistry

/** Callbacks the native socket fires back into Kotlin. */
interface NativeWebSocketListener {
    fun onText(text: String)

    fun onError(message: String)
}

/** A live native socket, implemented in Swift over `URLSessionWebSocketTask`. */
interface NativeWebSocket {
    fun send(text: String)

    fun close()
}

/** Swift-provided factory; opens a connected socket and streams frames to [listener]. */
interface NativeWebSocketFactory {
    fun create(
        url: String,
        origin: String,
        listener: NativeWebSocketListener,
    ): NativeWebSocket
}

/** Call once from Swift before chat is used (e.g. in the app initializer). */
fun registerIosWebSocket(factory: NativeWebSocketFactory) {
    IosWsTransportRegistry.connect = { url, origin, onText, onError ->
        val socket =
            factory.create(
                url,
                origin,
                object : NativeWebSocketListener {
                    override fun onText(text: String) = onText(text)

                    override fun onError(message: String) = onError(message)
                },
            )
        IosWsHandle(send = { socket.send(it) }, close = { socket.close() })
    }
}
