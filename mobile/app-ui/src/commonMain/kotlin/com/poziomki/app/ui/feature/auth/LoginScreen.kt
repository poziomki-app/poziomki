package com.poziomki.app.ui.feature.auth

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
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
import androidx.compose.ui.autofill.ContentType
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.poziomki.app.ui.designsystem.components.AppButton
import com.poziomki.app.ui.designsystem.components.ButtonVariant
import com.poziomki.app.ui.designsystem.components.PoziomkiLogo
import com.poziomki.app.ui.designsystem.components.PoziomkiPasswordField
import com.poziomki.app.ui.designsystem.components.PoziomkiTextField
import com.poziomki.app.ui.designsystem.theme.NunitoFamily
import com.poziomki.app.ui.designsystem.theme.PoziomkiTheme
import com.poziomki.app.ui.designsystem.theme.Primary
import com.poziomki.app.ui.designsystem.theme.TextMuted
import com.poziomki.app.ui.designsystem.theme.TextPrimary
import com.poziomki.app.ui.designsystem.theme.TextSecondary
import org.koin.compose.viewmodel.koinViewModel

@Suppress("LongParameterList", "LongMethod")
@Composable
fun LoginScreen(
    onNavigateToRegister: () -> Unit,
    onLoginSuccess: () -> Unit,
    onNeedsVerification: (String) -> Unit,
    onNeedsOnboarding: () -> Unit,
    onForgotPassword: () -> Unit,
    prefillEmail: String? = null,
    viewModel: AuthViewModel = koinViewModel(),
) {
    val uiState by viewModel.uiState.collectAsState()
    var email by remember { mutableStateOf(prefillEmail.orEmpty()) }
    var password by remember { mutableStateOf("") }

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
            text = "poznajmy si\u0119!",
            fontFamily = NunitoFamily,
            fontWeight = FontWeight.Normal,
            fontSize = 20.sp,
            color = TextSecondary,
        )

        Spacer(modifier = Modifier.height(48.dp))

        // Error banner
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
                    imeAction = ImeAction.Next,
                ),
            contentType = ContentType.Username + ContentType.EmailAddress,
        )

        Spacer(modifier = Modifier.height(PoziomkiTheme.spacing.lg))

        Row(
            modifier = Modifier.fillMaxWidth().padding(start = 4.dp, bottom = 8.dp),
            horizontalArrangement = Arrangement.SpaceBetween,
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Text(
                text = "has\u0142o",
                fontFamily = NunitoFamily,
                fontWeight = FontWeight.SemiBold,
                fontSize = 14.sp,
                color = TextPrimary,
            )
            Text(
                text = "nie pami\u0119tam has\u0142a",
                fontFamily = NunitoFamily,
                fontWeight = FontWeight.Normal,
                fontSize = 13.sp,
                color = TextMuted,
                modifier = Modifier.clickable(onClick = onForgotPassword),
            )
        }

        PoziomkiPasswordField(
            value = password,
            onValueChange = {
                password = it
                viewModel.clearError()
            },
            placeholder = "has\u0142o",
            contentType = ContentType.Password,
        )

        Spacer(modifier = Modifier.height(PoziomkiTheme.spacing.lg))

        AppButton(
            text = "zaloguj si\u0119",
            onClick = {
                viewModel.signIn(email, password, onLoginSuccess, onNeedsVerification, onNeedsOnboarding)
            },
            variant = ButtonVariant.PRIMARY,
            enabled = email.isNotBlank() && password.isNotBlank(),
            loading = uiState.isLoading,
            modifier = Modifier.fillMaxWidth(),
        )

        TextButton(
            onClick = onNavigateToRegister,
            modifier = Modifier.align(Alignment.CenterHorizontally),
        ) {
            Text(
                text = "zarejestruj si\u0119",
                fontFamily = NunitoFamily,
                fontWeight = FontWeight.SemiBold,
                fontSize = 14.sp,
                color = Primary,
            )
        }
    }
}
