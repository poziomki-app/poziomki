package com.poziomki.app.chat.cache

import com.poziomki.app.chat.api.TimelineItem

data class RoomTimelineCacheSnapshotData(
    val items: List<TimelineItem>,
    val isHydrated: Boolean,
    val cachedItemCount: Int,
    val updatedAtMillis: Long,
)

interface RoomTimelineCacheStore {
    fun loadSnapshot(
        roomId: String,
        limit: Int = 500,
    ): RoomTimelineCacheSnapshotData

    fun saveSnapshot(
        roomId: String,
        items: List<TimelineItem>,
        isHydrated: Boolean,
    )

    fun markHydrated(roomId: String)

    fun clear(roomId: String)

    fun clearAll()
}
