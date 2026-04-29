package com.poziomki.app.data.repository

import app.cash.sqldelight.coroutines.asFlow
import app.cash.sqldelight.coroutines.mapToList
import app.cash.sqldelight.coroutines.mapToOneOrNull
import com.poziomki.app.chat.api.ChatClient
import com.poziomki.app.connectivity.ConnectivityMonitor
import com.poziomki.app.data.mapper.toApiModel
import com.poziomki.app.data.sync.OperationType
import com.poziomki.app.data.sync.PendingOperationsManager
import com.poziomki.app.data.sync.SyncEngine
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
import kotlin.concurrent.Volatile
import kotlin.time.Clock
import kotlin.time.Instant
import com.poziomki.app.db.Event as DbEvent

private fun Event.hasFinished(now: Instant): Boolean {
    val end = endsAt ?: startsAt
    val endInstant = runCatching { Instant.parse(end) }.getOrNull() ?: return false
    return endInstant < now
}

private fun List<Event>.dropFinished(): List<Event> {
    val now = Clock.System.now()
    return filterNot { it.hasFinished(now) }
}

class EventRepository(
    private val db: PoziomkiDatabase,
    private val api: ApiService,
    private val connectivityMonitor: ConnectivityMonitor,
    private val pendingOps: PendingOperationsManager,
    private val chatClient: ChatClient,
    syncEngine: SyncEngine,
) {
    companion object {
        private const val EVENTS_LIST_CACHE_KEY = "events_list"
        private const val RECOMMENDED_CACHE_KEY = "recommended_events"
        private const val SAVED_CACHE_KEY = "saved_events"
        private const val MY_EVENTS_CACHE_KEY = "my_events"

        @Volatile
        private var lastLat: Double? = null

        @Volatile
        private var lastLng: Double? = null

        @Volatile
        private var lastRadius: Int? = null
    }

    private val eventRoomManager = EventRoomRepository(db = db, api = api, chatClient = chatClient)
    private val eventMutationManager =
        EventMutationRepository(
            db = db,
            api = api,
            connectivityMonitor = connectivityMonitor,
            pendingOps = pendingOps,
            eventRoomManager = eventRoomManager,
        )

    val syncErrors: Flow<String> =
        syncEngine.permanentFailures.map { op ->
            when (op.type) {
                OperationType.ATTEND_EVENT -> "Nie udało się zapisać na wydarzenie"
                OperationType.LEAVE_EVENT -> "Nie udało się wypisać z wydarzenia"
                OperationType.CREATE_EVENT -> "Nie udało się utworzyć wydarzenia"
                OperationType.DELETE_EVENT -> "Nie udało się usunąć wydarzenia"
                OperationType.SAVE_EVENT -> "Nie udało się zapisać wydarzenia"
                OperationType.UNSAVE_EVENT -> "Nie udało się usunąć z zapisanych"
                else -> "Nie udało się zsynchronizować danych"
            }
        }

    fun observeEvents(): Flow<List<Event>> =
        db.eventQueries
            .selectAll()
            .asFlow()
            .mapToList(Dispatchers.IO)
            .map { rows -> rows.map { it.toApiModel() }.dropFinished() }

    fun observeRecommendedEvents(): Flow<List<Event>> =
        db.eventQueries
            .selectRecommended()
            .asFlow()
            .mapToList(Dispatchers.IO)
            .map { rows -> rows.map { it.toApiModel() }.dropFinished() }

    fun observeSavedEvents(): Flow<List<Event>> =
        db.eventQueries
            .selectSaved()
            .asFlow()
            .mapToList(Dispatchers.IO)
            .map { rows -> rows.map { it.toApiModel() }.dropFinished() }

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

    fun observeEventsWithConversation(): Flow<List<Event>> =
        db.eventQueries
            .selectAllWithConversation(::DbEvent)
            .asFlow()
            .mapToList(Dispatchers.IO)
            .map { rows -> rows.map { it.toApiModel() } }

    suspend fun findByConversationId(conversationId: String): Event? =
        withContext(Dispatchers.IO) {
            db.eventQueries
                .selectByConversationId(conversationId)
                .executeAsOneOrNull()
                ?.toApiModel()
        }

    fun observeEventConversationIds(): Flow<Set<String>> =
        observeEventsWithConversation().map { events ->
            events
                .mapNotNull(Event::conversationId)
                .filter { it.isNotBlank() }
                .toSet()
        }

    suspend fun fetchRecommendedEvents(
        lat: Double? = null,
        lng: Double? = null,
        radiusM: Int? = null,
        forceRefresh: Boolean = false,
    ): List<Event> =
        withContext(Dispatchers.IO) {
            val locationChanged = lat != lastLat || lng != lastLng || radiusM != lastRadius

            if (!forceRefresh && !locationChanged) {
                val cachedAt =
                    db.cacheStateQueries
                        .selectByKey(RECOMMENDED_CACHE_KEY)
                        .executeAsOneOrNull()
                        ?.cached_at
                if (cachedAt != null && !CachePolicy.isStale(cachedAt)) {
                    return@withContext db.eventQueries
                        .selectRecommended()
                        .executeAsList()
                        .map { it.toApiModel() }
                        .dropFinished()
                }
            }

            when (val result = api.getMatchingEvents(lat = lat, lng = lng, radiusM = radiusM)) {
                is ApiResult.Success -> {
                    val now = Clock.System.now().toEpochMilliseconds()
                    db.transaction {
                        db.eventQueries.clearRecommendedFlags()
                        result.data.forEach { event ->
                            eventMutationManager.upsertEvent(
                                event = event,
                                cachedAt = now,
                                inListFeed = false,
                                isRecommended = true,
                            )
                        }
                        db.cacheStateQueries.upsert(RECOMMENDED_CACHE_KEY, now)
                    }
                    lastLat = lat
                    lastLng = lng
                    lastRadius = radiusM
                    result.data.dropFinished()
                }

                is ApiResult.Error -> {
                    db.eventQueries
                        .selectRecommended()
                        .executeAsList()
                        .map { it.toApiModel() }
                        .dropFinished()
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

    suspend fun refreshSavedEvents(forceRefresh: Boolean = false): Boolean =
        withContext(Dispatchers.IO) {
            if (!forceRefresh) {
                val cachedAt =
                    db.cacheStateQueries
                        .selectByKey(SAVED_CACHE_KEY)
                        .executeAsOneOrNull()
                        ?.cached_at
                if (cachedAt != null && !CachePolicy.isStale(cachedAt)) return@withContext true
            }
            when (val result = api.getSavedEvents()) {
                is ApiResult.Success -> {
                    val now = Clock.System.now().toEpochMilliseconds()
                    db.transaction {
                        db.eventQueries.clearSavedFlags()
                        result.data.forEachIndexed { index, event ->
                            eventMutationManager.upsertEvent(
                                event,
                                now - index,
                                inListFeed = false,
                            )
                        }
                        db.cacheStateQueries.upsert(SAVED_CACHE_KEY, now)
                    }
                    true
                }

                is ApiResult.Error -> {
                    false
                }
            }
        }

    suspend fun refreshMyEvents(forceRefresh: Boolean = false): Boolean =
        withContext(Dispatchers.IO) {
            if (!forceRefresh) {
                val cachedAt =
                    db.cacheStateQueries
                        .selectByKey(MY_EVENTS_CACHE_KEY)
                        .executeAsOneOrNull()
                        ?.cached_at
                if (cachedAt != null && !CachePolicy.isStale(cachedAt)) return@withContext true
            }
            when (val result = api.getMyEvents()) {
                is ApiResult.Success -> {
                    val now = Clock.System.now().toEpochMilliseconds()
                    db.transaction {
                        result.data.forEach { event ->
                            eventMutationManager.upsertEvent(event, now, inListFeed = false)
                        }
                        db.cacheStateQueries.upsert(MY_EVENTS_CACHE_KEY, now)
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
