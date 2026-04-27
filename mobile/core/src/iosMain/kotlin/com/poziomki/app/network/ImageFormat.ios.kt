package com.poziomki.app.network

import kotlinx.cinterop.ExperimentalForeignApi
import kotlinx.cinterop.useContents
import platform.Foundation.NSProcessInfo

@OptIn(ExperimentalForeignApi::class)
actual fun preferredImageFormat(): String =
    if (NSProcessInfo.processInfo.operatingSystemVersion.useContents { majorVersion } >= 16L) "avif" else "webp"
