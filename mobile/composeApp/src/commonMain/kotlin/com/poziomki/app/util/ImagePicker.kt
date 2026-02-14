package com.poziomki.app.util

import androidx.compose.runtime.Composable
import androidx.compose.ui.graphics.ImageBitmap

data class PickedFile(
    val name: String,
    val bytes: ByteArray,
    val mimeType: String? = null,
)

@Composable
expect fun rememberSingleImagePicker(onResult: (ByteArray?) -> Unit): () -> Unit

@Composable
expect fun rememberMultiImagePicker(onResult: (List<ByteArray>) -> Unit): () -> Unit

@Composable
expect fun rememberSingleFilePicker(onResult: (PickedFile?) -> Unit): () -> Unit

expect fun decodeImageBytes(bytes: ByteArray): ImageBitmap?
