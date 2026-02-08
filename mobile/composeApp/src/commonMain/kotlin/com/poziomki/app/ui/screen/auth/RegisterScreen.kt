package com.poziomki.app.ui.screen.auth

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
import com.poziomki.app.ui.component.PoziomkiButton
import com.poziomki.app.ui.component.PoziomkiLogo
import com.poziomki.app.ui.component.PoziomkiPasswordField
import com.poziomki.app.ui.component.PoziomkiTextField
import com.poziomki.app.ui.theme.NunitoFamily
import com.poziomki.app.ui.theme.PoziomkiTheme
import com.poziomki.app.ui.theme.Primary
import com.poziomki.app.ui.theme.TextSecondary
import org.koin.compose.viewmodel.koinViewModel

@Composable
fun RegisterScreen(
    onNavigateToLogin: () -> Unit,
    onRegisterSuccess: (String) -> Unit,
    viewModel: AuthViewModel = koinViewModel(),
) {
    val uiState by viewModel.uiState.collectAsState()
    var email by remember { mutableStateOf("") }
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
            text = "poznajmy si\u0119!",
            fontFamily = NunitoFamily,
            fontWeight = FontWeight.Normal,
            fontSize = 20.sp,
            color = TextSecondary,
        )

        Spacer(modifier = Modifier.height(48.dp))

        // Error banner
        if (uiState.error != null) {
            Text(
                text = uiState.error!!,
                fontFamily = NunitoFamily,
                fontWeight = FontWeight.Medium,
                fontSize = 14.sp,
                color = MaterialTheme.colorScheme.error,
                modifier = Modifier.padding(bottom = PoziomkiTheme.spacing.md),
            )
        }

        PoziomkiTextField(
            value = email,
            onValueChange = { email = it },
            label = "email",
            placeholder = "@example.com",
            modifier = Modifier.fillMaxWidth(),
            keyboardOptions =
                KeyboardOptions(
                    keyboardType = KeyboardType.Email,
                    imeAction = ImeAction.Next,
                ),
        )

        Spacer(modifier = Modifier.height(PoziomkiTheme.spacing.lg))

        PoziomkiPasswordField(
            value = password,
            onValueChange = { password = it },
            label = "has\u0142o",
            placeholder = "minimum 8 znak\u00f3w",
        )

        Spacer(modifier = Modifier.height(PoziomkiTheme.spacing.lg))

        PoziomkiPasswordField(
            value = confirmPassword,
            onValueChange = { confirmPassword = it },
            label = "potwierd\u017a has\u0142o",
            placeholder = "has\u0142o",
            error =
                if (confirmPassword.isNotEmpty() && confirmPassword != password) {
                    "has\u0142a nie s\u0105 takie same"
                } else {
                    null
                },
        )

        Spacer(modifier = Modifier.height(PoziomkiTheme.spacing.xl))

        PoziomkiButton(
            text = "zarejestruj si\u0119",
            onClick = {
                val placeholderName = email.substringBefore("@").ifBlank { "User" }
                viewModel.signUp(email, password, placeholderName, onRegisterSuccess)
            },
            enabled =
                email.isNotBlank() &&
                    password.length >= 8 &&
                    password == confirmPassword,
            loading = uiState.isLoading,
            loadingText = "tworzenie konta...",
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
}
