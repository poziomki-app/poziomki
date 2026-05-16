package com.poziomki.app.ui.feature.profile

import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.height
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.poziomki.app.ui.designsystem.components.PoziomkiPasswordField
import com.poziomki.app.ui.designsystem.theme.NunitoFamily
import com.poziomki.app.ui.designsystem.theme.TextSecondary

@Composable
fun ChangePasswordDialog(
    isLoading: Boolean,
    error: String?,
    onDismiss: () -> Unit,
    onSubmit: (currentPassword: String, newPassword: String, confirmPassword: String) -> Unit,
) {
    var current by remember { mutableStateOf("") }
    var new by remember { mutableStateOf("") }
    var confirm by remember { mutableStateOf("") }

    AlertDialog(
        onDismissRequest = { if (!isLoading) onDismiss() },
        title = {
            Text(
                text = "Zmień hasło",
                fontFamily = NunitoFamily,
                fontWeight = FontWeight.Bold,
            )
        },
        text = {
            Column {
                Text(
                    text = "Wpisz aktualne hasło i nowe hasło, aby je zmienić.",
                    fontFamily = NunitoFamily,
                    fontWeight = FontWeight.Normal,
                    fontSize = 14.sp,
                    color = TextSecondary,
                )
                Spacer(modifier = Modifier.height(16.dp))
                PoziomkiPasswordField(
                    value = current,
                    onValueChange = { current = it },
                    placeholder = "Aktualne hasło",
                )
                Spacer(modifier = Modifier.height(8.dp))
                PoziomkiPasswordField(
                    value = new,
                    onValueChange = { new = it },
                    placeholder = "Nowe hasło",
                )
                Spacer(modifier = Modifier.height(8.dp))
                PoziomkiPasswordField(
                    value = confirm,
                    onValueChange = { confirm = it },
                    placeholder = "Powtórz nowe hasło",
                )
                DialogInlineError(error)
            }
        },
        confirmButton = {
            TextButton(
                onClick = { onSubmit(current, new, confirm) },
                enabled = current.isNotBlank() && new.isNotBlank() && confirm.isNotBlank() && !isLoading,
            ) {
                Text("Zmień", fontFamily = NunitoFamily, fontWeight = FontWeight.Bold)
            }
        },
        dismissButton = {
            TextButton(onClick = onDismiss, enabled = !isLoading) {
                Text("Anuluj", fontFamily = NunitoFamily)
            }
        },
    )
}
