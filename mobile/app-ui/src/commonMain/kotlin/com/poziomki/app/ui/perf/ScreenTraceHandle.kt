package com.poziomki.app.ui.perf

interface ScreenTraceHandle {
    fun stop()
}

expect fun startScreenTrace(name: String): ScreenTraceHandle
