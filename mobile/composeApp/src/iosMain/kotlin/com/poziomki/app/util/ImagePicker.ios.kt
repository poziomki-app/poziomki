package com.poziomki.app.util

import androidx.compose.runtime.Composable
import androidx.compose.ui.graphics.ImageBitmap

@Composable
actual fun rememberSingleImagePicker(onResult: (ByteArray?) -> Unit): () -> Unit = { /* TODO: iOS image picker */ }

@Composable
actual fun rememberMultiImagePicker(onResult: (List<ByteArray>) -> Unit): () -> Unit = { /* TODO: iOS image picker */ }

actual fun decodeImageBytes(bytes: ByteArray): ImageBitmap? = null
