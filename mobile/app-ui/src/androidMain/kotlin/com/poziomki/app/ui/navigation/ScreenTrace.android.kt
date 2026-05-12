package com.poziomki.app.ui.navigation

import androidx.tracing.Trace

actual fun emitScreenTrace(name: String) {
    if (Trace.isEnabled()) {
        Trace.beginSection("screen:$name")
        Trace.endSection()
    }
}
