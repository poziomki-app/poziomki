package com.poziomki.app.ui.shared

import android.app.Activity
import android.content.Context
import android.content.Intent
import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.result.contract.ActivityResultContract

/**
 * Portrait-locked, Compose-based QR scanner activity. No Play Services, no ZXing Activity.
 * CameraX preview + ZXing-core frame decoder.
 *
 * Launch via [QrScanContract] from any Compose screen using `rememberLauncherForActivityResult`.
 */
class QrScannerActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContent {
            QrScannerScreen(
                onResult = { token ->
                    setResult(Activity.RESULT_OK, Intent().putExtra(EXTRA_TOKEN, token))
                    finish()
                },
                onCancel = {
                    setResult(Activity.RESULT_CANCELED)
                    finish()
                },
            )
        }
    }

    companion object {
        const val EXTRA_TOKEN = "token"
    }
}

class QrScanContract : ActivityResultContract<Unit, String?>() {
    override fun createIntent(
        context: Context,
        input: Unit,
    ): Intent = Intent(context, QrScannerActivity::class.java)

    override fun parseResult(
        resultCode: Int,
        intent: Intent?,
    ): String? =
        if (resultCode == Activity.RESULT_OK) {
            intent?.getStringExtra(QrScannerActivity.EXTRA_TOKEN)
        } else {
            null
        }
}
