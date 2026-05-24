package com.poziomki.app.ui.cache

import coil3.PlatformContext
import okio.Path
import okio.Path.Companion.toOkioPath

actual fun coilImageCacheDir(context: PlatformContext): Path = context.cacheDir.resolve("coil_images").toOkioPath()
