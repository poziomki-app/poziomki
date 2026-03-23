package com.poziomki.app.data.repository

import app.cash.sqldelight.coroutines.asFlow
import app.cash.sqldelight.coroutines.mapToList
import app.cash.sqldelight.coroutines.mapToOneOrNull
import com.poziomki.app.connectivity.ConnectivityMonitor
import com.poziomki.app.data.mapper.toApiModel
import com.poziomki.app.data.mapper.toApiModelWithTags
import com.poziomki.app.data.mapper.toProfile
import com.poziomki.app.data.sync.PendingOperationsManager
import com.poziomki.app.db.PoziomkiDatabase
import com.poziomki.app.network.ApiResult
import com.poziomki.app.network.ApiService
import com.poziomki.app.network.Profile
import com.poziomki.app.network.ProfileWithTags
import com.poziomki.app.network.UpdateProfileRequest
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.IO
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.combine
import kotlinx.coroutines.flow.map
import kotlinx.coroutines.withContext
import kotlinx.datetime.Clock
import kotlinx.serialization.encodeToString
import kotlinx.serialization.json.Json

@Suppress("TooManyFunctions")
class ProfileRepository(
    private val db: PoziomkiDatabase,
    private val api: ApiService,
    private val connectivityMonitor: ConnectivityMonitor,
    private val pendingOps: PendingOperationsManager,
) {
    private val json = Json { ignoreUnknownKeys = true }

    fun observeBookmarkedProfiles(): Flow<List<Profile>> =
        db.profileQueries
            .selectBookmarked()
            .asFlow()
            .mapToList(Dispatchers.IO)
            .map { rows -> rows.map { it.toApiModel() } }

    suspend fun refreshBookmarkedProfiles(): Boolean =
        withContext(Dispatchers.IO) {
            when (val result = api.getBookmarkedProfiles()) {
                is ApiResult.Success -> {
                    db.transaction {
                        db.profileQueries.clearBookmarkedFlags()
                        result.data.forEach { profile ->
                            upsertProfile(profile, isOwn = false, isBookmarked = true)
                        }
                    }
                    true
                }

                is ApiResult.Error -> {
                    false
                }
            }
        }

    fun observeOwnProfile(): Flow<Profile?> =
        db.profileQueries
            .selectOwn()
            .asFlow()
            .mapToOneOrNull(Dispatchers.IO)
            .map { it?.toApiModel() }

    fun observeOwnProfileWithTags(): Flow<ProfileWithTags?> {
        val profileFlow =
            db.profileQueries
                .selectOwn()
                .asFlow()
                .mapToOneOrNull(Dispatchers.IO)

        return profileFlow.map { dbProfile ->
            if (dbProfile == null) return@map null
            val tags =
                db.profileTagQueries
                    .selectByProfileId(dbProfile.id)
                    .executeAsList()
                    .map { it.toApiModel() }
            dbProfile.toApiModelWithTags(tags)
        }
    }

    fun observeProfile(id: String): Flow<ProfileWithTags?> {
        val profileFlow =
            db.profileQueries
                .selectById(id)
                .asFlow()
                .mapToOneOrNull(Dispatchers.IO)

        return profileFlow.map { dbProfile ->
            if (dbProfile == null) return@map null
            val tags =
                db.profileTagQueries
                    .selectByProfileId(id)
                    .executeAsList()
                    .map { it.toApiModel() }
            dbProfile.toApiModelWithTags(tags)
        }
    }

    suspend fun refreshOwnProfile(forceRefresh: Boolean = false): Boolean =
        withContext(Dispatchers.IO) {
            if (!forceRefresh) {
                val cachedAt = db.profileQueries.ownCachedAt().executeAsOneOrNull()
                if (cachedAt != null && !CachePolicy.isStale(cachedAt)) return@withContext true
            }
            when (val result = api.getMyProfile()) {
                is ApiResult.Success -> {
                    upsertProfile(result.data, isOwn = true)
                    // Also fetch full profile for tags
                    when (val fullResult = api.getProfileFull(result.data.id)) {
                        is ApiResult.Success -> {
                            upsertProfileTags(fullResult.data.id, fullResult.data.tags)
                        }

                        is ApiResult.Error -> {}
                    }
                    true
                }

                is ApiResult.Error -> {
                    false
                }
            }
        }

    suspend fun refreshProfile(
        id: String,
        forceRefresh: Boolean = false,
    ): Boolean =
        withContext(Dispatchers.IO) {
            if (!forceRefresh) {
                val cachedAt = db.profileQueries.cachedAtById(id).executeAsOneOrNull()
                if (cachedAt != null && !CachePolicy.isStale(cachedAt)) return@withContext true
            }
            when (val result = api.getProfileFull(id)) {
                is ApiResult.Success -> {
                    val existingIsOwn =
                        db.profileQueries
                            .selectById(id)
                            .executeAsOneOrNull()
                            ?.is_own == 1L
                    upsertProfile(
                        result.data.toProfile(),
                        isOwn = existingIsOwn,
                        isBookmarked = result.data.isBookmarked,
                    )
                    upsertProfileTags(result.data.id, result.data.tags)
                    true
                }

                is ApiResult.Error -> {
                    false
                }
            }
        }

    suspend fun updateProfile(
        id: String,
        request: UpdateProfileRequest,
    ): ApiResult<Profile> =
        withContext(Dispatchers.IO) {
            // Optimistic local update
            val current = db.profileQueries.selectById(id).executeAsOneOrNull()
            if (current != null) {
                db.profileQueries.upsert(
                    id = id,
                    user_id = current.user_id,
                    name = request.name ?: current.name,
                    bio = request.bio ?: current.bio,
                    profile_picture = request.profilePicture ?: current.profile_picture,
                    thumbhash = current.thumbhash,
                    images_json =
                        request.images?.let { json.encodeToString(it) }
                            ?: current.images_json,
                    program = request.program ?: current.program,
                    gradient_start =
                        request.gradientStart?.ifEmpty { null }
                            ?: current.gradient_start,
                    gradient_end =
                        request.gradientEnd?.ifEmpty { null }
                            ?: current.gradient_end,
                    is_own = current.is_own,
                    is_bookmarked = current.is_bookmarked,
                    created_at = current.created_at,
                    updated_at = current.updated_at,
                    cached_at = current.cached_at,
                    is_dirty = 1L,
                )
                request.tagIds?.let { tagIds ->
                    db.profileTagQueries.deleteByProfileId(id)
                    tagIds.forEach { tagId ->
                        db.profileTagQueries.insertTag(id, tagId)
                    }
                }
            }

            if (connectivityMonitor.isOnline.value) {
                when (val result = api.updateProfile(id, request)) {
                    is ApiResult.Success -> {
                        upsertProfile(result.data, isOwn = current?.is_own == 1L)
                        result
                    }

                    is ApiResult.Error -> {
                        pendingOps.enqueue(
                            type = "update_profile",
                            entityId = id,
                            payload = json.encodeToString(request),
                        )
                        current?.toApiModel()?.let { ApiResult.Success(it) } ?: result
                    }
                }
            } else {
                pendingOps.enqueue(
                    type = "update_profile",
                    entityId = id,
                    payload = json.encodeToString(request),
                )
                current?.toApiModel()?.let { ApiResult.Success(it) }
                    ?: ApiResult.Error("Offline and no cached data", "OFFLINE", 0)
            }
        }

    suspend fun queueImageUpload(
        bytes: ByteArray,
        fileName: String,
        context: String,
    ) {
        withContext(Dispatchers.IO) {
            val payload =
                json.encodeToString(
                    mapOf(
                        "fileName" to fileName,
                        "context" to context,
                    ),
                )
            pendingOps.enqueue(
                type = "upload_image",
                entityId = null,
                payload = payload,
            )
        }
    }

    fun updateBookmarked(
        id: String,
        isBookmarked: Boolean,
    ) {
        db.profileQueries.updateBookmarked(
            is_bookmarked = if (isBookmarked) 1L else 0L,
            id = id,
        )
    }

    private fun upsertProfile(
        profile: Profile,
        isOwn: Boolean,
        isBookmarked: Boolean = false,
    ) {
        val now = Clock.System.now().toEpochMilliseconds()
        db.transaction {
            if (isOwn) {
                db.profileQueries.clearOwnExcept(profile.id)
            }
            db.profileQueries.upsert(
                id = profile.id,
                user_id = profile.userId,
                name = profile.name,
                bio = profile.bio,
                profile_picture = profile.profilePicture,
                thumbhash = profile.thumbhash,
                images_json = json.encodeToString(profile.images),
                program = profile.program,
                gradient_start = profile.gradientStart,
                gradient_end = profile.gradientEnd,
                is_own = if (isOwn) 1L else 0L,
                is_bookmarked = if (isBookmarked) 1L else 0L,
                created_at = profile.createdAt,
                updated_at = profile.updatedAt,
                cached_at = now,
                is_dirty = 0L,
            )
        }
    }

    private fun upsertProfileTags(
        profileId: String,
        tags: List<com.poziomki.app.network.Tag>,
    ) {
        db.transaction {
            db.profileTagQueries.deleteByProfileId(profileId)
            tags.forEach { tag ->
                // Ensure tag exists in tag table
                db.tagQueries.upsert(
                    id = tag.id,
                    name = tag.name,
                    scope = tag.scope,
                    category = tag.category,
                    emoji = tag.emoji,
                    parent_id = tag.parentId,
                )
                db.profileTagQueries.insertTag(profileId, tag.id)
            }
        }
    }
}
