package com.poziomki.app.ui.feature.auth

import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
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
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Color
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
import com.poziomki.app.ui.designsystem.theme.Secondary
import com.poziomki.app.ui.designsystem.theme.TextMuted
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

    Box(
        modifier =
            Modifier
                .fillMaxSize()
                .background(MaterialTheme.colorScheme.background),
    ) {
        AuthBackgroundDecoration()

        Column(
            modifier =
                Modifier
                    .fillMaxSize()
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

            PoziomkiPasswordField(
                value = password,
                onValueChange = {
                    password = it
                    viewModel.clearError()
                },
                label = "has\u0142o",
                placeholder = "has\u0142o",
                contentType = ContentType.Password,
            )

            TextButton(
                onClick = onForgotPassword,
                modifier = Modifier.align(Alignment.Start),
                contentPadding = PaddingValues(horizontal = 4.dp, vertical = 4.dp),
            ) {
                Text(
                    text = "nie pami\u0119tam has\u0142a",
                    fontFamily = NunitoFamily,
                    fontWeight = FontWeight.Normal,
                    fontSize = 13.sp,
                    color = TextMuted,
                )
            }

            Spacer(modifier = Modifier.height(PoziomkiTheme.spacing.md))

            AppButton(
                text = "zaloguj si\u0119",
                onClick = {
                    viewModel.signIn(email, password, onLoginSuccess, onNeedsVerification, onNeedsOnboarding)
                },
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
}

@Composable
internal fun AuthBackgroundDecoration() {
    Canvas(modifier = Modifier.fillMaxSize()) {
        drawCircle(
            color = Color(0xFF22D3EE).copy(alpha = 0.06f),
            radius = 300.dp.toPx(),
            center = Offset(size.width * 0.85f, size.height * 0.12f),
        )
        drawCircle(
            color = Color(0xFFEDB923).copy(alpha = 0.04f),
            radius = 220.dp.toPx(),
            center = Offset(size.width * 0.1f, size.height * 0.75f),
        )
        drawCircle(
            color = Color(0xFF22D3EE).copy(alpha = 0.03f),
            radius = 160.dp.toPx(),
            center = Offset(size.width * 0.5f, size.height * 0.45f),
        )
    }
}
