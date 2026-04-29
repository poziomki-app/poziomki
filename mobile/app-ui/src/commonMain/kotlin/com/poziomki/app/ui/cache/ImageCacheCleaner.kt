package com.poziomki.app.ui.cache

interface ImageCacheCleaner {
    fun clear()
}

class NoopImageCacheCleaner : ImageCacheCleaner {
    override fun clear() = Unit
}
