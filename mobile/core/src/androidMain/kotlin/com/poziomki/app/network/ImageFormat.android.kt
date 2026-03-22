package com.poziomki.app.network

actual fun preferredImageFormat(): String = if (android.os.Build.VERSION.SDK_INT >= 31) "avif" else "webp"
