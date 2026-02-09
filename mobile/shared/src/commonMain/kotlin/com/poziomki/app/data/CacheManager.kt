package com.poziomki.app.data

import com.poziomki.app.db.PoziomkiDatabase
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.IO
import kotlinx.coroutines.withContext
import kotlinx.datetime.Clock

class CacheManager(
    private val db: PoziomkiDatabase,
) {
    companion object {
        private const val STALE_THRESHOLD_MS = 7L * 24 * 60 * 60 * 1000 // 7 days
    }

    suspend fun pruneStaleData() {
        withContext(Dispatchers.IO) {
            val cutoff = Clock.System.now().toEpochMilliseconds() - STALE_THRESHOLD_MS
            db.transaction {
                db.eventQueries.deleteStaleNonDirty(cutoff)
                db.profileQueries.deleteStaleNonDirty(cutoff)
                db.eventAttendeeQueries.deleteOrphans()
                db.profileTagQueries.deleteOrphans()
                db.pendingOperationQueries.deleteCompleted()
            }
        }
    }

    suspend fun clearAll() {
        withContext(Dispatchers.IO) {
            db.transaction {
                db.pendingOperationQueries.deleteAll()
                db.eventAttendeeQueries.deleteAll()
                db.eventQueries.deleteAll()
                db.profileTagQueries.deleteAll()
                db.profileQueries.deleteAll()
                db.tagQueries.deleteAll()
                db.degreeQueries.deleteAll()
                db.userSettingsQueries.deleteAll()
            }
        }
    }
}
