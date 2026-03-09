package com.poziomki.app.data.repository

import app.cash.sqldelight.coroutines.asFlow
import app.cash.sqldelight.coroutines.mapToList
import com.poziomki.app.data.mapper.toApiModel
import com.poziomki.app.db.PoziomkiDatabase
import com.poziomki.app.network.ApiResult
import com.poziomki.app.network.ApiService
import com.poziomki.app.network.CreateTagRequest
import com.poziomki.app.network.Tag
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.IO
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.map
import kotlinx.coroutines.withContext
import kotlinx.datetime.Clock

class TagRepository(
    private val db: PoziomkiDatabase,
    private val api: ApiService,
) {
    private var lastRefreshMs: Long = 0L

    fun observeTags(scope: String? = null): Flow<List<Tag>> {
        val query =
            if (scope != null) {
                db.tagQueries.selectByScope(scope)
            } else {
                db.tagQueries.selectAll()
            }
        return query
            .asFlow()
            .mapToList(Dispatchers.IO)
            .map { rows -> rows.map { it.toApiModel() } }
    }

    suspend fun refreshTags(
        scope: String? = null,
        forceRefresh: Boolean = false,
    ): Boolean =
        withContext(Dispatchers.IO) {
            if (scope == "interest") {
                ensureInterestSeedIfEmpty()
            }
            if (!forceRefresh && !CachePolicy.isCatalogStale(lastRefreshMs)) return@withContext true
            when (val result = api.getTags(scope)) {
                is ApiResult.Success -> {
                    db.transaction {
                        result.data.forEach { tag ->
                            db.tagQueries.upsert(
                                id = tag.id,
                                name = tag.name,
                                scope = tag.scope,
                                category = tag.category,
                                emoji = tag.emoji,
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

    suspend fun searchTags(
        scope: String,
        query: String,
    ): List<Tag> =
        withContext(Dispatchers.IO) {
            when (val result = api.searchTags(scope, query)) {
                is ApiResult.Success -> {
                    result.data
                }

                is ApiResult.Error -> {
                    // Fallback to local DB filtering
                    db.tagQueries
                        .selectByScope(scope)
                        .executeAsList()
                        .map { it.toApiModel() }
                        .filter { it.name.contains(query, ignoreCase = true) }
                }
            }
        }

    suspend fun createTag(
        name: String,
        scope: String,
    ): ApiResult<Tag> =
        withContext(Dispatchers.IO) {
            val result = api.createTag(CreateTagRequest(name = name, scope = scope))
            if (result is ApiResult.Success) {
                val tag = result.data
                db.tagQueries.upsert(
                    id = tag.id,
                    name = tag.name,
                    scope = tag.scope,
                    category = tag.category,
                    emoji = tag.emoji,
                )
            }
            result
        }

    suspend fun ensureInterestSeedIfEmpty() {
        withContext(Dispatchers.IO) {
            val hasInterestTags =
                db.tagQueries
                    .selectByScope("interest")
                    .executeAsList()
                    .isNotEmpty()
            if (hasInterestTags) return@withContext

            db.transaction {
                LOCAL_ONBOARDING_INTEREST_TAGS.forEach { tag ->
                    db.tagQueries.upsert(
                        id = tag.id,
                        name = tag.name,
                        scope = tag.scope,
                        category = tag.category,
                        emoji = tag.emoji,
                    )
                }
            }
        }
    }
}
