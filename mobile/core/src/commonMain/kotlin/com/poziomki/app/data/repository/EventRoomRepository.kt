package com.poziomki.app.data.repository

import com.poziomki.app.chat.api.ChatClient
import com.poziomki.app.db.PoziomkiDatabase
import com.poziomki.app.network.ApiResult
import com.poziomki.app.network.ApiService
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock

internal class EventRoomRepository(
    private val db: PoziomkiDatabase,
    private val api: ApiService,
    private val chatClient: ChatClient,
) {
    companion object {
        private const val EVENT_CHAT_ACCESS_DENIED_MESSAGE = "Brak dost\u0119pu do czatu wydarzenia"
        private const val HTTP_STATUS_UNAUTHORIZED = 401
        private const val HTTP_STATUS_FORBIDDEN = 403
    }

    private val eventRoomMutex = Mutex()

    suspend fun ensureEventRoom(eventId: String): Result<String> =
        eventRoomMutex.withLock {
            runCatching {
                val existingEvent = db.eventQueries.selectById(eventId).executeAsOneOrNull()
                existingEvent?.conversation_id?.takeIf { it.isNotBlank() }?.let { existingRoomId ->
                    return@runCatching existingRoomId
                }

                val conversationId = resolveEventConversationViaBackend(eventId)
                updateEventConversationId(eventId, conversationId)
                conversationId
            }
        }

    suspend fun reconcileMembershipAfterAttend(conversationId: String?) {
        val roomId = conversationId?.takeIf { it.isNotBlank() } ?: return
        chatClient.ensureStarted().getOrElse { return }
        chatClient.refreshRooms()
        chatClient.getJoinedRoom(roomId)
    }

    suspend fun reconcileMembershipAfterLeave() {
        chatClient.ensureStarted().getOrElse { return }
        chatClient.refreshRooms()
    }

    private suspend fun resolveEventConversationViaBackend(eventId: String): String =
        when (val result = api.getChatEventConversation(eventId)) {
            is ApiResult.Success -> {
                result.data.conversationId
            }

            is ApiResult.Error -> {
                if (
                    result.status == HTTP_STATUS_UNAUTHORIZED ||
                    result.status == HTTP_STATUS_FORBIDDEN
                ) {
                    error(EVENT_CHAT_ACCESS_DENIED_MESSAGE)
                }
                error(result.message)
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
            is_saved = current.is_saved,
            attendees_preview_json = current.attendees_preview_json,
            tags_json = current.tags_json,
            created_at = current.created_at,
            conversation_id = conversationId,
            score = current.score,
            cached_at = current.cached_at,
            in_list_feed = current.in_list_feed,
            is_dirty = current.is_dirty,
            requires_approval = current.requires_approval,
            is_pending = current.is_pending,
        )
    }
}
