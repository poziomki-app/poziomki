package com.poziomki.app.ui.shared

import coil3.PlatformContext
import okio.Path

/** Per-platform path used as Coil's persistent image disk cache. */
expect fun coilDiskCachePath(context: PlatformContext): Path
