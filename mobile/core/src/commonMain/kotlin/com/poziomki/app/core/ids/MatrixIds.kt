package com.poziomki.app.core.ids

/**
 * Derives a Matrix localpart from a Poziomki user ID (UUID).
 * Mirrors backend `matrix_localpart_from_user_id` in `backend/src/api/matrix/service.rs`.
 */
fun matrixLocalpartFromUserId(userId: String): String {
    if (userId.startsWith("@")) return userId // already a Matrix ID
    val raw = userId.filter { it.isLetterOrDigit() }.lowercase()
    return if (raw.isEmpty()) "poziomki_user" else "poziomki_$raw"
}

@Suppress("ReturnCount", "MagicNumber", "ComplexCondition")
fun appUserIdFromMatrixUserId(matrixUserId: String): String? {
    val localpart = matrixUserId.substringAfter("@", "").substringBefore(":")
    if (!localpart.startsWith("poziomki_")) return null
    val hex = localpart.removePrefix("poziomki_")
    if (hex.length != 32 || hex.any { !it.isDigit() && (it !in 'a'..'f') && (it !in 'A'..'F') }) return null
    val normalized = hex.lowercase()
    return buildString(36) {
        append(normalized.substring(0, 8))
        append('-')
        append(normalized.substring(8, 12))
        append('-')
        append(normalized.substring(12, 16))
        append('-')
        append(normalized.substring(16, 20))
        append('-')
        append(normalized.substring(20, 32))
    }
}
