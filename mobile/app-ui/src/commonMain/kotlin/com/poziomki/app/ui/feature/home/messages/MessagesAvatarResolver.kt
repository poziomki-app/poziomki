package com.poziomki.app.ui.feature.home.messages

import com.poziomki.app.chat.matrix.api.MatrixRoomSummary
import com.poziomki.app.core.ids.appUserIdFromMatrixUserId

fun resolveRoomProfilePicture(
    room: MatrixRoomSummary,
    profilePictures: Map<String, String>,
    profilePicturesByName: Map<String, String>,
): String? =
    room.directUserId
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
