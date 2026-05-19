package com.poziomki.app.chat.cache

import com.poziomki.app.chat.api.EventSendStatus
import com.poziomki.app.chat.api.RoomSummary
import com.poziomki.app.db.PoziomkiDatabase
import kotlin.time.Clock

interface RoomPreviewStore {
    fun loadAll(): List<RoomSummary>

    fun upsert(room: RoomSummary)

    fun upsertAll(rooms: List<RoomSummary>)

    fun updateUnreadCount(
        roomId: String,
        unreadCount: Int,
    )

    fun delete(roomId: String)
}

class SqlDelightRoomPreviewStore(
    private val db: PoziomkiDatabase,
) : RoomPreviewStore {
    override fun loadAll(): List<RoomSummary> =
        runCatching {
            db.roomPreviewQueries.selectAll().executeAsList().map { row ->
                RoomSummary(
                    roomId = row.room_id,
                    displayName = row.display_name.orEmpty(),
                    avatarUrl = row.avatar_url,
                    isDirect = row.is_direct != 0L,
                    directUserId = row.direct_user_id,
                    unreadCount = row.unread_count.toInt(),
                    latestMessage = row.message.takeIf { it.isNotEmpty() },
                    latestTimestampMillis = row.timestamp_millis.takeIf { it > 0L },
                    latestMessageIsMine = row.is_mine != 0L,
                    latestMessageSendStatus = row.send_status?.let(::parseSendStatus),
                    latestMessageReadByCount = row.read_by_count.toInt(),
                )
            }
        }.getOrDefault(emptyList())

    override fun upsert(room: RoomSummary) {
        runCatching {
            db.roomPreviewQueries.upsertAll(
                room_id = room.roomId,
                message = room.latestMessage.orEmpty(),
                timestamp_millis = room.latestTimestampMillis ?: 0L,
                is_mine = if (room.latestMessageIsMine) 1L else 0L,
                send_status = room.latestMessageSendStatus?.name,
                read_by_count = room.latestMessageReadByCount.toLong(),
                updated_at = Clock.System.now().toEpochMilliseconds(),
                display_name = room.displayName,
                avatar_url = room.avatarUrl,
                is_direct = if (room.isDirect) 1L else 0L,
                direct_user_id = room.directUserId,
                unread_count = room.unreadCount.toLong(),
            )
        }
    }

    override fun upsertAll(rooms: List<RoomSummary>) {
        if (rooms.isEmpty()) return
        runCatching {
            db.roomPreviewQueries.transaction {
                rooms.forEach { upsert(it) }
            }
        }
    }

    override fun updateUnreadCount(
        roomId: String,
        unreadCount: Int,
    ) {
        runCatching {
            db.roomPreviewQueries.updateUnreadCount(
                unread_count = unreadCount.toLong(),
                updated_at = Clock.System.now().toEpochMilliseconds(),
                room_id = roomId,
            )
        }
    }

    override fun delete(roomId: String) {
        runCatching { db.roomPreviewQueries.deleteByRoomId(roomId) }
    }
}

private fun parseSendStatus(raw: String): EventSendStatus? = EventSendStatus.entries.firstOrNull { it.name == raw }
