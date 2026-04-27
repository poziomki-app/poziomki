package com.poziomki.app.data.mapper

import com.poziomki.app.db.Matched_profile
import com.poziomki.app.network.MatchProfile
import com.poziomki.app.network.Tag
import kotlinx.serialization.json.Json

private val json = Json { ignoreUnknownKeys = true }

fun Matched_profile.toApiModel(): MatchProfile =
    MatchProfile(
        id = id,
        userId = user_id,
        name = name,
        bio = bio,
        status = status,
        profilePicture = profile_picture,
        thumbhash = thumbhash,
        images = parseImages(images_json),
        program = program,
        gradientStart = gradient_start,
        gradientEnd = gradient_end,
        tags = parseTags(tags_json),
        score = score,
    )

private fun parseImages(jsonStr: String?): List<String> =
    if (jsonStr.isNullOrBlank()) {
        emptyList()
    } else {
        runCatching { json.decodeFromString<List<String>>(jsonStr) }
            .getOrDefault(emptyList())
    }

private fun parseTags(jsonStr: String?): List<Tag> =
    if (jsonStr.isNullOrBlank()) {
        emptyList()
    } else {
        runCatching { json.decodeFromString<List<Tag>>(jsonStr) }
            .getOrDefault(emptyList())
    }
