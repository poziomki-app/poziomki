package com.poziomki.app.util

import android.graphics.Bitmap
import android.graphics.BitmapFactory
import android.net.Uri
import android.provider.OpenableColumns
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.PickVisualMediaRequest
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.runtime.Composable
import androidx.compose.ui.graphics.ImageBitmap
import androidx.compose.ui.graphics.asImageBitmap
import androidx.compose.ui.platform.LocalContext
import java.io.ByteArrayOutputStream
import kotlin.math.sqrt

private const val DEFAULT_MAX_DIMENSION = 1920
private const val DEFAULT_JPEG_QUALITY = 85
private const val DEFAULT_MAX_BYTES = 700 * 1024
private const val MIN_JPEG_QUALITY = 50
private const val QUALITY_STEP = 10
private const val MIN_PIXEL_DIMENSION = 1

@Composable
actual fun rememberSingleImagePicker(onResult: (ByteArray?) -> Unit): () -> Unit {
    val context = LocalContext.current
    val launcher =
        rememberLauncherForActivityResult(
            ActivityResultContracts.PickVisualMedia(),
        ) { uri: Uri? ->
            val bytes = uri?.let { compressImage(context, it) }
            onResult(bytes)
        }
    return {
        launcher.launch(PickVisualMediaRequest(ActivityResultContracts.PickVisualMedia.ImageOnly))
    }
}

@Composable
actual fun rememberMultiImagePicker(onResult: (List<ByteArray>) -> Unit): () -> Unit {
    val context = LocalContext.current
    val launcher =
        rememberLauncherForActivityResult(
            ActivityResultContracts.PickMultipleVisualMedia(6),
        ) { uris: List<Uri> ->
            val bytes = uris.mapNotNull { uri -> compressImage(context, uri) }
            onResult(bytes)
        }
    return {
        launcher.launch(PickVisualMediaRequest(ActivityResultContracts.PickVisualMedia.ImageOnly))
    }
}

@Composable
actual fun rememberSingleFilePicker(onResult: (PickedFile?) -> Unit): () -> Unit {
    val context = LocalContext.current
    val launcher =
        rememberLauncherForActivityResult(
            ActivityResultContracts.GetContent(),
        ) { uri: Uri? ->
            if (uri == null) {
                onResult(null)
                return@rememberLauncherForActivityResult
            }
            val bytes =
                runCatching {
                    context.contentResolver.openInputStream(uri)?.use { it.readBytes() }
                }.getOrNull()
            if (bytes == null) {
                onResult(null)
                return@rememberLauncherForActivityResult
            }
            val fileName = readDisplayName(context, uri) ?: "attachment"
            onResult(PickedFile(name = fileName, bytes = bytes, mimeType = context.contentResolver.getType(uri)))
        }
    return {
        launcher.launch("*/*")
    }
}

actual fun decodeImageBytes(bytes: ByteArray): ImageBitmap? = BitmapFactory.decodeByteArray(bytes, 0, bytes.size)?.asImageBitmap()

private fun compressImage(
    context: android.content.Context,
    uri: Uri,
    maxDimension: Int = DEFAULT_MAX_DIMENSION,
    quality: Int = DEFAULT_JPEG_QUALITY,
    maxBytes: Int = DEFAULT_MAX_BYTES,
): ByteArray? {
    return try {
        // Decode bounds first
        val options = BitmapFactory.Options().apply { inJustDecodeBounds = true }
        context.contentResolver.openInputStream(uri)?.use {
            BitmapFactory.decodeStream(it, null, options)
        }

        // Calculate sample size
        val width = options.outWidth
        val height = options.outHeight
        var sampleSize = 1
        while (width / sampleSize > maxDimension || height / sampleSize > maxDimension) {
            sampleSize *= 2
        }

        // Decode with sample size
        val decodeOptions = BitmapFactory.Options().apply { inSampleSize = sampleSize }
        val bitmap =
            context.contentResolver.openInputStream(uri)?.use {
                BitmapFactory.decodeStream(it, null, decodeOptions)
            } ?: return null

        // Compress to JPEG and enforce a byte ceiling to keep memory footprint bounded.
        val output = ByteArrayOutputStream()
        var currentQuality = quality
        bitmap.compress(Bitmap.CompressFormat.JPEG, currentQuality, output)
        while (output.size() > maxBytes && currentQuality > MIN_JPEG_QUALITY) {
            currentQuality -= QUALITY_STEP
            output.reset()
            bitmap.compress(Bitmap.CompressFormat.JPEG, currentQuality, output)
        }

        if (output.size() > maxBytes) {
            val scale = sqrt(maxBytes.toDouble() / output.size().toDouble()).coerceAtMost(1.0)
            val resizedWidth = (bitmap.width * scale).toInt().coerceAtLeast(MIN_PIXEL_DIMENSION)
            val resizedHeight = (bitmap.height * scale).toInt().coerceAtLeast(MIN_PIXEL_DIMENSION)
            val resizedBitmap = Bitmap.createScaledBitmap(bitmap, resizedWidth, resizedHeight, true)
            output.reset()
            resizedBitmap.compress(Bitmap.CompressFormat.JPEG, currentQuality, output)
            resizedBitmap.recycle()
        }

        bitmap.recycle()
        output.toByteArray()
    } catch (_: Exception) {
        null
    }
}

private fun readDisplayName(
    context: android.content.Context,
    uri: Uri,
): String? =
    runCatching {
        context.contentResolver.query(uri, arrayOf(OpenableColumns.DISPLAY_NAME), null, null, null)?.use { cursor ->
            val index = cursor.getColumnIndex(OpenableColumns.DISPLAY_NAME)
            if (index == -1) return@use null
            if (!cursor.moveToFirst()) return@use null
            cursor.getString(index)
        }
    }.getOrNull()
