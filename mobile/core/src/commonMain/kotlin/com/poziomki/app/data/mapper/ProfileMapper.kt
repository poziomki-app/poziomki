package com.poziomki.app.data.mapper

import com.poziomki.app.network.Profile
import com.poziomki.app.network.ProfileWithTags
import com.poziomki.app.network.Tag
import kotlinx.serialization.json.Json

private val json = Json { ignoreUnknownKeys = true }

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
        gradientStart = gradientStart,
        gradientEnd = gradientEnd,
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
        gradientStart = gradient_start,
        gradientEnd = gradient_end,
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
        gradientStart = gradient_start,
        gradientEnd = gradient_end,
        tags = tags,
    )

private fun parseImages(jsonStr: String?): List<String> =
    if (jsonStr.isNullOrBlank()) {
        emptyList()
    } else {
        runCatching { json.decodeFromString<List<String>>(jsonStr) }
            .getOrDefault(emptyList())
    }
