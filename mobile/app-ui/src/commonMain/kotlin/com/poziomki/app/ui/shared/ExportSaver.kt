package com.poziomki.app.ui.shared

import androidx.compose.runtime.Composable

@Composable
expect fun rememberExportFileSaver(
    onSaved: () -> Unit,
    onCancelled: () -> Unit,
): (bytes: ByteArray, filename: String) -> Unit
