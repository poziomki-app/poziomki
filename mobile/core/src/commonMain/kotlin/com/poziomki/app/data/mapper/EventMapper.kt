package com.poziomki.app.data.mapper

import com.poziomki.app.network.Event
import com.poziomki.app.network.EventAttendee
import com.poziomki.app.network.EventAttendeePreview
import com.poziomki.app.network.EventCreator
import com.poziomki.app.network.Tag
import kotlinx.datetime.Clock
import kotlinx.serialization.encodeToString
import kotlinx.serialization.json.Json

private val json = Json { ignoreUnknownKeys = true }

fun Event.toDbParams(): List<Any?> =
    listOf(
        id,
        title,
        description,
        coverImage,
        location,
        startsAt,
        endsAt,
        creatorId,
        creator?.name,
        creator?.profilePicture,
        attendeesCount.toLong(),
        if (isAttending) 1L else 0L,
        if (isSaved) 1L else 0L,
        json.encodeToString(attendeesPreview),
        json.encodeToString(tags),
        createdAt,
        conversationId,
        score,
        Clock.System.now().toEpochMilliseconds(),
        0L,
        latitude,
        longitude,
    )

fun com.poziomki.app.db.Event.toApiModel(): Event =
    Event(
        id = id,
        title = title,
        description = description,
        coverImage = cover_image,
        location = location,
        latitude = latitude,
        longitude = longitude,
        startsAt = starts_at,
        endsAt = ends_at,
        creatorId = creator_id,
        createdAt = created_at,
        attendeesCount = attendees_count.toInt(),
        isAttending = is_attending != 0L,
        isSaved = is_saved != 0L,
        creator =
            creator_id?.let {
                EventCreator(
                    id = it,
                    name = creator_name ?: "",
                    profilePicture = creator_profile_picture,
                )
            },
        attendeesPreview = parseAttendeesPreview(attendees_preview_json),
        tags = parseTags(tags_json),
        conversationId = conversation_id,
        score = score,
    )

private fun parseAttendeesPreview(jsonStr: String?): List<EventAttendeePreview> =
    if (jsonStr.isNullOrBlank()) {
        emptyList()
    } else {
        runCatching { json.decodeFromString<List<EventAttendeePreview>>(jsonStr) }
            .getOrDefault(emptyList())
    }

private fun parseTags(jsonStr: String?): List<Tag> =
    if (jsonStr.isNullOrBlank()) {
        emptyList()
    } else {
        runCatching { json.decodeFromString<List<Tag>>(jsonStr) }
            .getOrDefault(emptyList())
    }

fun EventAttendee.toDbParams(eventId: String): List<Any?> =
    listOf(
        eventId,
        profileId,
        userId,
        name,
        profilePicture,
        status,
    )

fun com.poziomki.app.db.Event_attendee.toApiModel(): EventAttendee =
    EventAttendee(
        profileId = profile_id,
        userId = user_id,
        name = name,
        profilePicture = profile_picture,
        status = status,
    )
