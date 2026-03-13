package com.poziomki.app.data.sync

import com.poziomki.app.connectivity.ConnectivityMonitor
import com.poziomki.app.data.CacheManager
import com.poziomki.app.db.Pending_operation
import com.poziomki.app.db.PoziomkiDatabase
import com.poziomki.app.network.ApiResult
import com.poziomki.app.network.ApiService
import com.poziomki.app.network.CreateEventRequest
import com.poziomki.app.network.UpdateEventRequest
import com.poziomki.app.network.UpdateProfileRequest
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.IO
import kotlinx.coroutines.Job
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.filter
import kotlinx.coroutines.launch
import kotlinx.datetime.Clock
import kotlinx.serialization.json.Json

class SyncEngine(
    private val pendingOps: PendingOperationsManager,
    private val api: ApiService,
    private val db: PoziomkiDatabase,
    private val connectivityMonitor: ConnectivityMonitor,
    private val cacheManager: CacheManager,
    private val scope: CoroutineScope,
) {
    private val json = Json { ignoreUnknownKeys = true }
    private var syncJob: Job? = null

    companion object {
        private const val MAX_RETRIES = 5
        private val BACKOFF_DELAYS = longArrayOf(1000L, 2000L, 4000L, 8000L, 16000L)
    }

    fun start() {
        syncJob =
            scope.launch(Dispatchers.IO) {
                // Trigger sync when connectivity is restored
                connectivityMonitor.isOnline
                    .filter { it }
                    .collect { processQueue() }
            }
    }

    fun stop() {
        syncJob?.cancel()
        syncJob = null
    }

    suspend fun triggerSync() {
        if (connectivityMonitor.isOnline.value) {
            processQueue()
        }
    }

    private suspend fun processQueue() {
        val pending = pendingOps.getPending()
        for (op in pending) {
            if (!connectivityMonitor.isOnline.value) break
            processOperation(op)
        }
        pendingOps.cleanCompleted()
        cacheManager.pruneStaleData()
    }

    @Suppress("TooGenericExceptionCaught")
    private suspend fun processOperation(op: Pending_operation) {
        try {
            val success =
                when (op.type) {
                    OperationType.CREATE_EVENT -> {
                        processCreateEvent(op)
                    }

                    OperationType.UPDATE_EVENT -> {
                        processUpdateEvent(op)
                    }

                    OperationType.DELETE_EVENT -> {
                        processDeleteEvent(op)
                    }

                    OperationType.ATTEND_EVENT -> {
                        processAttendEvent(op)
                    }

                    OperationType.LEAVE_EVENT -> {
                        processLeaveEvent(op)
                    }

                    OperationType.UPDATE_PROFILE -> {
                        processUpdateProfile(op)
                    }

                    OperationType.UPDATE_SETTINGS -> {
                        // Settings sync — requires backend endpoint
                        pendingOps.complete(op.id)
                        true
                    }

                    else -> {
                        pendingOps.complete(op.id)
                        true
                    }
                }

            if (!success && op.retry_count < MAX_RETRIES) {
                pendingOps.fail(op.id)
                pendingOps.resetForRetry(op.id)
                val delayIndex = minOf(op.retry_count.toInt(), BACKOFF_DELAYS.size - 1)
                delay(BACKOFF_DELAYS[delayIndex])
            } else if (!success) {
                pendingOps.fail(op.id)
            }
        } catch (_: Exception) {
            if (op.retry_count < MAX_RETRIES) {
                pendingOps.fail(op.id)
                pendingOps.resetForRetry(op.id)
            } else {
                pendingOps.fail(op.id)
            }
        }
    }

    private suspend fun processCreateEvent(op: Pending_operation): Boolean {
        val request = json.decodeFromString<CreateEventRequest>(op.payload_json)
        return when (val result = api.createEvent(request)) {
            is ApiResult.Success -> {
                val serverEvent = result.data
                val localId = op.entity_id ?: return true
                // Replace temp local ID with server ID
                val now = Clock.System.now().toEpochMilliseconds()
                db.eventQueries.upsert(
                    id = serverEvent.id,
                    title = serverEvent.title,
                    description = serverEvent.description,
                    cover_image = serverEvent.coverImage,
                    location = serverEvent.location,
                    latitude = serverEvent.latitude,
                    longitude = serverEvent.longitude,
                    starts_at = serverEvent.startsAt,
                    ends_at = serverEvent.endsAt,
                    creator_id = serverEvent.creatorId,
                    creator_name = serverEvent.creator?.name,
                    creator_profile_picture = serverEvent.creator?.profilePicture,
                    attendees_count = serverEvent.attendeesCount.toLong(),
                    max_attendees = serverEvent.maxAttendees?.toLong(),
                    is_attending = if (serverEvent.isAttending) 1L else 0L,
                    attendees_preview_json =
                        json.encodeToString(
                            kotlinx.serialization.builtins.ListSerializer(
                                com.poziomki.app.network.EventAttendeePreview
                                    .serializer(),
                            ),
                            serverEvent.attendeesPreview,
                        ),
                    created_at = serverEvent.createdAt,
                    conversation_id = serverEvent.conversationId,
                    cached_at = now,
                    is_dirty = 0L,
                )
                db.eventQueries.deleteById(localId)
                pendingOps.updateEntityId(localId, serverEvent.id)
                pendingOps.complete(op.id)
                true
            }

            is ApiResult.Error -> {
                false
            }
        }
    }

    private suspend fun processUpdateEvent(op: Pending_operation): Boolean {
        val entityId = op.entity_id ?: return true
        val request = json.decodeFromString<UpdateEventRequest>(op.payload_json)
        return when (val result = api.updateEvent(entityId, request)) {
            is ApiResult.Success -> {
                val now = Clock.System.now().toEpochMilliseconds()
                val event = result.data
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
                    max_attendees = event.maxAttendees?.toLong(),
                    is_attending = if (event.isAttending) 1L else 0L,
                    attendees_preview_json =
                        json.encodeToString(
                            kotlinx.serialization.builtins.ListSerializer(
                                com.poziomki.app.network.EventAttendeePreview
                                    .serializer(),
                            ),
                            event.attendeesPreview,
                        ),
                    created_at = event.createdAt,
                    conversation_id = event.conversationId,
                    cached_at = now,
                    is_dirty = 0L,
                )
                pendingOps.complete(op.id)
                true
            }

            is ApiResult.Error -> {
                false
            }
        }
    }

    private suspend fun processDeleteEvent(op: Pending_operation): Boolean {
        val entityId = op.entity_id ?: return true
        return when (api.deleteEvent(entityId)) {
            is ApiResult.Success -> {
                pendingOps.complete(op.id)
                true
            }

            is ApiResult.Error -> {
                false
            }
        }
    }

    private suspend fun processAttendEvent(op: Pending_operation): Boolean {
        val entityId = op.entity_id ?: return true
        return when (api.attendEvent(entityId)) {
            is ApiResult.Success -> {
                pendingOps.complete(op.id)
                true
            }

            is ApiResult.Error -> {
                false
            }
        }
    }

    private suspend fun processLeaveEvent(op: Pending_operation): Boolean {
        val entityId = op.entity_id ?: return true
        return when (api.leaveEvent(entityId)) {
            is ApiResult.Success -> {
                pendingOps.complete(op.id)
                true
            }

            is ApiResult.Error -> {
                false
            }
        }
    }

    private suspend fun processUpdateProfile(op: Pending_operation): Boolean {
        val entityId = op.entity_id ?: return true
        val request = json.decodeFromString<UpdateProfileRequest>(op.payload_json)
        return when (val result = api.updateProfile(entityId, request)) {
            is ApiResult.Success -> {
                val profile = result.data
                val now = Clock.System.now().toEpochMilliseconds()
                db.profileQueries.upsert(
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
                    is_own =
                        db.profileQueries
                            .selectById(profile.id)
                            .executeAsOneOrNull()
                            ?.is_own
                            ?: 0L,
                    created_at = profile.createdAt,
                    updated_at = profile.updatedAt,
                    cached_at = now,
                    is_dirty = 0L,
                )
                pendingOps.complete(op.id)
                true
            }

            is ApiResult.Error -> {
                false
            }
        }
    }
}
