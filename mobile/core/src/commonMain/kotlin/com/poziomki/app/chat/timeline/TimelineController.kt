package com.poziomki.app.chat.timeline

import com.poziomki.app.chat.matrix.api.MatrixTimelineMode
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow

class TimelineController {
    private val _mode = MutableStateFlow<MatrixTimelineMode>(MatrixTimelineMode.Live)
    val mode: StateFlow<MatrixTimelineMode> = _mode

    fun enterLive() {
        _mode.value = MatrixTimelineMode.Live
    }

    fun focusOnEvent(eventId: String) {
        _mode.value = MatrixTimelineMode.FocusedOnEvent(eventId)
    }
}
