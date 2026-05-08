package com.poziomki.app.ui.feature.auth

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.focus.FocusRequester
import androidx.compose.ui.focus.focusRequester
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.poziomki.app.ui.designsystem.components.AppButton
import com.poziomki.app.ui.designsystem.components.OtpInput
import com.poziomki.app.ui.designsystem.theme.NunitoFamily
import com.poziomki.app.ui.designsystem.theme.PoziomkiTheme
import com.poziomki.app.ui.designsystem.theme.Primary
import com.poziomki.app.ui.designsystem.theme.TextMuted
import com.poziomki.app.ui.designsystem.theme.TextPrimary
import com.poziomki.app.ui.designsystem.theme.TextSecondary
import org.koin.compose.viewmodel.koinViewModel

@Suppress("LongParameterList", "LongMethod")
@Composable
fun VerifyScreen(
    email: String,
    onVerifySuccess: () -> Unit = {},
    title: String = "weryfikacja",
    onSubmit: ((String, String, () -> Unit) -> Unit)? = null,
    onResend: ((String) -> Unit)? = null,
    viewModel: AuthViewModel = koinViewModel(),
) {
    val uiState by viewModel.uiState.collectAsState()
    var otp by remember { mutableStateOf("") }
    val focusRequester = remember { FocusRequester() }
    val submit: () -> Unit = {
        if (onSubmit != null) {
            onSubmit(email, otp, onVerifySuccess)
        } else {
            viewModel.verifyOtp(email, otp, onVerifySuccess)
        }
    }
    val resend: () -> Unit = {
        if (onResend != null) onResend(email) else viewModel.resendOtp(email)
    }

    LaunchedEffect(Unit) {
        focusRequester.requestFocus()
    }

    // Auto-submit when 6 digits entered (with guard against double-submit)
    var hasSubmitted by remember { mutableStateOf(false) }
    LaunchedEffect(otp) {
        if (otp.length < 6) {
            hasSubmitted = false
        } else if (!hasSubmitted && !uiState.isLoading) {
            hasSubmitted = true
            submit()
        }
    }

    Column(
        modifier =
            Modifier
                .fillMaxSize()
                .background(MaterialTheme.colorScheme.background)
                .padding(horizontal = PoziomkiTheme.spacing.lg),
        horizontalAlignment = Alignment.CenterHorizontally,
    ) {
        Spacer(modifier = Modifier.height(120.dp))

        Text(
            text = title,
            style = MaterialTheme.typography.headlineMedium,
            color = TextPrimary,
        )

        Spacer(modifier = Modifier.height(PoziomkiTheme.spacing.md))

        Text(
            text = "wpisz kod wys\u0142any na",
            fontFamily = NunitoFamily,
            fontWeight = FontWeight.Normal,
            fontSize = 16.sp,
            color = TextSecondary,
            textAlign = TextAlign.Center,
        )

        Text(
            text = email,
            fontFamily = NunitoFamily,
            fontWeight = FontWeight.SemiBold,
            fontSize = 16.sp,
            color = Primary,
            textAlign = TextAlign.Center,
        )

        Spacer(modifier = Modifier.height(PoziomkiTheme.spacing.xxl))

        // Error
        uiState.error?.let { error ->
            Text(
                text = error,
                fontFamily = NunitoFamily,
                fontWeight = FontWeight.Medium,
                fontSize = 14.sp,
                color = MaterialTheme.colorScheme.error,
                modifier = Modifier.padding(bottom = PoziomkiTheme.spacing.md),
            )
        }

        // OTP input boxes
        OtpInput(
            value = otp,
            onValueChange = { otp = it },
            modifier = Modifier.focusRequester(focusRequester),
        )

        Spacer(modifier = Modifier.height(PoziomkiTheme.spacing.xl))

        AppButton(
            text = "potwierd\u017a",
            onClick = submit,
            enabled = otp.length == 6,
            loading = uiState.isLoading,
        )

        Spacer(modifier = Modifier.height(PoziomkiTheme.spacing.md))

        if (uiState.otpResent) {
            Text(
                text = "kod wys\u0142any ponownie",
                fontFamily = NunitoFamily,
                fontWeight = FontWeight.Medium,
                fontSize = 14.sp,
                color = Primary,
                modifier = Modifier.padding(bottom = PoziomkiTheme.spacing.sm),
            )
        }

        TextButton(
            onClick = resend,
            enabled = uiState.resendCooldownSeconds == 0,
        ) {
            Text(
                text =
                    if (uiState.resendCooldownSeconds > 0) {
                        "wy\u015blij ponownie (${uiState.resendCooldownSeconds}s)"
                    } else {
                        "wy\u015blij ponownie"
                    },
                fontFamily = NunitoFamily,
                fontWeight = FontWeight.SemiBold,
                fontSize = 14.sp,
                color = if (uiState.resendCooldownSeconds > 0) TextMuted else Primary,
            )
        }
    }
}
