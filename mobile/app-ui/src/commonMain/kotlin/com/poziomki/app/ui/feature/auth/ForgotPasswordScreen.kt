package com.poziomki.app.ui.feature.auth

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.poziomki.app.ui.designsystem.components.AppButton
import com.poziomki.app.ui.designsystem.components.PoziomkiLogo
import com.poziomki.app.ui.designsystem.components.PoziomkiTextField
import com.poziomki.app.ui.designsystem.theme.NunitoFamily
import com.poziomki.app.ui.designsystem.theme.PoziomkiTheme
import com.poziomki.app.ui.designsystem.theme.Primary
import com.poziomki.app.ui.designsystem.theme.TextSecondary
import org.koin.compose.viewmodel.koinViewModel

@Suppress("LongMethod")
@Composable
fun ForgotPasswordScreen(
    onNavigateBack: () -> Unit,
    onSuccess: (String) -> Unit,
    prefillEmail: String? = null,
    viewModel: AuthViewModel = koinViewModel(),
) {
    val uiState by viewModel.uiState.collectAsState()
    var email by remember { mutableStateOf(prefillEmail.orEmpty()) }

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
            text = "nie pami\u0119tam has\u0142a",
            fontFamily = NunitoFamily,
            fontWeight = FontWeight.Normal,
            fontSize = 20.sp,
            color = TextSecondary,
        )

        Spacer(modifier = Modifier.height(16.dp))

        Text(
            text = "podaj sw\u00f3j email, a wy\u015blemy ci kod weryfikacyjny",
            fontFamily = NunitoFamily,
            fontWeight = FontWeight.Normal,
            fontSize = 14.sp,
            color = TextSecondary,
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

        PoziomkiTextField(
            value = email,
            onValueChange = {
                email = it
                viewModel.clearError()
            },
            label = "email",
            modifier = Modifier.fillMaxWidth(),
            keyboardOptions =
                KeyboardOptions(
                    keyboardType = KeyboardType.Email,
                    imeAction = ImeAction.Done,
                ),
        )

        Spacer(modifier = Modifier.height(PoziomkiTheme.spacing.xl))

        AppButton(
            text = "wy\u015blij kod",
            onClick = { viewModel.forgotPassword(email) { onSuccess(email) } },
            enabled = email.isNotBlank(),
            loading = uiState.isLoading,
        )

        Spacer(modifier = Modifier.height(PoziomkiTheme.spacing.md))

        TextButton(
            onClick = onNavigateBack,
            modifier = Modifier.align(Alignment.CenterHorizontally),
        ) {
            Text(
                text = "wr\u00f3\u0107 do logowania",
                fontFamily = NunitoFamily,
                fontWeight = FontWeight.SemiBold,
                fontSize = 14.sp,
                color = Primary,
            )
        }
    }
}
