package com.poziomki.app.ui.feature.auth

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
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
import androidx.compose.material3.Checkbox
import androidx.compose.material3.CheckboxDefaults
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
import com.poziomki.app.ui.designsystem.components.PoziomkiLogo
import com.poziomki.app.ui.designsystem.components.PoziomkiPasswordField
import com.poziomki.app.ui.designsystem.components.PoziomkiTextField
import com.poziomki.app.ui.designsystem.theme.MontserratFamily
import com.poziomki.app.ui.designsystem.theme.NunitoFamily
import com.poziomki.app.ui.designsystem.theme.PoziomkiTheme
import com.poziomki.app.ui.designsystem.theme.Primary
import com.poziomki.app.ui.designsystem.theme.TextPrimary
import com.poziomki.app.ui.designsystem.theme.TextSecondary
import org.koin.compose.viewmodel.koinViewModel

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
                viewModel.clearError()
            },
            label = "potwierd\u017a has\u0142o",
            placeholder = "has\u0142o",
            error =
                if (confirmPassword.isNotEmpty() && confirmPassword != password) {
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
            Text(
                text = "akceptuję ",
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

        Spacer(modifier = Modifier.height(PoziomkiTheme.spacing.lg))

        AppButton(
            text = "zarejestruj si\u0119",
            onClick = {
                val placeholderName = email.substringBefore("@").ifBlank { "User" }
                viewModel.signUp(email, password, placeholderName, onRegisterSuccess, onUserExists)
            },
            enabled =
                email.isNotBlank() &&
                    password.length >= 8 &&
                    password == confirmPassword &&
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

    if (showPolicy) {
        androidx.compose.ui.window.Dialog(
            onDismissRequest = { showPolicy = false },
            properties =
                androidx.compose.ui.window
                    .DialogProperties(usePlatformDefaultWidth = false),
        ) {
            androidx.compose.material3.Surface(
                modifier =
                    Modifier
                        .fillMaxSize()
                        .padding(16.dp),
                shape =
                    androidx.compose.foundation.shape
                        .RoundedCornerShape(20.dp),
                color = MaterialTheme.colorScheme.background,
            ) {
                Column(
                    modifier =
                        Modifier
                            .fillMaxSize()
                            .verticalScroll(rememberScrollState())
                            .padding(24.dp),
                ) {
                    Text(
                        text = "polityka prywatności",
                        fontFamily = MontserratFamily,
                        fontWeight = FontWeight.ExtraBold,
                        fontSize = 22.sp,
                        color = TextPrimary,
                    )
                    Spacer(modifier = Modifier.height(16.dp))
                    Text(
                        text = privacyPolicyText,
                        fontFamily = NunitoFamily,
                        fontWeight = FontWeight.Normal,
                        fontSize = 14.sp,
                        color = TextSecondary,
                        lineHeight = 22.sp,
                    )
                    Spacer(modifier = Modifier.height(24.dp))
                    AppButton(
                        text = "zamknij",
                        onClick = { showPolicy = false },
                        modifier = Modifier.fillMaxWidth(),
                    )
                }
            }
        }
    }
}

private val privacyPolicyText =
    """
    Niniejsza Polityka Prywatności określa zasady przetwarzania danych osobowych użytkowników aplikacji Poziomki.

    1. Administrator danych
    Administratorem danych osobowych jest zespół Poziomki. Kontakt: kontakt@poziomki.app

    2. Zakres zbieranych danych
    Zbieramy następujące dane: adres e-mail, imię, zdjęcia profilowe, zainteresowania oraz dane dotyczące uczestnictwa w wydarzeniach.

    3. Cel przetwarzania
    Dane przetwarzamy w celu: świadczenia usług aplikacji, dopasowywania rekomendacji wydarzeń i profili, komunikacji między użytkownikami oraz zapewnienia bezpieczeństwa.

    4. Udostępnianie danych
    Dane osobowe nie są sprzedawane ani udostępniane podmiotom trzecim w celach marketingowych. Dane mogą być udostępniane wyłącznie na żądanie organów uprawnionych na podstawie przepisów prawa.

    5. Przechowywanie danych
    Dane przechowywane są na serwerach zlokalizowanych w Unii Europejskiej. Dane są przechowywane przez okres korzystania z aplikacji oraz do 30 dni po usunięciu konta.

    6. Prawa użytkownika
    Każdy użytkownik ma prawo do: dostępu do swoich danych, ich sprostowania, usunięcia, ograniczenia przetwarzania, przenoszenia danych oraz wniesienia sprzeciwu. Eksport i usunięcie danych dostępne są w ustawieniach aplikacji.

    7. Pliki cookies i analityka
    Aplikacja nie wykorzystuje plików cookies. Zbieramy anonimowe dane analityczne w celu poprawy jakości usług.

    8. Zmiany polityki
    O istotnych zmianach w polityce prywatności użytkownicy zostaną poinformowani poprzez powiadomienie w aplikacji.

    9. Kontakt
    Pytania dotyczące prywatności prosimy kierować na adres: kontakt@poziomki.app

    Data ostatniej aktualizacji: marzec 2026
    """.trimIndent()
