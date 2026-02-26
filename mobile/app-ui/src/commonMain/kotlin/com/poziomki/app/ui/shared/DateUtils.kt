package com.poziomki.app.ui.shared

import kotlinx.datetime.Clock
import kotlinx.datetime.Instant
import kotlinx.datetime.TimeZone
import kotlinx.datetime.toLocalDateTime

enum class TimeFilter {
    ALL,
    TODAY,
    TOMORROW,
    WEEK,
    NEARBY,
}

private val POLISH_WEEKDAYS = arrayOf("pon.", "wt.", "śr.", "czw.", "pt.", "sob.", "niedz.")
private val POLISH_WEEKDAYS_FULL =
    arrayOf("poniedziałek", "wtorek", "środa", "czwartek", "piątek", "sobota", "niedziela")
private val POLISH_MONTHS_GENITIVE =
    arrayOf(
        "stycznia",
        "lutego",
        "marca",
        "kwietnia",
        "maja",
        "czerwca",
        "lipca",
        "sierpnia",
        "września",
        "października",
        "listopada",
        "grudnia",
    )
private val POLISH_MONTHS =
    arrayOf(
        "sty",
        "lut",
        "mar",
        "kwi",
        "maj",
        "cze",
        "lip",
        "sie",
        "wrz",
        "paź",
        "lis",
        "gru",
    )

fun formatEventDate(isoString: String): String {
    val instant = Instant.parse(isoString)
    val dt = instant.toLocalDateTime(TimeZone.currentSystemDefault())
    val weekday = POLISH_WEEKDAYS[dt.dayOfWeek.ordinal]
    val day = dt.dayOfMonth
    val month = POLISH_MONTHS[dt.monthNumber - 1]
    val hour = dt.hour.toString().padStart(2, '0')
    val minute = dt.minute.toString().padStart(2, '0')
    return "$weekday, $day $month · $hour:$minute"
}

fun formatEventDateFull(isoString: String): String {
    val instant = Instant.parse(isoString)
    val dt = instant.toLocalDateTime(TimeZone.currentSystemDefault())
    val weekday = POLISH_WEEKDAYS_FULL[dt.dayOfWeek.ordinal]
    val day = dt.dayOfMonth
    val month = POLISH_MONTHS_GENITIVE[dt.monthNumber - 1]
    val hour = dt.hour.toString().padStart(2, '0')
    val minute = dt.minute.toString().padStart(2, '0')
    return "$weekday, $day $month · $hour:$minute"
}

fun pluralizePolish(
    count: Int,
    one: String,
    few: String,
    many: String,
): String {
    val abs = kotlin.math.abs(count)
    return when {
        abs == 1 -> "$count $one"
        abs % 10 in 2..4 && abs % 100 !in 12..14 -> "$count $few"
        else -> "$count $many"
    }
}

fun eventDateKey(startsAt: String): Int {
    val tz = TimeZone.currentSystemDefault()
    val eventDate = Instant.parse(startsAt).toLocalDateTime(tz).date
    return eventDate.toEpochDays()
}

fun dayLabel(startsAt: String): String {
    val tz = TimeZone.currentSystemDefault()
    val eventDate = Instant.parse(startsAt).toLocalDateTime(tz).date
    val today = Clock.System.now().toLocalDateTime(tz).date
    val daysDiff = eventDate.toEpochDays() - today.toEpochDays()
    return when (daysDiff) {
        0 -> "dzisiaj"
        1 -> "jutro"
        else -> POLISH_WEEKDAYS_FULL[eventDate.dayOfWeek.ordinal]
    }
}

fun matchesTimeFilter(
    startsAt: String,
    filter: TimeFilter,
): Boolean {
    if (filter == TimeFilter.ALL) return true
    val tz = TimeZone.currentSystemDefault()
    val eventInstant = Instant.parse(startsAt)
    val eventDate = eventInstant.toLocalDateTime(tz).date
    val now = Clock.System.now()
    val today = now.toLocalDateTime(tz).date
    return when (filter) {
        TimeFilter.ALL -> {
            true
        }

        TimeFilter.TODAY -> {
            eventDate == today
        }

        TimeFilter.TOMORROW -> {
            eventDate.toEpochDays() == today.toEpochDays() + 1
        }

        TimeFilter.WEEK -> {
            val daysDiff = eventDate.toEpochDays() - today.toEpochDays()
            daysDiff in 0..6
        }

        TimeFilter.NEARBY -> {
            true
        }
    }
}
