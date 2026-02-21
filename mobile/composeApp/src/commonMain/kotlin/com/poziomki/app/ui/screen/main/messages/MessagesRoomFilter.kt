package com.poziomki.app.ui.screen.main.messages

enum class MessagesRoomFilter {
    Direct,
    Groups,
    Events,
}

fun roomFilterTabs(): List<Pair<MessagesRoomFilter, String>> =
    listOf(
        MessagesRoomFilter.Direct to "znajomi",
        MessagesRoomFilter.Groups to "grupy",
        MessagesRoomFilter.Events to "wydarzenia",
    )
