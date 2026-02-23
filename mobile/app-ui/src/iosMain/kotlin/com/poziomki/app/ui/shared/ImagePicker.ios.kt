package com.poziomki.app.ui.shared

import androidx.compose.runtime.Composable
import androidx.compose.ui.graphics.ImageBitmap

@Composable
actual fun rememberSingleImagePicker(onResult: (ByteArray?) -> Unit): () -> Unit = { /* TODO: iOS image picker */ }

@Composable
actual fun rememberMultiImagePicker(onResult: (List<ByteArray>) -> Unit): () -> Unit = { /* TODO: iOS image picker */ }

@Composable
actual fun rememberSingleFilePicker(onResult: (PickedFile?) -> Unit): () -> Unit = { /* TODO: iOS file picker */ }

actual fun decodeImageBytes(bytes: ByteArray): ImageBitmap? = null
