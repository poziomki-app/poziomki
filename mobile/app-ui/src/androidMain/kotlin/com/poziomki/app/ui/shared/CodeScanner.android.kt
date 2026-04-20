package com.poziomki.app.ui.shared

import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberUpdatedState

/**
 * Fully self-contained QR scanner. Launches [QrScannerActivity] — a portrait-locked
 * Compose screen using CameraX preview + ZXing-core decoder. No Play Services.
 */
@Composable
actual fun rememberCodeScanner(onResult: (String?) -> Unit): () -> Unit {
    val currentOnResult by rememberUpdatedState(onResult)
    val launcher =
        rememberLauncherForActivityResult(QrScanContract()) { token ->
            currentOnResult(token)
        }
    return remember {
        { launcher.launch(Unit) }
    }
}
