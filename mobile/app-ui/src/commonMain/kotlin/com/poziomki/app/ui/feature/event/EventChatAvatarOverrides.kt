package com.poziomki.app.ui.feature.event

import com.poziomki.app.network.EventAttendee

fun buildEventAvatarOverrides(attendees: List<EventAttendee>): Map<String, String> =
    buildMap {
        attendees
            .asSequence()
            .filter { !it.userId.isNullOrBlank() && !it.profilePicture.isNullOrBlank() }
            .forEach { attendee ->
                val userId = attendee.userId!!
                val picture = attendee.profilePicture!!
                val normalized = userId.filter { it.isLetterOrDigit() }.lowercase()
                val plain = userId.lowercase()
                val withAt = if (plain.startsWith("@")) plain else "@$plain"
                val localpart = withAt.removePrefix("@").substringBefore(":")
                val uuid = localpart.removePrefix("poziomki_")

                put(normalized, picture)
                put(plain, picture)
                put(withAt, picture)
                put(localpart, picture)
                put(uuid, picture)
                put(uuid.lowercase(), picture)
            }
    }
