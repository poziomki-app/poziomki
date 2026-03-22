package com.poziomki.app.storage

import kotlinx.cinterop.ExperimentalForeignApi
import platform.Foundation.NSData
import platform.Foundation.NSDocumentDirectory
import platform.Foundation.NSFileManager
import platform.Foundation.NSSearchPathForDirectoriesInDomains
import platform.Foundation.NSUserDomainMask
import platform.Foundation.create

class IosFileSaver : FileSaver {
    @OptIn(ExperimentalForeignApi::class)
    override suspend fun saveToDownloads(
        bytes: ByteArray,
        filename: String,
        mimeType: String,
    ): Boolean =
        try {
            val paths =
                NSSearchPathForDirectoriesInDomains(
                    NSDocumentDirectory,
                    NSUserDomainMask,
                    true,
                )
            val documentsDir = paths.firstOrNull() as? String ?: return false
            val safe = sanitizeFilename(filename)
            val filePath = "$documentsDir/$safe"
            val data = bytes.toNSData()
            data.writeToFile(filePath, atomically = true)
        } catch (_: Exception) {
            false
        }
}

@OptIn(ExperimentalForeignApi::class)
private fun ByteArray.toNSData(): NSData =
    if (isEmpty()) {
        NSData()
    } else {
        kotlinx.cinterop.usePinned { pinned ->
            NSData.create(
                bytes = pinned.addressOf(0),
                length = size.toULong(),
            )
        }
    }
