package com.poziomki.app.ui.screen.chat

import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.rememberLazyListState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.filled.MoreVert
import androidx.compose.material.icons.filled.Search
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import com.poziomki.app.ui.theme.Background
import com.poziomki.app.ui.theme.Border
import com.poziomki.app.ui.theme.Primary
import com.poziomki.app.ui.theme.TextPrimary
import com.poziomki.app.ui.theme.TextSecondary
import org.koin.compose.viewmodel.koinViewModel
import com.poziomki.app.ui.theme.Surface as SurfaceColor

@Composable
fun ChatScreen(
    chatId: String,
    onBack: () -> Unit,
    onNavigateToProfile: (String) -> Unit,
    viewModel: ChatViewModel = koinViewModel(),
) {
    val state by viewModel.uiState.collectAsState()
    val timelineListState = rememberLazyListState()

    LaunchedEffect(chatId) {
        viewModel.loadRoom(chatId)
    }

    Scaffold(
        containerColor = Background,
        topBar = {
            ChatTopBar(
                title = state.roomDisplayName.ifBlank { "Chat" },
                onBack = onBack,
            )
        },
    ) { padding ->
        ChatContent(
            state = state,
            timelineListState = timelineListState,
            onDraftChanged = viewModel::onDraftChanged,
            onSendMessage = viewModel::sendMessage,
            onSendImageAttachment = viewModel::sendImageAttachment,
            onSendFileAttachment = viewModel::sendFileAttachment,
            onToggleReaction = viewModel::toggleReaction,
            onPaginateBackwards = viewModel::paginateBackwards,
            onMarkAsRead = viewModel::markAsRead,
            onViewportChanged = viewModel::onTimelineViewportChanged,
            onJumpToLatest = viewModel::jumpToLatestHandled,
            onStartReply = viewModel::startReply,
            onStartEdit = viewModel::startEdit,
            onCancelComposerMode = viewModel::cancelComposerMode,
            onRedactEvent = viewModel::redactEvent,
            onClearError = viewModel::clearError,
            onNavigateToProfile = onNavigateToProfile,
            resolveDisplayNames = viewModel::resolveDisplayNames,
            modifier = Modifier.padding(padding),
        )
    }
}

@Composable
private fun ChatTopBar(
    title: String,
    onBack: () -> Unit,
) {
    Surface(
        color = Background,
        border = BorderStroke(1.dp, Border.copy(alpha = 0.35f)),
    ) {
        Row(
            modifier =
                Modifier
                    .fillMaxWidth()
                    .padding(horizontal = 8.dp, vertical = 6.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            IconButton(onClick = onBack) {
                Icon(
                    imageVector = Icons.AutoMirrored.Filled.ArrowBack,
                    contentDescription = "Wstecz",
                    tint = TextPrimary,
                )
            }

            Surface(
                modifier = Modifier.size(34.dp),
                shape = CircleShape,
                color = Primary.copy(alpha = 0.2f),
            ) {
                androidx.compose.foundation.layout.Box(contentAlignment = Alignment.Center) {
                    Text(
                        text = title.firstOrNull()?.uppercase() ?: "?",
                        style = MaterialTheme.typography.labelLarge,
                        color = Primary,
                    )
                }
            }

            Spacer(modifier = Modifier.width(10.dp))

            Text(
                text = title,
                style = MaterialTheme.typography.titleMedium,
                color = TextPrimary,
                maxLines = 1,
                overflow = TextOverflow.Ellipsis,
                modifier = Modifier.weight(1f),
            )

            IconButton(onClick = { }) {
                Icon(
                    imageVector = Icons.Filled.Search,
                    contentDescription = "Szukaj",
                    tint = TextSecondary,
                )
            }
            IconButton(onClick = { }) {
                Icon(
                    imageVector = Icons.Filled.MoreVert,
                    contentDescription = "Więcej",
                    tint = TextSecondary,
                )
            }
        }
    }
}
