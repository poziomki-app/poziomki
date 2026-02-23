package com.poziomki.app.ui.feature.home.messages

import com.poziomki.app.chat.matrix.api.MatrixRoomSummary
import com.poziomki.app.core.ids.matrixLocalpartFromUserId
import com.poziomki.app.network.MatchProfile

fun deduplicateRooms(rooms: List<MatrixRoomSummary>): List<MatrixRoomSummary> {
    val deduplicated = LinkedHashMap<String, MatrixRoomSummary>()

    rooms.forEach { room ->
        val key =
            if (room.isDirect) {
                room.directUserId
                    ?.trim()
                    ?.lowercase()
                    ?.ifBlank { null }
                    ?: room.roomId
            } else {
                room.roomId
            }

        val existing = deduplicated[key]
        if (existing == null) {
            deduplicated[key] = room
        } else {
            val roomTs = room.latestTimestampMillis ?: Long.MIN_VALUE
            val existingTs = existing.latestTimestampMillis ?: Long.MIN_VALUE
            if (roomTs > existingTs) {
                deduplicated[key] = room
            }
        }
    }

    return deduplicated.values.toList()
}

fun buildProfilePicturesByUserId(userIdToPic: Map<String, String>): Map<String, String> {
    val pictureMap = mutableMapOf<String, String>()
    userIdToPic.forEach { (userId, pic) ->
        val localpart = matrixLocalpartFromUserId(userId)
        pictureMap[userId] = pic
        pictureMap[userId.lowercase()] = pic
        pictureMap[localpart] = pic
        pictureMap["@$localpart"] = pic
    }
    return pictureMap
}

fun buildProfilePicturesByName(profiles: List<MatchProfile>): Map<String, String> =
    profiles
        .asSequence()
        .filter { !it.name.isBlank() && !it.profilePicture.isNullOrBlank() }
        .groupBy { it.name.trim().lowercase() }
        .mapNotNull { (name, sameNameProfiles) ->
            val uniquePictures =
                sameNameProfiles
                    .mapNotNull { it.profilePicture?.takeIf { picture -> picture.isNotBlank() } }
                    .distinct()
            if (uniquePictures.size == 1) {
                name to uniquePictures.first()
            } else {
                null
            }
        }.toMap()
