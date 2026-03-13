package com.poziomki.app.data.repository

import com.poziomki.app.chat.matrix.api.MatrixClient
import com.poziomki.app.db.PoziomkiDatabase
import com.poziomki.app.network.ApiResult
import com.poziomki.app.network.ApiService
import com.poziomki.app.network.resolveRoomId
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock

internal class EventRoomRepository(
    private val db: PoziomkiDatabase,
    private val api: ApiService,
    private val matrixClient: MatrixClient,
) {
    companion object {
        private const val EVENT_CHAT_ACCESS_DENIED_MESSAGE = "Brak dostępu do czatu wydarzenia"
        private const val HTTP_STATUS_UNAUTHORIZED = 401
        private const val HTTP_STATUS_FORBIDDEN = 403
    }

    private val eventRoomMutex = Mutex()

    suspend fun ensureEventRoom(eventId: String): Result<String> =
        eventRoomMutex.withLock {
            runCatching {
                val existingEvent = db.eventQueries.selectById(eventId).executeAsOneOrNull()
                existingEvent?.conversation_id?.takeIf { it.startsWith("!") }?.let { existingRoomId ->
                    return@runCatching existingRoomId
                }

                val roomId = resolveEventRoomViaBackend(eventId)
                updateEventConversationId(eventId, roomId)
                roomId
            }
        }

    suspend fun reconcileMembershipAfterAttend(conversationId: String?) {
        val roomId = conversationId?.takeIf { it.startsWith("!") } ?: return

        matrixClient.ensureStarted().getOrElse { return }
        matrixClient.refreshRooms()

        // getJoinedRoom auto-joins invited rooms in RustMatrixClient; this keeps attendee
        // state and room membership aligned without additional UI steps.
        matrixClient.getJoinedRoom(roomId)
    }

    suspend fun reconcileMembershipAfterLeave() {
        matrixClient.ensureStarted().getOrElse { return }
        matrixClient.refreshRooms()
    }

    private suspend fun resolveEventRoomViaBackend(eventId: String): String =
        when (val backendResult = api.getMatrixEventRoom(eventId)) {
            is ApiResult.Success -> {
                backendResult.data.resolveRoomId()
                    ?: error("Backend returned empty event room id")
            }

            is ApiResult.Error -> {
                if (
                    backendResult.status == HTTP_STATUS_UNAUTHORIZED ||
                    backendResult.status == HTTP_STATUS_FORBIDDEN
                ) {
                    error(EVENT_CHAT_ACCESS_DENIED_MESSAGE)
                }
                error(backendResult.message)
            }
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
            max_attendees = current.max_attendees,
            is_attending = current.is_attending,
            attendees_preview_json = current.attendees_preview_json,
            created_at = current.created_at,
            conversation_id = conversationId,
            cached_at = current.cached_at,
            is_dirty = current.is_dirty,
            requires_approval = current.requires_approval,
            is_pending = current.is_pending,
        )
    }
}
