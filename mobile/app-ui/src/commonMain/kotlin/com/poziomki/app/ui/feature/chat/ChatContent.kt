package com.poziomki.app.ui.feature.chat

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
import androidx.compose.foundation.layout.imePadding
import androidx.compose.foundation.layout.navigationBarsPadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.LazyListState
import androidx.compose.foundation.lazy.itemsIndexed
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.BasicTextField
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.FloatingActionButton
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.ScrollableTabRow
import androidx.compose.material3.Surface
import androidx.compose.material3.Tab
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.derivedStateOf
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.runtime.snapshotFlow
import androidx.compose.ui.Alignment
import androidx.compose.ui.ExperimentalComposeUiApi
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalClipboardManager
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.semantics.testTagsAsResourceId
import androidx.compose.ui.text.AnnotatedString
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.window.Dialog
import androidx.compose.ui.window.DialogProperties
import com.adamglin.PhosphorIcons
import com.adamglin.phosphoricons.Bold
import com.adamglin.phosphoricons.Fill
import com.adamglin.phosphoricons.bold.ArrowBendUpLeft
import com.adamglin.phosphoricons.bold.CaretDown
import com.adamglin.phosphoricons.bold.Copy
import com.adamglin.phosphoricons.bold.PencilSimple
import com.adamglin.phosphoricons.bold.Trash
import com.adamglin.phosphoricons.fill.PaperPlaneRight
import com.poziomki.app.chat.api.Reaction
import com.poziomki.app.chat.api.TimelineItem
import com.poziomki.app.ui.designsystem.components.AppSnackbar
import com.poziomki.app.ui.designsystem.components.UserAvatar
import com.poziomki.app.ui.designsystem.theme.Background
import com.poziomki.app.ui.designsystem.theme.Border
import com.poziomki.app.ui.designsystem.theme.PoziomkiTheme
import com.poziomki.app.ui.designsystem.theme.Primary
import com.poziomki.app.ui.designsystem.theme.TextPrimary
import com.poziomki.app.ui.designsystem.theme.TextSecondary
import com.poziomki.app.ui.feature.chat.model.ChatUiState
import com.poziomki.app.ui.feature.chat.model.ComposerMode
import kotlinx.coroutines.flow.distinctUntilChanged
import kotlinx.coroutines.launch
import kotlinx.datetime.DayOfWeek
import kotlinx.datetime.LocalDate
import kotlinx.datetime.TimeZone
import kotlinx.datetime.toLocalDateTime
import kotlin.time.Clock
import kotlin.time.Instant
import com.poziomki.app.ui.designsystem.theme.Surface as SurfaceColor

@OptIn(ExperimentalMaterial3Api::class)
@Suppress("LongParameterList", "LongMethod", "CyclomaticComplexMethod")
@Composable
fun ChatContent(
    state: ChatUiState,
    timelineListState: LazyListState,
    onDraftChanged: (String) -> Unit,
    onSendMessage: () -> Unit,
    onToggleReaction: (String, String) -> Unit,
    onMarkAsRead: () -> Unit,
    onViewportChanged: (Int?) -> Unit,
    onJumpToLatest: () -> Unit,
    onStartReply: (TimelineItem.Event) -> Unit,
    onStartEdit: (TimelineItem.Event) -> Unit,
    onCancelComposerMode: () -> Unit,
    onRedactEvent: (String) -> Unit,
    onRevealModeration: (TimelineItem.Event) -> Unit,
    onReportFlagged: (TimelineItem.Event) -> Unit,
    onClearError: () -> Unit,
    onClearTransientNotice: () -> Unit,
    onNavigateToProfile: (String) -> Unit,
    resolveDisplayNames: suspend (List<String>) -> Map<String, String>,
    resolveAvatarUrls: suspend (List<String>) -> Map<String, String>,
    showSenderMeta: Boolean = !state.isDirectRoom,
    modifier: Modifier = Modifier,
    avatarOverrides: Map<String, String> = emptyMap(),
    avatarOverridesByName: Map<String, String> = emptyMap(),
    searchQuery: String = "",
    currentMatchEventId: String? = null,
    headerContent: (@Composable () -> Unit)? = null,
) {
    val coroutineScope = rememberCoroutineScope()

    var selectedActionEvent by remember { mutableStateOf<TimelineItem.Event?>(null) }
    var selectedReactionEvent by remember { mutableStateOf<TimelineItem.Event?>(null) }

    // In reversed layout, index 0 = newest (bottom of screen).
    // "Away from latest" means the first visible item is not near index 0.
    LaunchedEffect(timelineListState) {
        snapshotFlow {
            val layoutInfo = timelineListState.layoutInfo
            val firstVisibleIndex = layoutInfo.visibleItemsInfo.firstOrNull()?.index
            firstVisibleIndex
        }.distinctUntilChanged()
            .collect { firstVisibleIndex ->
                onViewportChanged(firstVisibleIndex)
                if (firstVisibleIndex == null || firstVisibleIndex == 0) {
                    onMarkAsRead()
                }
            }
    }

    Column(
        modifier =
            modifier
                .fillMaxSize()
                .imePadding()
                .background(Background),
    ) {
        if (state.error != null) {
            Text(
                text = state.error,
                color = MaterialTheme.colorScheme.error,
                style = MaterialTheme.typography.bodySmall,
                modifier =
                    Modifier
                        .fillMaxWidth()
                        .padding(horizontal = PoziomkiTheme.spacing.md, vertical = PoziomkiTheme.spacing.sm)
                        .clickable { onClearError() },
            )
        }

        run {
            val itemsWithDividers =
                remember(state.timelineItems) { withDateDividers(state.timelineItems) }
            val reversedItems = itemsWithDividers.asReversed()
            val topTimelineLabel by remember(reversedItems, timelineListState) {
                derivedStateOf {
                    val topVisibleIndex = timelineListState.layoutInfo.visibleItemsInfo.maxOfOrNull { it.index }
                    formatTimelineCapsuleLabel(topVisibleIndex?.let(reversedItems::getOrNull))
                }
            }
            Box(
                modifier =
                    Modifier
                        .weight(1f)
                        .fillMaxWidth(),
            ) {
                val itemPadding = Modifier.padding(horizontal = 10.dp)

                @OptIn(ExperimentalComposeUiApi::class)
                LazyColumn(
                    state = timelineListState,
                    reverseLayout = true,
                    modifier =
                        Modifier
                            .fillMaxSize()
                            .testTag("chatMessages")
                            .semantics { testTagsAsResourceId = true },
                ) {
                    // In reverseLayout, first items render at the BOTTOM of the screen.
                    // Order: typing indicator (bottom) → timeline items → pagination button (top)

                    item { Spacer(modifier = Modifier.height(8.dp)) }
                    item {
                        if (state.typingUserIds.isNotEmpty()) {
                            TypingIndicator(
                                avatarUrl = state.typingAvatarUrls.firstOrNull(),
                                displayName = state.typingDisplayNames.firstOrNull(),
                                showAvatar = !state.isDirectRoom,
                                modifier = itemPadding.padding(vertical = 8.dp),
                            )
                        }
                    }

                    itemsIndexed(
                        items = reversedItems,
                        key = { _, item -> timelineItemKey(item) },
                    ) { index, item ->
                        Box(modifier = itemPadding) {
                            when (item) {
                                is TimelineItem.DateDivider -> {
                                    DateDivider(item.timestampMillis)
                                }

                                is TimelineItem.Event -> {
                                    // "Visually previous" (above) = older event = index + 1 in reversed list
                                    val previousEvent =
                                        reversedItems.getOrNull(index + 1) as? TimelineItem.Event
                                    val isMatch =
                                        searchQuery.length >= 2 &&
                                            item.body.contains(searchQuery, ignoreCase = true)
                                    MessageEventRow(
                                        event = item,
                                        groupedWithPrevious = shouldGroupWithPrevious(previousEvent, item),
                                        showSenderMeta = showSenderMeta,
                                        onToggleReaction = { emoji ->
                                            onToggleReaction(item.eventOrTransactionId, emoji)
                                        },
                                        onReactionsClick = { selectedReactionEvent = item },
                                        onFocusOnReply = { },
                                        onSenderClick = { item.senderPid?.let { onNavigateToProfile(it) } },
                                        onActionsLongPress = { selectedActionEvent = item },
                                        onSwipeReply = { onStartReply(item) },
                                        onRevealModeration = { onRevealModeration(item) },
                                        onReportFlagged = { onReportFlagged(item) },
                                        compactTimestamp = showSenderMeta,
                                        avatarOverride =
                                            resolveAvatarOverride(item.senderId, avatarOverrides)
                                                ?: item.senderDisplayName
                                                    ?.trim()
                                                    ?.lowercase()
                                                    ?.let { avatarOverridesByName[it] },
                                        isHighlighted = isMatch && item.eventOrTransactionId == currentMatchEventId,
                                    )
                                }

                                TimelineItem.ReadMarker -> {
                                    NewMessagesDivider()
                                }

                                TimelineItem.TimelineStart -> {
                                    StatusDivider(text = "Początek rozmowy")
                                }
                            }
                        }
                    }

                    if (headerContent != null) {
                        item(key = "event_header") {
                            headerContent()
                        }
                    }
                }

                if (timelineListState.isScrollInProgress && topTimelineLabel != null) {
                    Surface(
                        shape = RoundedCornerShape(14.dp),
                        color = Color.Black.copy(alpha = 0.86f),
                        shadowElevation = 1.dp,
                        modifier =
                            Modifier
                                .align(Alignment.TopCenter)
                                .padding(top = 10.dp),
                    ) {
                        Text(
                            text = topTimelineLabel.orEmpty(),
                            style = MaterialTheme.typography.labelMedium,
                            color = Color.White,
                            modifier = Modifier.padding(horizontal = 10.dp, vertical = 4.dp),
                        )
                    }
                }

                state.transientNotice?.let { notice ->
                    LaunchedEffect(notice) {
                        kotlinx.coroutines.delay(2_500L)
                        onClearTransientNotice()
                    }
                    AppSnackbar(
                        message = notice.message,
                        type = notice.type,
                        modifier =
                            Modifier
                                .align(Alignment.BottomCenter)
                                .padding(bottom = 12.dp, start = 16.dp, end = 16.dp),
                    )
                }

                if (state.isAwayFromLatest && state.unreadBelowCount > 0) {
                    FloatingActionButton(
                        onClick = {
                            coroutineScope.launch {
                                timelineListState.animateScrollToItem(0)
                                onJumpToLatest()
                            }
                        },
                        containerColor = Primary,
                        contentColor = Background,
                        modifier =
                            Modifier
                                .align(Alignment.BottomEnd)
                                .padding(bottom = 10.dp, end = 18.dp),
                    ) {
                        Row(
                            modifier = Modifier.padding(horizontal = 10.dp),
                            verticalAlignment = Alignment.CenterVertically,
                        ) {
                            Icon(
                                imageVector = PhosphorIcons.Bold.CaretDown,
                                contentDescription = "Najnowsze",
                            )
                            Spacer(modifier = Modifier.width(4.dp))
                            Text(state.unreadBelowCount.toString())
                        }
                    }
                }
            }
        }

        ComposerModeBanner(
            mode = state.composerMode,
            onCancel = onCancelComposerMode,
        )

        Row(
            modifier =
                Modifier
                    .fillMaxWidth()
                    .navigationBarsPadding()
                    .padding(horizontal = 10.dp, vertical = 8.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Surface(
                shape = RoundedCornerShape(28.dp),
                color = SurfaceColor,
                border = BorderStroke(1.dp, Border),
                modifier = Modifier.weight(1f),
            ) {
                Row(
                    modifier =
                        Modifier
                            .fillMaxWidth()
                            .padding(horizontal = 12.dp, vertical = 10.dp),
                    verticalAlignment = Alignment.CenterVertically,
                ) {
                    BasicTextField(
                        value = state.messageDraft,
                        onValueChange = { onDraftChanged(it) },
                        textStyle = MaterialTheme.typography.bodyLarge.copy(color = TextPrimary),
                        singleLine = true,
                        modifier = Modifier.weight(1f).padding(horizontal = 4.dp),
                        decorationBox = { innerTextField ->
                            if (state.messageDraft.isBlank()) {
                                Text(
                                    text = composerPlaceholder(state.composerMode),
                                    style = MaterialTheme.typography.bodyLarge,
                                    color = TextSecondary,
                                )
                            }
                            innerTextField()
                        },
                    )
                }
            }

            Spacer(modifier = Modifier.width(8.dp))

            Surface(
                color = if (state.messageDraft.isBlank()) SurfaceColor else Primary,
                shape = CircleShape,
                border = BorderStroke(1.dp, Border),
                modifier =
                    Modifier
                        .size(46.dp)
                        .clickable(enabled = state.messageDraft.isNotBlank()) { onSendMessage() },
            ) {
                Box(contentAlignment = Alignment.Center) {
                    Icon(
                        imageVector = PhosphorIcons.Fill.PaperPlaneRight,
                        contentDescription = "Wyślij",
                        tint = if (state.messageDraft.isBlank()) TextSecondary else Background,
                    )
                }
            }
        }
    }

    selectedActionEvent?.let { event ->
        MessageActionDialog(
            event = event,
            onDismiss = { selectedActionEvent = null },
            onReaction = { emoji ->
                onToggleReaction(event.eventOrTransactionId, emoji)
                selectedActionEvent = null
            },
            onReply = {
                onStartReply(event)
                selectedActionEvent = null
            },
            onCopy = { selectedActionEvent = null },
            onEdit = {
                onStartEdit(event)
                selectedActionEvent = null
            },
            onDelete = {
                onRedactEvent(event.eventOrTransactionId)
                selectedActionEvent = null
            },
        )
    }

    selectedReactionEvent?.let { event ->
        val senderIds =
            remember(event) {
                event.reactions
                    .flatMap { reaction -> reaction.senders.map { it.senderId } }
                    .distinct()
            }
        var senderNames by remember { mutableStateOf<Map<String, String>>(emptyMap()) }
        var senderAvatars by remember { mutableStateOf<Map<String, String>>(emptyMap()) }
        LaunchedEffect(senderIds) {
            senderNames = resolveDisplayNames(senderIds)
            senderAvatars = resolveAvatarUrls(senderIds)
        }
        ReactionBreakdownSheet(
            reactions = event.reactions,
            senderDisplayNames = senderNames,
            senderAvatarUrls = senderAvatars,
            avatarOverrides = avatarOverrides,
            avatarOverridesByName = avatarOverridesByName,
            onDismiss = { selectedReactionEvent = null },
        )
    }
}

@Composable
private fun MessageActionDialog(
    event: TimelineItem.Event,
    onDismiss: () -> Unit,
    onReaction: (String) -> Unit,
    onReply: () -> Unit,
    onCopy: () -> Unit,
    onEdit: () -> Unit,
    onDelete: () -> Unit,
) {
    @Suppress("DEPRECATION")
    val clipboardManager = LocalClipboardManager.current
    val eventId = event.eventId

    Dialog(
        onDismissRequest = onDismiss,
        properties = DialogProperties(usePlatformDefaultWidth = false),
    ) {
        Surface(
            shape = RoundedCornerShape(20.dp),
            color = SurfaceColor,
            modifier =
                Modifier
                    .fillMaxWidth()
                    .padding(horizontal = 32.dp),
        ) {
            Column(modifier = Modifier.padding(16.dp)) {
                Row(
                    modifier =
                        Modifier
                            .fillMaxWidth()
                            .padding(bottom = 12.dp),
                    horizontalArrangement = Arrangement.SpaceEvenly,
                ) {
                    listOf("❤️", "👍", "👎", "😂", "😮", "😢", "🔥", "🎉").forEach { emoji ->
                        Text(
                            text = emoji,
                            style = MaterialTheme.typography.headlineSmall,
                            modifier =
                                Modifier
                                    .clickable { onReaction(emoji) }
                                    .padding(4.dp),
                        )
                    }
                }

                Surface(
                    color = if (event.isMine) Primary.copy(alpha = 0.68f) else Background.copy(alpha = 0.5f),
                    shape = RoundedCornerShape(14.dp),
                    modifier = Modifier.fillMaxWidth(),
                ) {
                    Text(
                        text = event.body,
                        style = MaterialTheme.typography.bodyMedium,
                        color = TextPrimary,
                        maxLines = 3,
                        overflow = TextOverflow.Ellipsis,
                        modifier = Modifier.padding(12.dp),
                    )
                }

                Spacer(modifier = Modifier.height(8.dp))
                HorizontalDivider(color = Border)

                if (event.canReply && eventId != null) {
                    ActionMenuItem(
                        icon = PhosphorIcons.Bold.ArrowBendUpLeft,
                        label = "Odpowiedz",
                        onClick = onReply,
                    )
                }

                ActionMenuItem(
                    icon = PhosphorIcons.Bold.Copy,
                    label = "Skopiuj",
                    onClick = {
                        clipboardManager.setText(AnnotatedString(event.body))
                        onCopy()
                    },
                )

                if (event.isEditable) {
                    ActionMenuItem(
                        icon = PhosphorIcons.Bold.PencilSimple,
                        label = "Edytuj",
                        onClick = onEdit,
                    )
                    ActionMenuItem(
                        icon = PhosphorIcons.Bold.Trash,
                        label = "Usuń",
                        onClick = onDelete,
                    )
                }
            }
        }
    }
}

@Suppress("DEPRECATION")
@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun ReactionBreakdownSheet(
    reactions: List<Reaction>,
    senderDisplayNames: Map<String, String>,
    senderAvatarUrls: Map<String, String>,
    avatarOverrides: Map<String, String>,
    avatarOverridesByName: Map<String, String>,
    onDismiss: () -> Unit,
) {
    var selectedTab by remember { mutableIntStateOf(0) }
    val allSenders =
        reactions
            .flatMap { reaction ->
                reaction.senders
                    .distinctBy { sender -> sender.senderId }
                    .map { sender -> sender to reaction.emoji }
            }.distinctBy { (sender, emoji) -> sender.senderId to emoji }
    val totalCount = if (allSenders.isNotEmpty()) allSenders.size else reactions.sumOf { it.count }

    ModalBottomSheet(
        onDismissRequest = onDismiss,
        containerColor = SurfaceColor,
    ) {
        Column(
            modifier =
                Modifier
                    .fillMaxWidth()
                    .padding(bottom = 24.dp),
        ) {
            ScrollableTabRow(
                selectedTabIndex = selectedTab,
                containerColor = SurfaceColor,
                contentColor = Primary,
                edgePadding = 16.dp,
                divider = {},
            ) {
                Tab(
                    selected = selectedTab == 0,
                    onClick = { selectedTab = 0 },
                    text = {
                        Text(
                            text = "Wszystkie · $totalCount",
                            fontWeight = if (selectedTab == 0) FontWeight.Bold else FontWeight.Normal,
                        )
                    },
                )
                reactions.forEachIndexed { index, reaction ->
                    Tab(
                        selected = selectedTab == index + 1,
                        onClick = { selectedTab = index + 1 },
                        text = {
                            val senderCount =
                                reaction.senders.distinctBy { it.senderId }.size
                            val uniqueCount =
                                senderCount.takeIf { it > 0 } ?: reaction.count
                            Text(
                                text = "${reaction.emoji} $uniqueCount",
                                fontWeight = if (selectedTab == index + 1) FontWeight.Bold else FontWeight.Normal,
                            )
                        },
                    )
                }
            }

            Spacer(modifier = Modifier.height(8.dp))

            val displaySenders =
                if (selectedTab == 0) {
                    allSenders
                } else {
                    val reaction = reactions[selectedTab - 1]
                    reaction.senders
                        .distinctBy { sender -> sender.senderId }
                        .map { sender -> sender to reaction.emoji }
                }

            displaySenders.forEach { (sender, emoji) ->
                val name = senderDisplayNames[sender.senderId] ?: sender.senderId
                Row(
                    modifier =
                        Modifier
                            .fillMaxWidth()
                            .padding(horizontal = 16.dp, vertical = 10.dp),
                    verticalAlignment = Alignment.CenterVertically,
                ) {
                    UserAvatar(
                        picture =
                            senderAvatarUrls[sender.senderId]
                                ?: resolveAvatarOverride(sender.senderId, avatarOverrides)
                                ?: avatarOverridesByName[name.trim().lowercase()],
                        displayName = name,
                        size = 36.dp,
                        backgroundColor = Primary.copy(alpha = 0.2f),
                        iconTint = Primary,
                    )

                    Spacer(modifier = Modifier.width(12.dp))

                    Text(
                        text = name,
                        style = MaterialTheme.typography.bodyLarge,
                        color = TextPrimary,
                        maxLines = 1,
                        overflow = TextOverflow.Ellipsis,
                        modifier = Modifier.weight(1f),
                    )

                    Text(
                        text = emoji,
                        style = MaterialTheme.typography.bodyMedium,
                    )
                }
            }
        }
    }
}

@Composable
internal fun ActionMenuItem(
    icon: androidx.compose.ui.graphics.vector.ImageVector,
    label: String,
    onClick: () -> Unit,
    iconTint: Color = TextSecondary,
    labelColor: Color = TextPrimary,
) {
    Row(
        modifier =
            Modifier
                .fillMaxWidth()
                .clickable(onClick = onClick)
                .padding(horizontal = 12.dp, vertical = 12.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Icon(
            imageVector = icon,
            contentDescription = label,
            tint = iconTint,
            modifier = Modifier.size(18.dp),
        )
        Spacer(modifier = Modifier.width(12.dp))
        Text(
            text = label,
            style = MaterialTheme.typography.bodyMedium,
            fontWeight = FontWeight.Medium,
            color = labelColor,
        )
    }
}

@Composable
internal fun NewMessagesDivider() {
    Row(
        modifier = Modifier.fillMaxWidth().padding(vertical = 10.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        HorizontalDivider(modifier = Modifier.weight(1f), color = Primary.copy(alpha = 0.3f))
        Text(
            text = "NOWE WIADOMOŚCI",
            style = MaterialTheme.typography.labelSmall,
            color = Primary,
            modifier = Modifier.padding(horizontal = 8.dp),
        )
        HorizontalDivider(modifier = Modifier.weight(1f), color = Primary.copy(alpha = 0.3f))
    }
}

@Composable
internal fun ComposerModeBanner(
    mode: ComposerMode,
    onCancel: () -> Unit,
) {
    when (mode) {
        ComposerMode.NewMessage -> {
            Unit
        }

        is ComposerMode.Edit -> {
            Surface(
                modifier = Modifier.fillMaxWidth().padding(horizontal = 10.dp),
                color = SurfaceColor,
                border = BorderStroke(1.dp, Border),
                shape = RoundedCornerShape(12.dp),
            ) {
                Row(
                    modifier = Modifier.fillMaxWidth().padding(horizontal = 12.dp, vertical = 8.dp),
                    verticalAlignment = Alignment.CenterVertically,
                ) {
                    Text(
                        text = "Edycja wiadomości",
                        style = MaterialTheme.typography.labelMedium,
                        color = TextPrimary,
                        modifier = Modifier.weight(1f),
                    )
                    TextButton(onClick = onCancel) {
                        Text("Anuluj")
                    }
                }
            }
        }

        is ComposerMode.Reply -> {
            Surface(
                modifier = Modifier.fillMaxWidth().padding(horizontal = 10.dp),
                color = SurfaceColor,
                border = BorderStroke(1.dp, Border),
                shape = RoundedCornerShape(12.dp),
            ) {
                Row(
                    modifier = Modifier.fillMaxWidth().padding(horizontal = 12.dp, vertical = 8.dp),
                    verticalAlignment = Alignment.CenterVertically,
                ) {
                    Icon(
                        imageVector = PhosphorIcons.Bold.ArrowBendUpLeft,
                        contentDescription = null,
                        tint = TextSecondary,
                        modifier = Modifier.size(16.dp),
                    )
                    Spacer(modifier = Modifier.width(6.dp))
                    Column(modifier = Modifier.weight(1f)) {
                        Text(
                            text = "Odpowiedź do ${mode.senderDisplayName ?: "użytkownik"}",
                            style = MaterialTheme.typography.labelMedium,
                            color = TextPrimary,
                            maxLines = 1,
                        )
                        Text(
                            text = mode.bodyPreview,
                            style = MaterialTheme.typography.bodySmall,
                            color = TextSecondary,
                            maxLines = 1,
                        )
                    }
                    TextButton(onClick = onCancel) {
                        Text("Anuluj")
                    }
                }
            }
        }
    }
}

@Composable
internal fun DateDivider(timestampMillis: Long) {
    StatusDivider(text = formatDate(timestampMillis))
}

@Composable
internal fun StatusDivider(text: String) {
    Box(
        modifier = Modifier.fillMaxWidth().padding(vertical = 8.dp),
        contentAlignment = Alignment.Center,
    ) {
        Text(
            text = text,
            style = MaterialTheme.typography.labelSmall,
            color = TextSecondary,
        )
    }
}

internal fun composerPlaceholder(mode: ComposerMode): String =
    when (mode) {
        ComposerMode.NewMessage -> "Wiadomość"
        is ComposerMode.Reply -> "Odpowiedz..."
        is ComposerMode.Edit -> "Edytuj wiadomość..."
    }

internal fun shouldGroupWithPrevious(
    previous: TimelineItem.Event?,
    current: TimelineItem.Event,
): Boolean {
    if (previous == null) return false
    if (previous.senderId != current.senderId) return false
    if (previous.isMine != current.isMine) return false
    val delta = current.timestampMillis - previous.timestampMillis
    return delta in 0..(5 * 60 * 1000)
}

internal fun withDateDividers(items: List<TimelineItem>): List<TimelineItem> {
    if (items.isEmpty()) return items
    val zone = TimeZone.currentSystemDefault()
    val out = ArrayList<TimelineItem>(items.size + 4)
    var lastDate: LocalDate? = null
    for (item in items) {
        if (item is TimelineItem.Event) {
            val date = Instant.fromEpochMilliseconds(item.timestampMillis).toLocalDateTime(zone).date
            if (date != lastDate) {
                out.add(TimelineItem.DateDivider(item.timestampMillis))
                lastDate = date
            }
        }
        out.add(item)
    }
    return out
}

internal fun formatDate(timestampMillis: Long): String {
    val zone = TimeZone.currentSystemDefault()
    val now =
        Clock.System
            .now()
            .toLocalDateTime(zone)
            .date
    val date = Instant.fromEpochMilliseconds(timestampMillis).toLocalDateTime(zone).date
    val daysDiff = now.toEpochDays() - date.toEpochDays()

    return when {
        daysDiff == 0L -> "Dzisiaj"
        daysDiff == 1L -> "Wczoraj"
        daysDiff in 2L..6L -> weekdayShort(date.dayOfWeek)
        now.year == date.year -> "${date.day} ${monthShort(date.month.ordinal + 1)}"
        else -> "${date.day} ${monthShort(date.month.ordinal + 1)} ${date.year}"
    }
}

private fun formatTimelineCapsuleLabel(item: TimelineItem?): String? =
    when (item) {
        is TimelineItem.DateDivider -> formatDate(item.timestampMillis)
        is TimelineItem.Event -> formatDate(item.timestampMillis)
        else -> null
    }

private fun weekdayShort(dayOfWeek: DayOfWeek): String =
    when (dayOfWeek) {
        DayOfWeek.MONDAY -> "pon"
        DayOfWeek.TUESDAY -> "wt"
        DayOfWeek.WEDNESDAY -> "śr"
        DayOfWeek.THURSDAY -> "czw"
        DayOfWeek.FRIDAY -> "pt"
        DayOfWeek.SATURDAY -> "sob"
        DayOfWeek.SUNDAY -> "niedz"
    }

private fun monthShort(monthNumber: Int): String =
    when (monthNumber) {
        1 -> "sty"
        2 -> "lut"
        3 -> "mar"
        4 -> "kwi"
        5 -> "maj"
        6 -> "cze"
        7 -> "lip"
        8 -> "sie"
        9 -> "wrz"
        10 -> "paź"
        11 -> "lis"
        12 -> "gru"
        else -> "?"
    }

internal fun timelineItemKey(item: TimelineItem): String =
    when (item) {
        is TimelineItem.Event -> "event_${item.eventOrTransactionId}"
        is TimelineItem.DateDivider -> "date_${item.timestampMillis}"
        TimelineItem.ReadMarker -> "read_marker"
        TimelineItem.TimelineStart -> "timeline_start"
    }

/**
 * Resolve a profile picture URL for a sender ID.
 * User IDs are plain UUIDs or integer strings; the [overrides] map is keyed by userId.
 */
internal fun resolveAvatarOverride(
    senderId: String,
    overrides: Map<String, String>,
): String? {
    if (overrides.isEmpty()) return null
    val id = senderId.trim()
    return overrides[id] ?: overrides[id.lowercase()]
}
