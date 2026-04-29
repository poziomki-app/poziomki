package com.poziomki.app.ui.cache

import com.poziomki.app.chat.cache.RoomTimelineCacheStore
import com.poziomki.app.data.CacheManager
import com.poziomki.app.session.SessionManager

class AppUpdateMigrator(
    private val sessionManager: SessionManager,
    private val cacheManager: CacheManager,
    private val roomTimelineCacheStore: RoomTimelineCacheStore,
    private val imageCacheCleaner: ImageCacheCleaner,
) {
    suspend fun runIfVersionChanged(currentVersionCode: Int) {
        val previous = sessionManager.getLastSeenVersionCode()
        if (previous == currentVersionCode) return
        if (previous != null) {
            // Only wipe on actual upgrades, not on first install where there
            // is no prior cache to invalidate.
            cacheManager.clearAll()
            roomTimelineCacheStore.clearAll()
            runCatching { imageCacheCleaner.clear() }
        }
        sessionManager.setLastSeenVersionCode(currentVersionCode)
    }
}
