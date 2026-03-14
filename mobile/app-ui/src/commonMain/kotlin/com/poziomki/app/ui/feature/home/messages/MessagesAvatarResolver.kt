package com.poziomki.app.ui.feature.home.messages

import com.poziomki.app.chat.api.RoomSummary

fun resolveRoomProfilePicture(
    room: RoomSummary,
    profilePictures: Map<String, String>,
    profilePicturesByName: Map<String, String>,
    eventRoomAvatars: Map<String, String> = emptyMap(),
): String? =
    eventRoomAvatars[room.roomId]
        ?: room.directUserId
            ?.let { directUserId ->
                profilePictures[directUserId]
                    ?: profilePictures[directUserId.lowercase()]
            } ?: profilePicturesByName[room.displayName.trim().lowercase()]

fun resolveRoomDisplayName(
    room: RoomSummary,
    displayNameOverrides: Map<String, String>,
): String? {
    val directUserId = room.directUserId ?: return null
    return displayNameOverrides[directUserId]
        ?: displayNameOverrides[directUserId.lowercase()]
}
