package com.poziomki.app.chat.push

import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow

/**
 * Holds a deep-link URL queued by a broadcast notification tap until the
 * Compose nav graph consumes it. Mirrors [NotificationChatTarget], but
 * carries a free-form string the navigation layer parses by itself.
 *
 * Recognised forms today: `poziomki://chat/<roomId>`. Unknown schemes are
 * a no-op (the app just opens at its current destination).
 */
object NotificationDeepLinkTarget {
    const val EXTRA_OPEN_DEEP_LINK = "open_deep_link"

    private val _link = MutableStateFlow<String?>(null)
    val link: StateFlow<String?> = _link

    fun open(deepLink: String?) {
        _link.value = deepLink?.takeIf { it.isNotBlank() }
    }

    fun consume(deepLink: String) {
        if (_link.value == deepLink) {
            _link.value = null
        }
    }
}
