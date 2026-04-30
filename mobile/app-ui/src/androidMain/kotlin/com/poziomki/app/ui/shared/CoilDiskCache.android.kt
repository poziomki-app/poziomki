package com.poziomki.app.ui.shared

import coil3.PlatformContext
import okio.Path
import okio.Path.Companion.toOkioPath

actual fun coilDiskCachePath(context: PlatformContext): Path = context.cacheDir.resolve("coil_images").toOkioPath()
