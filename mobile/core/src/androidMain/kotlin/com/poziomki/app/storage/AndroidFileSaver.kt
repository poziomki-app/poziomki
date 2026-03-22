package com.poziomki.app.storage

import android.content.ContentValues
import android.content.Context
import android.os.Build
import android.os.Environment
import android.provider.MediaStore
import java.io.File

class AndroidFileSaver(
    private val context: Context,
) : FileSaver {
    override suspend fun saveToDownloads(
        bytes: ByteArray,
        filename: String,
        mimeType: String,
    ): Boolean =
        try {
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q) {
                saveViaMediaStore(bytes, filename, mimeType)
            } else {
                saveToLegacyDownloads(bytes, filename)
            }
        } catch (_: Exception) {
            false
        }

    private fun saveViaMediaStore(
        bytes: ByteArray,
        filename: String,
        mimeType: String,
    ): Boolean {
        val contentValues =
            ContentValues().apply {
                put(MediaStore.Downloads.DISPLAY_NAME, filename)
                put(MediaStore.Downloads.MIME_TYPE, mimeType)
                put(MediaStore.Downloads.RELATIVE_PATH, Environment.DIRECTORY_DOWNLOADS)
            }

        val uri =
            context.contentResolver.insert(
                MediaStore.Downloads.EXTERNAL_CONTENT_URI,
                contentValues,
            ) ?: return false

        context.contentResolver.openOutputStream(uri)?.use { it.write(bytes) } ?: return false
        return true
    }

    @Suppress("DEPRECATION")
    private fun saveToLegacyDownloads(
        bytes: ByteArray,
        filename: String,
    ): Boolean {
        val downloadsDir =
            Environment.getExternalStoragePublicDirectory(Environment.DIRECTORY_DOWNLOADS)
        val file = File(downloadsDir, filename)
        file.writeBytes(bytes)
        return true
    }
}
