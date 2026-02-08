package com.poziomki.app.util

import androidx.compose.runtime.Composable
import androidx.compose.ui.graphics.ImageBitmap

@Composable
expect fun rememberSingleImagePicker(onResult: (ByteArray?) -> Unit): () -> Unit

@Composable
expect fun rememberMultiImagePicker(onResult: (List<ByteArray>) -> Unit): () -> Unit

expect fun decodeImageBytes(bytes: ByteArray): ImageBitmap?
