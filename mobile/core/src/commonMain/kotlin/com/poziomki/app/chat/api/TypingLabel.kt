package com.poziomki.app.chat.api

/**
 * Render a localized "X is typing…" label from a set of typing users.
 *
 * Behavior matches Signal/iMessage for groups:
 *   * 0 → null (caller hides indicator)
 *   * 1 → "Anna pisze…"
 *   * 2 → "Anna i Jan piszą…"
 *   * 3+ → "Kilka osób pisze…"
 *
 * `nameOf` resolves user IDs to display names; the caller is
 * expected to supply a member-cache lookup. Users without a
 * resolvable name fall back to "ktoś" so we never render a raw ID.
 */
fun typingLabel(
    typingUserIds: Set<String>,
    nameOf: (String) -> String?,
): String? {
    if (typingUserIds.isEmpty()) return null
    val resolved =
        typingUserIds
            .map { id -> nameOf(id)?.takeIf { it.isNotBlank() } ?: "ktoś" }
            .toList()
    return when (resolved.size) {
        1 -> "${resolved[0]} pisze…"
        2 -> "${resolved[0]} i ${resolved[1]} piszą…"
        else -> "Kilka osób pisze…"
    }
}
