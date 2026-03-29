package com.poziomki.app.ui.feature.profile

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.asPaddingValues
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.navigationBars
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.statusBars
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.adamglin.PhosphorIcons
import com.adamglin.phosphoricons.Bold
import com.adamglin.phosphoricons.bold.DownloadSimple
import com.adamglin.phosphoricons.bold.Trash
import com.poziomki.app.ui.designsystem.components.AppButton
import com.poziomki.app.ui.designsystem.components.ButtonVariant
import com.poziomki.app.ui.designsystem.components.PoziomkiPasswordField
import com.poziomki.app.ui.designsystem.components.ScreenHeader
import com.poziomki.app.ui.designsystem.components.SectionLabel
import com.poziomki.app.ui.designsystem.theme.MontserratFamily
import com.poziomki.app.ui.designsystem.theme.PoziomkiTheme
import com.poziomki.app.ui.designsystem.theme.TextMuted
import com.poziomki.app.ui.designsystem.theme.TextSecondary
import com.poziomki.app.ui.shared.rememberExportFileSaver
import org.koin.compose.viewmodel.koinViewModel

@Suppress("LongMethod")
@Composable
fun PrivacyScreen(
    onBack: () -> Unit,
    onPasswordChanged: () -> Unit = {},
    onAccountDeleted: () -> Unit = {},
    viewModel: PrivacyViewModel = koinViewModel(),
) {
    val state by viewModel.state.collectAsState()
    val nunito = MontserratFamily
    val navBarBottom = WindowInsets.navigationBars.asPaddingValues().calculateBottomPadding()
    var showDeleteDialog by remember { mutableStateOf(false) }
    var showPasswordDialog by remember { mutableStateOf(false) }

    val saveExport =
        rememberExportFileSaver(
            onSaved = { viewModel.onExportSaved() },
            onCancelled = { viewModel.clearExportBytes() },
        )

    LaunchedEffect(state.exportBytes) {
        state.exportBytes?.let { bytes ->
            saveExport(bytes, "poziomki-export.zip")
        }
    }

    Column(
        modifier =
            Modifier
                .fillMaxSize()
                .background(MaterialTheme.colorScheme.background),
    ) {
        val statusBarPadding = WindowInsets.statusBars.asPaddingValues().calculateTopPadding()
        ScreenHeader(
            title = "prywatność",
            onBack = onBack,
            modifier = Modifier.padding(top = statusBarPadding),
        )

        PrivacyContent(
            state = state,
            navBarBottom = navBarBottom,
            nunito = nunito,
            onOpenPasswordDialog = { showPasswordDialog = true },
            onExport = { viewModel.exportData() },
            onDelete = { showDeleteDialog = true },
        )
    }

    if (showPasswordDialog && state.error == null) {
        ChangePasswordDialog(
            isLoading = state.isChangingPassword,
            onDismiss = { showPasswordDialog = false },
            onSubmit = { current, new, confirm ->
                viewModel.changePassword(current, new, confirm) {
                    showPasswordDialog = false
                    onPasswordChanged()
                }
            },
        )
    }

    if (showDeleteDialog && state.error == null) {
        DeleteAccountDialog(
            isLoading = state.isDeleting,
            onDismiss = { showDeleteDialog = false },
            onConfirm = { password ->
                viewModel.deleteAccount(password) {
                    showDeleteDialog = false
                    onAccountDeleted()
                }
            },
        )
    }
}

@Suppress("LongParameterList")
@Composable
private fun PrivacyContent(
    state: PrivacyState,
    navBarBottom: androidx.compose.ui.unit.Dp,
    nunito: androidx.compose.ui.text.font.FontFamily,
    onOpenPasswordDialog: () -> Unit,
    onExport: () -> Unit,
    onDelete: () -> Unit,
) {
    Column(
        modifier =
            Modifier
                .fillMaxSize()
                .verticalScroll(rememberScrollState())
                .padding(horizontal = PoziomkiTheme.spacing.lg),
    ) {
        Spacer(modifier = Modifier.height(PoziomkiTheme.spacing.md))
        PrivacyMessages(state.error, state.passwordSuccessMessage, nunito)

        SectionLabel("HASŁO", color = TextMuted)
        Spacer(modifier = Modifier.height(PoziomkiTheme.spacing.md))
        AppButton(
            text = "zmień hasło",
            onClick = onOpenPasswordDialog,
            variant = ButtonVariant.OUTLINE,
        )

        Spacer(modifier = Modifier.height(PoziomkiTheme.spacing.xl))
        ExportDataSection(state.exportSuccess, state.isExporting, onExport, nunito)
        Spacer(modifier = Modifier.height(PoziomkiTheme.spacing.xl))
        DeleteAccountSection(state.isDeleting, onDelete, nunito)
        Spacer(modifier = Modifier.height(navBarBottom + PoziomkiTheme.spacing.xl))
    }
}

@Composable
private fun PrivacyMessages(
    error: String?,
    passwordSuccessMessage: String?,
    nunito: androidx.compose.ui.text.font.FontFamily,
) {
    error?.let {
        Text(
            text = it,
            fontFamily = nunito,
            fontWeight = FontWeight.Medium,
            fontSize = 14.sp,
            color = MaterialTheme.colorScheme.error,
            modifier = Modifier.padding(bottom = PoziomkiTheme.spacing.md),
        )
    }

    passwordSuccessMessage?.let {
        Text(
            text = it,
            fontFamily = nunito,
            fontWeight = FontWeight.Medium,
            fontSize = 14.sp,
            color = TextSecondary,
            modifier = Modifier.padding(bottom = PoziomkiTheme.spacing.md),
        )
    }
}

@Composable
private fun ExportDataSection(
    exportSuccess: Boolean,
    isExporting: Boolean,
    onExport: () -> Unit,
    nunito: androidx.compose.ui.text.font.FontFamily,
) {
    SectionLabel("TWOJE DANE", color = TextMuted)
    Spacer(modifier = Modifier.height(PoziomkiTheme.spacing.sm))
    Text(
        text =
            "Możesz wyeksportować wszystkie dane powiązane z Twoim kontem. " +
                "Otrzymasz plik zawierający Twoje informacje profilowe, preferencje, " +
                "historię aktywności i zdjęcia.",
        fontFamily = nunito,
        fontWeight = FontWeight.Normal,
        fontSize = 14.sp,
        color = TextSecondary,
        lineHeight = 20.sp,
    )
    Spacer(modifier = Modifier.height(PoziomkiTheme.spacing.md))
    AppButton(
        text = "eksportuj dane",
        onClick = onExport,
        variant = ButtonVariant.OUTLINE,
        icon = PhosphorIcons.Bold.DownloadSimple,
        loading = isExporting,
    )

    if (exportSuccess) {
        Spacer(modifier = Modifier.height(PoziomkiTheme.spacing.md))
        Text(
            text = "Dane wyeksportowane pomyślnie.",
            fontFamily = nunito,
            fontWeight = FontWeight.SemiBold,
            fontSize = 14.sp,
            color = TextSecondary,
        )
    }
}

@Composable
private fun DeleteAccountSection(
    isDeleting: Boolean,
    onDelete: () -> Unit,
    nunito: androidx.compose.ui.text.font.FontFamily,
) {
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
    AppButton(
        text = "usuń konto",
        onClick = onDelete,
        variant = ButtonVariant.DESTRUCTIVE,
        icon = PhosphorIcons.Bold.Trash,
        loading = isDeleting,
    )
}

@Composable
private fun DeleteAccountDialog(
    isLoading: Boolean,
    onDismiss: () -> Unit,
    onConfirm: (String) -> Unit,
) {
    var password by remember { mutableStateOf("") }

    AlertDialog(
        onDismissRequest = { if (!isLoading) onDismiss() },
        title = {
            Text(
                text = "Usunąć konto?",
                fontFamily = MontserratFamily,
                fontWeight = FontWeight.Bold,
            )
        },
        text = {
            Column {
                Text(
                    text = "Ta operacja jest nieodwracalna. Wpisz hasło, aby potwierdzić.",
                    fontFamily = MontserratFamily,
                    fontWeight = FontWeight.Normal,
                    fontSize = 14.sp,
                    color = TextSecondary,
                )
                Spacer(modifier = Modifier.height(16.dp))
                PoziomkiPasswordField(
                    value = password,
                    onValueChange = { password = it },
                    placeholder = "Hasło",
                )
            }
        },
        confirmButton = {
            TextButton(
                onClick = { onConfirm(password) },
                enabled = password.isNotBlank() && !isLoading,
            ) {
                Text(
                    text = "Usuń",
                    color = MaterialTheme.colorScheme.error,
                    fontFamily = MontserratFamily,
                    fontWeight = FontWeight.Bold,
                )
            }
        },
        dismissButton = {
            TextButton(
                onClick = onDismiss,
                enabled = !isLoading,
            ) {
                Text(
                    text = "Anuluj",
                    fontFamily = MontserratFamily,
                )
            }
        },
    )
}
