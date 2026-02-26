package com.poziomki.app.ui.shared

import androidx.compose.runtime.Composable

@Composable
actual fun rememberLocationPermissionLauncher(onResult: (Boolean) -> Unit): () -> Unit = {}
