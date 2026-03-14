package com.poziomki.app.chat.timeline

import com.poziomki.app.chat.api.TimelineMode
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow

class TimelineController {
    private val _mode = MutableStateFlow<TimelineMode>(TimelineMode.Live)
    val mode: StateFlow<TimelineMode> = _mode

    fun enterLive() {
        _mode.value = TimelineMode.Live
    }

    fun focusOnEvent(eventId: String) {
        _mode.value = TimelineMode.FocusedOnEvent(eventId)
    }
}
