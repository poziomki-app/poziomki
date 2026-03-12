package com.poziomki.app.ui.feature.event

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
import androidx.compose.foundation.lazy.LazyRow
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Button
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.unit.dp
import coil3.compose.AsyncImage
import com.adamglin.PhosphorIcons
import com.adamglin.phosphoricons.Bold
import com.adamglin.phosphoricons.bold.ArrowLeft
import com.adamglin.phosphoricons.bold.EnvelopeSimple
import com.poziomki.app.ui.designsystem.components.ConfirmDialog
import com.poziomki.app.ui.designsystem.components.PoziomkiSnackbar
import com.poziomki.app.ui.designsystem.components.SnackbarType
import com.poziomki.app.ui.designsystem.theme.PoziomkiTheme
import com.poziomki.app.ui.shared.isImageUrl
import com.poziomki.app.ui.shared.resolveImageUrl
import kotlinx.coroutines.delay
import org.koin.compose.viewmodel.koinViewModel

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun EventDetailScreen(
    onBack: () -> Unit,
    onNavigateToChat: (String) -> Unit,
    onNavigateToProfile: (String) -> Unit,
    viewModel: EventDetailViewModel = koinViewModel(),
) {
    val state by viewModel.state.collectAsState()
    var showLeaveDialog by remember { mutableStateOf(false) }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text(state.event?.title ?: "Event") },
                navigationIcon = {
                    IconButton(onClick = onBack) {
                        Icon(PhosphorIcons.Bold.ArrowLeft, contentDescription = "Back")
                    }
                },
            )
        },
    ) { padding ->
        Box(modifier = Modifier.fillMaxSize().padding(padding)) {
            when {
                state.isLoading -> {
                    Box(Modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
                        CircularProgressIndicator()
                    }
                }

                state.event != null -> {
                    state.event?.let { event ->
                        Column(
                            modifier =
                                Modifier
                                    .fillMaxSize()
                                    .verticalScroll(rememberScrollState()),
                        ) {
                            event.coverImage?.let { url ->
                                AsyncImage(
                                    model = url,
                                    contentDescription = event.title,
                                    modifier = Modifier.fillMaxWidth().height(200.dp),
                                    contentScale = ContentScale.Crop,
                                )
                            }

                            Column(modifier = Modifier.padding(PoziomkiTheme.spacing.lg)) {
                                Text(
                                    text = event.title,
                                    style = MaterialTheme.typography.headlineSmall,
                                )

                                Spacer(modifier = Modifier.height(PoziomkiTheme.spacing.sm))

                                Text(
                                    text = event.startsAt,
                                    style = MaterialTheme.typography.bodyLarge,
                                    color = MaterialTheme.colorScheme.primary,
                                )

                                event.location?.let {
                                    Text(
                                        text = it,
                                        style = MaterialTheme.typography.bodyMedium,
                                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                                    )
                                }

                                event.description?.let {
                                    Spacer(modifier = Modifier.height(PoziomkiTheme.spacing.lg))
                                    Text(
                                        text = it,
                                        style = MaterialTheme.typography.bodyMedium,
                                    )
                                }

                                Spacer(modifier = Modifier.height(PoziomkiTheme.spacing.lg))

                                Row(
                                    modifier = Modifier.fillMaxWidth(),
                                    horizontalArrangement = Arrangement.spacedBy(PoziomkiTheme.spacing.md),
                                ) {
                                    if (event.isAttending) {
                                        OutlinedButton(
                                            onClick = { showLeaveDialog = true },
                                            modifier = Modifier.weight(1f),
                                        ) {
                                            Text("Leave")
                                        }
                                    } else {
                                        val isFull = event.maxAttendees != null &&
                                            event.attendeesCount >= event.maxAttendees
                                        Button(
                                            onClick = { viewModel.attendEvent() },
                                            modifier = Modifier.weight(1f),
                                            enabled = !isFull,
                                        ) {
                                            Text(if (isFull) "Brak miejsc" else "Dołącz")
                                        }
                                    }
                                }

                                Spacer(modifier = Modifier.height(PoziomkiTheme.spacing.sm))

                                OutlinedButton(
                                    onClick = { viewModel.openEventChat(onNavigateToChat) },
                                    enabled = event.isAttending && !state.isOpeningChat,
                                    modifier = Modifier.fillMaxWidth(),
                                ) {
                                    if (state.isOpeningChat) {
                                        CircularProgressIndicator(
                                            modifier = Modifier.size(18.dp),
                                            strokeWidth = 2.dp,
                                        )
                                    } else {
                                        Icon(
                                            imageVector = PhosphorIcons.Bold.EnvelopeSimple,
                                            contentDescription = null,
                                        )
                                        Spacer(modifier = Modifier.width(8.dp))
                                        Text(if (event.isAttending) "Czat wydarzenia" else "Dołącz, aby czatować")
                                    }
                                }

                                state.error?.let { error ->
                                    Spacer(modifier = Modifier.height(PoziomkiTheme.spacing.sm))
                                    Text(
                                        text = error,
                                        style = MaterialTheme.typography.bodySmall,
                                        color = MaterialTheme.colorScheme.error,
                                    )
                                }

                                // Attendees
                                if (state.attendees.isNotEmpty()) {
                                    Spacer(modifier = Modifier.height(PoziomkiTheme.spacing.lg))
                                    val attendeesLabel = if (event.maxAttendees != null) {
                                        "Uczestnicy (${state.attendees.size} / ${event.maxAttendees})"
                                    } else {
                                        "Uczestnicy (${state.attendees.size})"
                                    }
                                    Text(
                                        text = attendeesLabel,
                                        style = MaterialTheme.typography.titleMedium,
                                    )
                                    Spacer(modifier = Modifier.height(PoziomkiTheme.spacing.sm))
                                    LazyRow(
                                        horizontalArrangement = Arrangement.spacedBy(PoziomkiTheme.spacing.sm),
                                    ) {
                                        items(state.attendees, key = { it.profileId }) { attendee ->
                                            Column(
                                                horizontalAlignment = Alignment.CenterHorizontally,
                                                modifier =
                                                    Modifier
                                                        .width(72.dp)
                                                        .clickable { onNavigateToProfile(attendee.profileId) },
                                            ) {
                                                AsyncImage(
                                                    model = attendee.profilePicture?.let { resolveImageUrl(it) },
                                                    contentDescription = attendee.name,
                                                    modifier = Modifier.size(48.dp).clip(CircleShape),
                                                    contentScale = ContentScale.Crop,
                                                )
                                                Text(
                                                    text = attendee.name,
                                                    style = MaterialTheme.typography.labelSmall,
                                                    maxLines = 1,
                                                )
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                else -> {
                    Box(Modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
                        Text(
                            state.error ?: "Event not found",
                            color = MaterialTheme.colorScheme.error,
                        )
                    }
                }
            }

            // Snackbar for attend/leave errors
            state.snackbarMessage?.let { message ->
                PoziomkiSnackbar(
                    message = message,
                    type = state.snackbarType,
                    modifier =
                        Modifier
                            .align(Alignment.BottomCenter)
                            .padding(PoziomkiTheme.spacing.md),
                )
                LaunchedEffect(message) {
                    delay(3000)
                    viewModel.clearSnackbar()
                }
            }
        }
    }

    if (showLeaveDialog) {
        ConfirmDialog(
            title = "opu\u015b\u0107 wydarzenie",
            message = "czy na pewno chcesz opu\u015bci\u0107 to wydarzenie?",
            confirmText = "opu\u015b\u0107",
            isDestructive = true,
            onConfirm = {
                viewModel.leaveEvent()
                showLeaveDialog = false
            },
            onDismiss = { showLeaveDialog = false },
        )
    }
}
