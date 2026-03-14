package com.poziomki.app.ui.feature.home.messages

import com.poziomki.app.chat.api.RoomSummary

fun List<RoomSummary>.filterMessagesRooms(
    selectedFilter: MessagesRoomFilter,
    searchQuery: String,
    eventRoomIds: Set<String>,
    searchMatchingRoomIds: Set<String>? = null,
): List<RoomSummary> {
    val normalizedQuery = searchQuery.trim().lowercase()

    return asSequence()
        .filter { room ->
            when (selectedFilter) {
                MessagesRoomFilter.All -> true
                MessagesRoomFilter.Direct -> room.isDirect
                MessagesRoomFilter.Events -> room.roomId in eventRoomIds
            }
        }.filter { room ->
            if (normalizedQuery.isBlank()) {
                true
            } else if (searchMatchingRoomIds != null) {
                room.roomId in searchMatchingRoomIds
            } else {
                room.displayName.lowercase().contains(normalizedQuery) ||
                    (room.latestMessage?.lowercase()?.contains(normalizedQuery) == true)
            }
        }.toList()
}
