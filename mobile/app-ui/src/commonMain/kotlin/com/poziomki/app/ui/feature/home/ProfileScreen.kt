package com.poziomki.app.ui.feature.home

import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Snackbar
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.pulltorefresh.PullToRefreshBox
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.adamglin.PhosphorIcons
import com.adamglin.phosphoricons.Bold
import com.adamglin.phosphoricons.bold.CaretRight
import com.adamglin.phosphoricons.bold.PencilSimple
import com.adamglin.phosphoricons.bold.Shield
import com.adamglin.phosphoricons.bold.SignOut
import com.poziomki.app.ui.designsystem.components.ConfirmDialog
import com.poziomki.app.ui.designsystem.components.EmptyView
import com.poziomki.app.ui.designsystem.components.LoadingView
import com.poziomki.app.ui.designsystem.components.ProfileCard
import com.poziomki.app.ui.designsystem.components.ScreenHeader
import com.poziomki.app.ui.designsystem.theme.Border
import com.poziomki.app.ui.designsystem.theme.Error
import com.poziomki.app.ui.designsystem.theme.NunitoFamily
import com.poziomki.app.ui.designsystem.theme.PoziomkiTheme
import com.poziomki.app.ui.designsystem.theme.TextMuted
import com.poziomki.app.ui.designsystem.theme.TextPrimary
import com.poziomki.app.ui.navigation.LocalNavBarPadding
import kotlinx.coroutines.delay
import org.koin.compose.viewmodel.koinViewModel

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun ProfileScreen(
    onNavigateToEdit: () -> Unit,
    onNavigateToPrivacy: () -> Unit,
    onNavigateToProfileView: (String) -> Unit,
    onSignOut: () -> Unit,
    viewModel: ProfileViewModel = koinViewModel(),
) {
    val state by viewModel.state.collectAsState()
    var showLogoutDialog by remember { mutableStateOf(false) }

    Column(
        modifier =
            Modifier
                .fillMaxSize()
                .background(MaterialTheme.colorScheme.background),
    ) {
        ScreenHeader(title = "profil")

        Box(modifier = Modifier.fillMaxSize()) {
            when {
                state.isLoading && state.profile == null -> {
                    LoadingView()
                }

                state.profile != null -> {
                    state.profile?.let { profile ->
                        PullToRefreshBox(
                            isRefreshing = state.isRefreshing,
                            onRefresh = { viewModel.pullToRefresh() },
                        ) {
                            Column(
                                modifier =
                                    Modifier
                                        .fillMaxSize()
                                        .verticalScroll(rememberScrollState())
                                        .padding(horizontal = PoziomkiTheme.spacing.lg),
                            ) {
                                // ProfileCard
                                ProfileCard(
                                    name = profile.name,
                                    program = profile.program,
                                    profilePicture = profile.profilePicture,
                                    gradientStart = profile.gradientStart,
                                    gradientEnd = profile.gradientEnd,
                                    onClick = { onNavigateToProfileView(profile.id) },
                                )

                                Spacer(modifier = Modifier.height(PoziomkiTheme.spacing.lg))

                                // Settings menu
                                Column {
                                    SettingsMenuItem(
                                        icon = PhosphorIcons.Bold.PencilSimple,
                                        label = "edytuj profil",
                                        onClick = onNavigateToEdit,
                                    )
                                    HorizontalDivider(color = Border, thickness = 1.dp)
                                    SettingsMenuItem(
                                        icon = PhosphorIcons.Bold.Shield,
                                        label = "prywatność",
                                        onClick = onNavigateToPrivacy,
                                    )
                                }

                                Spacer(modifier = Modifier.height(PoziomkiTheme.spacing.lg))

                                // Logout button
                                Surface(
                                    modifier = Modifier.fillMaxWidth(),
                                    shape = RoundedCornerShape(16.dp),
                                    color = MaterialTheme.colorScheme.background,
                                    border = BorderStroke(1.dp, Error),
                                ) {
                                    Row(
                                        modifier =
                                            Modifier
                                                .fillMaxWidth()
                                                .clickable { showLogoutDialog = true }
                                                .padding(
                                                    horizontal = PoziomkiTheme.spacing.md,
                                                    vertical = 14.dp,
                                                ),
                                        verticalAlignment = Alignment.CenterVertically,
                                        horizontalArrangement = Arrangement.Center,
                                    ) {
                                        Icon(
                                            imageVector = PhosphorIcons.Bold.SignOut,
                                            contentDescription = null,
                                            tint = Error,
                                            modifier = Modifier.size(20.dp),
                                        )
                                        Spacer(modifier = Modifier.width(8.dp))
                                        Text(
                                            text = "wyloguj się",
                                            fontFamily = NunitoFamily,
                                            fontWeight = FontWeight.SemiBold,
                                            fontSize = 16.sp,
                                            color = Error,
                                        )
                                    }
                                }

                                Spacer(modifier = Modifier.height(LocalNavBarPadding.current))
                            }
                        }
                    }
                }

                else -> {
                    EmptyView(state.error ?: "nie udało się załadować profilu")
                }
            }

            // Refresh error snackbar
            state.refreshError?.let { error ->
                Snackbar(
                    modifier =
                        Modifier
                            .align(Alignment.BottomCenter)
                            .padding(PoziomkiTheme.spacing.md),
                ) {
                    Text(text = error)
                }
                LaunchedEffect(error) {
                    delay(3000)
                    viewModel.clearRefreshError()
                }
            }
        }
    }

    if (showLogoutDialog) {
        ConfirmDialog(
            title = "wyloguj si\u0119",
            message = "czy na pewno chcesz si\u0119 wylogowa\u0107?",
            confirmText = "wyloguj",
            isDestructive = true,
            onConfirm = {
                showLogoutDialog = false
                viewModel.signOut()
                onSignOut()
            },
            onDismiss = { showLogoutDialog = false },
        )
    }
}

@Composable
private fun SettingsMenuItem(
    icon: ImageVector,
    label: String,
    onClick: () -> Unit,
) {
    Row(
        modifier =
            Modifier
                .fillMaxWidth()
                .clickable(onClick = onClick)
                .padding(horizontal = PoziomkiTheme.spacing.md, vertical = 14.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Icon(
            imageVector = icon,
            contentDescription = null,
            tint = TextPrimary,
            modifier = Modifier.size(22.dp),
        )
        Spacer(modifier = Modifier.width(12.dp))
        Text(
            text = label,
            fontFamily = NunitoFamily,
            fontWeight = FontWeight.Medium,
            fontSize = 16.sp,
            color = TextPrimary,
            modifier = Modifier.weight(1f),
        )
        Icon(
            imageVector = PhosphorIcons.Bold.CaretRight,
            contentDescription = null,
            tint = TextMuted,
            modifier = Modifier.size(20.dp),
        )
    }
}
