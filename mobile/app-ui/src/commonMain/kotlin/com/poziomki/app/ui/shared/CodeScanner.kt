package com.poziomki.app.ui.shared

import androidx.compose.runtime.Composable

@Composable
expect fun rememberCodeScanner(onResult: (String?) -> Unit): () -> Unit
