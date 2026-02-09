package com.poziomki.app.ui.screen.chat

import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.imePadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.itemsIndexed
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.automirrored.filled.Reply
import androidx.compose.material.icons.automirrored.filled.Send
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
import androidx.compose.material3.TextButton
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
import androidx.compose.ui.unit.dp
import com.poziomki.app.chat.matrix.api.MatrixTimelineItem
import com.poziomki.app.chat.matrix.api.MatrixTimelineMode
import com.poziomki.app.ui.screen.chat.model.ComposerMode
import com.poziomki.app.ui.theme.Background
import com.poziomki.app.ui.theme.Border
import com.poziomki.app.ui.theme.NunitoFamily
import com.poziomki.app.ui.theme.PoziomkiTheme
import com.poziomki.app.ui.theme.Primary
import com.poziomki.app.ui.theme.TextPrimary
import com.poziomki.app.ui.theme.TextSecondary
import kotlinx.datetime.Instant
import kotlinx.datetime.TimeZone
import kotlinx.datetime.toLocalDateTime
import org.koin.compose.viewmodel.koinViewModel
import com.poziomki.app.ui.theme.Surface as SurfaceColor

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun ChatScreen(
    chatId: String,
    onBack: () -> Unit,
    onNavigateToProfile: (String) -> Unit,
    viewModel: ChatViewModel = koinViewModel(),
) {
    val state by viewModel.uiState.collectAsState()

    var actionMenuEventId by remember { mutableStateOf<String?>(null) }

    LaunchedEffect(chatId) {
        viewModel.loadRoom(chatId)
    }

    LaunchedEffect(state.timelineItems.size) {
        if (state.timelineItems.isNotEmpty()) {
            viewModel.markAsRead()
        }
    }

    Scaffold(
        containerColor = Background,
        topBar = {
            TopAppBar(
                title = {
                    Text(
                        state.roomDisplayName.ifBlank { "chat" },
                        color = TextPrimary,
                    )
                },
                navigationIcon = {
                    IconButton(onClick = onBack) {
                        Icon(
                            Icons.AutoMirrored.Filled.ArrowBack,
                            contentDescription = "Back",
                            tint = TextPrimary,
                        )
                    }
                },
                actions = {
                    if (state.timelineMode is MatrixTimelineMode.FocusedOnEvent) {
                        TextButton(onClick = { viewModel.enterLiveTimeline() }) {
                            Text("na zywo")
                        }
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
                            .clickable { viewModel.clearError() },
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
                LazyColumn(
                    modifier =
                        Modifier
                            .weight(1f)
                            .fillMaxWidth()
                            .padding(horizontal = PoziomkiTheme.spacing.md),
                ) {
                    item {
                        Button(
                            onClick = { viewModel.paginateBackwards() },
                            enabled = state.hasMoreBackwards && !state.isPaginatingBackwards,
                            modifier =
                                Modifier
                                    .fillMaxWidth()
                                    .padding(top = PoziomkiTheme.spacing.sm),
                        ) {
                            val label =
                                when {
                                    state.isPaginatingBackwards -> "ladowanie..."
                                    state.hasMoreBackwards -> "starsze wiadomosci"
                                    else -> "brak starszych wiadomosci"
                                }
                            Text(label)
                        }
                    }

                    itemsIndexed(
                        items = state.timelineItems,
                        key = { index, item -> timelineItemKey(index, item) },
                    ) { index, item ->
                        when (item) {
                            is MatrixTimelineItem.DateDivider -> {
                                DateDivider(timestampMillis = item.timestampMillis)
                            }

                            is MatrixTimelineItem.Event -> {
                                val previousEvent = state.timelineItems.getOrNull(index - 1) as? MatrixTimelineItem.Event
                                MessageEventRow(
                                    event = item,
                                    groupedWithPrevious = shouldGroupWithPrevious(previousEvent, item),
                                    menuExpanded = actionMenuEventId == item.eventOrTransactionId,
                                    onToggleReaction = { emoji ->
                                        viewModel.toggleReaction(item.eventOrTransactionId, emoji)
                                    },
                                    onFocusOnEvent = {
                                        item.eventId?.let(viewModel::focusOnEvent)
                                    },
                                    onFocusOnReply = {
                                        item.inReplyTo?.eventId?.let(viewModel::focusOnEvent)
                                    },
                                    onSenderClick = { onNavigateToProfile(item.senderId) },
                                    onMenuOpen = { actionMenuEventId = item.eventOrTransactionId },
                                    onMenuDismiss = { actionMenuEventId = null },
                                    onReply = {
                                        viewModel.startReply(item)
                                        actionMenuEventId = null
                                    },
                                    onEdit = {
                                        viewModel.startEdit(item)
                                        actionMenuEventId = null
                                    },
                                    onDelete = {
                                        viewModel.redactEvent(item.eventOrTransactionId)
                                        actionMenuEventId = null
                                    },
                                )
                            }

                            MatrixTimelineItem.ReadMarker -> {
                                StatusDivider(text = "przeczytano")
                            }

                            MatrixTimelineItem.TimelineStart -> {
                                StatusDivider(text = "poczatek rozmowy")
                            }
                        }
                    }

                    item {
                        if (state.typingUserIds.isNotEmpty()) {
                            Text(
                                text = "pisze: ${state.typingUserIds.joinToString()}",
                                fontFamily = NunitoFamily,
                                color = TextSecondary,
                                modifier = Modifier.padding(vertical = PoziomkiTheme.spacing.sm),
                            )
                        }
                    }
                }
            }

            ComposerModeBanner(
                mode = state.composerMode,
                onCancel = viewModel::cancelComposerMode,
            )

            Row(
                modifier =
                    Modifier
                        .fillMaxWidth()
                        .padding(PoziomkiTheme.spacing.md),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                OutlinedTextField(
                    value = state.messageDraft,
                    onValueChange = { viewModel.onDraftChanged(it) },
                    modifier = Modifier.weight(1f),
                    placeholder = {
                        Text(composerPlaceholder(state.composerMode))
                    },
                    singleLine = true,
                )
                Spacer(modifier = Modifier.width(PoziomkiTheme.spacing.sm))
                Surface(
                    color = Primary,
                    shape = RoundedCornerShape(12.dp),
                    modifier =
                        Modifier
                            .size(46.dp)
                            .clickable(enabled = state.messageDraft.isNotBlank()) { viewModel.sendMessage() },
                ) {
                    Box(contentAlignment = Alignment.Center) {
                        Icon(
                            imageVector = Icons.AutoMirrored.Filled.Send,
                            contentDescription = "Send",
                            tint = Background,
                        )
                    }
                }
            }
        }
    }
}

@Composable
private fun ComposerModeBanner(
    mode: ComposerMode,
    onCancel: () -> Unit,
) {
    when (mode) {
        ComposerMode.NewMessage -> {
            Unit
        }

        is ComposerMode.Edit -> {
            Surface(
                modifier = Modifier.fillMaxWidth().padding(horizontal = PoziomkiTheme.spacing.md),
                color = SurfaceColor,
                border = BorderStroke(1.dp, Border),
                shape = RoundedCornerShape(10.dp),
            ) {
                Row(
                    modifier = Modifier.fillMaxWidth().padding(horizontal = 10.dp, vertical = 8.dp),
                    verticalAlignment = Alignment.CenterVertically,
                ) {
                    Text(
                        text = "edycja wiadomosci",
                        style = MaterialTheme.typography.labelMedium,
                        color = TextPrimary,
                        modifier = Modifier.weight(1f),
                    )
                    TextButton(onClick = onCancel) {
                        Text("anuluj")
                    }
                }
            }
        }

        is ComposerMode.Reply -> {
            Surface(
                modifier = Modifier.fillMaxWidth().padding(horizontal = PoziomkiTheme.spacing.md),
                color = SurfaceColor,
                border = BorderStroke(1.dp, Border),
                shape = RoundedCornerShape(10.dp),
            ) {
                Row(
                    modifier = Modifier.fillMaxWidth().padding(horizontal = 10.dp, vertical = 8.dp),
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
                            text = "odpowiedz do ${mode.senderDisplayName ?: "uzytkownik"}",
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
                        Text("anuluj")
                    }
                }
            }
        }
    }
}

@Composable
private fun DateDivider(timestampMillis: Long) {
    StatusDivider(text = formatDate(timestampMillis))
}

@Composable
private fun StatusDivider(text: String) {
    Box(
        modifier =
            Modifier
                .fillMaxWidth()
                .padding(vertical = 8.dp),
        contentAlignment = Alignment.Center,
    ) {
        Text(
            text = text,
            style = MaterialTheme.typography.labelSmall,
            color = TextSecondary,
        )
    }
}

private fun composerPlaceholder(mode: ComposerMode): String =
    when (mode) {
        ComposerMode.NewMessage -> "napisz wiadomosc..."
        is ComposerMode.Reply -> "odpowiedz..."
        is ComposerMode.Edit -> "edytuj wiadomosc..."
    }

private fun shouldGroupWithPrevious(
    previous: MatrixTimelineItem.Event?,
    current: MatrixTimelineItem.Event,
): Boolean {
    if (previous == null) return false
    if (previous.senderId != current.senderId) return false
    if (previous.isMine != current.isMine) return false
    val delta = current.timestampMillis - previous.timestampMillis
    return delta in 0..(5 * 60 * 1000)
}

private fun formatDate(timestampMillis: Long): String {
    val localDateTime = Instant.fromEpochMilliseconds(timestampMillis).toLocalDateTime(TimeZone.currentSystemDefault())
    val day = localDateTime.dayOfMonth.toString().padStart(2, '0')
    val month = localDateTime.monthNumber.toString().padStart(2, '0')
    val year = localDateTime.year
    return "$day.$month.$year"
}

private fun timelineItemKey(
    index: Int,
    item: MatrixTimelineItem,
): String =
    when (item) {
        is MatrixTimelineItem.Event -> "event_${item.eventOrTransactionId}"
        is MatrixTimelineItem.DateDivider -> "date_${item.timestampMillis}_$index"
        MatrixTimelineItem.ReadMarker -> "read_$index"
        MatrixTimelineItem.TimelineStart -> "start_$index"
    }
