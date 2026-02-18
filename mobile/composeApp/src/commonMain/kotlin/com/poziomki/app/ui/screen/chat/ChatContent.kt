package com.poziomki.app.ui.screen.chat

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
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.LazyListState
import androidx.compose.foundation.lazy.itemsIndexed
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.BasicTextField
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.Reply
import androidx.compose.material.icons.automirrored.filled.Send
import androidx.compose.material.icons.filled.Add
import androidx.compose.material.icons.filled.ContentCopy
import androidx.compose.material.icons.filled.Delete
import androidx.compose.material.icons.filled.Edit
import androidx.compose.material.icons.filled.KeyboardArrowDown
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
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
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.runtime.snapshotFlow
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalClipboardManager
import androidx.compose.ui.text.AnnotatedString
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.window.Dialog
import androidx.compose.ui.window.DialogProperties
import com.poziomki.app.chat.matrix.api.MatrixReaction
import com.poziomki.app.chat.matrix.api.MatrixTimelineItem
import com.poziomki.app.ui.screen.chat.model.ChatUiState
import com.poziomki.app.ui.screen.chat.model.ComposerMode
import com.poziomki.app.ui.theme.Background
import com.poziomki.app.ui.theme.Border
import com.poziomki.app.ui.theme.PoziomkiTheme
import com.poziomki.app.ui.theme.Primary
import com.poziomki.app.ui.theme.TextPrimary
import com.poziomki.app.ui.theme.TextSecondary
import com.poziomki.app.util.PickedFile
import com.poziomki.app.util.rememberSingleFilePicker
import com.poziomki.app.util.rememberSingleImagePicker
import kotlinx.coroutines.flow.distinctUntilChanged
import kotlinx.coroutines.launch
import kotlinx.datetime.Instant
import kotlinx.datetime.TimeZone
import kotlinx.datetime.toLocalDateTime
import com.poziomki.app.ui.theme.Surface as SurfaceColor

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun ChatContent(
    state: ChatUiState,
    timelineListState: LazyListState,
    onDraftChanged: (String) -> Unit,
    onSendMessage: () -> Unit,
    onSendImageAttachment: (ByteArray) -> Unit,
    onSendFileAttachment: (PickedFile) -> Unit,
    onToggleReaction: (String, String) -> Unit,
    onPaginateBackwards: () -> Unit,
    onMarkAsRead: () -> Unit,
    onViewportChanged: (Int?) -> Unit,
    onJumpToLatest: () -> Unit,
    onStartReply: (MatrixTimelineItem.Event) -> Unit,
    onStartEdit: (MatrixTimelineItem.Event) -> Unit,
    onCancelComposerMode: () -> Unit,
    onRedactEvent: (String) -> Unit,
    onClearError: () -> Unit,
    onNavigateToProfile: (String) -> Unit,
    resolveDisplayNames: suspend (List<String>) -> Map<String, String>,
    modifier: Modifier = Modifier,
    headerContent: (@Composable () -> Unit)? = null,
) {
    val coroutineScope = rememberCoroutineScope()

    var selectedActionEvent by remember { mutableStateOf<MatrixTimelineItem.Event?>(null) }
    var selectedReactionEvent by remember { mutableStateOf<MatrixTimelineItem.Event?>(null) }
    var showAttachmentMenu by remember { mutableStateOf(false) }

    val pickImage = rememberSingleImagePicker { bytes -> bytes?.let(onSendImageAttachment) }
    val pickFile = rememberSingleFilePicker { file -> file?.let(onSendFileAttachment) }

    LaunchedEffect(timelineListState) {
        snapshotFlow {
            val layoutInfo = timelineListState.layoutInfo
            val lastVisibleIndex = layoutInfo.visibleItemsInfo.lastOrNull()?.index
            val totalItems = layoutInfo.totalItemsCount
            Triple(
                lastVisibleIndex,
                totalItems,
                totalItems > 0 && (lastVisibleIndex ?: -1) >= (totalItems - 2),
            )
        }.distinctUntilChanged()
            .collect { (lastVisibleIndex, _, atLatest) ->
                onViewportChanged(lastVisibleIndex)
                if (atLatest) {
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
                text = state.error ?: "",
                color = MaterialTheme.colorScheme.error,
                style = MaterialTheme.typography.bodySmall,
                modifier =
                    Modifier
                        .fillMaxWidth()
                        .padding(horizontal = PoziomkiTheme.spacing.md, vertical = PoziomkiTheme.spacing.sm)
                        .clickable { onClearError() },
            )
        }

        if (state.isLoading && state.timelineItems.isEmpty()) {
            Box(
                modifier = Modifier.weight(1f).fillMaxWidth(),
                contentAlignment = Alignment.Center,
            ) {
                CircularProgressIndicator(color = Primary)
            }
        } else {
            Box(
                modifier =
                    Modifier
                        .weight(1f)
                        .fillMaxWidth(),
            ) {
                val itemPadding = Modifier.padding(horizontal = 10.dp)

                LazyColumn(
                    state = timelineListState,
                    modifier = Modifier.fillMaxSize(),
                ) {
                    if (headerContent != null) {
                        item(key = "event_header") {
                            headerContent()
                        }
                    }

                    item {
                        TextButton(
                            onClick = { onPaginateBackwards() },
                            enabled = state.hasMoreBackwards && !state.isPaginatingBackwards,
                            modifier = itemPadding.fillMaxWidth(),
                        ) {
                            val label =
                                when {
                                    state.isPaginatingBackwards -> "Ładowanie..."
                                    state.hasMoreBackwards -> "Pokaż starsze wiadomości"
                                    else -> "Brak starszych wiadomości"
                                }
                            Text(label)
                        }
                    }

                    itemsIndexed(
                        items = state.timelineItems,
                        key = { index, item -> timelineItemKey(index, item) },
                    ) { index, item ->
                        Box(modifier = itemPadding) {
                            when (item) {
                                is MatrixTimelineItem.DateDivider -> {
                                    DateDivider(timestampMillis = item.timestampMillis)
                                }

                                is MatrixTimelineItem.Event -> {
                                    val previousEvent =
                                        state.timelineItems.getOrNull(index - 1) as? MatrixTimelineItem.Event
                                    MessageEventRow(
                                        event = item,
                                        groupedWithPrevious = shouldGroupWithPrevious(previousEvent, item),
                                        onToggleReaction = { emoji ->
                                            onToggleReaction(item.eventOrTransactionId, emoji)
                                        },
                                        onReactionsClick = { selectedReactionEvent = item },
                                        onFocusOnReply = { },
                                        onSenderClick = { onNavigateToProfile(item.senderId) },
                                        onActionsLongPress = { selectedActionEvent = item },
                                        onSwipeReply = { onStartReply(item) },
                                    )
                                }

                                MatrixTimelineItem.ReadMarker -> {
                                    NewMessagesDivider()
                                }

                                MatrixTimelineItem.TimelineStart -> {
                                    StatusDivider(text = "Początek rozmowy")
                                }
                            }
                        }
                    }

                    item {
                        if (state.typingUserIds.isNotEmpty()) {
                            Surface(
                                shape = RoundedCornerShape(14.dp),
                                color = SurfaceColor,
                                modifier = itemPadding.padding(vertical = 8.dp),
                            ) {
                                Text(
                                    text = "Pisze: ${state.typingUserIds.joinToString()}",
                                    style = MaterialTheme.typography.bodySmall,
                                    color = TextSecondary,
                                    modifier = Modifier.padding(horizontal = 10.dp, vertical = 6.dp),
                                )
                            }
                        }
                    }
                    item { Spacer(modifier = Modifier.height(8.dp)) }
                }

                if (state.isAwayFromLatest && state.unreadBelowCount > 0) {
                    FloatingActionButton(
                        onClick = {
                            coroutineScope.launch {
                                val lastIndex = (timelineListState.layoutInfo.totalItemsCount - 1).coerceAtLeast(0)
                                timelineListState.animateScrollToItem(lastIndex)
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
                                imageVector = Icons.Filled.KeyboardArrowDown,
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
                            .padding(horizontal = 6.dp, vertical = 6.dp),
                    verticalAlignment = Alignment.CenterVertically,
                ) {
                    Box {
                        IconButton(onClick = { showAttachmentMenu = true }) {
                            Icon(
                                imageVector = Icons.Filled.Add,
                                contentDescription = "Załącznik",
                                tint = TextSecondary,
                            )
                        }
                        DropdownMenu(
                            expanded = showAttachmentMenu,
                            onDismissRequest = { showAttachmentMenu = false },
                        ) {
                            DropdownMenuItem(
                                text = { Text("Wyślij zdjęcie") },
                                onClick = {
                                    pickImage()
                                    showAttachmentMenu = false
                                },
                            )
                            DropdownMenuItem(
                                text = { Text("Wyślij plik") },
                                onClick = {
                                    pickFile()
                                    showAttachmentMenu = false
                                },
                            )
                        }
                    }
                    BasicTextField(
                        value = state.messageDraft,
                        onValueChange = { onDraftChanged(it) },
                        textStyle = MaterialTheme.typography.bodyLarge.copy(color = TextPrimary),
                        singleLine = true,
                        modifier = Modifier.weight(1f).padding(horizontal = 2.dp),
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
                        imageVector = Icons.AutoMirrored.Filled.Send,
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
        LaunchedEffect(senderIds) {
            senderNames = resolveDisplayNames(senderIds)
        }
        ReactionBreakdownSheet(
            reactions = event.reactions,
            senderDisplayNames = senderNames,
            onDismiss = { selectedReactionEvent = null },
        )
    }
}

@Composable
private fun MessageActionDialog(
    event: MatrixTimelineItem.Event,
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
                        icon = Icons.AutoMirrored.Filled.Reply,
                        label = "Odpowiedz",
                        onClick = onReply,
                    )
                }

                ActionMenuItem(
                    icon = Icons.Filled.ContentCopy,
                    label = "Skopiuj",
                    onClick = {
                        clipboardManager.setText(AnnotatedString(event.body))
                        onCopy()
                    },
                )

                if (event.isEditable) {
                    ActionMenuItem(
                        icon = Icons.Filled.Edit,
                        label = "Edytuj",
                        onClick = onEdit,
                    )
                    ActionMenuItem(
                        icon = Icons.Filled.Delete,
                        label = "Usuń",
                        onClick = onDelete,
                    )
                }
            }
        }
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun ReactionBreakdownSheet(
    reactions: List<MatrixReaction>,
    senderDisplayNames: Map<String, String>,
    onDismiss: () -> Unit,
) {
    var selectedTab by remember { mutableIntStateOf(0) }
    val allSenders = reactions.flatMap { r -> r.senders.map { s -> s to r.emoji } }
    val totalCount = reactions.sumOf { it.count }

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
                            Text(
                                text = "${reaction.emoji} ${reaction.count}",
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
                    reaction.senders.map { s -> s to reaction.emoji }
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
                    Surface(
                        modifier = Modifier.size(36.dp),
                        shape = CircleShape,
                        color = Primary.copy(alpha = 0.2f),
                    ) {
                        Box(contentAlignment = Alignment.Center) {
                            Text(
                                text = name.first().uppercase(),
                                style = MaterialTheme.typography.labelMedium,
                                color = Primary,
                            )
                        }
                    }

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
                        style = MaterialTheme.typography.titleMedium,
                    )
                }
            }
        }
    }
}

@Composable
private fun ActionMenuItem(
    icon: androidx.compose.ui.graphics.vector.ImageVector,
    label: String,
    onClick: () -> Unit,
) {
    Row(
        modifier =
            Modifier
                .fillMaxWidth()
                .clickable(onClick = onClick)
                .padding(horizontal = 4.dp, vertical = 14.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Icon(
            imageVector = icon,
            contentDescription = label,
            tint = TextSecondary,
            modifier = Modifier.size(22.dp),
        )
        Spacer(modifier = Modifier.width(16.dp))
        Text(
            text = label,
            style = MaterialTheme.typography.bodyLarge,
            fontWeight = FontWeight.Medium,
            color = TextPrimary,
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
                        imageVector = Icons.AutoMirrored.Filled.Reply,
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
    previous: MatrixTimelineItem.Event?,
    current: MatrixTimelineItem.Event,
): Boolean {
    if (previous == null) return false
    if (previous.senderId != current.senderId) return false
    if (previous.isMine != current.isMine) return false
    val delta = current.timestampMillis - previous.timestampMillis
    return delta in 0..(5 * 60 * 1000)
}

internal fun formatDate(timestampMillis: Long): String {
    val localDateTime = Instant.fromEpochMilliseconds(timestampMillis).toLocalDateTime(TimeZone.currentSystemDefault())
    val day = localDateTime.dayOfMonth.toString().padStart(2, '0')
    val month = localDateTime.monthNumber.toString().padStart(2, '0')
    val year = localDateTime.year
    return "$day.$month.$year"
}

internal fun timelineItemKey(
    index: Int,
    item: MatrixTimelineItem,
): String =
    when (item) {
        is MatrixTimelineItem.Event -> "event_${item.eventOrTransactionId}"
        is MatrixTimelineItem.DateDivider -> "date_${item.timestampMillis}_$index"
        MatrixTimelineItem.ReadMarker -> "read_$index"
        MatrixTimelineItem.TimelineStart -> "start_$index"
    }
