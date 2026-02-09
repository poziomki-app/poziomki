package com.poziomki.app.data.repository

import app.cash.sqldelight.coroutines.asFlow
import app.cash.sqldelight.coroutines.mapToList
import com.poziomki.app.api.ApiResult
import com.poziomki.app.api.ApiService
import com.poziomki.app.api.Tag
import com.poziomki.app.data.mapper.toApiModel
import com.poziomki.app.db.PoziomkiDatabase
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.IO
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.map
import kotlinx.coroutines.withContext

class TagRepository(
    private val db: PoziomkiDatabase,
    private val api: ApiService,
) {
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

    suspend fun refreshTags(scope: String? = null) {
        withContext(Dispatchers.IO) {
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
                }

                is ApiResult.Error -> {}
            }
        }
    }
}
