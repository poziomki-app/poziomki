package com.poziomki.app.ui.feature.chat

import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.statusBarsPadding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.rememberLazyListState
import androidx.compose.foundation.text.BasicTextField
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.focus.FocusRequester
import androidx.compose.ui.focus.focusRequester
import androidx.compose.ui.graphics.SolidColor
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.adamglin.PhosphorIcons
import com.adamglin.phosphoricons.Bold
import com.adamglin.phosphoricons.bold.ArrowLeft
import com.adamglin.phosphoricons.bold.CaretDown
import com.adamglin.phosphoricons.bold.CaretUp
import com.adamglin.phosphoricons.bold.DotsThreeVertical
import com.adamglin.phosphoricons.bold.MagnifyingGlass
import com.adamglin.phosphoricons.bold.X
import com.poziomki.app.chat.ActiveChat
import com.poziomki.app.chat.api.TimelineItem
import com.poziomki.app.ui.designsystem.components.UserAvatar
import com.poziomki.app.ui.designsystem.theme.Background
import com.poziomki.app.ui.designsystem.theme.Border
import com.poziomki.app.ui.designsystem.theme.MontserratFamily
import com.poziomki.app.ui.designsystem.theme.Primary
import com.poziomki.app.ui.designsystem.theme.TextMuted
import com.poziomki.app.ui.designsystem.theme.TextPrimary
import com.poziomki.app.ui.designsystem.theme.TextSecondary
import org.koin.compose.viewmodel.koinViewModel

@Composable
@Suppress("LongParameterList")
fun ChatScreen(
    chatId: String,
    initialTitle: String? = null,
    initialDirectUserId: String? = null,
    initialDirectProfileId: String? = null,
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
            fallbackProfileId = initialDirectProfileId,
        )
    }

    DisposableEffect(chatId) {
        ActiveChat.roomId = chatId
        onDispose { ActiveChat.roomId = null }
    }

    // Scroll to current search match
    val currentMatchIndex = state.currentSearchMatchIndex
    val matchIndices = state.searchMatchIndices
    val timelineItems = state.timelineItems
    LaunchedEffect(currentMatchIndex, matchIndices) {
        if (currentMatchIndex < 0 || currentMatchIndex >= matchIndices.size) return@LaunchedEffect
        val itemIndex = matchIndices[currentMatchIndex]
        val reversedIndex = timelineItems.size - 1 - itemIndex
        if (reversedIndex >= 0) {
            timelineListState.animateScrollToItem(reversedIndex)
        }
    }

    Scaffold(
        containerColor = Background,
        topBar = {
            if (state.isSearchActive) {
                ChatSearchBar(
                    query = state.searchQuery,
                    onQueryChange = viewModel::onSearchQueryChanged,
                    matchCount = state.searchMatchIndices.size,
                    currentMatch = if (state.currentSearchMatchIndex >= 0) state.currentSearchMatchIndex + 1 else 0,
                    onPrev = viewModel::prevSearchMatch,
                    onNext = viewModel::nextSearchMatch,
                    onClose = viewModel::toggleSearch,
                )
            } else {
                ChatTopBar(
                    title =
                        state.roomDisplayName.ifBlank {
                            initialTitle?.trim()?.takeIf { it.isNotBlank() } ?: ""
                        },
                    avatarUrl = state.roomAvatarUrl,
                    onBack = onBack,
                    onSearchClick = viewModel::toggleSearch,
                    onProfileClick =
                        (initialDirectProfileId ?: state.directProfileId)
                            ?.let { id -> { onNavigateToProfile(id) } },
                )
            }
        },
    ) { padding ->
        val currentMatchEventId =
            if (currentMatchIndex >= 0 && currentMatchIndex < matchIndices.size) {
                val idx = matchIndices[currentMatchIndex]
                (timelineItems.getOrNull(idx) as? TimelineItem.Event)?.eventOrTransactionId
            } else {
                null
            }
        ChatContent(
            state = state,
            timelineListState = timelineListState,
            onDraftChanged = viewModel::onDraftChanged,
            onSendMessage = viewModel::sendMessage,
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
            searchQuery = state.searchQuery,
            currentMatchEventId = currentMatchEventId,
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
    onSearchClick: () -> Unit,
    onProfileClick: (() -> Unit)? = null,
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

            val profileModifier =
                Modifier
                    .weight(1f)
                    .let { if (onProfileClick != null) it.clickable(onClick = onProfileClick) else it }
            Row(modifier = profileModifier, verticalAlignment = Alignment.CenterVertically) {
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
                )
            }

            IconButton(onClick = onSearchClick) {
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

@Composable
private fun ChatSearchBar(
    query: String,
    onQueryChange: (String) -> Unit,
    matchCount: Int,
    currentMatch: Int,
    onPrev: () -> Unit,
    onNext: () -> Unit,
    onClose: () -> Unit,
) {
    val focusRequester = remember { FocusRequester() }
    LaunchedEffect(Unit) { focusRequester.requestFocus() }

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
            IconButton(onClick = onClose) {
                Icon(
                    imageVector = PhosphorIcons.Bold.ArrowLeft,
                    contentDescription = "Zamknij",
                    tint = TextPrimary,
                )
            }

            Box(modifier = Modifier.weight(1f)) {
                if (query.isEmpty()) {
                    Text(
                        text = "szukaj wiadomości...",
                        fontFamily = MontserratFamily,
                        color = TextMuted,
                        fontSize = 15.sp,
                    )
                }
                BasicTextField(
                    value = query,
                    onValueChange = onQueryChange,
                    singleLine = true,
                    textStyle =
                        TextStyle(
                            fontFamily = MontserratFamily,
                            color = TextPrimary,
                            fontSize = 15.sp,
                        ),
                    cursorBrush = SolidColor(Primary),
                    modifier =
                        Modifier
                            .fillMaxWidth()
                            .focusRequester(focusRequester),
                )
            }

            if (query.length >= 2) {
                Text(
                    text = if (matchCount > 0) "$currentMatch z $matchCount" else "0",
                    fontFamily = MontserratFamily,
                    color = TextMuted,
                    fontSize = 13.sp,
                    modifier = Modifier.padding(horizontal = 4.dp),
                )
                IconButton(onClick = onPrev, modifier = Modifier.size(36.dp)) {
                    Icon(
                        imageVector = PhosphorIcons.Bold.CaretUp,
                        contentDescription = "Poprzedni",
                        tint = if (matchCount > 0) TextPrimary else TextMuted,
                        modifier = Modifier.size(20.dp),
                    )
                }
                IconButton(onClick = onNext, modifier = Modifier.size(36.dp)) {
                    Icon(
                        imageVector = PhosphorIcons.Bold.CaretDown,
                        contentDescription = "Następny",
                        tint = if (matchCount > 0) TextPrimary else TextMuted,
                        modifier = Modifier.size(20.dp),
                    )
                }
            }

            IconButton(onClick = onClose, modifier = Modifier.size(36.dp)) {
                Icon(
                    imageVector = PhosphorIcons.Bold.X,
                    contentDescription = "Zamknij",
                    tint = TextSecondary,
                    modifier = Modifier.size(20.dp),
                )
            }
        }
    }
}
