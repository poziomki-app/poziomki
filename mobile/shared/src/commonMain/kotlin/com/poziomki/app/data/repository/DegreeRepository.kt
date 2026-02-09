package com.poziomki.app.data.repository

import app.cash.sqldelight.coroutines.asFlow
import app.cash.sqldelight.coroutines.mapToList
import com.poziomki.app.api.ApiResult
import com.poziomki.app.api.ApiService
import com.poziomki.app.api.Degree
import com.poziomki.app.db.PoziomkiDatabase
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.IO
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.map
import kotlinx.coroutines.withContext

class DegreeRepository(
    private val db: PoziomkiDatabase,
    private val api: ApiService,
) {
    fun observeDegrees(): Flow<List<Degree>> =
        db.degreeQueries
            .selectAll()
            .asFlow()
            .mapToList(Dispatchers.IO)
            .map { rows ->
                rows.map { Degree(id = it.id, name = it.name) }
            }

    suspend fun refreshDegrees() {
        withContext(Dispatchers.IO) {
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
                }

                is ApiResult.Error -> {}
            }
        }
    }
}
