package com.poziomki.app.ui.screen.main

import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
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
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ExitToApp
import androidx.compose.material.icons.automirrored.filled.KeyboardArrowRight
import androidx.compose.material.icons.outlined.Edit
import androidx.compose.material.icons.outlined.Settings
import androidx.compose.material.icons.outlined.Shield
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.poziomki.app.ui.component.ProfileCard
import com.poziomki.app.ui.theme.Border
import com.poziomki.app.ui.theme.Error
import com.poziomki.app.ui.theme.NunitoFamily
import com.poziomki.app.ui.theme.PoziomkiTheme
import com.poziomki.app.ui.theme.Primary
import com.poziomki.app.ui.theme.TextMuted
import com.poziomki.app.ui.theme.TextPrimary
import org.koin.compose.viewmodel.koinViewModel
import com.poziomki.app.ui.theme.Surface as SurfaceColor

@Composable
fun ProfileScreen(
    onNavigateToEdit: () -> Unit,
    onNavigateToPrivacy: () -> Unit,
    onNavigateToProfileView: (String) -> Unit,
    onSignOut: () -> Unit,
    viewModel: ProfileViewModel = koinViewModel(),
) {
    val state by viewModel.state.collectAsState()
    val nunito = NunitoFamily

    Column(
        modifier =
            Modifier
                .fillMaxSize()
                .background(MaterialTheme.colorScheme.background),
    ) {
        // Header
        Text(
            text = "profil",
            style = MaterialTheme.typography.headlineLarge,
            color = TextPrimary,
            modifier =
                Modifier.padding(
                    horizontal = PoziomkiTheme.spacing.lg,
                    vertical = PoziomkiTheme.spacing.md,
                ),
        )

        when {
            state.isLoading -> {
                Box(Modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
                    CircularProgressIndicator(color = Primary)
                }
            }

            state.profile != null -> {
                val profile = state.profile!!

                Column(
                    modifier =
                        Modifier
                            .fillMaxSize()
                            .verticalScroll(rememberScrollState())
                            .padding(horizontal = PoziomkiTheme.spacing.lg),
                ) {
                    // ProfileCard
                    ProfileCard(
                        name = "${profile.name}, ${profile.age}",
                        program = profile.program,
                        profilePicture = profile.profilePicture,
                        tags = state.tags,
                        onClick = { onNavigateToProfileView(profile.id) },
                    )

                    Spacer(modifier = Modifier.height(PoziomkiTheme.spacing.lg))

                    // Settings menu
                    Surface(
                        shape = RoundedCornerShape(16.dp),
                        color = SurfaceColor,
                        border = BorderStroke(1.dp, Border),
                    ) {
                        Column {
                            SettingsMenuItem(
                                icon = Icons.Outlined.Edit,
                                label = "edytuj profil",
                                onClick = onNavigateToEdit,
                            )
                            HorizontalDivider(color = Border, thickness = 1.dp)
                            SettingsMenuItem(
                                icon = Icons.Outlined.Shield,
                                label = "prywatność",
                                onClick = onNavigateToPrivacy,
                            )
                            HorizontalDivider(color = Border, thickness = 1.dp)
                            SettingsMenuItem(
                                icon = Icons.Outlined.Settings,
                                label = "ustawienia aplikacji",
                                onClick = {},
                            )
                        }
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
                                    .clickable {
                                        viewModel.signOut()
                                        onSignOut()
                                    }.padding(horizontal = PoziomkiTheme.spacing.md, vertical = 14.dp),
                            verticalAlignment = Alignment.CenterVertically,
                            horizontalArrangement = androidx.compose.foundation.layout.Arrangement.Center,
                        ) {
                            Icon(
                                imageVector = Icons.AutoMirrored.Filled.ExitToApp,
                                contentDescription = null,
                                tint = Error,
                                modifier = Modifier.size(20.dp),
                            )
                            Spacer(modifier = Modifier.width(8.dp))
                            Text(
                                text = "wyloguj się",
                                fontFamily = nunito,
                                fontWeight = FontWeight.SemiBold,
                                fontSize = 16.sp,
                                color = Error,
                            )
                        }
                    }

                    Spacer(modifier = Modifier.height(PoziomkiTheme.spacing.xl))
                }
            }

            else -> {
                Box(Modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
                    Text(
                        state.error ?: "nie udało się załadować profilu",
                        fontFamily = nunito,
                        color = MaterialTheme.colorScheme.error,
                    )
                }
            }
        }
    }
}

@Composable
private fun SettingsMenuItem(
    icon: ImageVector,
    label: String,
    onClick: () -> Unit,
) {
    val nunito = NunitoFamily

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
            fontFamily = nunito,
            fontWeight = FontWeight.Medium,
            fontSize = 16.sp,
            color = TextPrimary,
            modifier = Modifier.weight(1f),
        )
        Icon(
            imageVector = Icons.AutoMirrored.Filled.KeyboardArrowRight,
            contentDescription = null,
            tint = TextMuted,
            modifier = Modifier.size(20.dp),
        )
    }
}
