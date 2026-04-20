package com.poziomki.app.ui.shared

import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.ui.platform.LocalContext
import com.google.mlkit.vision.barcode.common.Barcode
import com.google.mlkit.vision.codescanner.GmsBarcodeScannerOptions
import com.google.mlkit.vision.codescanner.GmsBarcodeScanning

@Composable
actual fun rememberCodeScanner(onResult: (String?) -> Unit): () -> Unit {
    val context = LocalContext.current
    val currentOnResult by rememberUpdatedState(onResult)
    return remember {
        {
            val options =
                GmsBarcodeScannerOptions
                    .Builder()
                    .setBarcodeFormats(Barcode.FORMAT_QR_CODE)
                    .enableAutoZoom()
                    .build()
            GmsBarcodeScanning
                .getClient(context, options)
                .startScan()
                .addOnSuccessListener { barcode -> currentOnResult(barcode.rawValue) }
                .addOnCanceledListener { currentOnResult(null) }
                .addOnFailureListener { currentOnResult(null) }
        }
    }
}
