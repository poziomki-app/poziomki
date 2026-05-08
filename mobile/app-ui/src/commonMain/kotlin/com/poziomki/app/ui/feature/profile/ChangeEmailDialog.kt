package com.poziomki.app.ui.feature.profile

import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.poziomki.app.ui.designsystem.components.OtpInput
import com.poziomki.app.ui.designsystem.components.PoziomkiPasswordField
import com.poziomki.app.ui.designsystem.components.PoziomkiTextField
import com.poziomki.app.ui.designsystem.theme.NunitoFamily
import com.poziomki.app.ui.designsystem.theme.Primary
import com.poziomki.app.ui.designsystem.theme.TextSecondary

@Composable
@Suppress("LongParameterList")
fun ChangeEmailDialog(
    isRequesting: Boolean,
    isConfirming: Boolean,
    otpSent: Boolean,
    pendingNewEmail: String?,
    error: String?,
    onDismiss: () -> Unit,
    onRequestOtp: (newEmail: String, currentPassword: String) -> Unit,
    onConfirmOtp: (code: String) -> Unit,
) {
    if (otpSent && pendingNewEmail != null) {
        OtpStep(
            newEmail = pendingNewEmail,
            isLoading = isConfirming,
            error = error,
            onConfirm = onConfirmOtp,
            onDismiss = onDismiss,
        )
    } else {
        EnterEmailStep(
            isLoading = isRequesting,
            error = error,
            onSubmit = onRequestOtp,
            onDismiss = onDismiss,
        )
    }
}

@Composable
@Suppress("LongMethod")
private fun EnterEmailStep(
    isLoading: Boolean,
    error: String?,
    onSubmit: (String, String) -> Unit,
    onDismiss: () -> Unit,
) {
    var email by remember { mutableStateOf("") }
    var password by remember { mutableStateOf("") }

    AlertDialog(
        onDismissRequest = { if (!isLoading) onDismiss() },
        title = {
            Text(
                text = "Zmień email",
                fontFamily = NunitoFamily,
                fontWeight = FontWeight.Bold,
            )
        },
        text = {
            Column {
                Text(
                    text = "Wpisz nowy adres email i aktualne hasło. Wyślemy kod weryfikacyjny na nowy adres.",
                    fontFamily = NunitoFamily,
                    fontWeight = FontWeight.Normal,
                    fontSize = 14.sp,
                    color = TextSecondary,
                )
                Spacer(modifier = Modifier.height(16.dp))
                PoziomkiTextField(
                    value = email,
                    onValueChange = { email = it.trim() },
                    placeholder = "nowy email",
                    modifier = Modifier.fillMaxWidth(),
                )
                Spacer(modifier = Modifier.height(8.dp))
                PoziomkiPasswordField(
                    value = password,
                    onValueChange = { password = it },
                    placeholder = "Aktualne hasło",
                )
                error?.let {
                    Spacer(modifier = Modifier.height(8.dp))
                    Text(
                        text = it,
                        fontFamily = NunitoFamily,
                        fontWeight = FontWeight.Medium,
                        fontSize = 13.sp,
                        color = MaterialTheme.colorScheme.error,
                    )
                }
            }
        },
        confirmButton = {
            TextButton(
                onClick = { onSubmit(email, password) },
                enabled = email.isNotBlank() && password.isNotBlank() && !isLoading,
            ) {
                Text("Wyślij kod", fontFamily = NunitoFamily, fontWeight = FontWeight.Bold)
            }
        },
        dismissButton = {
            TextButton(onClick = onDismiss, enabled = !isLoading) {
                Text("Anuluj", fontFamily = NunitoFamily)
            }
        },
    )
}

@Composable
@Suppress("LongMethod")
private fun OtpStep(
    newEmail: String,
    isLoading: Boolean,
    error: String?,
    onConfirm: (String) -> Unit,
    onDismiss: () -> Unit,
) {
    var code by remember { mutableStateOf("") }
    var hasSubmitted by remember { mutableStateOf(false) }

    LaunchedEffect(code) {
        if (code.length < 6) {
            hasSubmitted = false
        } else if (!hasSubmitted && !isLoading) {
            hasSubmitted = true
            onConfirm(code)
        }
    }

    AlertDialog(
        onDismissRequest = { if (!isLoading) onDismiss() },
        title = {
            Text(
                text = "Potwierdź email",
                fontFamily = NunitoFamily,
                fontWeight = FontWeight.Bold,
            )
        },
        text = {
            Column {
                Text(
                    text = "Wpisz 6-cyfrowy kod wysłany na",
                    fontFamily = NunitoFamily,
                    fontSize = 14.sp,
                    color = TextSecondary,
                )
                Text(
                    text = newEmail,
                    fontFamily = NunitoFamily,
                    fontWeight = FontWeight.SemiBold,
                    fontSize = 14.sp,
                    color = Primary,
                )
                Spacer(modifier = Modifier.height(16.dp))
                OtpInput(
                    value = code,
                    onValueChange = { code = it },
                    modifier = Modifier.fillMaxWidth(),
                )
                error?.let {
                    Spacer(modifier = Modifier.height(8.dp))
                    Text(
                        text = it,
                        fontFamily = NunitoFamily,
                        fontWeight = FontWeight.Medium,
                        fontSize = 13.sp,
                        color = MaterialTheme.colorScheme.error,
                    )
                }
            }
        },
        confirmButton = {
            TextButton(
                onClick = { onConfirm(code) },
                enabled = code.length == 6 && !isLoading,
            ) {
                Text("Potwierdź", fontFamily = NunitoFamily, fontWeight = FontWeight.Bold)
            }
        },
        dismissButton = {
            TextButton(onClick = onDismiss, enabled = !isLoading) {
                Text("Anuluj", fontFamily = NunitoFamily)
            }
        },
    )
}
