package com.poziomki.app.ui.feature.event

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.rememberLazyListState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import com.poziomki.app.chat.ActiveChat
import com.poziomki.app.ui.designsystem.components.AppSnackbar
import com.poziomki.app.ui.designsystem.components.ReportDialog
import com.poziomki.app.ui.designsystem.theme.Background
import com.poziomki.app.ui.designsystem.theme.PoziomkiTheme
import com.poziomki.app.ui.feature.chat.ChatContent
import com.poziomki.app.ui.feature.chat.ChatViewModel
import org.koin.compose.viewmodel.koinViewModel

@Composable
@Suppress("LongMethod", "CyclomaticComplexMethod", "Indentation")
fun EventChatScreen(
    onBack: () -> Unit,
    onNavigateToProfile: (String) -> Unit,
    onNavigateToEditEvent: (String) -> Unit,
    eventDetailViewModel: EventDetailViewModel = koinViewModel(),
    chatViewModel: ChatViewModel = koinViewModel(),
) {
    val eventState by eventDetailViewModel.state.collectAsState()
    val chatState by chatViewModel.uiState.collectAsState()
    val timelineListState = rememberLazyListState()
    var showReportDialog by remember { mutableStateOf(false) }

    val conversationId = eventState.event?.conversationId

    LaunchedEffect(conversationId, eventState.event?.isAttending) {
        val event = eventState.event
        if (event?.isAttending == true && conversationId != null && conversationId.isNotBlank()) {
            chatViewModel.loadRoom(conversationId, fallbackDisplayName = event.title)
        }
    }

    DisposableEffect(conversationId) {
        ActiveChat.roomId = conversationId
        onDispose { ActiveChat.roomId = null }
    }

    LaunchedEffect(eventState.event?.id, eventState.event?.isAttending, eventState.event?.conversationId) {
        val event = eventState.event ?: return@LaunchedEffect
        if (event.isAttending && event.conversationId == null && !eventState.isOpeningChat) {
            eventDetailViewModel.openEventChat()
        }
    }

    val avatarOverrides =
        remember(eventState.attendees) {
            buildEventAvatarOverrides(eventState.attendees)
        }
    val avatarOverridesByName =
        remember(eventState.attendees) {
            eventState.attendees
                .asSequence()
                .filter { attendee -> attendee.profilePicture?.isNotBlank() == true }
                .groupBy { attendee -> attendee.name.trim().lowercase() }
                .mapNotNull { (name, sameNameAttendees) ->
                    val pictures = sameNameAttendees.mapNotNull { attendee -> attendee.profilePicture }.distinct()
                    if (pictures.size == 1) name to pictures.first() else null
                }.toMap()
        }

    Box(modifier = Modifier.fillMaxSize().background(Background)) {
        Column(modifier = Modifier.fillMaxSize()) {
            when {
                eventState.isLoading && eventState.event == null -> {
                    var showSpinner by remember { mutableStateOf(false) }
                    LaunchedEffect(Unit) {
                        kotlinx.coroutines.delay(300)
                        showSpinner = true
                    }
                    if (showSpinner) EventChatLoadingView(onBack = onBack)
                }

                eventState.event == null -> {
                    EventChatNotFoundView(onBack = onBack)
                }

                eventState.event?.isAttending != true -> {
                    val event = requireNotNull(eventState.event)
                    EventChatJoinRequiredView(
                        event = event,
                        isUpdatingAttendance = eventState.isUpdatingAttendance,
                        onJoin = eventDetailViewModel::attendEvent,
                        onBack = onBack,
                    )
                }

                eventState.isOpeningChat || eventState.event?.conversationId?.isNullOrBlank() ?: true -> {
                    var showSpinner by remember { mutableStateOf(false) }
                    LaunchedEffect(Unit) {
                        kotlinx.coroutines.delay(300)
                        showSpinner = true
                    }
                    if (showSpinner) EventChatLoadingView(onBack = onBack)
                }

                else -> {
                    val event = requireNotNull(eventState.event)
                    EventChatHeader(
                        event = event,
                        attendees = eventState.attendees,
                        isCreator = eventState.isCreator,
                        onBack = onBack,
                        onNavigateToProfile = onNavigateToProfile,
                        onJoin = eventDetailViewModel::attendEvent,
                        onLeave = eventDetailViewModel::leaveEvent,
                        onDelete = { eventDetailViewModel.deleteEvent(onBack) },
                        onEdit = { onNavigateToEditEvent(event.id) },
                        onReport = { showReportDialog = true },
                        onRemove = {
                            chatViewModel.removeConversation()
                            onBack()
                        },
                    )
                    ChatContent(
                        modifier = Modifier.weight(1f),
                        state = chatState,
                        timelineListState = timelineListState,
                        onDraftChanged = chatViewModel::onDraftChanged,
                        onSendMessage = chatViewModel::sendMessage,
                        onToggleReaction = chatViewModel::toggleReaction,
                        onMarkAsRead = chatViewModel::markAsRead,
                        onViewportChanged = chatViewModel::onTimelineViewportChanged,
                        onJumpToLatest = chatViewModel::jumpToLatestHandled,
                        onStartReply = chatViewModel::startReply,
                        onStartEdit = chatViewModel::startEdit,
                        onCancelComposerMode = chatViewModel::cancelComposerMode,
                        onRedactEvent = chatViewModel::redactEvent,
                        onClearError = chatViewModel::clearError,
                        onNavigateToProfile = onNavigateToProfile,
                        resolveDisplayNames = chatViewModel::resolveDisplayNames,
                        resolveAvatarUrls = chatViewModel::resolveAvatarUrls,
                        showSenderMeta = true,
                        avatarOverrides = avatarOverrides,
                        avatarOverridesByName = avatarOverridesByName,
                    )
                }
            }
        }

        if (showReportDialog) {
            ReportDialog(
                onConfirm = { reason, description ->
                    showReportDialog = false
                    chatViewModel.reportConversation(reason, description)
                },
                onDismiss = { showReportDialog = false },
            )
        }

        // Snackbar overlay
        eventState.snackbarMessage?.let { message ->
            AppSnackbar(
                message = message,
                type = eventState.snackbarType,
                modifier =
                    Modifier
                        .align(Alignment.BottomCenter)
                        .padding(PoziomkiTheme.spacing.md),
            )
            LaunchedEffect(message) {
                kotlinx.coroutines.delay(3000)
                eventDetailViewModel.clearSnackbar()
            }
        }
    }
}
