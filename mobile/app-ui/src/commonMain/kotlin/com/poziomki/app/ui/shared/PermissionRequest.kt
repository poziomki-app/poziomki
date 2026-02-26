package com.poziomki.app.ui.shared

import androidx.compose.runtime.Composable

@Composable
expect fun rememberLocationPermissionLauncher(onResult: (Boolean) -> Unit): () -> Unit
