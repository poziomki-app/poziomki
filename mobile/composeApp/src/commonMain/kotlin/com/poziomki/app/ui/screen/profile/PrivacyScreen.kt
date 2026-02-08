package com.poziomki.app.ui.screen.profile

import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.asPaddingValues
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.statusBars
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.filled.Delete
import androidx.compose.material.icons.filled.Download
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.poziomki.app.ui.theme.Error
import com.poziomki.app.ui.theme.NunitoFamily
import com.poziomki.app.ui.theme.PoziomkiTheme
import com.poziomki.app.ui.theme.Primary
import com.poziomki.app.ui.theme.TextMuted
import com.poziomki.app.ui.theme.TextPrimary
import com.poziomki.app.ui.theme.TextSecondary

@Composable
fun PrivacyScreen(onBack: () -> Unit) {
    val nunito = NunitoFamily

    Column(
        modifier =
            Modifier
                .fillMaxSize()
                .background(MaterialTheme.colorScheme.background),
    ) {
        // Top bar
        val statusBarPadding = WindowInsets.statusBars.asPaddingValues().calculateTopPadding()
        Row(
            modifier =
                Modifier
                    .fillMaxWidth()
                    .padding(
                        start = PoziomkiTheme.spacing.sm,
                        end = PoziomkiTheme.spacing.sm,
                        top = statusBarPadding + PoziomkiTheme.spacing.sm,
                        bottom = PoziomkiTheme.spacing.sm,
                    ),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            IconButton(onClick = onBack) {
                Icon(
                    Icons.AutoMirrored.Filled.ArrowBack,
                    contentDescription = "Wstecz",
                    tint = TextPrimary,
                )
            }
            Text(
                text = "prywatność",
                fontFamily = com.poziomki.app.ui.theme.MontserratFamily,
                fontWeight = FontWeight.ExtraBold,
                fontSize = 20.sp,
                color = TextPrimary,
            )
        }

        Column(
            modifier =
                Modifier
                    .fillMaxSize()
                    .verticalScroll(rememberScrollState())
                    .padding(horizontal = PoziomkiTheme.spacing.lg),
        ) {
            Spacer(modifier = Modifier.height(PoziomkiTheme.spacing.md))

            // TWOJE DANE section
            Text(
                text = "TWOJE DANE",
                fontFamily = nunito,
                fontWeight = FontWeight.SemiBold,
                fontSize = 13.sp,
                color = TextMuted,
            )
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
            Surface(
                modifier =
                    Modifier
                        .fillMaxWidth()
                        .clip(RoundedCornerShape(50))
                        .clickable { /* TODO: export data */ },
                shape = RoundedCornerShape(50),
                color = Color.Transparent,
                border = BorderStroke(1.dp, Primary),
            ) {
                Row(
                    modifier =
                        Modifier
                            .fillMaxWidth()
                            .padding(vertical = 14.dp),
                    horizontalArrangement = androidx.compose.foundation.layout.Arrangement.Center,
                    verticalAlignment = Alignment.CenterVertically,
                ) {
                    Icon(
                        Icons.Filled.Download,
                        contentDescription = null,
                        tint = Primary,
                        modifier = Modifier.size(20.dp),
                    )
                    Spacer(modifier = Modifier.width(8.dp))
                    Text(
                        text = "eksportuj dane",
                        fontFamily = nunito,
                        fontWeight = FontWeight.SemiBold,
                        fontSize = 16.sp,
                        color = Primary,
                    )
                }
            }

            Spacer(modifier = Modifier.height(PoziomkiTheme.spacing.xl))

            // USUŃ KONTO section
            Text(
                text = "USUŃ KONTO",
                fontFamily = nunito,
                fontWeight = FontWeight.SemiBold,
                fontSize = 13.sp,
                color = TextMuted,
            )
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
            Surface(
                modifier =
                    Modifier
                        .fillMaxWidth()
                        .clip(RoundedCornerShape(50))
                        .clickable { /* TODO: delete account */ },
                shape = RoundedCornerShape(50),
                color = Error,
            ) {
                Row(
                    modifier =
                        Modifier
                            .fillMaxWidth()
                            .padding(vertical = 14.dp),
                    horizontalArrangement = androidx.compose.foundation.layout.Arrangement.Center,
                    verticalAlignment = Alignment.CenterVertically,
                ) {
                    Icon(
                        Icons.Filled.Delete,
                        contentDescription = null,
                        tint = Color.White,
                        modifier = Modifier.size(20.dp),
                    )
                    Spacer(modifier = Modifier.width(8.dp))
                    Text(
                        text = "usuń konto",
                        fontFamily = nunito,
                        fontWeight = FontWeight.SemiBold,
                        fontSize = 16.sp,
                        color = Color.White,
                    )
                }
            }

            Spacer(modifier = Modifier.height(PoziomkiTheme.spacing.xl))
        }
    }
}
