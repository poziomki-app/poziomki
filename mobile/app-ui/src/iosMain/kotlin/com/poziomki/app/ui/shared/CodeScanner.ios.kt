package com.poziomki.app.ui.shared

import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember

@Composable
actual fun rememberCodeScanner(onResult: (String?) -> Unit): () -> Unit = remember { { onResult(null) } }
