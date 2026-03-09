package com.poziomki.app.ui.feature.home.messages

import com.poziomki.app.chat.matrix.api.MatrixRoomSummary
import com.poziomki.app.core.ids.appUserIdFromMatrixUserId

fun resolveRoomProfilePicture(
    room: MatrixRoomSummary,
    profilePictures: Map<String, String>,
    profilePicturesByName: Map<String, String>,
    eventRoomAvatars: Map<String, String> = emptyMap(),
): String? =
    eventRoomAvatars[room.roomId]
        ?: room.directUserId
            ?.let { directUserId ->
                val localpart = directUserId.substringAfter("@").substringBefore(":")
                val appUserId = appUserIdFromMatrixUserId(directUserId)
                listOfNotNull(
                    profilePictures[directUserId],
                    profilePictures[directUserId.substringBefore(":")],
                    profilePictures[localpart],
                    appUserId?.let { profilePictures[it] },
                ).firstOrNull()
            } ?: profilePicturesByName[room.displayName.trim().lowercase()]

fun resolveRoomDisplayName(
    room: MatrixRoomSummary,
    displayNameOverrides: Map<String, String>,
): String? {
    val directUserId = room.directUserId ?: return null
    val localpart = directUserId.substringAfter("@").substringBefore(":")
    val appUserId = appUserIdFromMatrixUserId(directUserId)
    return listOfNotNull(
        displayNameOverrides[directUserId],
        displayNameOverrides[directUserId.substringBefore(":")],
        displayNameOverrides[localpart],
        appUserId?.let { displayNameOverrides[it] },
    ).firstOrNull()
}
