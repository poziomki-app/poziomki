package com.poziomki.app.data.repository

import app.cash.sqldelight.coroutines.asFlow
import app.cash.sqldelight.coroutines.mapToList
import app.cash.sqldelight.coroutines.mapToOneOrNull
import com.poziomki.app.chat.matrix.api.MatrixClient
import com.poziomki.app.connectivity.ConnectivityMonitor
import com.poziomki.app.data.mapper.toApiModel
import com.poziomki.app.data.sync.PendingOperationsManager
import com.poziomki.app.db.PoziomkiDatabase
import com.poziomki.app.network.ApiResult
import com.poziomki.app.network.ApiService
import com.poziomki.app.network.CreateEventRequest
import com.poziomki.app.network.Event
import com.poziomki.app.network.EventAttendee
import com.poziomki.app.network.UpdateEventRequest
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.IO
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.map
import kotlinx.coroutines.withContext
import kotlinx.datetime.Clock

class EventRepository(
    private val db: PoziomkiDatabase,
    private val api: ApiService,
    private val connectivityMonitor: ConnectivityMonitor,
    private val pendingOps: PendingOperationsManager,
    private val matrixClient: MatrixClient,
) {
    companion object {
        private const val EVENTS_LIST_CACHE_KEY = "events_list"
        private const val RECOMMENDED_EVENTS_CACHE_KEY = "recommended_events"
    }

    private val eventRoomManager = EventRoomRepository(db = db, api = api, matrixClient = matrixClient)
    private val eventMutationManager =
        EventMutationRepository(
            db = db,
            api = api,
            connectivityMonitor = connectivityMonitor,
            pendingOps = pendingOps,
            eventRoomManager = eventRoomManager,
        )

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

    fun observeEventConversationIds(): Flow<Set<String>> =
        observeEvents().map { events ->
            events
                .filter { it.isAttending }
                .mapNotNull(Event::conversationId)
                .filter { it.isNotBlank() }
                .toSet()
        }

    suspend fun fetchRecommendedEvents(): List<Event> =
        withContext(Dispatchers.IO) {
            when (val result = api.getMatchingEvents()) {
                is ApiResult.Success -> {
                    val now = Clock.System.now().toEpochMilliseconds()
                    db.transaction {
                        result.data.forEach { event ->
                            eventMutationManager.upsertEvent(event, now, inListFeed = false)
                        }
                        db.cacheStateQueries.upsert(RECOMMENDED_EVENTS_CACHE_KEY, now)
                    }
                    result.data
                }

                is ApiResult.Error -> {
                    emptyList()
                }
            }
        }

    suspend fun refreshEvents(forceRefresh: Boolean = false): Boolean =
        withContext(Dispatchers.IO) {
            if (!forceRefresh) {
                val cachedAt =
                    db.cacheStateQueries
                        .selectByKey(EVENTS_LIST_CACHE_KEY)
                        .executeAsOneOrNull()
                        ?.cached_at
                if (cachedAt != null && !CachePolicy.isStale(cachedAt)) return@withContext true
            }
            when (val result = api.getEvents()) {
                is ApiResult.Success -> {
                    val now = Clock.System.now().toEpochMilliseconds()
                    db.transaction {
                        db.eventQueries.clearListFeedFlags()
                        result.data.forEach { event ->
                            eventMutationManager.upsertEvent(event, now, inListFeed = true)
                        }
                        db.cacheStateQueries.upsert(EVENTS_LIST_CACHE_KEY, now)
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
                    eventMutationManager.upsertEvent(result.data, now)
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
                                is_creator = if (attendee.isCreator) 1L else 0L,
                            )
                        }
                    }
                }

                is ApiResult.Error -> {}
            }
        }
    }

    suspend fun createEvent(request: CreateEventRequest): ApiResult<Event> = eventMutationManager.createEvent(request)

    suspend fun updateEvent(
        id: String,
        request: UpdateEventRequest,
    ): ApiResult<Event> = eventMutationManager.updateEvent(id, request)

    suspend fun attendEvent(id: String): ApiResult<Unit> = eventMutationManager.attendEvent(id)

    suspend fun leaveEvent(id: String): ApiResult<Unit> = eventMutationManager.leaveEvent(id)

    suspend fun saveEvent(id: String): ApiResult<Unit> = eventMutationManager.saveEvent(id)

    suspend fun unsaveEvent(id: String): ApiResult<Unit> = eventMutationManager.unsaveEvent(id)

    suspend fun deleteEvent(id: String): ApiResult<Unit> = eventMutationManager.deleteEvent(id)

    suspend fun approveAttendee(
        eventId: String,
        profileId: String,
    ): ApiResult<Unit> =
        withContext(Dispatchers.IO) {
            when (val result = api.approveAttendee(eventId, profileId)) {
                is ApiResult.Success -> ApiResult.Success(Unit)
                is ApiResult.Error -> result
            }
        }

    suspend fun rejectAttendee(
        eventId: String,
        profileId: String,
    ): ApiResult<Unit> =
        withContext(Dispatchers.IO) {
            when (val result = api.rejectAttendee(eventId, profileId)) {
                is ApiResult.Success -> ApiResult.Success(Unit)
                is ApiResult.Error -> result
            }
        }

    suspend fun ensureEventRoom(eventId: String): Result<String> =
        withContext(Dispatchers.IO) {
            eventRoomManager.ensureEventRoom(
                eventId = eventId,
            )
        }
}
