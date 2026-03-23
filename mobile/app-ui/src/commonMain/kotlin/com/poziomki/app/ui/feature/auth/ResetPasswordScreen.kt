package com.poziomki.app.ui.feature.auth

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.poziomki.app.ui.designsystem.components.PoziomkiButton
import com.poziomki.app.ui.designsystem.components.PoziomkiLogo
import com.poziomki.app.ui.designsystem.components.PoziomkiPasswordField
import com.poziomki.app.ui.designsystem.theme.NunitoFamily
import com.poziomki.app.ui.designsystem.theme.PoziomkiTheme
import com.poziomki.app.ui.designsystem.theme.Primary
import com.poziomki.app.ui.designsystem.theme.TextSecondary
import org.koin.compose.viewmodel.koinViewModel

@Suppress("LongMethod")
@Composable
fun ResetPasswordScreen(
    email: String,
    onSuccess: () -> Unit,
    onNeedsOnboarding: () -> Unit,
    viewModel: AuthViewModel = koinViewModel(),
) {
    val uiState by viewModel.uiState.collectAsState()
    var password by remember { mutableStateOf("") }
    var confirmPassword by remember { mutableStateOf("") }

    Column(
        modifier =
            Modifier
                .fillMaxSize()
                .background(MaterialTheme.colorScheme.background)
                .verticalScroll(rememberScrollState())
                .padding(horizontal = PoziomkiTheme.spacing.lg)
                .padding(bottom = PoziomkiTheme.spacing.xl),
    ) {
        Spacer(modifier = Modifier.height(64.dp))

        PoziomkiLogo(size = 48.dp)

        Spacer(modifier = Modifier.height(4.dp))

        Text(
            text = "nowe has\u0142o",
            fontFamily = NunitoFamily,
            fontWeight = FontWeight.Normal,
            fontSize = 20.sp,
            color = TextSecondary,
        )

        Spacer(modifier = Modifier.height(16.dp))

        Text(
            text = "ustaw nowe has\u0142o dla",
            fontFamily = NunitoFamily,
            fontWeight = FontWeight.Normal,
            fontSize = 14.sp,
            color = TextSecondary,
        )

        Text(
            text = email,
            fontFamily = NunitoFamily,
            fontWeight = FontWeight.SemiBold,
            fontSize = 14.sp,
            color = Primary,
        )

        Spacer(modifier = Modifier.height(32.dp))

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

        PoziomkiPasswordField(
            value = password,
            onValueChange = {
                password = it
                viewModel.clearError()
            },
            label = "nowe has\u0142o",
            placeholder = "minimum 8 znak\u00f3w",
            modifier = Modifier.fillMaxWidth(),
        )

        Spacer(modifier = Modifier.height(PoziomkiTheme.spacing.lg))

        PoziomkiPasswordField(
            value = confirmPassword,
            onValueChange = {
                confirmPassword = it
                viewModel.clearError()
            },
            label = "potwierd\u017a has\u0142o",
            placeholder = "has\u0142o",
            modifier = Modifier.fillMaxWidth(),
            error =
                if (confirmPassword.isNotEmpty() && confirmPassword != password) {
                    "has\u0142a nie s\u0105 takie same"
                } else {
                    null
                },
        )

        Spacer(modifier = Modifier.height(PoziomkiTheme.spacing.xl))

        PoziomkiButton(
            text = "zapisz has\u0142o",
            onClick = {
                viewModel.resetPassword(email, password, onSuccess, onNeedsOnboarding)
            },
            enabled = password.length >= 8 && password == confirmPassword,
            loading = uiState.isLoading,
        )
    }
}
