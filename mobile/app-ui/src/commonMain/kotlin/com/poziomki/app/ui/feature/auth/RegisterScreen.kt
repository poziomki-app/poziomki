package com.poziomki.app.ui.feature.auth

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.imePadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Checkbox
import androidx.compose.material3.CheckboxDefaults
import androidx.compose.material3.MaterialTheme
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
import com.poziomki.app.ui.designsystem.Text
import com.poziomki.app.ui.designsystem.components.AppButton
import com.poziomki.app.ui.designsystem.components.PoziomkiLogo
import com.poziomki.app.ui.designsystem.components.PoziomkiPasswordField
import com.poziomki.app.ui.designsystem.components.PoziomkiTextField
import com.poziomki.app.ui.designsystem.theme.NunitoFamily
import com.poziomki.app.ui.designsystem.theme.PoziomkiTheme
import com.poziomki.app.ui.designsystem.theme.Primary
import com.poziomki.app.ui.designsystem.theme.TextSecondary
import org.koin.compose.viewmodel.koinViewModel

@OptIn(ExperimentalLayoutApi::class)
@Suppress("LongMethod")
@Composable
fun RegisterScreen(
    onNavigateToLogin: () -> Unit,
    onRegisterSuccess: (String) -> Unit,
    onUserExists: (String) -> Unit = {},
    viewModel: AuthViewModel = koinViewModel(),
) {
    val uiState by viewModel.uiState.collectAsState()
    var email by remember { mutableStateOf("") }
    var password by remember { mutableStateOf("") }
    var confirmPassword by remember { mutableStateOf("") }
    var acceptedPolicy by remember { mutableStateOf(false) }
    var showPolicy by remember { mutableStateOf(false) }
    var showRegulamin by remember { mutableStateOf(false) }
    var showPasswordMismatch by remember { mutableStateOf(false) }

    Column(
        modifier =
            Modifier
                .fillMaxSize()
                .background(MaterialTheme.colorScheme.background)
                .imePadding()
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
            placeholder = "@example.com",
            modifier = Modifier.fillMaxWidth(),
            keyboardOptions =
                KeyboardOptions(
                    keyboardType = KeyboardType.Email,
                    imeAction = ImeAction.Next,
                ),
            contentType = ContentType.EmailAddress,
        )

        Spacer(modifier = Modifier.height(PoziomkiTheme.spacing.lg))

        PoziomkiPasswordField(
            value = password,
            onValueChange = {
                password = it
                viewModel.clearError()
            },
            label = "has\u0142o",
            placeholder = "minimum 8 znak\u00f3w",
            contentType = ContentType.NewPassword,
        )

        Spacer(modifier = Modifier.height(PoziomkiTheme.spacing.lg))

        PoziomkiPasswordField(
            value = confirmPassword,
            onValueChange = {
                confirmPassword = it
                showPasswordMismatch = false
                viewModel.clearError()
            },
            label = "potwierd\u017a has\u0142o",
            placeholder = "has\u0142o",
            error =
                if (showPasswordMismatch && confirmPassword != password) {
                    "has\u0142a nie s\u0105 takie same"
                } else {
                    null
                },
            contentType = ContentType.NewPassword,
        )

        Spacer(modifier = Modifier.height(PoziomkiTheme.spacing.lg))

        Row(
            verticalAlignment = Alignment.CenterVertically,
            modifier = Modifier.fillMaxWidth(),
        ) {
            Checkbox(
                checked = acceptedPolicy,
                onCheckedChange = { acceptedPolicy = it },
                colors =
                    CheckboxDefaults.colors(
                        checkedColor = Primary,
                        uncheckedColor = TextSecondary,
                        checkmarkColor = MaterialTheme.colorScheme.background,
                    ),
            )
            FlowRow(verticalArrangement = Arrangement.Center) {
                Text(
                    text = "akceptuję ",
                    fontFamily = NunitoFamily,
                    fontWeight = FontWeight.Normal,
                    fontSize = 14.sp,
                    color = TextSecondary,
                )
                Text(
                    text = "regulamin",
                    fontFamily = NunitoFamily,
                    fontWeight = FontWeight.SemiBold,
                    fontSize = 14.sp,
                    color = Primary,
                    modifier = Modifier.clickable { showRegulamin = true },
                )
                Text(
                    text = " i ",
                    fontFamily = NunitoFamily,
                    fontWeight = FontWeight.Normal,
                    fontSize = 14.sp,
                    color = TextSecondary,
                )
                Text(
                    text = "politykę prywatności",
                    fontFamily = NunitoFamily,
                    fontWeight = FontWeight.SemiBold,
                    fontSize = 14.sp,
                    color = Primary,
                    modifier = Modifier.clickable { showPolicy = true },
                )
                Text(
                    text = " *",
                    fontFamily = NunitoFamily,
                    fontWeight = FontWeight.Bold,
                    fontSize = 14.sp,
                    color = MaterialTheme.colorScheme.error,
                )
            }
        }

        Spacer(modifier = Modifier.height(PoziomkiTheme.spacing.lg))

        AppButton(
            text = "zarejestruj si\u0119",
            onClick = {
                if (password != confirmPassword) {
                    showPasswordMismatch = true
                    return@AppButton
                }
                val placeholderName = email.substringBefore("@").ifBlank { "User" }
                viewModel.signUp(email, password, placeholderName, onRegisterSuccess, onUserExists)
            },
            enabled =
                email.isNotBlank() &&
                    password.length >= 8 &&
                    confirmPassword.isNotBlank() &&
                    acceptedPolicy,
            loading = uiState.isLoading,
            loadingText = "tworzenie konta...",
            modifier = Modifier.fillMaxWidth(),
        )

        Spacer(modifier = Modifier.height(PoziomkiTheme.spacing.md))

        TextButton(
            onClick = onNavigateToLogin,
            modifier = Modifier.align(Alignment.CenterHorizontally),
        ) {
            Text(
                text = "zaloguj si\u0119",
                fontFamily = NunitoFamily,
                fontWeight = FontWeight.SemiBold,
                fontSize = 14.sp,
                color = Primary,
            )
        }
    }

    if (showRegulamin) {
        LegalDocumentDialog(
            title = "regulamin",
            body = regulaminText,
            onDismiss = { showRegulamin = false },
        )
    }

    if (showPolicy) {
        LegalDocumentDialog(
            title = "polityka prywatności",
            body = privacyPolicyText,
            onDismiss = { showPolicy = false },
        )
    }
}
