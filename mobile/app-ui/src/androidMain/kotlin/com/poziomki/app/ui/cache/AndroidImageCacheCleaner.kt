package com.poziomki.app.ui.cache

import android.content.Context
import coil3.imageLoader
import com.poziomki.app.cache.ImageCacheCleaner

class AndroidImageCacheCleaner(
    private val context: Context,
) : ImageCacheCleaner {
    override fun clear() {
        val loader = context.imageLoader
        loader.memoryCache?.clear()
        loader.diskCache?.clear()
    }
}
