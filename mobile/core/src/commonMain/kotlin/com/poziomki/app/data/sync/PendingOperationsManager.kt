package com.poziomki.app.data.sync

import app.cash.sqldelight.coroutines.asFlow
import app.cash.sqldelight.coroutines.mapToList
import com.poziomki.app.db.Pending_operation
import com.poziomki.app.db.PoziomkiDatabase
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.IO
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.map
import kotlinx.coroutines.withContext
import kotlin.time.Clock

class PendingOperationsManager(
    private val db: PoziomkiDatabase,
) {
    suspend fun enqueue(
        type: String,
        entityId: String?,
        payload: String,
    ) {
        withContext(Dispatchers.IO) {
            db.pendingOperationQueries.insert(
                type = type,
                entity_id = entityId,
                payload_json = payload,
                created_at = Clock.System.now().toEpochMilliseconds(),
            )
        }
    }

    suspend fun getPending(): List<Pending_operation> =
        withContext(Dispatchers.IO) {
            db.pendingOperationQueries.selectPending().executeAsList()
        }

    suspend fun complete(id: Long) {
        withContext(Dispatchers.IO) {
            db.pendingOperationQueries.markCompleted(id)
        }
    }

    suspend fun fail(id: Long) {
        withContext(Dispatchers.IO) {
            db.pendingOperationQueries.markFailed(id)
        }
    }

    suspend fun resetForRetry(id: Long) {
        withContext(Dispatchers.IO) {
            db.pendingOperationQueries.resetForRetry(id)
        }
    }

    suspend fun cleanCompleted() {
        withContext(Dispatchers.IO) {
            db.pendingOperationQueries.deleteCompleted()
        }
    }

    suspend fun updateEntityId(
        oldId: String,
        newId: String,
    ) {
        withContext(Dispatchers.IO) {
            db.pendingOperationQueries.updateEntityId(
                entity_id = newId,
                entity_id_ = oldId,
            )
        }
    }

    fun observePendingCount(): Flow<Long> =
        db.pendingOperationQueries
            .countPending()
            .asFlow()
            .map {
                withContext(Dispatchers.IO) {
                    db.pendingOperationQueries.countPending().executeAsOne()
                }
            }
}
