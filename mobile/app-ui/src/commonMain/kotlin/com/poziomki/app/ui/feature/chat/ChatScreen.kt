package com.poziomki.app.ui.feature.chat

import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.statusBarsPadding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.rememberLazyListState
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
import com.adamglin.PhosphorIcons
import com.adamglin.phosphoricons.Bold
import com.adamglin.phosphoricons.bold.ArrowLeft
import com.adamglin.phosphoricons.bold.DotsThreeVertical
import com.adamglin.phosphoricons.bold.MagnifyingGlass
import com.poziomki.app.ui.designsystem.components.UserAvatar
import com.poziomki.app.ui.designsystem.theme.Background
import com.poziomki.app.ui.designsystem.theme.Border
import com.poziomki.app.ui.designsystem.theme.Primary
import com.poziomki.app.ui.designsystem.theme.TextPrimary
import com.poziomki.app.ui.designsystem.theme.TextSecondary
import org.koin.compose.viewmodel.koinViewModel

@Composable
fun ChatScreen(
    chatId: String,
    initialTitle: String? = null,
    initialDirectUserId: String? = null,
    onBack: () -> Unit,
    onNavigateToProfile: (String) -> Unit,
    viewModel: ChatViewModel = koinViewModel(),
) {
    val state by viewModel.uiState.collectAsState()
    val timelineListState = rememberLazyListState()
    LaunchedEffect(chatId, initialTitle, initialDirectUserId) {
        viewModel.loadRoom(
            roomId = chatId,
            fallbackDisplayName = initialTitle,
            fallbackDirectUserId = initialDirectUserId,
        )
    }

    Scaffold(
        containerColor = Background,
        topBar = {
            ChatTopBar(
                title =
                    state.roomDisplayName.ifBlank {
                        initialTitle?.trim()?.takeIf { it.isNotBlank() } ?: ""
                    },
                avatarUrl = state.roomAvatarUrl,
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
            resolveAvatarUrls = viewModel::resolveAvatarUrls,
            showSenderMeta = !state.isDirectRoom,
            avatarOverrides = state.avatarOverrides,
            avatarOverridesByName = emptyMap(),
            modifier = Modifier.padding(top = padding.calculateTopPadding()),
        )
    }
}

@Composable
@Suppress("FunctionNaming")
private fun ChatTopBar(
    title: String,
    avatarUrl: String?,
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
                    .statusBarsPadding()
                    .padding(horizontal = 8.dp, vertical = 6.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            IconButton(onClick = onBack) {
                Icon(
                    imageVector = PhosphorIcons.Bold.ArrowLeft,
                    contentDescription = "Wstecz",
                    tint = TextPrimary,
                )
            }

            UserAvatar(
                picture = avatarUrl,
                displayName = title,
                size = 34.dp,
                backgroundColor = Primary.copy(alpha = 0.2f),
                iconTint = Primary,
            )

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
                    imageVector = PhosphorIcons.Bold.MagnifyingGlass,
                    contentDescription = "Szukaj",
                    tint = TextSecondary,
                )
            }
            IconButton(onClick = { }) {
                Icon(
                    imageVector = PhosphorIcons.Bold.DotsThreeVertical,
                    contentDescription = "Więcej",
                    tint = TextSecondary,
                )
            }
        }
    }
}
