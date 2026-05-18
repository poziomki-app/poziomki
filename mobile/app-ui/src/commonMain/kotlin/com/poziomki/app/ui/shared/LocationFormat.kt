package com.poziomki.app.ui.shared

// Picks the first comma-separated segment that reads as a place name, falling
// back to a street address with the "ul. " / "ulica " prefix stripped.
// Examples:
//   "Plaża Wilanów, Aleja..."          -> "Plaża Wilanów"
//   "Elektrownia Powiśle, ul."         -> "Elektrownia Powiśle"
//   "ul. Koszykowa 86"                 -> "Koszykowa 86"
//   "pjatk, aula a1, ul. koszykowa 86" -> "pjatk"
fun formatEventLocation(raw: String): String {
    val segments = raw.split(",").map { it.trim() }.filter { it.isNotEmpty() }
    if (segments.isEmpty()) return raw.trim()
    val descriptive = segments.firstOrNull { !it.startsWithStreetPrefix() }
    if (descriptive != null) return descriptive
    return segments.first().stripStreetPrefix()
}

private fun String.startsWithStreetPrefix(): Boolean {
    val lower = lowercase()
    return lower.startsWith("ul.") || lower.startsWith("ul ") || lower.startsWith("ulica ")
}

private fun String.stripStreetPrefix(): String {
    val lower = lowercase()
    return when {
        lower.startsWith("ul. ") -> substring(4).trim()
        lower.startsWith("ul.") -> substring(3).trim()
        lower.startsWith("ul ") -> substring(3).trim()
        lower.startsWith("ulica ") -> substring(6).trim()
        else -> trim()
    }
}
