package com.poziomki.app.util

import org.koin.mp.KoinPlatform

private val apiBaseUrl: String by lazy {
    KoinPlatform.getKoin().getProperty("API_BASE_URL", "http://localhost:5150")
}

private val uploadFilenameRegex =
    Regex(
        "^[A-Za-z0-9][A-Za-z0-9._-]*\\.(jpeg|jpg|png|webp|avif)$",
        RegexOption.IGNORE_CASE,
    )
private const val HEX_BYTE_WIDTH = 2
private const val HEX_RADIX = 16
private const val MATRIX_THUMBNAIL_SIZE = 160

private fun hasSupportedImageScheme(value: String): Boolean =
    value.startsWith("https://") ||
        value.startsWith("http://") ||
        value.startsWith("mxc://") ||
        value.startsWith("content://") ||
        value.startsWith("file://")

private fun looksLikeUploadFilename(value: String): Boolean = uploadFilenameRegex.matches(value)

private fun matrixMediaHttpUrl(mxcUrl: String): String? {
    val withoutScheme = mxcUrl.removePrefix("mxc://")
    val slashIndex = withoutScheme.indexOf('/')
    val serverName = if (slashIndex > 0) withoutScheme.substring(0, slashIndex) else ""
    val mediaId = if (slashIndex in 1 until withoutScheme.lastIndex) withoutScheme.substring(slashIndex + 1) else ""
    return if (serverName.isBlank() || mediaId.isBlank()) {
        null
    } else {
        val encodedServerName = encodePathSegment(serverName)
        val encodedMediaId = encodePathSegment(mediaId)
        "https://$serverName/_matrix/client/v1/media/thumbnail/$encodedServerName/$encodedMediaId?" +
            "width=$MATRIX_THUMBNAIL_SIZE&height=$MATRIX_THUMBNAIL_SIZE&method=crop"
    }
}

private fun encodePathSegment(value: String): String {
    val sb = StringBuilder(value.length)
    value.forEach { ch ->
        val isUnreserved =
            (ch in 'A'..'Z') ||
                (ch in 'a'..'z') ||
                (ch in '0'..'9') ||
                ch == '-' ||
                ch == '_' ||
                ch == '.' ||
                ch == '~'
        if (isUnreserved) {
            sb.append(ch)
        } else {
            val hex =
                ch.code
                    .toString(HEX_RADIX)
                    .uppercase()
                    .padStart(HEX_BYTE_WIDTH, '0')
            sb.append('%').append(hex)
        }
    }
    return sb.toString()
}

fun resolveImageUrl(url: String): String {
    val normalized = url.trim()
    if (normalized.isEmpty()) return normalized
    return when {
        normalized.startsWith("/") -> "$apiBaseUrl$normalized"
        normalized.startsWith("mxc://") -> matrixMediaHttpUrl(normalized) ?: normalized
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
