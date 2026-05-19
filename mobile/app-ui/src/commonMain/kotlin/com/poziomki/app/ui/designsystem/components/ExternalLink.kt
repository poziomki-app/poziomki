package com.poziomki.app.ui.designsystem.components

import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.platform.LocalUriHandler
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.poziomki.app.ui.designsystem.Text
import com.poziomki.app.ui.designsystem.theme.NunitoFamily
import com.poziomki.app.ui.designsystem.theme.Primary
import com.poziomki.app.ui.designsystem.theme.SurfaceElevated
import com.poziomki.app.ui.designsystem.theme.TextMuted
import com.poziomki.app.ui.designsystem.theme.TextPrimary
import com.poziomki.app.ui.designsystem.theme.TextSecondary

/**
 * Returns an `(url) -> Unit` that confirms with the user before opening an external link.
 * Place the call site high enough in the tree that the confirm dialog renders over the screen.
 */
@Composable
fun rememberExternalLinkOpener(): (String) -> Unit {
    val uriHandler = LocalUriHandler.current
    var pending by remember { mutableStateOf<String?>(null) }

    pending?.let { url ->
        val isMaps = url.startsWith("geo:")
        AlertDialog(
            onDismissRequest = { pending = null },
            shape = RoundedCornerShape(16.dp),
            containerColor = SurfaceElevated,
            tonalElevation = 0.dp,
            title = {
                Text(
                    text = if (isMaps) "otworzyć w mapach?" else "otworzyć link zewnętrzny?",
                    fontFamily = NunitoFamily,
                    fontWeight = FontWeight.Bold,
                    fontSize = 18.sp,
                    color = TextPrimary,
                )
            },
            text = {
                Text(
                    text = humanReadable(url),
                    fontFamily = NunitoFamily,
                    fontWeight = FontWeight.Normal,
                    fontSize = 13.sp,
                    color = TextSecondary,
                    lineHeight = 18.sp,
                )
            },
            confirmButton = {
                TextButton(onClick = {
                    val toOpen = url
                    pending = null
                    runCatching { uriHandler.openUri(toOpen) }
                }) {
                    Text(
                        text = "otwórz",
                        fontFamily = NunitoFamily,
                        fontWeight = FontWeight.SemiBold,
                        color = Primary,
                    )
                }
            },
            dismissButton = {
                TextButton(onClick = { pending = null }) {
                    Text(
                        text = "anuluj",
                        fontFamily = NunitoFamily,
                        fontWeight = FontWeight.SemiBold,
                        color = TextMuted,
                    )
                }
            },
        )
    }

    return { url -> pending = url }
}

/** Builds a Maps deeplink that prefers the device Maps app. */
fun mapsDeeplink(
    latitude: Double,
    longitude: Double,
    label: String? = null,
): String {
    val q = if (label.isNullOrBlank()) "$latitude,$longitude" else "$latitude,$longitude($label)"
    return "geo:$latitude,$longitude?q=$q"
}

private fun humanReadable(url: String): String {
    if (!url.startsWith("geo:")) return url
    val labelInParens = url.substringAfter('(', "").substringBeforeLast(')', "")
    if (labelInParens.isNotBlank()) return labelInParens
    return url.removePrefix("geo:").substringBefore("?").replace(",", ", ")
}
