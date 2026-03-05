package com.poziomki.app.chat.cache

import com.poziomki.app.chat.matrix.api.MatrixTimelineItem

data class RoomTimelineCacheSnapshotData(
    val items: List<MatrixTimelineItem>,
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
        items: List<MatrixTimelineItem>,
        isHydrated: Boolean,
    )

    fun markHydrated(roomId: String)

    fun clear(roomId: String)
}
