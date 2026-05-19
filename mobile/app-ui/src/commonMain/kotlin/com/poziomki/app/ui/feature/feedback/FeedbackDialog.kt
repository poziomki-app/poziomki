package com.poziomki.app.ui.feature.feedback

import androidx.compose.foundation.ScrollState
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Icon
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.adamglin.PhosphorIcons
import com.adamglin.phosphoricons.Fill
import com.adamglin.phosphoricons.Regular
import com.adamglin.phosphoricons.fill.Star
import com.adamglin.phosphoricons.regular.Star
import com.poziomki.app.ui.designsystem.Text
import com.poziomki.app.ui.designsystem.theme.NunitoFamily
import com.poziomki.app.ui.designsystem.theme.Primary
import com.poziomki.app.ui.designsystem.theme.SurfaceElevated
import com.poziomki.app.ui.designsystem.theme.TextMuted
import com.poziomki.app.ui.designsystem.theme.TextSecondary

@Suppress("LongMethod", "LongParameterList")
@Composable
fun FeedbackDialog(
    rating: Int,
    message: String,
    featureRequest: String,
    isSubmitting: Boolean,
    error: String?,
    onRatingChange: (Int) -> Unit,
    onMessageChange: (String) -> Unit,
    onFeatureRequestChange: (String) -> Unit,
    onSubmit: () -> Unit,
    onDismiss: () -> Unit,
) {
    AlertDialog(
        onDismissRequest = { if (!isSubmitting) onDismiss() },
        containerColor = SurfaceElevated,
        tonalElevation = 0.dp,
        title = {
            Text(
                text = "Zostaw opinię",
                fontFamily = NunitoFamily,
                fontWeight = FontWeight.Bold,
            )
        },
        text = {
            val scrollState = remember { ScrollState(0) }
            Column(modifier = Modifier.verticalScroll(scrollState)) {
                Text(
                    text = "Jak oceniasz aplikację?",
                    fontFamily = NunitoFamily,
                    fontSize = 14.sp,
                    color = TextSecondary,
                )
                Spacer(modifier = Modifier.height(8.dp))
                Row(
                    modifier = Modifier.fillMaxWidth(),
                    horizontalArrangement = Arrangement.spacedBy(8.dp),
                ) {
                    for (i in 1..5) {
                        val filled = i <= rating
                        Icon(
                            imageVector = if (filled) PhosphorIcons.Fill.Star else PhosphorIcons.Regular.Star,
                            contentDescription = "$i",
                            tint = if (filled) Primary else TextMuted,
                            modifier = Modifier.size(36.dp).clickable(enabled = !isSubmitting) { onRatingChange(i) },
                        )
                    }
                }
                Spacer(modifier = Modifier.height(16.dp))
                Text(
                    text = "Co działa, co nie, co poprawić?",
                    fontFamily = NunitoFamily,
                    fontSize = 14.sp,
                    color = TextSecondary,
                )
                Spacer(modifier = Modifier.height(4.dp))
                OutlinedTextField(
                    value = message,
                    onValueChange = onMessageChange,
                    placeholder = {
                        Text(
                            text = "Twoja opinia…",
                            fontFamily = NunitoFamily,
                            color = TextMuted,
                        )
                    },
                    minLines = 3,
                    maxLines = 6,
                    enabled = !isSubmitting,
                    modifier = Modifier.fillMaxWidth(),
                )
                Spacer(modifier = Modifier.height(12.dp))
                Text(
                    text = "Nowe funkcjonalności — co dodać do apki?",
                    fontFamily = NunitoFamily,
                    fontSize = 14.sp,
                    color = TextSecondary,
                )
                Spacer(modifier = Modifier.height(4.dp))
                OutlinedTextField(
                    value = featureRequest,
                    onValueChange = onFeatureRequestChange,
                    placeholder = {
                        Text(
                            text = "np. powiadomienia o znajomych w okolicy",
                            fontFamily = NunitoFamily,
                            color = TextMuted,
                        )
                    },
                    minLines = 2,
                    maxLines = 5,
                    enabled = !isSubmitting,
                    modifier = Modifier.fillMaxWidth(),
                )
                Spacer(modifier = Modifier.height(12.dp))
                Text(
                    text = "Lub napisz: kontakt@poziomki.app",
                    fontFamily = NunitoFamily,
                    fontSize = 12.sp,
                    color = TextMuted,
                )
                if (error != null) {
                    Spacer(modifier = Modifier.height(8.dp))
                    Text(
                        text = error,
                        fontFamily = NunitoFamily,
                        fontSize = 13.sp,
                        color = Color(0xFFD32F2F),
                    )
                }
            }
        },
        confirmButton = {
            TextButton(
                onClick = onSubmit,
                enabled = rating in 1..5 && !isSubmitting,
            ) {
                Text(
                    text = if (isSubmitting) "Wysyłanie…" else "Wyślij",
                    fontFamily = NunitoFamily,
                    fontWeight = FontWeight.Bold,
                )
            }
        },
        dismissButton = {
            TextButton(onClick = onDismiss, enabled = !isSubmitting) {
                Text("Anuluj", fontFamily = NunitoFamily)
            }
        },
    )
}
