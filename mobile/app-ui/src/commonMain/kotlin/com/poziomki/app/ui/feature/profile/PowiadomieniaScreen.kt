package com.poziomki.app.ui.feature.profile

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.asPaddingValues
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.navigationBars
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.statusBars
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Switch
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.draw.clip
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.poziomki.app.ui.designsystem.components.ScreenHeader
import com.poziomki.app.ui.designsystem.components.SectionLabel
import com.poziomki.app.ui.designsystem.theme.NunitoFamily
import com.poziomki.app.ui.designsystem.theme.PoziomkiTheme
import com.poziomki.app.ui.designsystem.theme.TextMuted
import com.poziomki.app.ui.designsystem.theme.TextSecondary
import org.koin.compose.viewmodel.koinViewModel

@Suppress("LongMethod")
@Composable
fun PowiadomieniaScreen(
    onBack: () -> Unit,
    viewModel: PowiadomieniaViewModel = koinViewModel(),
) {
    val state by viewModel.state.collectAsState()
    val nunito = NunitoFamily
    val navBarBottom = WindowInsets.navigationBars.asPaddingValues().calculateBottomPadding()

    Column(
        modifier =
            Modifier
                .fillMaxSize()
                .background(MaterialTheme.colorScheme.background),
    ) {
        val statusBarPadding = WindowInsets.statusBars.asPaddingValues().calculateTopPadding()
        ScreenHeader(
            title = "powiadomienia",
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

            NotificationToggleRow(
                label = "powiadomienia",
                description = "Główny włącznik. Wyłącz, aby nic nie wpadało.",
                checked = state.masterEnabled,
                enabled = true,
                onCheckedChange = viewModel::toggleMaster,
                nunito = nunito,
            )

            Spacer(modifier = Modifier.height(PoziomkiTheme.spacing.xl))
            SectionLabel("KANAŁY", color = TextMuted)
            Spacer(modifier = Modifier.height(PoziomkiTheme.spacing.sm))

            NotificationToggleRow(
                label = "wiadomości prywatne",
                description = "Powiadomienia o nowych wiadomościach od osób.",
                checked = state.dms,
                enabled = state.masterEnabled,
                onCheckedChange = viewModel::toggleDms,
                nunito = nunito,
            )
            Spacer(modifier = Modifier.height(PoziomkiTheme.spacing.md))
            NotificationToggleRow(
                label = "czaty wydarzeń",
                description = "Powiadomienia o nowych wiadomościach w czatach wydarzeń.",
                checked = state.eventChats,
                enabled = state.masterEnabled,
                onCheckedChange = viewModel::toggleEventChats,
                nunito = nunito,
            )
            Spacer(modifier = Modifier.height(PoziomkiTheme.spacing.md))
            NotificationToggleRow(
                label = "wydarzenia w tagach",
                description = "Nowe wydarzenia pasujące do obserwowanych tagów.",
                checked = state.tagEvents,
                enabled = state.masterEnabled,
                onCheckedChange = viewModel::toggleTagEvents,
                nunito = nunito,
                trailingBadge = "wkrótce",
            )

            Spacer(modifier = Modifier.height(PoziomkiTheme.spacing.xl))
            Text(
                text = "Wyciszysz pojedynczy czat w jego ustawieniach (przycisk w nagłówku).",
                fontFamily = nunito,
                fontWeight = FontWeight.Normal,
                fontSize = 12.sp,
                color = TextSecondary,
                lineHeight = 16.sp,
            )

            Spacer(modifier = Modifier.height(navBarBottom + PoziomkiTheme.spacing.xl))
        }
    }
}

@Suppress("LongParameterList")
@Composable
private fun NotificationToggleRow(
    label: String,
    description: String,
    checked: Boolean,
    enabled: Boolean,
    onCheckedChange: (Boolean) -> Unit,
    nunito: androidx.compose.ui.text.font.FontFamily,
    trailingBadge: String? = null,
) {
    val alpha = if (enabled) 1f else 0.5f
    Row(
        modifier = Modifier.fillMaxWidth().alpha(alpha),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.SpaceBetween,
    ) {
        Column(modifier = Modifier.weight(1f)) {
            Row(verticalAlignment = Alignment.CenterVertically) {
                Text(
                    text = label,
                    fontFamily = nunito,
                    fontWeight = FontWeight.SemiBold,
                    fontSize = 15.sp,
                    color = MaterialTheme.colorScheme.onBackground,
                )
                if (trailingBadge != null) {
                    Spacer(modifier = Modifier.width(8.dp))
                    Box(
                        modifier =
                            Modifier
                                .clip(RoundedCornerShape(6.dp))
                                .background(MaterialTheme.colorScheme.surfaceVariant)
                                .padding(horizontal = 6.dp, vertical = 2.dp),
                    ) {
                        Text(
                            text = trailingBadge,
                            fontFamily = nunito,
                            fontWeight = FontWeight.SemiBold,
                            fontSize = 10.sp,
                            color = TextSecondary,
                        )
                    }
                }
            }
            Text(
                text = description,
                fontFamily = nunito,
                fontWeight = FontWeight.Normal,
                fontSize = 13.sp,
                color = TextSecondary,
                lineHeight = 18.sp,
            )
        }
        Spacer(modifier = Modifier.width(8.dp))
        Switch(
            checked = checked,
            onCheckedChange = onCheckedChange,
            enabled = enabled,
        )
    }
}
