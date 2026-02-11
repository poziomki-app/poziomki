package com.poziomki.app.util

/**
 * Derives a Matrix localpart from a Poziomki user ID (UUID).
 * Mirrors backend `matrix_localpart_from_user_id` in `matrix_support.rs`.
 */
fun matrixLocalpartFromUserId(userId: String): String {
    if (userId.startsWith("@")) return userId // already a Matrix ID
    val raw = userId.filter { it.isLetterOrDigit() }.lowercase()
    return if (raw.isEmpty()) "poziomki_user" else "poziomki_$raw"
}
