package com.poziomki.app.data.repository

import com.poziomki.app.api.ApiResult
import com.poziomki.app.api.ApiService
import com.poziomki.app.api.resolveRoomId
import com.poziomki.app.api.supportsLegacyMatrixFallback
import com.poziomki.app.chat.matrix.api.MatrixClient
import com.poziomki.app.chat.matrix.api.MatrixClientState
import com.poziomki.app.db.PoziomkiDatabase
import com.poziomki.app.util.matrixLocalpartFromUserId
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock

internal class EventRoomManager(
    private val db: PoziomkiDatabase,
    private val api: ApiService,
    private val matrixClient: MatrixClient,
) {
    companion object {
        private const val EVENT_CHAT_ACCESS_DENIED_MESSAGE = "Brak dostępu do czatu wydarzenia"
    }

    private val eventRoomMutex = Mutex()

    suspend fun ensureEventRoom(
        eventId: String,
        fallbackName: String,
        attendeeUserIds: List<String>,
    ): Result<String> =
        eventRoomMutex.withLock {
            runCatching {
                val existingEvent = db.eventQueries.selectById(eventId).executeAsOneOrNull()
                existingEvent?.conversation_id?.takeIf { it.startsWith("!") }?.let { existingRoomId ->
                    return@runCatching existingRoomId
                }

                resolveEventRoomViaBackend(eventId)?.let { roomId ->
                    updateEventConversationId(eventId, roomId)
                    return@runCatching roomId
                }

                val roomId = createLegacyEventRoom(fallbackName, attendeeUserIds).getOrThrow()
                updateEventConversationId(eventId, roomId)
                roomId
            }
        }

    suspend fun reconcileMembershipAfterAttend(conversationId: String?) {
        val roomId = conversationId?.takeIf { it.startsWith("!") } ?: return

        matrixClient.ensureStarted().getOrElse { return }
        matrixClient.refreshRooms().getOrElse { return }

        // getJoinedRoom auto-joins invited rooms in RustMatrixClient; this keeps attendee
        // state and room membership aligned without additional UI steps.
        matrixClient.getJoinedRoom(roomId)
        matrixClient.refreshRooms()
    }

    suspend fun reconcileMembershipAfterLeave() {
        matrixClient.ensureStarted().getOrElse { return }
        matrixClient.refreshRooms()
    }

    private suspend fun resolveEventRoomViaBackend(eventId: String): String? =
        when (val backendResult = api.getMatrixEventRoom(eventId)) {
            is ApiResult.Success -> {
                backendResult.data.resolveRoomId()
                    ?: throw IllegalStateException("Backend returned empty event room id")
            }

            is ApiResult.Error -> {
                if (backendResult.status == 401 || backendResult.status == 403) {
                    throw IllegalStateException(EVENT_CHAT_ACCESS_DENIED_MESSAGE)
                }
                if (backendResult.supportsLegacyMatrixFallback()) {
                    null
                } else {
                    throw IllegalStateException(backendResult.message)
                }
            }
        }

    private suspend fun createLegacyEventRoom(
        fallbackName: String,
        attendeeUserIds: List<String>,
    ): Result<String> {
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

        return matrixClient.createRoom(
            name = resolvedRoomName,
            invitedUserIds = invitedUsers,
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
