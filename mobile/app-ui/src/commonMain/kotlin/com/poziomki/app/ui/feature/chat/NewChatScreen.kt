package com.poziomki.app.ui.feature.chat

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
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import com.adamglin.PhosphorIcons
import com.adamglin.phosphoricons.Bold
import com.adamglin.phosphoricons.bold.ArrowLeft
import com.poziomki.app.network.MatchProfile
import com.poziomki.app.ui.designsystem.components.UserAvatar
import com.poziomki.app.ui.designsystem.theme.Background
import com.poziomki.app.ui.designsystem.theme.Border
import com.poziomki.app.ui.designsystem.theme.MontserratFamily
import com.poziomki.app.ui.designsystem.theme.PoziomkiTheme
import com.poziomki.app.ui.designsystem.theme.Primary
import com.poziomki.app.ui.designsystem.theme.SurfaceElevated
import com.poziomki.app.ui.designsystem.theme.TextPrimary
import com.poziomki.app.ui.designsystem.theme.TextSecondary
import org.koin.compose.viewmodel.koinViewModel
import com.poziomki.app.ui.designsystem.theme.Surface as SurfaceColor

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun NewChatScreen(
    onBack: () -> Unit,
    onUserSelected: (userId: String, displayName: String, profileId: String?) -> Unit,
    viewModel: NewChatViewModel = koinViewModel(),
) {
    val state by viewModel.uiState.collectAsState()

    Scaffold(
        containerColor = Background,
        topBar = {
            TopAppBar(
                title = { Text("nowa wiadomość", color = TextPrimary) },
                navigationIcon = {
                    IconButton(onClick = onBack) {
                        Icon(PhosphorIcons.Bold.ArrowLeft, contentDescription = "Back", tint = TextPrimary)
                    }
                },
            )
        },
    ) { padding ->
        when {
            state.isLoading && state.profiles.isEmpty() -> {
                Box(
                    modifier = Modifier.fillMaxSize().padding(padding),
                    contentAlignment = Alignment.Center,
                ) {
                    CircularProgressIndicator(color = Primary)
                }
            }

            state.profiles.isEmpty() -> {
                Box(
                    modifier = Modifier.fillMaxSize().padding(padding),
                    contentAlignment = Alignment.Center,
                ) {
                    Text(
                        text = state.error ?: "brak osób do napisania",
                        fontFamily = MontserratFamily,
                        color = TextSecondary,
                    )
                }
            }

            else -> {
                LazyColumn(
                    modifier =
                        Modifier
                            .fillMaxSize()
                            .padding(padding)
                            .padding(horizontal = PoziomkiTheme.spacing.md),
                ) {
                    items(state.profiles, key = { it.id }) { profile ->
                        ProfileRow(
                            profile = profile,
                            onClick = { onUserSelected(profile.userId, profile.name, profile.id) },
                        )
                        Spacer(modifier = Modifier.height(8.dp))
                    }
                    item { Spacer(modifier = Modifier.height(80.dp)) }
                }
            }
        }
    }
}

@Composable
private fun ProfileRow(
    profile: MatchProfile,
    onClick: () -> Unit,
) {
    Surface(
        modifier =
            Modifier
                .fillMaxWidth()
                .clickable(onClick = onClick),
        shape = RoundedCornerShape(14.dp),
        color = SurfaceColor,
        border = androidx.compose.foundation.BorderStroke(1.dp, Border),
    ) {
        Row(
            modifier =
                Modifier
                    .fillMaxWidth()
                    .padding(horizontal = 12.dp, vertical = 10.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            UserAvatar(
                picture = profile.profilePicture,
                displayName = profile.name,
                size = 40.dp,
                backgroundColor = SurfaceElevated,
                iconTint = TextPrimary,
            )
            Spacer(modifier = Modifier.width(12.dp))
            Column(modifier = Modifier.weight(1f)) {
                Text(
                    text = profile.name,
                    style = MaterialTheme.typography.titleSmall,
                    color = TextPrimary,
                    maxLines = 1,
                )
                val program = profile.program
                if (!program.isNullOrBlank()) {
                    Text(
                        text = program,
                        style = MaterialTheme.typography.bodySmall,
                        color = TextSecondary,
                        maxLines = 1,
                    )
                }
            }
        }
    }
}
