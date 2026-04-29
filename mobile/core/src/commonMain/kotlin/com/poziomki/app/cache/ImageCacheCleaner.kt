package com.poziomki.app.cache

interface ImageCacheCleaner {
    fun clear()
}

class NoopImageCacheCleaner : ImageCacheCleaner {
    override fun clear() = Unit
}
