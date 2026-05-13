package com.poziomki.app.data.repository

import com.poziomki.app.connectivity.ConnectivityMonitor
import com.poziomki.app.data.mapper.toApiModel
import com.poziomki.app.data.sync.PendingOperationsManager
import com.poziomki.app.db.PoziomkiDatabase
import com.poziomki.app.network.ApiResult
import com.poziomki.app.network.ApiService
import com.poziomki.app.network.CreateEventRequest
import com.poziomki.app.network.Event
import com.poziomki.app.network.Tag
import com.poziomki.app.network.UpdateEventRequest
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.IO
import kotlinx.coroutines.withContext
import kotlinx.coroutines.withTimeoutOrNull
import kotlinx.serialization.encodeToString
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.JsonNull
import kotlinx.serialization.json.JsonPrimitive
import kotlinx.serialization.json.long
import kotlin.time.Clock

@Suppress("TooManyFunctions")
internal class EventMutationRepository(
    private val db: PoziomkiDatabase,
    private val api: ApiService,
    private val connectivityMonitor: ConnectivityMonitor,
    private val pendingOps: PendingOperationsManager,
    private val eventRoomManager: EventRoomRepository,
) {
    companion object {
        private const val CREATE_EVENT_ONLINE_TIMEOUT_MS = 1500L
        private const val MUTATION_TIMEOUT_MS = 10_000L
        private const val NETWORK_STATUS_CODE = 0
        private const val REQUEST_TIMEOUT_STATUS_CODE = 408
        private const val TOO_MANY_REQUESTS_STATUS_CODE = 429
        private const val SERVER_ERROR_MIN_STATUS_CODE = 500
    }

    private val json = Json { ignoreUnknownKeys = true }

    suspend fun createEvent(request: CreateEventRequest): ApiResult<Event> =
        withContext(Dispatchers.IO) {
            val tempId = "local_${Clock.System.now().toEpochMilliseconds()}"
            val optimisticTags = loadTagsByIds(request.tagIds)
            val tempEvent =
                Event(
                    id = tempId,
                    title = request.title,
                    description = request.description,
                    coverImage = request.coverImage,
                    location = request.location,
                    latitude = request.latitude,
                    longitude = request.longitude,
                    startsAt = request.startsAt,
                    endsAt = request.endsAt,
                    tags = optimisticTags,
                    maxAttendees = request.maxAttendees,
                    isAttending = true,
                    attendeesCount = 1,
                )
            val now = Clock.System.now().toEpochMilliseconds()
            upsertEvent(tempEvent, now, isDirty = true, inListFeed = true)

            if (!connectivityMonitor.isOnline.value) {
                return@withContext enqueueCreate(tempId, request, tempEvent)
            }

            val onlineResult = withTimeoutOrNull(CREATE_EVENT_ONLINE_TIMEOUT_MS) { api.createEvent(request) }
            when (onlineResult) {
                is ApiResult.Success -> {
                    db.eventQueries.deleteById(tempId)
                    upsertEvent(onlineResult.data, Clock.System.now().toEpochMilliseconds(), inListFeed = true)
                    ApiResult.Success(onlineResult.data)
                }

                is ApiResult.Error -> {
                    if (shouldRetry(onlineResult.status)) {
                        enqueueCreate(tempId, request, tempEvent)
                    } else {
                        db.eventQueries.deleteById(tempId)
                        onlineResult
                    }
                }

                null -> {
                    enqueueCreate(tempId, request, tempEvent)
                }
            }
        }

    @Suppress("CyclomaticComplexMethod", "LongMethod")
    suspend fun updateEvent(
        id: String,
        request: UpdateEventRequest,
    ): ApiResult<Event> =
        withContext(Dispatchers.IO) {
            val current = db.eventQueries.selectById(id).executeAsOneOrNull()
            val optimisticEvent = current?.let { buildOptimisticEvent(it, request) }
            if (current != null) {
                val optimisticTagsJson =
                    request.tagIds
                        ?.let(::loadTagsByIds)
                        ?.let(json::encodeToString)
                        ?: current.tags_json
                db.eventQueries.upsert(
                    id = id,
                    title = request.title ?: current.title,
                    description = request.description ?: current.description,
                    cover_image = request.coverImage ?: current.cover_image,
                    location = request.location ?: current.location,
                    latitude = request.latitude ?: current.latitude,
                    longitude = request.longitude ?: current.longitude,
                    starts_at = request.startsAt ?: current.starts_at,
                    ends_at = request.endsAt ?: current.ends_at,
                    creator_id = current.creator_id,
                    creator_name = current.creator_name,
                    creator_profile_picture = current.creator_profile_picture,
                    attendees_count = current.attendees_count,
                    max_attendees = maxAttendeesFromRequest(request),
                    is_attending = current.is_attending,
                    is_saved = current.is_saved,
                    attendees_preview_json = current.attendees_preview_json,
                    tags_json = optimisticTagsJson,
                    created_at = current.created_at,
                    conversation_id = current.conversation_id,
                    score = current.score,
                    cached_at = current.cached_at,
                    in_list_feed = current.in_list_feed,
                    is_recommended = current.is_recommended,
                    is_dirty = 1L,
                    requires_approval = current.requires_approval,
                    is_pending = current.is_pending,
                    visibility = request.visibility ?: current.visibility,
                )
            }

            if (!connectivityMonitor.isOnline.value) {
                pendingOps.enqueue(
                    type = "update_event",
                    entityId = id,
                    payload = json.encodeToString(request),
                )
                optimisticEvent?.let { ApiResult.Success(it) }
                    ?: ApiResult.Error("Offline and no cached data", "OFFLINE", 0)
            } else {
                withTimeoutOrNull(MUTATION_TIMEOUT_MS) {
                    updateEventOnline(id, request, optimisticEvent)
                } ?: run {
                    pendingOps.enqueue(
                        type = "update_event",
                        entityId = id,
                        payload = json.encodeToString(request),
                    )
                    optimisticEvent?.let { ApiResult.Success(it) }
                        ?: ApiResult.Error("Timeout", "TIMEOUT", 0)
                }
            }
        }

    suspend fun attendEvent(id: String): ApiResult<Unit> =
        withContext(Dispatchers.IO) {
            val current = db.eventQueries.selectById(id).executeAsOneOrNull()
            val previousAttending = current?.is_attending == 1L
            val previousCount = current?.attendees_count ?: 0L
            if (current != null && !previousAttending) {
                db.eventQueries.updateAttendance(
                    is_attending = 1L,
                    attendees_count = current.attendees_count + 1,
                    id = id,
                )
            }

            if (connectivityMonitor.isOnline.value) {
                val result = withTimeoutOrNull(MUTATION_TIMEOUT_MS) { api.attendEvent(id) }
                when (result) {
                    is ApiResult.Success -> {
                        upsertEvent(result.data, Clock.System.now().toEpochMilliseconds())
                        eventRoomManager.reconcileMembershipAfterAttend(result.data.conversationId)
                        ApiResult.Success(Unit)
                    }

                    is ApiResult.Error -> {
                        if (shouldRetry(result.status)) {
                            pendingOps.enqueue("attend_event", id, "{}")
                            ApiResult.Success(Unit)
                        } else {
                            restoreAttendance(
                                id = id,
                                isAttending = previousAttending,
                                attendeesCount = previousCount,
                            )
                            result
                        }
                    }

                    null -> {
                        pendingOps.enqueue("attend_event", id, "{}")
                        ApiResult.Success(Unit)
                    }
                }
            } else {
                pendingOps.enqueue("attend_event", id, "{}")
                ApiResult.Success(Unit)
            }
        }

    suspend fun leaveEvent(id: String): ApiResult<Unit> =
        withContext(Dispatchers.IO) {
            val current = db.eventQueries.selectById(id).executeAsOneOrNull()
            val previousAttending = current?.is_attending == 1L
            val previousCount = current?.attendees_count ?: 0L
            if (current != null && previousAttending) {
                db.eventQueries.updateAttendance(
                    is_attending = 0L,
                    attendees_count = maxOf(0L, current.attendees_count - 1),
                    id = id,
                )
            }

            if (connectivityMonitor.isOnline.value) {
                val result = withTimeoutOrNull(MUTATION_TIMEOUT_MS) { api.leaveEvent(id) }
                when (result) {
                    is ApiResult.Success -> {
                        upsertEvent(result.data, Clock.System.now().toEpochMilliseconds())
                        eventRoomManager.reconcileMembershipAfterLeave()
                        ApiResult.Success(Unit)
                    }

                    is ApiResult.Error -> {
                        if (shouldRetry(result.status)) {
                            pendingOps.enqueue("leave_event", id, "{}")
                            ApiResult.Success(Unit)
                        } else {
                            restoreAttendance(
                                id = id,
                                isAttending = previousAttending,
                                attendeesCount = previousCount,
                            )
                            result
                        }
                    }

                    null -> {
                        pendingOps.enqueue("leave_event", id, "{}")
                        ApiResult.Success(Unit)
                    }
                }
            } else {
                pendingOps.enqueue("leave_event", id, "{}")
                ApiResult.Success(Unit)
            }
        }

    suspend fun deleteEvent(id: String): ApiResult<Unit> =
        withContext(Dispatchers.IO) {
            val current = db.eventQueries.selectById(id).executeAsOneOrNull()
            db.eventQueries.deleteById(id)
            if (connectivityMonitor.isOnline.value) {
                val result = withTimeoutOrNull(MUTATION_TIMEOUT_MS) { api.deleteEvent(id) }
                when (result) {
                    is ApiResult.Success -> {
                        ApiResult.Success(Unit)
                    }

                    is ApiResult.Error -> {
                        if (shouldRetry(result.status)) {
                            pendingOps.enqueue("delete_event", id, "{}")
                            ApiResult.Success(Unit)
                        } else {
                            current?.let(::restoreEvent)
                            result
                        }
                    }

                    null -> {
                        pendingOps.enqueue("delete_event", id, "{}")
                        ApiResult.Success(Unit)
                    }
                }
            } else {
                pendingOps.enqueue("delete_event", id, "{}")
                ApiResult.Success(Unit)
            }
        }

    suspend fun saveEvent(id: String): ApiResult<Unit> = toggleSaved(id, saved = true)

    suspend fun unsaveEvent(id: String): ApiResult<Unit> = toggleSaved(id, saved = false)

    private suspend fun toggleSaved(
        id: String,
        saved: Boolean,
    ): ApiResult<Unit> =
        withContext(Dispatchers.IO) {
            val previousSaved =
                db.eventQueries
                    .selectById(id)
                    .executeAsOneOrNull()
                    ?.is_saved ?: 0L
            db.eventQueries.updateSaved(is_saved = if (saved) 1L else 0L, id = id)

            if (connectivityMonitor.isOnline.value) {
                val result =
                    withTimeoutOrNull(MUTATION_TIMEOUT_MS) {
                        if (saved) api.saveEvent(id) else api.unsaveEvent(id)
                    }
                when (result) {
                    is ApiResult.Success -> {
                        upsertEvent(result.data, Clock.System.now().toEpochMilliseconds())
                        ApiResult.Success(Unit)
                    }

                    is ApiResult.Error -> {
                        if (shouldRetry(result.status)) {
                            pendingOps.enqueue(
                                if (saved) "save_event" else "unsave_event",
                                id,
                                "{}",
                            )
                            ApiResult.Success(Unit)
                        } else {
                            db.eventQueries.updateSaved(is_saved = previousSaved, id = id)
                            result
                        }
                    }

                    null -> {
                        pendingOps.enqueue(if (saved) "save_event" else "unsave_event", id, "{}")
                        ApiResult.Success(Unit)
                    }
                }
            } else {
                pendingOps.enqueue(if (saved) "save_event" else "unsave_event", id, "{}")
                ApiResult.Success(Unit)
            }
        }

    @Suppress("CyclomaticComplexMethod")
    fun upsertEvent(
        event: Event,
        cachedAt: Long,
        isDirty: Boolean = false,
        inListFeed: Boolean? = null,
        isRecommended: Boolean? = null,
    ) {
        val existing = db.eventQueries.selectById(event.id).executeAsOneOrNull()
        val existingConversationId = existing?.conversation_id
        val conversationId = event.conversationId ?: existingConversationId
        val effectiveInListFeed =
            when (inListFeed) {
                true -> 1L
                false -> if (existing?.in_list_feed == 1L) 1L else 0L
                null -> existing?.in_list_feed ?: 1L
            }
        val effectiveIsRecommended =
            when (isRecommended) {
                true -> 1L
                false -> 0L
                null -> existing?.is_recommended ?: 0L
            }
        val finalScore = if (isRecommended == true) event.score else (existing?.score ?: event.score)

        db.eventQueries.upsert(
            id = event.id,
            title = event.title,
            description = event.description,
            cover_image = event.coverImage,
            location = event.location,
            latitude = event.latitude,
            longitude = event.longitude,
            starts_at = event.startsAt,
            ends_at = event.endsAt,
            creator_id = event.creator?.id,
            creator_name = event.creator?.name,
            creator_profile_picture = event.creator?.profilePicture,
            attendees_count = event.attendeesCount.toLong(),
            max_attendees = event.maxAttendees?.toLong(),
            is_attending = if (event.isAttending) 1L else 0L,
            is_saved = if (event.isSaved) 1L else 0L,
            attendees_preview_json = json.encodeToString(event.attendeesPreview),
            tags_json = json.encodeToString(event.tags),
            created_at = event.createdAt,
            conversation_id = conversationId,
            score = finalScore,
            cached_at = cachedAt,
            in_list_feed = effectiveInListFeed,
            is_recommended = effectiveIsRecommended,
            is_dirty = if (isDirty) 1L else 0L,
            requires_approval = if (event.requiresApproval) 1L else 0L,
            is_pending = if (event.isPending) 1L else 0L,
            visibility = event.visibility,
        )
    }

    private fun shouldRetry(statusCode: Int): Boolean =
        statusCode == NETWORK_STATUS_CODE ||
            statusCode == REQUEST_TIMEOUT_STATUS_CODE ||
            statusCode == TOO_MANY_REQUESTS_STATUS_CODE ||
            statusCode >= SERVER_ERROR_MIN_STATUS_CODE

    private suspend fun enqueueCreate(
        tempId: String,
        request: CreateEventRequest,
        tempEvent: Event,
    ): ApiResult.Success<Event> {
        pendingOps.enqueue(type = "create_event", entityId = tempId, payload = json.encodeToString(request))
        return ApiResult.Success(tempEvent)
    }

    private suspend fun updateEventOnline(
        id: String,
        request: UpdateEventRequest,
        optimisticEvent: Event?,
    ): ApiResult<Event> =
        when (val result = api.updateEvent(id, request)) {
            is ApiResult.Success -> {
                val now = Clock.System.now().toEpochMilliseconds()
                upsertEvent(result.data, now)
                result
            }

            is ApiResult.Error -> {
                pendingOps.enqueue(
                    type = "update_event",
                    entityId = id,
                    payload = json.encodeToString(request),
                )
                optimisticEvent?.let { ApiResult.Success(it) } ?: result
            }
        }

    private fun loadTagsByIds(tagIds: List<String>): List<Tag> =
        tagIds.mapNotNull { tagId ->
            db.tagQueries
                .selectById(tagId)
                .executeAsOneOrNull()
                ?.toApiModel()
        }

    private fun buildOptimisticEvent(
        current: com.poziomki.app.db.Event,
        request: UpdateEventRequest,
    ): Event {
        val cached = current.toApiModel()
        val optimisticTags = request.tagIds?.let(::loadTagsByIds) ?: cached.tags
        return cached.copy(
            title = request.title ?: cached.title,
            description = request.description ?: cached.description,
            coverImage = request.coverImage ?: cached.coverImage,
            location = request.location ?: cached.location,
            latitude = request.latitude ?: cached.latitude,
            longitude = request.longitude ?: cached.longitude,
            startsAt = request.startsAt ?: cached.startsAt,
            endsAt = request.endsAt ?: cached.endsAt,
            visibility = request.visibility ?: cached.visibility,
            tags = optimisticTags,
        )
    }

    private fun restoreAttendance(
        id: String,
        isAttending: Boolean,
        attendeesCount: Long,
    ) {
        db.eventQueries.updateAttendance(
            is_attending = if (isAttending) 1L else 0L,
            attendees_count = attendeesCount,
            id = id,
        )
    }

    private fun maxAttendeesFromRequest(request: UpdateEventRequest): Long? {
        val element = request.maxAttendees
        return if (element is JsonPrimitive && element !is JsonNull) element.long else null
    }

    private fun restoreEvent(event: com.poziomki.app.db.Event) {
        db.eventQueries.upsert(
            id = event.id,
            title = event.title,
            description = event.description,
            cover_image = event.cover_image,
            location = event.location,
            latitude = event.latitude,
            longitude = event.longitude,
            starts_at = event.starts_at,
            ends_at = event.ends_at,
            creator_id = event.creator_id,
            creator_name = event.creator_name,
            creator_profile_picture = event.creator_profile_picture,
            attendees_count = event.attendees_count,
            max_attendees = event.max_attendees,
            is_attending = event.is_attending,
            is_saved = event.is_saved,
            attendees_preview_json = event.attendees_preview_json,
            tags_json = event.tags_json,
            created_at = event.created_at,
            conversation_id = event.conversation_id,
            score = event.score,
            cached_at = event.cached_at,
            in_list_feed = event.in_list_feed,
            is_recommended = event.is_recommended,
            is_dirty = event.is_dirty,
            requires_approval = event.requires_approval,
            is_pending = event.is_pending,
            visibility = event.visibility,
        )
    }
}
