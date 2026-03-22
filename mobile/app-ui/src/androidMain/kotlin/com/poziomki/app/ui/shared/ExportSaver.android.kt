package com.poziomki.app.ui.shared

import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.platform.LocalContext

@Composable
actual fun rememberExportFileSaver(
    onSaved: () -> Unit,
    onCancelled: () -> Unit,
): (bytes: ByteArray, filename: String) -> Unit {
    val context = LocalContext.current
    var pendingBytes by remember { mutableStateOf<ByteArray?>(null) }

    val launcher =
        rememberLauncherForActivityResult(
            ActivityResultContracts.CreateDocument("application/zip"),
        ) { uri ->
            val bytes = pendingBytes
            pendingBytes = null
            if (uri != null && bytes != null) {
                val written =
                    runCatching {
                        context.contentResolver.openOutputStream(uri)?.use { it.write(bytes) }
                    }.isSuccess
                if (written) onSaved() else onCancelled()
            } else {
                onCancelled()
            }
        }

    return { bytes, filename ->
        pendingBytes = bytes
        launcher.launch(filename)
    }
}
