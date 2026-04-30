package com.poziomki.app.ui.shared

import coil3.PlatformContext
import okio.Path
import okio.Path.Companion.toPath
import platform.Foundation.NSCachesDirectory
import platform.Foundation.NSSearchPathForDirectoriesInDomains
import platform.Foundation.NSUserDomainMask

@Suppress("UNUSED_PARAMETER")
actual fun coilDiskCachePath(context: PlatformContext): Path {
    val dirs = NSSearchPathForDirectoriesInDomains(NSCachesDirectory, NSUserDomainMask, true)
    val base = (dirs.firstOrNull() as? String) ?: "/tmp"
    return "$base/coil_images".toPath()
}
