package com.poziomki.app.chat.push

import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow

object NotificationChatTarget {
    const val EXTRA_OPEN_CHAT_ROOM_ID = "open_chat_room_id"

    private val _roomId = MutableStateFlow<String?>(null)
    val roomId: StateFlow<String?> = _roomId

    fun open(roomId: String?) {
        _roomId.value = roomId?.takeIf { it.isNotBlank() }
    }

    fun consume(roomId: String) {
        if (_roomId.value == roomId) {
            _roomId.value = null
        }
    }
}
