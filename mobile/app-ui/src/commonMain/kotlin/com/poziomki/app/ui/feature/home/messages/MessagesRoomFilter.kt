package com.poziomki.app.ui.feature.home.messages

enum class MessagesRoomFilter {
    All,
    Direct,
    Events,
}

fun roomFilterTabs(): List<Pair<MessagesRoomFilter, String>> =
    listOf(
        MessagesRoomFilter.All to "wszystkie",
        MessagesRoomFilter.Direct to "znajomi",
        MessagesRoomFilter.Events to "wydarzenia",
    )
