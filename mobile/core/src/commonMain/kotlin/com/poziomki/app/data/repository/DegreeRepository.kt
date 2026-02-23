package com.poziomki.app.data.repository

import app.cash.sqldelight.coroutines.asFlow
import app.cash.sqldelight.coroutines.mapToList
import com.poziomki.app.db.PoziomkiDatabase
import com.poziomki.app.network.ApiResult
import com.poziomki.app.network.ApiService
import com.poziomki.app.network.Degree
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.IO
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.map
import kotlinx.coroutines.withContext
import kotlinx.datetime.Clock

class DegreeRepository(
    private val db: PoziomkiDatabase,
    private val api: ApiService,
) {
    private var lastRefreshMs: Long = 0L

    fun observeDegrees(): Flow<List<Degree>> =
        db.degreeQueries
            .selectAll()
            .asFlow()
            .mapToList(Dispatchers.IO)
            .map { rows ->
                rows.map { Degree(id = it.id, name = it.name) }
            }

    suspend fun refreshDegrees(forceRefresh: Boolean = false): Boolean =
        withContext(Dispatchers.IO) {
            ensureLocalSeedIfEmpty()
            if (!forceRefresh && !CachePolicy.isCatalogStale(lastRefreshMs)) return@withContext true
            when (val result = api.getDegrees()) {
                is ApiResult.Success -> {
                    db.transaction {
                        result.data.forEach { degree ->
                            db.degreeQueries.upsert(
                                id = degree.id,
                                name = degree.name,
                            )
                        }
                    }
                    lastRefreshMs = Clock.System.now().toEpochMilliseconds()
                    true
                }

                is ApiResult.Error -> {
                    false
                }
            }
        }

    suspend fun ensureLocalSeedIfEmpty() {
        withContext(Dispatchers.IO) {
            val hasAny =
                db.degreeQueries
                    .selectAll()
                    .executeAsList()
                    .isNotEmpty()
            if (hasAny) return@withContext
            db.transaction {
                LOCAL_ONBOARDING_DEGREES.forEach { degree ->
                    db.degreeQueries.upsert(
                        id = degree.id,
                        name = degree.name,
                    )
                }
            }
        }
    }
}
