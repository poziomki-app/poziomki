package com.poziomki.app.data.repository

import app.cash.sqldelight.coroutines.asFlow
import app.cash.sqldelight.coroutines.mapToList
import app.cash.sqldelight.coroutines.mapToOneOrNull
import com.poziomki.app.api.ApiResult
import com.poziomki.app.api.ApiService
import com.poziomki.app.api.CreateEventRequest
import com.poziomki.app.api.Event
import com.poziomki.app.api.EventAttendee
import com.poziomki.app.api.UpdateEventRequest
import com.poziomki.app.chat.matrix.api.MatrixClient
import com.poziomki.app.chat.matrix.api.MatrixClientState
import com.poziomki.app.data.connectivity.ConnectivityMonitor
import com.poziomki.app.data.mapper.toApiModel
import com.poziomki.app.data.sync.PendingOperationsManager
import com.poziomki.app.db.PoziomkiDatabase
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.IO
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.map
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import com.poziomki.app.util.matrixLocalpartFromUserId
import kotlinx.coroutines.withContext
import kotlinx.coroutines.withTimeoutOrNull
import kotlinx.datetime.Clock
import kotlinx.serialization.encodeToString
import kotlinx.serialization.json.Json

class EventRepository(
    private val db: PoziomkiDatabase,
    private val api: ApiService,
    private val connectivityMonitor: ConnectivityMonitor,
    private val pendingOps: PendingOperationsManager,
    private val matrixClient: MatrixClient,
) {
    companion object {
        private const val CREATE_EVENT_ONLINE_TIMEOUT_MS = 1500L
        private const val NETWORK_STATUS_CODE = 0
        private const val REQUEST_TIMEOUT_STATUS_CODE = 408
        private const val TOO_MANY_REQUESTS_STATUS_CODE = 429
        private const val SERVER_ERROR_MIN_STATUS_CODE = 500
    }

    private val json = Json { ignoreUnknownKeys = true }
    private val eventRoomMutex = Mutex()

    fun observeEvents(): Flow<List<Event>> =
        db.eventQueries
            .selectAll()
            .asFlow()
            .mapToList(Dispatchers.IO)
            .map { rows -> rows.map { it.toApiModel() } }

    fun observeEvent(id: String): Flow<Event?> =
        db.eventQueries
            .selectById(id)
            .asFlow()
            .mapToOneOrNull(Dispatchers.IO)
            .map { it?.toApiModel() }

    fun observeAttendees(eventId: String): Flow<List<EventAttendee>> =
        db.eventAttendeeQueries
            .selectByEventId(eventId)
            .asFlow()
            .mapToList(Dispatchers.IO)
            .map { rows -> rows.map { it.toApiModel() } }

    suspend fun fetchRecommendedEvents(): List<Event> =
        withContext(Dispatchers.IO) {
            when (val result = api.getMatchingEvents()) {
                is ApiResult.Success -> result.data
                is ApiResult.Error -> emptyList()
            }
        }

    suspend fun refreshEvents(forceRefresh: Boolean = false): Boolean =
        withContext(Dispatchers.IO) {
            if (!forceRefresh) {
                val cachedAt =
                    db.eventQueries
                        .latestCachedAt()
                        .executeAsOneOrNull()
                        ?.MAX
                if (cachedAt != null && !CachePolicy.isStale(cachedAt)) return@withContext true
            }
            when (val result = api.getEvents()) {
                is ApiResult.Success -> {
                    val now = Clock.System.now().toEpochMilliseconds()
                    db.transaction {
                        result.data.forEach { event ->
                            upsertEvent(event, now)
                        }
                    }
                    true
                }

                is ApiResult.Error -> {
                    false
                }
            }
        }

    suspend fun refreshEvent(
        id: String,
        forceRefresh: Boolean = false,
    ): Boolean =
        withContext(Dispatchers.IO) {
            if (!forceRefresh) {
                val existing = db.eventQueries.selectById(id).executeAsOneOrNull()
                if (existing != null && !CachePolicy.isStale(existing.cached_at)) return@withContext true
            }
            when (val result = api.getEvent(id)) {
                is ApiResult.Success -> {
                    val now = Clock.System.now().toEpochMilliseconds()
                    upsertEvent(result.data, now)
                    true
                }

                is ApiResult.Error -> {
                    false
                }
            }
        }

    suspend fun refreshAttendees(eventId: String) {
        withContext(Dispatchers.IO) {
            when (val result = api.getEventAttendees(eventId)) {
                is ApiResult.Success -> {
                    db.transaction {
                        db.eventAttendeeQueries.deleteByEventId(eventId)
                        result.data.forEach { attendee ->
                            db.eventAttendeeQueries.upsert(
                                event_id = eventId,
                                profile_id = attendee.profileId,
                                user_id = attendee.userId,
                                name = attendee.name,
                                profile_picture = attendee.profilePicture,
                                status = attendee.status,
                            )
                        }
                    }
                }

                is ApiResult.Error -> {}
            }
        }
    }

    suspend fun createEvent(request: CreateEventRequest): ApiResult<Event> =
        withContext(Dispatchers.IO) {
            val tempId = "local_${Clock.System.now().toEpochMilliseconds()}"
            val tempEvent =
                Event(
                    id = tempId,
                    title = request.title,
                    description = request.description,
                    location = request.location,
                    latitude = request.latitude,
                    longitude = request.longitude,
                    startsAt = request.startsAt,
                    endsAt = request.endsAt,
                )
            val now = Clock.System.now().toEpochMilliseconds()
            upsertEvent(tempEvent, now, isDirty = true)

            if (connectivityMonitor.isOnline.value) {
                val onlineResult = withTimeoutOrNull(CREATE_EVENT_ONLINE_TIMEOUT_MS) { api.createEvent(request) }
                when (onlineResult) {
                    is ApiResult.Success -> {
                        db.eventQueries.deleteById(tempId)
                        upsertEvent(onlineResult.data, Clock.System.now().toEpochMilliseconds())
                        ApiResult.Success(onlineResult.data)
                    }

                    is ApiResult.Error -> {
                        if (shouldRetry(onlineResult.status)) {
                            pendingOps.enqueue(
                                type = "create_event",
                                entityId = tempId,
                                payload = json.encodeToString(request),
                            )
                            ApiResult.Success(tempEvent)
                        } else {
                            db.eventQueries.deleteById(tempId)
                            onlineResult
                        }
                    }

                    null -> {
                        pendingOps.enqueue(
                            type = "create_event",
                            entityId = tempId,
                            payload = json.encodeToString(request),
                        )
                        ApiResult.Success(tempEvent)
                    }
                }
            } else {
                pendingOps.enqueue(
                    type = "create_event",
                    entityId = tempId,
                    payload = json.encodeToString(request),
                )
                ApiResult.Success(tempEvent)
            }
        }

    suspend fun updateEvent(
        id: String,
        request: UpdateEventRequest,
    ): ApiResult<Event> =
        withContext(Dispatchers.IO) {
            // Optimistic local update
            val current = db.eventQueries.selectById(id).executeAsOneOrNull()
            if (current != null) {
                db.eventQueries.upsert(
                    id = id,
                    title = request.title ?: current.title,
                    description = request.description ?: current.description,
                    cover_image = current.cover_image,
                    location = request.location ?: current.location,
                    latitude = request.latitude ?: current.latitude,
                    longitude = request.longitude ?: current.longitude,
                    starts_at = request.startsAt ?: current.starts_at,
                    ends_at = request.endsAt ?: current.ends_at,
                    creator_id = current.creator_id,
                    creator_name = current.creator_name,
                    creator_profile_picture = current.creator_profile_picture,
                    attendees_count = current.attendees_count,
                    is_attending = current.is_attending,
                    attendees_preview_json = current.attendees_preview_json,
                    created_at = current.created_at,
                    conversation_id = current.conversation_id,
                    cached_at = current.cached_at,
                    is_dirty = 1L,
                )
            }

            if (connectivityMonitor.isOnline.value) {
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
                        // Return success since we applied optimistically
                        current?.toApiModel()?.let { ApiResult.Success(it) } ?: result
                    }
                }
            } else {
                pendingOps.enqueue(
                    type = "update_event",
                    entityId = id,
                    payload = json.encodeToString(request),
                )
                current?.toApiModel()?.let { ApiResult.Success(it) }
                    ?: ApiResult.Error("Offline and no cached data", "OFFLINE", 0)
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
                when (val result = api.attendEvent(id)) {
                    is ApiResult.Success -> {
                        upsertEvent(result.data, Clock.System.now().toEpochMilliseconds())
                        ApiResult.Success(Unit)
                    }

                    is ApiResult.Error -> {
                        restoreAttendance(
                            id = id,
                            isAttending = previousAttending,
                            attendeesCount = previousCount,
                        )
                        if (shouldRetry(result.status)) {
                            pendingOps.enqueue("attend_event", id, "{}")
                            ApiResult.Success(Unit)
                        } else {
                            result
                        }
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
                when (val result = api.leaveEvent(id)) {
                    is ApiResult.Success -> {
                        upsertEvent(result.data, Clock.System.now().toEpochMilliseconds())
                        ApiResult.Success(Unit)
                    }

                    is ApiResult.Error -> {
                        restoreAttendance(
                            id = id,
                            isAttending = previousAttending,
                            attendeesCount = previousCount,
                        )
                        if (shouldRetry(result.status)) {
                            pendingOps.enqueue("leave_event", id, "{}")
                            ApiResult.Success(Unit)
                        } else {
                            result
                        }
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
                when (val result = api.deleteEvent(id)) {
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
                }
            } else {
                pendingOps.enqueue("delete_event", id, "{}")
                ApiResult.Success(Unit)
            }
        }

    suspend fun ensureEventRoom(
        eventId: String,
        fallbackName: String,
        attendeeUserIds: List<String>,
    ): Result<String> =
        withContext(Dispatchers.IO) {
            eventRoomMutex.withLock {
                runCatching {
                    val existingEvent = db.eventQueries.selectById(eventId).executeAsOneOrNull()
                    existingEvent?.conversation_id?.takeIf { it.startsWith("!") }?.let { existingRoomId ->
                        return@runCatching existingRoomId
                    }

                    matrixClient.ensureStarted().getOrThrow()
                    val ownMatrixUserId = (matrixClient.state.value as? MatrixClientState.Ready)?.userId
                    val ownLocalpart = ownMatrixUserId?.removePrefix("@")?.substringBefore(":")

                    val invitedUsers =
                        attendeeUserIds
                            .map(String::trim)
                            .filter(String::isNotEmpty)
                            .map(::matrixLocalpartFromUserId)
                            .filterNot { it == ownMatrixUserId || it == ownLocalpart }
                            .distinct()

                    val resolvedRoomName =
                        fallbackName
                            .trim()
                            .ifBlank { "Wydarzenie" }

                    val roomId =
                        matrixClient
                            .createRoom(
                                name = resolvedRoomName,
                                invitedUserIds = invitedUsers,
                            ).getOrThrow()

                    updateEventConversationId(eventId, roomId)
                    roomId
                }
            }
        }

    private fun shouldRetry(statusCode: Int): Boolean =
        statusCode == NETWORK_STATUS_CODE ||
            statusCode == REQUEST_TIMEOUT_STATUS_CODE ||
            statusCode == TOO_MANY_REQUESTS_STATUS_CODE ||
            statusCode >= SERVER_ERROR_MIN_STATUS_CODE

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
            is_attending = event.is_attending,
            attendees_preview_json = event.attendees_preview_json,
            created_at = event.created_at,
            conversation_id = event.conversation_id,
            cached_at = event.cached_at,
            is_dirty = event.is_dirty,
        )
    }

    private fun upsertEvent(
        event: Event,
        cachedAt: Long,
        isDirty: Boolean = false,
    ) {
        val existingConversationId =
            db.eventQueries
                .selectById(event.id)
                .executeAsOneOrNull()
                ?.conversation_id
        val conversationId = event.conversationId ?: existingConversationId

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
            creator_id = event.creatorId,
            creator_name = event.creator?.name,
            creator_profile_picture = event.creator?.profilePicture,
            attendees_count = event.attendeesCount.toLong(),
            is_attending = if (event.isAttending) 1L else 0L,
            attendees_preview_json = json.encodeToString(event.attendeesPreview),
            created_at = event.createdAt,
            conversation_id = conversationId,
            cached_at = cachedAt,
            is_dirty = if (isDirty) 1L else 0L,
        )
    }

    private fun updateEventConversationId(
        eventId: String,
        conversationId: String,
    ) {
        val current = db.eventQueries.selectById(eventId).executeAsOneOrNull() ?: return
        db.eventQueries.upsert(
            id = current.id,
            title = current.title,
            description = current.description,
            cover_image = current.cover_image,
            location = current.location,
            latitude = current.latitude,
            longitude = current.longitude,
            starts_at = current.starts_at,
            ends_at = current.ends_at,
            creator_id = current.creator_id,
            creator_name = current.creator_name,
            creator_profile_picture = current.creator_profile_picture,
            attendees_count = current.attendees_count,
            is_attending = current.is_attending,
            attendees_preview_json = current.attendees_preview_json,
            created_at = current.created_at,
            conversation_id = conversationId,
            cached_at = current.cached_at,
            is_dirty = current.is_dirty,
        )
    }
}
