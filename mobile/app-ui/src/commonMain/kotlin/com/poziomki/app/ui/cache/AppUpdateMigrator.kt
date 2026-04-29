package com.poziomki.app.ui.cache

import com.poziomki.app.cache.ImageCacheCleaner
import com.poziomki.app.chat.cache.RoomTimelineCacheStore
import com.poziomki.app.data.CacheManager
import com.poziomki.app.session.SessionManager
import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.Deferred

class AppUpdateMigrator(
    private val sessionManager: SessionManager,
    private val cacheManager: CacheManager,
    private val roomTimelineCacheStore: RoomTimelineCacheStore,
    private val imageCacheCleaner: ImageCacheCleaner,
) {
    private val _ready = CompletableDeferred<Unit>()

    /**
     * Completes once the per-version migration has finished (or was a no-op).
     * Cache consumers — chat client, sync engine — must await this before
     * reading from or writing to DataStore / SQLDelight, otherwise an upgrade
     * could wipe data that just came back from the server.
     */
    val ready: Deferred<Unit> = _ready

    suspend fun runIfVersionChanged(currentVersionCode: Int) {
        try {
            val previous = sessionManager.getLastSeenVersionCode()
            if (previous == currentVersionCode) return
            if (previous != null) {
                // Only wipe on actual upgrades, not on first install where
                // there is no prior cache to invalidate.
                cacheManager.clearAll()
                roomTimelineCacheStore.clearAll()
                runCatching { imageCacheCleaner.clear() }
            }
            sessionManager.setLastSeenVersionCode(currentVersionCode)
        } finally {
            _ready.complete(Unit)
        }
    }
}
