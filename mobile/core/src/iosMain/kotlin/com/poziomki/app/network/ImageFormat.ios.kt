package com.poziomki.app.network

import platform.Foundation.NSProcessInfo

actual fun preferredImageFormat(): String =
    if (NSProcessInfo.processInfo.operatingSystemVersion.useContents { majorVersion } >= 16L) "avif" else "webp"
