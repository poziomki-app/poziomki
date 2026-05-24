package com.poziomki.app.ui.cache

import coil3.PlatformContext
import okio.Path

expect fun coilImageCacheDir(context: PlatformContext): Path
