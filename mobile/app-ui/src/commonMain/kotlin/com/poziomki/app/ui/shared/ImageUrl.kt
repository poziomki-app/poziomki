package com.poziomki.app.ui.shared

import org.koin.mp.KoinPlatform

private val apiBaseUrl: String by lazy {
    KoinPlatform.getKoin().getProperty("API_BASE_URL", "http://localhost:5150")
}

private val uploadFilenameRegex =
    Regex(
        "^[A-Za-z0-9][A-Za-z0-9._-]*\\.(jpeg|jpg|png|webp|avif)$",
        RegexOption.IGNORE_CASE,
    )

private fun hasSupportedImageScheme(value: String): Boolean =
    value.startsWith("https://") ||
        value.startsWith("http://") ||
        value.startsWith("mxc://") ||
        value.startsWith("content://") ||
        value.startsWith("file://")

private fun looksLikeUploadFilename(value: String): Boolean = uploadFilenameRegex.matches(value)

fun resolveImageUrl(url: String): String {
    val normalized = url.trim()
    if (normalized.isEmpty()) return normalized
    return when {
        normalized.startsWith("/") -> "$apiBaseUrl$normalized"
        normalized.startsWith("mxc://") -> normalized
        hasSupportedImageScheme(normalized) -> normalized
        looksLikeUploadFilename(normalized) -> "$apiBaseUrl/api/v1/uploads/$normalized"
        else -> normalized
    }
}

fun isImageUrl(value: String): Boolean {
    val normalized = value.trim()
    if (normalized.isEmpty()) return false
    return normalized.startsWith("/") || hasSupportedImageScheme(normalized) || looksLikeUploadFilename(normalized)
}
