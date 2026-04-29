package com.poziomki.app.ui.feature.home.messages

import com.poziomki.app.chat.api.RoomSummary

fun resolveRoomProfilePicture(
    room: RoomSummary,
    profilePicturesByName: Map<String, String>,
    eventRoomAvatars: Map<String, String> = emptyMap(),
): String? {
    val eventCover = eventRoomAvatars[room.roomId]
    // Non-direct rooms must never fall back to profilePicturesByName — that
    // map is built from user profiles, so an event whose title happens to
    // collide with someone's display name would leak that user's avatar
    // into the room. Use only the explicit event cover or the Matrix room
    // avatar that the server set on room creation.
    if (!room.isDirect) {
        return eventCover ?: room.avatarUrl
    }
    return eventCover ?: room.avatarUrl ?: profilePicturesByName[room.displayName.trim().lowercase()]
}
