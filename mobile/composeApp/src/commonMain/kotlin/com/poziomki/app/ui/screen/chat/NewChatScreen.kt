package com.poziomki.app.ui.screen.chat

import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material3.Button
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import com.poziomki.app.ui.theme.Background
import com.poziomki.app.ui.theme.Border
import com.poziomki.app.ui.theme.NunitoFamily
import com.poziomki.app.ui.theme.PoziomkiTheme
import com.poziomki.app.ui.theme.Primary
import com.poziomki.app.ui.theme.TextPrimary
import org.koin.compose.viewmodel.koinViewModel

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun NewChatScreen(
    onBack: () -> Unit,
    onChatCreated: (String) -> Unit,
    viewModel: NewChatViewModel = koinViewModel(),
) {
    val state by viewModel.uiState.collectAsState()

    Scaffold(
        containerColor = Background,
        topBar = {
            TopAppBar(
                title = { Text("nowy chat", color = TextPrimary) },
                navigationIcon = {
                    IconButton(onClick = onBack) {
                        Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = "Back", tint = TextPrimary)
                    }
                },
            )
        },
    ) { padding ->
        Column(
            modifier =
                Modifier
                    .fillMaxSize()
                    .padding(padding)
                    .padding(PoziomkiTheme.spacing.md),
        ) {
            if (state.error != null) {
                Text(
                    text = state.error ?: "",
                    color = MaterialTheme.colorScheme.error,
                    style = MaterialTheme.typography.bodySmall,
                    modifier = Modifier.padding(bottom = PoziomkiTheme.spacing.sm),
                )
            }

            Surface(
                color = MaterialTheme.colorScheme.surface,
                border = BorderStroke(1.dp, Border),
                shape =
                    androidx.compose.foundation.shape
                        .RoundedCornerShape(12.dp),
            ) {
                Column(modifier = Modifier.fillMaxWidth().padding(PoziomkiTheme.spacing.md)) {
                    Text(
                        text = "rozmowa 1:1",
                        style = MaterialTheme.typography.titleMedium,
                        color = TextPrimary,
                    )
                    Text(
                        text = "to ten sam model pokoju co grupa, tylko z 1 zaproszona osoba",
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                    Spacer(modifier = Modifier.height(PoziomkiTheme.spacing.sm))
                    OutlinedTextField(
                        value = state.dmUserId,
                        onValueChange = viewModel::onDmUserIdChanged,
                        label = { Text("matrix user id") },
                        placeholder = { Text("@user:example.org") },
                        modifier = Modifier.fillMaxWidth(),
                        singleLine = true,
                    )
                    Spacer(modifier = Modifier.height(PoziomkiTheme.spacing.sm))
                    Button(
                        onClick = { viewModel.createDm(onChatCreated) },
                        enabled = !state.isSubmitting,
                        modifier = Modifier.fillMaxWidth(),
                    ) {
                        if (state.isSubmitting) {
                            CircularProgressIndicator(modifier = Modifier.height(18.dp), color = Background, strokeWidth = 2.dp)
                        } else {
                            Text("utworz dm", fontFamily = NunitoFamily)
                        }
                    }
                }
            }

            Spacer(modifier = Modifier.height(PoziomkiTheme.spacing.md))

            Surface(
                color = MaterialTheme.colorScheme.surface,
                border = BorderStroke(1.dp, Border),
                shape =
                    androidx.compose.foundation.shape
                        .RoundedCornerShape(12.dp),
            ) {
                Column(modifier = Modifier.fillMaxWidth().padding(PoziomkiTheme.spacing.md)) {
                    Text(
                        text = "group room",
                        style = MaterialTheme.typography.titleMedium,
                        color = TextPrimary,
                    )
                    Spacer(modifier = Modifier.height(PoziomkiTheme.spacing.sm))
                    OutlinedTextField(
                        value = state.roomName,
                        onValueChange = viewModel::onRoomNameChanged,
                        label = { Text("room name") },
                        modifier = Modifier.fillMaxWidth(),
                        singleLine = true,
                    )
                    Spacer(modifier = Modifier.height(PoziomkiTheme.spacing.sm))
                    OutlinedTextField(
                        value = state.inviteUserIdsRaw,
                        onValueChange = viewModel::onInviteUserIdsRawChanged,
                        label = { Text("invite user ids") },
                        placeholder = { Text("@a:hs, @b:hs") },
                        modifier = Modifier.fillMaxWidth(),
                        minLines = 2,
                    )
                    Spacer(modifier = Modifier.height(PoziomkiTheme.spacing.sm))
                    Button(
                        onClick = { viewModel.createRoom(onChatCreated) },
                        enabled = !state.isSubmitting,
                        modifier = Modifier.fillMaxWidth(),
                    ) {
                        if (state.isSubmitting) {
                            CircularProgressIndicator(modifier = Modifier.height(18.dp), color = Primary, strokeWidth = 2.dp)
                        } else {
                            Text("utworz pokoj", fontFamily = NunitoFamily)
                        }
                    }
                }
            }
        }
    }
}
