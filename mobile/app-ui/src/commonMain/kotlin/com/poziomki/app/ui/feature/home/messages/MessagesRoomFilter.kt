package com.poziomki.app.ui.feature.home.messages

enum class MessagesRoomFilter {
    Direct,
    Events,
}

fun roomFilterTabs(): List<Pair<MessagesRoomFilter, String>> =
    listOf(
        MessagesRoomFilter.Direct to "znajomi",
        MessagesRoomFilter.Events to "wydarzenia",
    )
