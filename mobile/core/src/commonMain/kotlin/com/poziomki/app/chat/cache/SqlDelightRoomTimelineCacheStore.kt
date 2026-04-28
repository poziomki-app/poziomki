package com.poziomki.app.chat.cache

import com.poziomki.app.chat.api.EventSendStatus
import com.poziomki.app.chat.api.Reaction
import com.poziomki.app.chat.api.ReactionSender
import com.poziomki.app.chat.api.ReplyDetails
import com.poziomki.app.chat.api.TimelineItem
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
                // cache corrupted, clearing
                runCatching { db.roomTimelineCacheQueries.deleteByRoomId(roomId) }
                emptyList()
            }

        val snapshot = decodedItems.takeLast(limit)
        return RoomTimelineCacheSnapshotData(
            items = snapshot,
            isHydrated = decodedItems.any { it is TimelineItem.TimelineStart },
            cachedItemCount = decodedItems.size,
            updatedAtMillis = row.updated_at,
        )
    }

    override fun saveSnapshot(
        roomId: String,
        items: List<TimelineItem>,
        isHydrated: Boolean,
    ) {
        if (roomId.isBlank() || items.isEmpty()) return
        val snapshotItems =
            if (isHydrated && items.none { it is TimelineItem.TimelineStart }) {
                items + TimelineItem.TimelineStart
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

    override fun clearAll() {
        runCatching { db.roomTimelineCacheQueries.deleteAll() }
    }
}

@Serializable
private sealed interface CachedTimelineItem {
    fun toDomain(): TimelineItem

    @Serializable
    data class Event(
        val eventOrTransactionId: String,
        val eventId: String?,
        val senderId: String,
        val senderPid: String? = null,
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
        val moderationVerdict: String? = null,
        val moderationCategories: List<String> = emptyList(),
        val locallyRevealed: Boolean = false,
        val locallyReported: Boolean = false,
    ) : CachedTimelineItem {
        override fun toDomain(): TimelineItem =
            TimelineItem.Event(
                eventOrTransactionId = eventOrTransactionId,
                eventId = eventId,
                senderId = senderId,
                senderPid = senderPid,
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
                moderationVerdict = moderationVerdict,
                moderationCategories = moderationCategories,
                locallyRevealed = locallyRevealed,
                locallyReported = locallyReported,
            )
    }

    @Serializable
    data class DateDivider(
        val timestampMillis: Long,
    ) : CachedTimelineItem {
        override fun toDomain(): TimelineItem = TimelineItem.DateDivider(timestampMillis = timestampMillis)
    }

    @Serializable
    data object ReadMarker : CachedTimelineItem {
        override fun toDomain(): TimelineItem = TimelineItem.ReadMarker
    }

    @Serializable
    data object TimelineStart : CachedTimelineItem {
        override fun toDomain(): TimelineItem = TimelineItem.TimelineStart
    }

    companion object {
        fun fromDomain(item: TimelineItem): CachedTimelineItem =
            when (item) {
                is TimelineItem.Event -> {
                    Event(
                        eventOrTransactionId = item.eventOrTransactionId,
                        eventId = item.eventId,
                        senderId = item.senderId,
                        senderPid = item.senderPid,
                        senderDisplayName = item.senderDisplayName,
                        senderAvatarUrl = item.senderAvatarUrl,
                        isMine = item.isMine,
                        body = item.body,
                        timestampMillis = item.timestampMillis,
                        inReplyTo = item.inReplyTo?.toCached(),
                        reactions = item.reactions.map(Reaction::toCached),
                        isEditable = item.isEditable,
                        sendStatus = item.sendStatus?.toCached(),
                        readByCount = item.readByCount,
                        canReply = item.canReply,
                        moderationVerdict = item.moderationVerdict,
                        moderationCategories = item.moderationCategories,
                        locallyRevealed = item.locallyRevealed,
                        locallyReported = item.locallyReported,
                    )
                }

                is TimelineItem.DateDivider -> {
                    DateDivider(timestampMillis = item.timestampMillis)
                }

                TimelineItem.ReadMarker -> {
                    ReadMarker
                }

                TimelineItem.TimelineStart -> {
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
    fun toDomain(): Reaction =
        Reaction(
            emoji = emoji,
            count = count,
            reactedByMe = reactedByMe,
            senders = senders.map(CachedReactionSender::toDomain),
        )
}

private fun Reaction.toCached(): CachedReaction =
    CachedReaction(
        emoji = emoji,
        count = count,
        reactedByMe = reactedByMe,
        senders = senders.map(ReactionSender::toCached),
    )

@Serializable
private data class CachedReactionSender(
    val senderId: String,
    val displayName: String?,
) {
    fun toDomain(): ReactionSender =
        ReactionSender(
            senderId = senderId,
            displayName = displayName,
        )
}

private fun ReactionSender.toCached(): CachedReactionSender =
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
    fun toDomain(): ReplyDetails =
        ReplyDetails(
            eventId = eventId,
            senderDisplayName = senderDisplayName,
            body = body,
        )
}

private fun ReplyDetails.toCached(): CachedReplyDetails =
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

private fun CachedEventSendStatus.toDomain(): EventSendStatus =
    when (this) {
        CachedEventSendStatus.Sending -> EventSendStatus.Sending
        CachedEventSendStatus.Sent -> EventSendStatus.Sent
        CachedEventSendStatus.Failed -> EventSendStatus.Failed
    }

private fun EventSendStatus.toCached(): CachedEventSendStatus =
    when (this) {
        EventSendStatus.Sending -> CachedEventSendStatus.Sending
        EventSendStatus.Sent -> CachedEventSendStatus.Sent
        EventSendStatus.Failed -> CachedEventSendStatus.Failed
    }
