package com.poziomki.app.ui.feature.profile

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.asPaddingValues
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
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
import com.poziomki.app.ui.designsystem.components.ButtonVariant
import com.poziomki.app.ui.designsystem.components.PoziomkiButton
import com.poziomki.app.ui.designsystem.components.PoziomkiPasswordField
import com.poziomki.app.ui.designsystem.components.ScreenHeader
import com.poziomki.app.ui.designsystem.components.SectionLabel
import com.poziomki.app.ui.designsystem.theme.NunitoFamily
import com.poziomki.app.ui.designsystem.theme.PoziomkiTheme
import com.poziomki.app.ui.designsystem.theme.TextMuted
import com.poziomki.app.ui.designsystem.theme.TextSecondary
import org.koin.compose.viewmodel.koinViewModel

private data class PasswordFormState(
    val currentPassword: String,
    val newPassword: String,
    val confirmPassword: String,
)

private data class PasswordSectionProps(
    val form: PasswordFormState,
    val isLoading: Boolean,
    val onCurrentPasswordChange: (String) -> Unit,
    val onNewPasswordChange: (String) -> Unit,
    val onConfirmPasswordChange: (String) -> Unit,
    val onSubmit: () -> Unit,
)

private data class PasswordFormBindings(
    val form: PasswordFormState,
    val onCurrentPasswordChange: (String) -> Unit,
    val onNewPasswordChange: (String) -> Unit,
    val onConfirmPasswordChange: (String) -> Unit,
)

private data class PasswordFormController(
    val bindings: PasswordFormBindings,
    val clear: () -> Unit,
)

private data class PrivacyContentProps(
    val state: PrivacyState,
    val navBarBottom: androidx.compose.ui.unit.Dp,
    val passwordSection: PasswordSectionProps,
    val onExport: () -> Unit,
    val onDelete: () -> Unit,
)

private data class PrivacyActions(
    val onChangePassword: () -> Unit,
    val onExport: () -> Unit,
    val onDelete: () -> Unit,
)

@Composable
fun PrivacyScreen(
    onBack: () -> Unit,
    onPasswordChanged: () -> Unit = {},
    onAccountDeleted: () -> Unit = {},
    viewModel: PrivacyViewModel = koinViewModel(),
) {
    val state by viewModel.state.collectAsState()
    val nunito = NunitoFamily
    val navBarBottom = WindowInsets.navigationBars.asPaddingValues().calculateBottomPadding()
    var showDeleteDialog by remember { mutableStateOf(false) }
    val passwordController = rememberPasswordFormController()
    val actions =
        PrivacyActions(
            onChangePassword = {
                viewModel.changePassword(
                    currentPassword = passwordController.bindings.form.currentPassword,
                    newPassword = passwordController.bindings.form.newPassword,
                    confirmPassword = passwordController.bindings.form.confirmPassword,
                ) {
                    passwordController.clear()
                    onPasswordChanged()
                }
            },
            onExport = { viewModel.exportData() },
            onDelete = { showDeleteDialog = true },
        )
    val contentProps =
        buildPrivacyContentProps(
            state = state,
            navBarBottom = navBarBottom,
            passwordBindings = passwordController.bindings,
            actions = actions,
        )

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

        PrivacyContent(props = contentProps, nunito = nunito)
    }

    DeleteAccountDialogHost(
        showDialog = showDeleteDialog,
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

@Composable
private fun rememberPasswordFormController(): PasswordFormController {
    var currentPassword by remember { mutableStateOf("") }
    var newPassword by remember { mutableStateOf("") }
    var confirmPassword by remember { mutableStateOf("") }

    return PasswordFormController(
        bindings =
            PasswordFormBindings(
                form =
                    PasswordFormState(
                        currentPassword = currentPassword,
                        newPassword = newPassword,
                        confirmPassword = confirmPassword,
                    ),
                onCurrentPasswordChange = { currentPassword = it },
                onNewPasswordChange = { newPassword = it },
                onConfirmPasswordChange = { confirmPassword = it },
            ),
        clear = {
            currentPassword = ""
            newPassword = ""
            confirmPassword = ""
        },
    )
}

private fun buildPrivacyContentProps(
    state: PrivacyState,
    navBarBottom: androidx.compose.ui.unit.Dp,
    passwordBindings: PasswordFormBindings,
    actions: PrivacyActions,
): PrivacyContentProps =
    PrivacyContentProps(
        state = state,
        navBarBottom = navBarBottom,
        passwordSection =
            PasswordSectionProps(
                form = passwordBindings.form,
                isLoading = state.isChangingPassword,
                onCurrentPasswordChange = passwordBindings.onCurrentPasswordChange,
                onNewPasswordChange = passwordBindings.onNewPasswordChange,
                onConfirmPasswordChange = passwordBindings.onConfirmPasswordChange,
                onSubmit = actions.onChangePassword,
            ),
        onExport = actions.onExport,
        onDelete = actions.onDelete,
    )

@Composable
private fun PrivacyContent(
    props: PrivacyContentProps,
    nunito: androidx.compose.ui.text.font.FontFamily,
) {
    Column(
        modifier =
            Modifier
                .fillMaxSize()
                .verticalScroll(rememberScrollState())
                .padding(horizontal = PoziomkiTheme.spacing.lg),
    ) {
        Spacer(modifier = Modifier.height(PoziomkiTheme.spacing.md))
        PrivacyMessages(
            error = props.state.error,
            passwordSuccessMessage = props.state.passwordSuccessMessage,
            nunito = nunito,
        )
        ChangePasswordSection(
            props = props.passwordSection,
            nunito = nunito,
        )
        Spacer(modifier = Modifier.height(PoziomkiTheme.spacing.xl))
        ExportDataSection(
            exportedJson = props.state.exportedJson,
            isExporting = props.state.isExporting,
            onExport = props.onExport,
            nunito = nunito,
        )
        Spacer(modifier = Modifier.height(PoziomkiTheme.spacing.xl))
        DeleteAccountSection(
            isDeleting = props.state.isDeleting,
            onDelete = props.onDelete,
            nunito = nunito,
        )
        Spacer(modifier = Modifier.height(props.navBarBottom + PoziomkiTheme.spacing.xl))
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
private fun ChangePasswordSection(
    props: PasswordSectionProps,
    nunito: androidx.compose.ui.text.font.FontFamily,
) {
    SectionLabel("ZMIEŃ HASŁO", color = TextMuted)
    Spacer(modifier = Modifier.height(PoziomkiTheme.spacing.sm))
    Text(
        text = "Po zmianie hasła wylogujemy Cię ze wszystkich urządzeń i poprosimy o ponowne zalogowanie.",
        fontFamily = nunito,
        fontWeight = FontWeight.Normal,
        fontSize = 14.sp,
        color = TextSecondary,
        lineHeight = 20.sp,
    )
    Spacer(modifier = Modifier.height(PoziomkiTheme.spacing.md))
    PoziomkiPasswordField(
        value = props.form.currentPassword,
        onValueChange = props.onCurrentPasswordChange,
        placeholder = "Aktualne hasło",
    )
    Spacer(modifier = Modifier.height(PoziomkiTheme.spacing.sm))
    PoziomkiPasswordField(
        value = props.form.newPassword,
        onValueChange = props.onNewPasswordChange,
        placeholder = "Nowe hasło",
    )
    Spacer(modifier = Modifier.height(PoziomkiTheme.spacing.sm))
    PoziomkiPasswordField(
        value = props.form.confirmPassword,
        onValueChange = props.onConfirmPasswordChange,
        placeholder = "Powtórz nowe hasło",
    )
    Spacer(modifier = Modifier.height(PoziomkiTheme.spacing.md))
    PoziomkiButton(
        text = "zmień hasło",
        onClick = props.onSubmit,
        variant = ButtonVariant.OUTLINE,
        loading = props.isLoading,
    )
}

@Composable
private fun ExportDataSection(
    exportedJson: String?,
    isExporting: Boolean,
    onExport: () -> Unit,
    nunito: androidx.compose.ui.text.font.FontFamily,
) {
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
    PoziomkiButton(
        text = "eksportuj dane",
        onClick = onExport,
        variant = ButtonVariant.OUTLINE,
        icon = PhosphorIcons.Bold.DownloadSimple,
        loading = isExporting,
    )

    exportedJson?.let { json ->
        Spacer(modifier = Modifier.height(PoziomkiTheme.spacing.md))
        Text(
            text = "Dane wyeksportowane pomyślnie:",
            fontFamily = nunito,
            fontWeight = FontWeight.SemiBold,
            fontSize = 14.sp,
            color = TextSecondary,
        )
        Spacer(modifier = Modifier.height(PoziomkiTheme.spacing.sm))
        Text(
            text = json.take(2000),
            fontFamily = nunito,
            fontWeight = FontWeight.Normal,
            fontSize = 12.sp,
            color = TextMuted,
            lineHeight = 16.sp,
            modifier =
                Modifier
                    .fillMaxWidth()
                    .background(
                        MaterialTheme.colorScheme.surfaceVariant,
                        MaterialTheme.shapes.small,
                    ).padding(PoziomkiTheme.spacing.md),
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
    PoziomkiButton(
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
                fontFamily = NunitoFamily,
                fontWeight = FontWeight.Bold,
            )
        },
        text = {
            Column {
                Text(
                    text = "Ta operacja jest nieodwracalna. Wpisz hasło, aby potwierdzić.",
                    fontFamily = NunitoFamily,
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
                    fontFamily = NunitoFamily,
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
                    fontFamily = NunitoFamily,
                )
            }
        },
    )
}

@Composable
private fun DeleteAccountDialogHost(
    showDialog: Boolean,
    isLoading: Boolean,
    onDismiss: () -> Unit,
    onConfirm: (String) -> Unit,
) {
    if (showDialog) {
        DeleteAccountDialog(
            isLoading = isLoading,
            onDismiss = onDismiss,
            onConfirm = onConfirm,
        )
    }
}
