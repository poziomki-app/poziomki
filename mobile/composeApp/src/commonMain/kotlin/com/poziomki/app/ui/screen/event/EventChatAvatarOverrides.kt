package com.poziomki.app.ui.screen.event

import com.poziomki.app.api.EventAttendee

fun buildEventAvatarOverrides(attendees: List<EventAttendee>): Map<String, String> =
    attendees
        .asSequence()
        .filter { !it.userId.isNullOrBlank() && !it.profilePicture.isNullOrBlank() }
        .associate { attendee ->
            val key =
                attendee.userId!!
                    .filter { it.isLetterOrDigit() }
                    .lowercase()
            key to attendee.profilePicture!!
        }
