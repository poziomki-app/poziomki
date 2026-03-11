package com.poziomki.app.chat.cache

import com.poziomki.app.chat.matrix.api.MatrixEventSendStatus
import com.poziomki.app.chat.matrix.api.MatrixReaction
import com.poziomki.app.chat.matrix.api.MatrixReactionSender
import com.poziomki.app.chat.matrix.api.MatrixReplyDetails
import com.poziomki.app.chat.matrix.api.MatrixTimelineItem
import com.poziomki.app.db.PoziomkiDatabase
import kotlinx.datetime.Clock
import kotlinx.serialization.Serializable
import kotlinx.serialization.builtins.ListSerializer
import kotlinx.serialization.encodeToString
import kotlinx.serialization.json.Json

class SqlDelightRoomTimelineCacheStore(
    private val db: PoziomkiDatabase,
) : RoomTimelineCacheStore {
    private val json =
        Json {
            ignoreUnknownKeys = true
            encodeDefaults = true
            explicitNulls = false
        }

    override fun loadSnapshot(
        roomId: String,
        limit: Int,
    ): RoomTimelineCacheSnapshotData {
        if (roomId.isBlank() || limit <= 0) {
            return RoomTimelineCacheSnapshotData(
                items = emptyList(),
                isHydrated = false,
                cachedItemCount = 0,
                updatedAtMillis = 0L,
            )
        }

        val row =
            runCatching {
                db.roomTimelineCacheQueries.selectByRoomId(roomId).executeAsOneOrNull()
            }.getOrNull()
                ?: return RoomTimelineCacheSnapshotData(
                    items = emptyList(),
                    isHydrated = false,
                    cachedItemCount = 0,
                    updatedAtMillis = 0L,
                )

        val decodedItems =
            runCatching {
                json
                    .decodeFromString(ListSerializer(CachedTimelineItem.serializer()), row.payload_json)
                    .map(CachedTimelineItem::toDomain)
            }.getOrElse {
                runCatching { db.roomTimelineCacheQueries.deleteByRoomId(roomId) }
                emptyList()
            }

        val snapshot = decodedItems.takeLast(limit)
        return RoomTimelineCacheSnapshotData(
            items = snapshot,
            isHydrated = decodedItems.any { it is MatrixTimelineItem.TimelineStart },
            cachedItemCount = decodedItems.size,
            updatedAtMillis = row.updated_at,
        )
    }

    override fun saveSnapshot(
        roomId: String,
        items: List<MatrixTimelineItem>,
        isHydrated: Boolean,
    ) {
        if (roomId.isBlank() || items.isEmpty()) return
        val snapshotItems =
            if (isHydrated && items.none { it is MatrixTimelineItem.TimelineStart }) {
                items + MatrixTimelineItem.TimelineStart
            } else {
                items
            }

        val payload =
            runCatching {
                json.encodeToString(
                    ListSerializer(CachedTimelineItem.serializer()),
                    snapshotItems.map(CachedTimelineItem.Companion::fromDomain),
                )
            }.getOrNull() ?: return

        runCatching {
            db.roomTimelineCacheQueries.upsertSnapshot(
                room_id = roomId,
                payload_json = payload,
                updated_at = Clock.System.now().toEpochMilliseconds(),
            )
        }
    }

    override fun markHydrated(roomId: String) {
        // Hydration is derived from payload content (TimelineStart marker).
        // Keep this as no-op for schema backward compatibility.
    }

    override fun clear(roomId: String) {
        if (roomId.isBlank()) return
        runCatching { db.roomTimelineCacheQueries.deleteByRoomId(roomId) }
    }
}

@Serializable
private sealed interface CachedTimelineItem {
    fun toDomain(): MatrixTimelineItem

    @Serializable
    data class Event(
        val eventOrTransactionId: String,
        val eventId: String?,
        val senderId: String,
        val senderDisplayName: String?,
        val senderAvatarUrl: String?,
        val isMine: Boolean,
        val body: String,
        val timestampMillis: Long,
        val inReplyTo: CachedReplyDetails?,
        val reactions: List<CachedReaction>,
        val isEditable: Boolean,
        val sendStatus: CachedEventSendStatus?,
        val readByCount: Int,
        val canReply: Boolean,
    ) : CachedTimelineItem {
        override fun toDomain(): MatrixTimelineItem =
            MatrixTimelineItem.Event(
                eventOrTransactionId = eventOrTransactionId,
                eventId = eventId,
                senderId = senderId,
                senderDisplayName = senderDisplayName,
                senderAvatarUrl = senderAvatarUrl,
                isMine = isMine,
                body = body,
                timestampMillis = timestampMillis,
                inReplyTo = inReplyTo?.toDomain(),
                reactions = reactions.map(CachedReaction::toDomain),
                isEditable = isEditable,
                sendStatus = sendStatus?.toDomain(),
                readByCount = readByCount,
                canReply = canReply,
            )
    }

    @Serializable
    data class DateDivider(
        val timestampMillis: Long,
    ) : CachedTimelineItem {
        override fun toDomain(): MatrixTimelineItem = MatrixTimelineItem.DateDivider(timestampMillis = timestampMillis)
    }

    @Serializable
    data object ReadMarker : CachedTimelineItem {
        override fun toDomain(): MatrixTimelineItem = MatrixTimelineItem.ReadMarker
    }

    @Serializable
    data object TimelineStart : CachedTimelineItem {
        override fun toDomain(): MatrixTimelineItem = MatrixTimelineItem.TimelineStart
    }

    companion object {
        fun fromDomain(item: MatrixTimelineItem): CachedTimelineItem =
            when (item) {
                is MatrixTimelineItem.Event -> {
                    Event(
                        eventOrTransactionId = item.eventOrTransactionId,
                        eventId = item.eventId,
                        senderId = item.senderId,
                        senderDisplayName = item.senderDisplayName,
                        senderAvatarUrl = item.senderAvatarUrl,
                        isMine = item.isMine,
                        body = item.body,
                        timestampMillis = item.timestampMillis,
                        inReplyTo = item.inReplyTo?.toCached(),
                        reactions = item.reactions.map(MatrixReaction::toCached),
                        isEditable = item.isEditable,
                        sendStatus = item.sendStatus?.toCached(),
                        readByCount = item.readByCount,
                        canReply = item.canReply,
                    )
                }

                is MatrixTimelineItem.DateDivider -> {
                    DateDivider(timestampMillis = item.timestampMillis)
                }

                MatrixTimelineItem.ReadMarker -> {
                    ReadMarker
                }

                MatrixTimelineItem.TimelineStart -> {
                    TimelineStart
                }
            }
    }
}

@Serializable
private data class CachedReaction(
    val emoji: String,
    val count: Int,
    val reactedByMe: Boolean,
    val senders: List<CachedReactionSender>,
) {
    fun toDomain(): MatrixReaction =
        MatrixReaction(
            emoji = emoji,
            count = count,
            reactedByMe = reactedByMe,
            senders = senders.map(CachedReactionSender::toDomain),
        )
}

private fun MatrixReaction.toCached(): CachedReaction =
    CachedReaction(
        emoji = emoji,
        count = count,
        reactedByMe = reactedByMe,
        senders = senders.map(MatrixReactionSender::toCached),
    )

@Serializable
private data class CachedReactionSender(
    val senderId: String,
    val displayName: String?,
) {
    fun toDomain(): MatrixReactionSender =
        MatrixReactionSender(
            senderId = senderId,
            displayName = displayName,
        )
}

private fun MatrixReactionSender.toCached(): CachedReactionSender =
    CachedReactionSender(
        senderId = senderId,
        displayName = displayName,
    )

@Serializable
private data class CachedReplyDetails(
    val eventId: String,
    val senderDisplayName: String?,
    val body: String?,
) {
    fun toDomain(): MatrixReplyDetails =
        MatrixReplyDetails(
            eventId = eventId,
            senderDisplayName = senderDisplayName,
            body = body,
        )
}

private fun MatrixReplyDetails.toCached(): CachedReplyDetails =
    CachedReplyDetails(
        eventId = eventId,
        senderDisplayName = senderDisplayName,
        body = body,
    )

@Serializable
private enum class CachedEventSendStatus {
    Sending,
    Sent,
    Failed,
}

private fun CachedEventSendStatus.toDomain(): MatrixEventSendStatus =
    when (this) {
        CachedEventSendStatus.Sending -> MatrixEventSendStatus.Sending
        CachedEventSendStatus.Sent -> MatrixEventSendStatus.Sent
        CachedEventSendStatus.Failed -> MatrixEventSendStatus.Failed
    }

private fun MatrixEventSendStatus.toCached(): CachedEventSendStatus =
    when (this) {
        MatrixEventSendStatus.Sending -> CachedEventSendStatus.Sending
        MatrixEventSendStatus.Sent -> CachedEventSendStatus.Sent
        MatrixEventSendStatus.Failed -> CachedEventSendStatus.Failed
    }
