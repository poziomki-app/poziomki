package com.poziomki.app.chat.ws

/** Handle to a live native socket, returned by [IosWsTransportRegistry.connect]. */
class IosWsHandle(
    val send: (String) -> Unit,
    val close: () -> Unit,
)

/**
 * Seam filled in from `app-ui`'s iOS bridge at startup, which in turn is backed
 * by a Swift `URLSessionWebSocketTask`. Plain function types keep `core`
 * unexported — the Swift-facing interfaces live in the framework module.
 */
object IosWsTransportRegistry {
    var connect: (
        (
            url: String,
            origin: String,
            onText: (String) -> Unit,
            onError: (String) -> Unit,
        ) -> IosWsHandle
    )? = null
}
