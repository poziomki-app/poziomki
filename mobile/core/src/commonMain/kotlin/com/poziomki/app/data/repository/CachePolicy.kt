package com.poziomki.app.data.repository

import kotlinx.datetime.Clock

object CachePolicy {
    private const val DEFAULT_STALE_MS = 5L * 60L * 1000L // 5 minutes
    private const val CATALOG_STALE_MS = 30L * 60L * 1000L // 30 minutes

    fun isStale(
        cachedAtMs: Long,
        maxAgeMs: Long = DEFAULT_STALE_MS,
    ): Boolean {
        if (cachedAtMs == 0L) return true
        return Clock.System.now().toEpochMilliseconds() - cachedAtMs > maxAgeMs
    }

    fun isCatalogStale(cachedAtMs: Long): Boolean = isStale(cachedAtMs, CATALOG_STALE_MS)
}
