package com.poziomki.app.storage

interface FileSaver {
    suspend fun saveToDownloads(
        bytes: ByteArray,
        filename: String,
        mimeType: String,
    ): Boolean
}

/** Strip path separators so the filename can never escape the target directory. */
fun sanitizeFilename(filename: String): String {
    val base = filename.substringAfterLast('/').substringAfterLast('\\')
    return base.ifBlank { "export" }
}
