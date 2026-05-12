package com.poziomki.app.ui.perf

import com.google.firebase.perf.FirebasePerformance
import com.google.firebase.perf.metrics.Trace

private class FirebaseScreenTraceHandle(
    private val trace: Trace,
) : ScreenTraceHandle {
    override fun stop() = trace.stop()
}

actual fun startScreenTrace(name: String): ScreenTraceHandle {
    val trace = FirebasePerformance.getInstance().newTrace("screen_$name")
    trace.start()
    return FirebaseScreenTraceHandle(trace)
}
