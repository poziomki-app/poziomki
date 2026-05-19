package com.poziomki.app.chat.push

import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow

/**
 * Queues a `poziomki://event/<id>` deep link until the Compose nav graph
 * picks it up. Mirrors [NotificationChatTarget] / [NotificationDeepLinkTarget];
 * kept as a separate target so the broadcast deep-link parser stays a pure
 * dispatcher.
 */
object NotificationEventTarget {
    private val _eventId = MutableStateFlow<String?>(null)
    val eventId: StateFlow<String?> = _eventId

    fun open(eventId: String?) {
        _eventId.value = eventId?.takeIf { it.isNotBlank() }
    }

    fun consume(eventId: String) {
        if (_eventId.value == eventId) {
            _eventId.value = null
        }
    }
}
