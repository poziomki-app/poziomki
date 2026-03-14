package com.poziomki.app.ui.feature.home.messages

import com.poziomki.app.chat.api.RoomSummary

fun resolveRoomProfilePicture(
    room: RoomSummary,
    profilePicturesByName: Map<String, String>,
    eventRoomAvatars: Map<String, String> = emptyMap(),
): String? =
    eventRoomAvatars[room.roomId]
        ?: room.avatarUrl
        ?: profilePicturesByName[room.displayName.trim().lowercase()]
