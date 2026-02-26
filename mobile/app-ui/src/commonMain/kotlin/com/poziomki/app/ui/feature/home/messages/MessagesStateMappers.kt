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

fun buildDisplayNameOverrides(profiles: List<MatchProfile>): Map<String, String> {
    val nameMap = mutableMapOf<String, String>()
    profiles.forEach { profile ->
        val name = profile.name.trim()
        if (name.isBlank()) return@forEach
        val userId = profile.userId
        val localpart = matrixLocalpartFromUserId(userId)
        nameMap[userId] = name
        nameMap[userId.lowercase()] = name
        nameMap[localpart] = name
        nameMap["@$localpart"] = name
    }
    return nameMap
}

fun buildProfilePicturesByName(profiles: List<MatchProfile>): Map<String, String> =
    profiles
        .asSequence()
        .filter { !it.name.isBlank() }
        .groupBy { it.name.trim().lowercase() }
        .mapNotNull { (name, sameNameProfiles) ->
            val allPictures = sameNameProfiles.map { it.profilePicture?.takeIf { p -> p.isNotBlank() } }
            if (allPictures.any { it == null }) return@mapNotNull null
            val uniquePictures = allPictures.filterNotNull().distinct()
            if (uniquePictures.size == 1) {
                name to uniquePictures.first()
            } else {
                null
            }
        }.toMap()
