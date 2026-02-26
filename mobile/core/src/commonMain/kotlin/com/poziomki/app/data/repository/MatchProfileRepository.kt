package com.poziomki.app.data.repository

import app.cash.sqldelight.coroutines.asFlow
import app.cash.sqldelight.coroutines.mapToList
import com.poziomki.app.data.mapper.toApiModel
import com.poziomki.app.db.PoziomkiDatabase
import com.poziomki.app.network.ApiResult
import com.poziomki.app.network.ApiService
import com.poziomki.app.network.MatchProfile
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.IO
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.map
import kotlinx.coroutines.withContext
import kotlinx.datetime.Clock
import kotlinx.serialization.encodeToString
import kotlinx.serialization.json.Json

class MatchProfileRepository(
    private val db: PoziomkiDatabase,
    private val api: ApiService,
) {
    private val json = Json { ignoreUnknownKeys = true }

    fun observeProfiles(): Flow<List<MatchProfile>> =
        db.matchedProfileQueries
            .selectAll()
            .asFlow()
            .mapToList(Dispatchers.IO)
            .map { rows -> rows.map { it.toApiModel() } }

    /** Emits userId -> profilePicture map for all cached match profiles. */
    fun observeProfilePicturesByUserId(): Flow<Map<String, String>> =
        db.matchedProfileQueries
            .profilePictureByUserId()
            .asFlow()
            .mapToList(Dispatchers.IO)
            .map { rows ->
                val pictureMap = mutableMapOf<String, String>()
                rows.forEach { row ->
                    val pic = row.profile_picture
                    if (!pic.isNullOrBlank() && row.user_id.isNotBlank()) {
                        pictureMap[row.user_id] = pic
                    }
                }
                pictureMap
            }

    suspend fun refreshProfiles(forceRefresh: Boolean = false): Boolean =
        withContext(Dispatchers.IO) {
            if (!forceRefresh) {
                val cachedAt =
                    db.matchedProfileQueries
                        .latestCachedAt()
                        .executeAsOneOrNull()
                        ?.MAX
                if (cachedAt != null && !CachePolicy.isStale(cachedAt)) return@withContext true
            }
            when (val result = api.getMatchingProfiles()) {
                is ApiResult.Success -> {
                    val now = Clock.System.now().toEpochMilliseconds()
                    db.transaction {
                        result.data.forEach { profile ->
                            db.matchedProfileQueries.upsert(
                                id = profile.id,
                                user_id = profile.userId,
                                name = profile.name,
                                bio = profile.bio,
                                age = profile.age?.toLong(),
                                profile_picture = profile.profilePicture,
                                thumbhash = profile.thumbhash,
                                images_json = json.encodeToString(profile.images),
                                program = profile.program,
                                gradient_start = profile.gradientStart,
                                gradient_end = profile.gradientEnd,
                                tags_json = json.encodeToString(profile.tags),
                                score = profile.score,
                                cached_at = now,
                            )
                        }
                    }
                    true
                }

                is ApiResult.Error -> {
                    false
                }
            }
        }
}
