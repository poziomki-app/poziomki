package com.poziomki.app.ui.designsystem.components

import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.compose.ui.window.Dialog
import androidx.compose.ui.window.DialogProperties
import com.poziomki.app.ui.designsystem.theme.Border
import com.poziomki.app.ui.designsystem.theme.NunitoFamily
import com.poziomki.app.ui.designsystem.theme.Primary
import com.poziomki.app.ui.designsystem.theme.TextMuted
import com.poziomki.app.ui.designsystem.theme.TextPrimary
import com.poziomki.app.ui.designsystem.theme.Surface as SurfaceColor

private val REPORT_REASONS =
    listOf(
        "spam" to "spam",
        "inappropriate" to "nieodpowiednie",
        "misleading" to "mylące",
        "harassment" to "nękanie",
        "hate_speech" to "mowa nienawiści",
        "violence" to "przemoc",
        "scam" to "oszustwo",
        "other" to "inne",
    )

@OptIn(ExperimentalLayoutApi::class)
@Composable
@Suppress("LongMethod")
fun ReportDialog(
    onConfirm: (reason: String, description: String?) -> Unit,
    onDismiss: () -> Unit,
) {
    var selectedReason by remember { mutableStateOf<String?>(null) }

    Dialog(
        onDismissRequest = onDismiss,
        properties = DialogProperties(usePlatformDefaultWidth = false),
    ) {
        Surface(
            shape = RoundedCornerShape(20.dp),
            color = SurfaceColor,
            modifier = Modifier.fillMaxWidth().padding(horizontal = 32.dp),
        ) {
            Column(modifier = Modifier.padding(16.dp)) {
                ReportHeader()
                Spacer(modifier = Modifier.height(12.dp))
                ReasonChips(selectedReason = selectedReason, onSelect = { selectedReason = it })
                Spacer(modifier = Modifier.height(16.dp))
                ReportButtons(selectedReason, onDismiss) { reason -> onConfirm(reason, null) }
            }
        }
    }
}

@Composable
private fun ReportHeader() {
    Text(
        text = "zgłoś",
        fontFamily = NunitoFamily,
        fontWeight = FontWeight.Bold,
        fontSize = 18.sp,
        color = TextPrimary,
    )
    Spacer(modifier = Modifier.height(4.dp))
    Text(
        text = "wybierz powód zgłoszenia",
        fontFamily = NunitoFamily,
        fontSize = 14.sp,
        color = TextMuted,
    )
}

@OptIn(ExperimentalLayoutApi::class)
@Composable
private fun ReasonChips(
    selectedReason: String?,
    onSelect: (String) -> Unit,
) {
    FlowRow(
        horizontalArrangement = Arrangement.spacedBy(8.dp),
        verticalArrangement = Arrangement.spacedBy(8.dp),
        modifier = Modifier.fillMaxWidth(),
    ) {
        REPORT_REASONS.forEach { (key, label) ->
            val isSelected = selectedReason == key
            Surface(
                shape = RoundedCornerShape(20.dp),
                color = if (isSelected) Primary.copy(alpha = 0.2f) else Border,
                border = if (isSelected) BorderStroke(1.dp, Primary) else null,
                modifier = Modifier.clickable { onSelect(key) },
            ) {
                Text(
                    text = label,
                    fontFamily = NunitoFamily,
                    fontWeight = if (isSelected) FontWeight.SemiBold else FontWeight.Normal,
                    fontSize = 14.sp,
                    color = if (isSelected) Primary else TextPrimary,
                    modifier = Modifier.padding(horizontal = 14.dp, vertical = 8.dp),
                )
            }
        }
    }
}

@Composable
private fun ReportButtons(
    selectedReason: String?,
    onDismiss: () -> Unit,
    onConfirm: (String) -> Unit,
) {
    Row(
        modifier = Modifier.fillMaxWidth(),
        horizontalArrangement = Arrangement.End,
        verticalAlignment = Alignment.CenterVertically,
    ) {
        TextButton(onClick = onDismiss) {
            Text("anuluj", fontFamily = NunitoFamily, fontWeight = FontWeight.SemiBold, color = TextMuted)
        }
        TextButton(
            onClick = { selectedReason?.let(onConfirm) },
            enabled = selectedReason != null,
        ) {
            Text(
                "zgłoś",
                fontFamily = NunitoFamily,
                fontWeight = FontWeight.SemiBold,
                color = if (selectedReason != null) Primary else TextMuted,
            )
        }
    }
}
