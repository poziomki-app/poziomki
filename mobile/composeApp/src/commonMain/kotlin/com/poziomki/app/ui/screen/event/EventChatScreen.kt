package com.poziomki.app.ui.screen.event

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.lazy.rememberLazyListState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import com.poziomki.app.ui.screen.chat.ChatContent
import com.poziomki.app.ui.screen.chat.ChatViewModel
import com.poziomki.app.ui.theme.Background
import org.koin.compose.viewmodel.koinViewModel

@Composable
fun EventChatScreen(
    onBack: () -> Unit,
    onNavigateToProfile: (String) -> Unit,
    eventDetailViewModel: EventDetailViewModel = koinViewModel(),
    chatViewModel: ChatViewModel = koinViewModel(),
) {
    val eventState by eventDetailViewModel.state.collectAsState()
    val chatState by chatViewModel.uiState.collectAsState()
    val timelineListState = rememberLazyListState()

    LaunchedEffect(eventState.event?.conversationId, eventState.event?.isAttending) {
        val event = eventState.event
        val roomId = event?.conversationId
        if (event?.isAttending == true && roomId != null && roomId.startsWith("!")) {
            chatViewModel.loadRoom(roomId)
        }
    }

    LaunchedEffect(eventState.event?.id, eventState.event?.isAttending, eventState.event?.conversationId, eventState.isOpeningChat) {
        val event = eventState.event ?: return@LaunchedEffect
        if (event.isAttending && event.conversationId == null && !eventState.isOpeningChat) {
            eventDetailViewModel.openEventChat { }
        }
    }

    val avatarOverrides =
        remember(eventState.attendees) {
            buildEventAvatarOverrides(eventState.attendees)
        }

    Column(modifier = Modifier.fillMaxSize().background(Background)) {
        when {
            eventState.isLoading && eventState.event == null -> {
                EventChatLoadingView()
            }

            eventState.event == null -> {
                EventChatNotFoundView()
            }

            eventState.event?.isAttending != true -> {
                EventChatJoinRequiredView(onJoin = eventDetailViewModel::attendEvent)
            }

            eventState.isOpeningChat || eventState.event?.conversationId?.startsWith("!") != true -> {
                EventChatLoadingView()
            }

            else -> {
                val event = requireNotNull(eventState.event)
                ChatContent(
                    state = chatState,
                    timelineListState = timelineListState,
                    onDraftChanged = chatViewModel::onDraftChanged,
                    onSendMessage = chatViewModel::sendMessage,
                    onSendImageAttachment = chatViewModel::sendImageAttachment,
                    onSendFileAttachment = chatViewModel::sendFileAttachment,
                    onToggleReaction = chatViewModel::toggleReaction,
                    onPaginateBackwards = chatViewModel::paginateBackwards,
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
                    avatarOverrides = avatarOverrides,
                    headerContent = {
                        EventChatHeader(
                            event = event,
                            onBack = onBack,
                            onNavigateToProfile = onNavigateToProfile,
                            onJoin = eventDetailViewModel::attendEvent,
                            onLeave = eventDetailViewModel::leaveEvent,
                        )
                    },
                )
            }
        }
    }
}
