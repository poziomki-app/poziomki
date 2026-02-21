package com.poziomki.app.ui.screen.main.messages

import com.poziomki.app.chat.matrix.api.MatrixRoomSummary

fun List<MatrixRoomSummary>.filterMessagesRooms(
    selectedFilter: MessagesRoomFilter,
    searchQuery: String,
    eventRoomIds: Set<String>,
): List<MatrixRoomSummary> {
    val normalizedQuery = searchQuery.trim().lowercase()

    return asSequence()
        .filter { room ->
            when (selectedFilter) {
                MessagesRoomFilter.Direct -> room.isDirect
                MessagesRoomFilter.Groups -> !room.isDirect && room.roomId !in eventRoomIds
                MessagesRoomFilter.Events -> !room.isDirect && room.roomId in eventRoomIds
            }
        }.filter { room ->
            if (normalizedQuery.isBlank()) {
                true
            } else {
                room.displayName.lowercase().contains(normalizedQuery) ||
                    (room.latestMessage?.lowercase()?.contains(normalizedQuery) == true)
            }
        }.toList()
}
