package com.poziomki.app.ui.perf

private object NoopScreenTraceHandle : ScreenTraceHandle {
    override fun stop() = Unit
}

actual fun startScreenTrace(name: String): ScreenTraceHandle = NoopScreenTraceHandle
