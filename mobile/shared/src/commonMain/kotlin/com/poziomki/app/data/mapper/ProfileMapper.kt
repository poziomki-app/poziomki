package com.poziomki.app.data.mapper

import com.poziomki.app.api.Profile
import com.poziomki.app.api.ProfileWithTags
import com.poziomki.app.api.Tag
import kotlinx.datetime.Clock
import kotlinx.serialization.encodeToString
import kotlinx.serialization.json.Json

private val json = Json { ignoreUnknownKeys = true }

fun Profile.toDbParams(isOwn: Boolean): List<Any?> =
    listOf(
        id,
        userId,
        name,
        bio,
        age.toLong(),
        profilePicture,
        json.encodeToString(images),
        program,
        if (isOwn) 1L else 0L,
        createdAt,
        updatedAt,
        Clock.System.now().toEpochMilliseconds(),
        0L,
    )

fun ProfileWithTags.toProfile(): Profile =
    Profile(
        id = id,
        userId = userId,
        name = name,
        bio = bio,
        age = age,
        profilePicture = profilePicture,
        images = images,
        program = program,
    )

fun com.poziomki.app.db.Profile.toApiModel(): Profile =
    Profile(
        id = id,
        userId = user_id,
        name = name,
        bio = bio,
        age = age.toInt(),
        profilePicture = profile_picture,
        images = parseImages(images_json),
        program = program,
        createdAt = created_at,
        updatedAt = updated_at,
    )

fun com.poziomki.app.db.Profile.toApiModelWithTags(tags: List<Tag>): ProfileWithTags =
    ProfileWithTags(
        id = id,
        userId = user_id,
        name = name,
        bio = bio,
        age = age.toInt(),
        profilePicture = profile_picture,
        images = parseImages(images_json),
        program = program,
        tags = tags,
    )

private fun parseImages(jsonStr: String?): List<String> =
    if (jsonStr.isNullOrBlank()) {
        emptyList()
    } else {
        runCatching { json.decodeFromString<List<String>>(jsonStr) }
            .getOrDefault(emptyList())
    }
