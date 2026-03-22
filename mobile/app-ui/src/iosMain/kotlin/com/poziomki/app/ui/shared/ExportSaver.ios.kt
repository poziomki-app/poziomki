package com.poziomki.app.ui.shared

import androidx.compose.runtime.Composable
import kotlinx.cinterop.ExperimentalForeignApi
import platform.Foundation.NSData
import platform.Foundation.NSTemporaryDirectory
import platform.Foundation.NSURL
import platform.Foundation.create
import platform.Foundation.writeToFile
import platform.UIKit.UIActivityViewController
import platform.UIKit.UIApplication

@Composable
actual fun rememberExportFileSaver(
    onSaved: () -> Unit,
    onCancelled: () -> Unit,
): (bytes: ByteArray, filename: String) -> Unit =
    { bytes, filename ->
        val saved = presentShareSheet(bytes, filename)
        if (saved) onSaved() else onCancelled()
    }

@OptIn(ExperimentalForeignApi::class)
private fun presentShareSheet(
    bytes: ByteArray,
    filename: String,
): Boolean =
    try {
        val tempDir = NSTemporaryDirectory()
        val filePath = "$tempDir/$filename"
        val data = bytes.toNSData()
        data.writeToFile(filePath, atomically = true)

        val fileUrl = NSURL.fileURLWithPath(filePath)
        val activityVC =
            UIActivityViewController(
                activityItems = listOf(fileUrl),
                applicationActivities = null,
            )

        val rootVC = UIApplication.sharedApplication.keyWindow?.rootViewController
        rootVC?.presentViewController(activityVC, animated = true, completion = null)
        true
    } catch (_: Exception) {
        false
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
