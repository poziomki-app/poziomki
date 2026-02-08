package com.poziomki.app.chat.timeline

import com.poziomki.app.chat.matrix.api.MatrixTimelineMode
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow

class TimelineController {
    private val _mode = MutableStateFlow<MatrixTimelineMode>(MatrixTimelineMode.Live)
    val mode: StateFlow<MatrixTimelineMode> = _mode

    private val _isPaginatingBackwards = MutableStateFlow(false)
    val isPaginatingBackwards: StateFlow<Boolean> = _isPaginatingBackwards

    private val _hasMoreBackwards = MutableStateFlow(true)
    val hasMoreBackwards: StateFlow<Boolean> = _hasMoreBackwards

    fun enterLive() {
        _mode.value = MatrixTimelineMode.Live
    }

    fun focusOnEvent(eventId: String) {
        _mode.value = MatrixTimelineMode.FocusedOnEvent(eventId)
    }

    fun onPaginationStarted() {
        _isPaginatingBackwards.value = true
    }

    fun onPaginationFinished(hasMore: Boolean) {
        _isPaginatingBackwards.value = false
        _hasMoreBackwards.value = hasMore
    }
}
