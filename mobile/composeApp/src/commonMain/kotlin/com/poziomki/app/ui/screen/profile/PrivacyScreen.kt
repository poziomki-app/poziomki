package com.poziomki.app.ui.screen.profile

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.asPaddingValues
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.navigationBars
import androidx.compose.foundation.layout.statusBars
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Delete
import androidx.compose.material.icons.filled.Download
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.poziomki.app.ui.component.ButtonVariant
import com.poziomki.app.ui.component.PoziomkiButton
import com.poziomki.app.ui.component.ScreenHeader
import com.poziomki.app.ui.component.SectionLabel
import com.poziomki.app.ui.theme.NunitoFamily
import com.poziomki.app.ui.theme.PoziomkiTheme
import com.poziomki.app.ui.theme.TextMuted
import com.poziomki.app.ui.theme.TextSecondary

@Composable
fun PrivacyScreen(onBack: () -> Unit) {
    val nunito = NunitoFamily
    val navBarBottom = WindowInsets.navigationBars.asPaddingValues().calculateBottomPadding()

    Column(
        modifier =
            Modifier
                .fillMaxSize()
                .background(MaterialTheme.colorScheme.background),
    ) {
        // Top bar
        val statusBarPadding = WindowInsets.statusBars.asPaddingValues().calculateTopPadding()
        ScreenHeader(
            title = "prywatność",
            onBack = onBack,
            modifier = Modifier.padding(top = statusBarPadding),
        )

        Column(
            modifier =
                Modifier
                    .fillMaxSize()
                    .verticalScroll(rememberScrollState())
                    .padding(horizontal = PoziomkiTheme.spacing.lg),
        ) {
            Spacer(modifier = Modifier.height(PoziomkiTheme.spacing.md))

            // TWOJE DANE section
            SectionLabel("TWOJE DANE", color = TextMuted)
            Spacer(modifier = Modifier.height(PoziomkiTheme.spacing.sm))
            Text(
                text =
                    "Możesz wyeksportować wszystkie dane powiązane z Twoim kontem. " +
                        "Otrzymasz plik zawierający Twoje informacje profilowe, preferencje " +
                        "i historię aktywności.",
                fontFamily = nunito,
                fontWeight = FontWeight.Normal,
                fontSize = 14.sp,
                color = TextSecondary,
                lineHeight = 20.sp,
            )
            Spacer(modifier = Modifier.height(PoziomkiTheme.spacing.md))

            // Export button
            PoziomkiButton(
                text = "eksportuj dane",
                onClick = { /* TODO: export data */ },
                variant = ButtonVariant.OUTLINE,
                icon = Icons.Filled.Download,
            )

            Spacer(modifier = Modifier.height(PoziomkiTheme.spacing.xl))

            // USUŃ KONTO section
            SectionLabel("USUŃ KONTO", color = TextMuted)
            Spacer(modifier = Modifier.height(PoziomkiTheme.spacing.sm))
            Text(
                text =
                    "Usunięcie konta jest nieodwracalne. Wszystkie Twoje dane, " +
                        "w tym profil, wiadomości i historia aktywności, " +
                        "zostaną trwale usunięte.",
                fontFamily = nunito,
                fontWeight = FontWeight.Normal,
                fontSize = 14.sp,
                color = TextSecondary,
                lineHeight = 20.sp,
            )
            Spacer(modifier = Modifier.height(PoziomkiTheme.spacing.md))

            // Delete button
            PoziomkiButton(
                text = "usuń konto",
                onClick = { /* TODO: delete account */ },
                variant = ButtonVariant.DESTRUCTIVE,
                icon = Icons.Filled.Delete,
            )

            Spacer(modifier = Modifier.height(navBarBottom + PoziomkiTheme.spacing.xl))
        }
    }
}
