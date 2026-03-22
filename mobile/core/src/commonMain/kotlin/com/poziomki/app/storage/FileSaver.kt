package com.poziomki.app.storage

interface FileSaver {
    suspend fun saveToDownloads(
        bytes: ByteArray,
        filename: String,
        mimeType: String,
    ): Boolean
}
