package com.poziomki.app.ui.screen.event

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.aspectRatio
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.rememberLazyListState
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.filled.BookmarkBorder
import androidx.compose.material.icons.filled.CalendarMonth
import androidx.compose.material.icons.filled.Groups
import androidx.compose.material.icons.filled.LocationOn
import androidx.compose.material.icons.filled.MoreVert
import androidx.compose.material.icons.filled.Share
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import coil3.compose.AsyncImage
import com.poziomki.app.api.Event
import com.poziomki.app.ui.component.UserAvatar
import com.poziomki.app.ui.screen.chat.ChatContent
import com.poziomki.app.ui.screen.chat.ChatViewModel
import com.poziomki.app.ui.theme.Background
import com.poziomki.app.ui.theme.PoziomkiTheme
import com.poziomki.app.ui.theme.Primary
import com.poziomki.app.ui.theme.TextPrimary
import com.poziomki.app.ui.theme.TextSecondary
import com.poziomki.app.util.formatEventDateFull
import com.poziomki.app.util.pluralizePolish
import com.poziomki.app.util.resolveImageUrl
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

    // Auto-load chat when conversation ID is available
    LaunchedEffect(eventState.event?.conversationId) {
        val convId = eventState.event?.conversationId
        if (convId != null && convId.startsWith("!")) {
            chatViewModel.loadRoom(convId)
        }
    }

    // Auto-open event chat if no conversation ID yet
    LaunchedEffect(eventState.event) {
        val event = eventState.event ?: return@LaunchedEffect
        if (event.conversationId == null && !eventState.isOpeningChat) {
            eventDetailViewModel.openEventChat { }
        }
    }

    Column(modifier = Modifier.fillMaxSize().background(Background)) {
        if (eventState.isLoading && eventState.event == null) {
            Box(
                modifier = Modifier.fillMaxSize(),
                contentAlignment = Alignment.Center,
            ) {
                CircularProgressIndicator(color = Primary)
            }
        } else {
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
                headerContent = {
                    eventState.event?.let { event ->
                        EventChatHeader(
                            event = event,
                            onBack = onBack,
                            onNavigateToProfile = onNavigateToProfile,
                        )
                    }
                },
            )
        }
    }
}

@Composable
private fun EventChatHeader(
    event: Event,
    onBack: () -> Unit,
    onNavigateToProfile: (String) -> Unit,
) {
    Column(
        modifier =
            Modifier
                .fillMaxWidth()
                .padding(bottom = PoziomkiTheme.spacing.md),
    ) {
        // Cover image with overlaid navigation and title
        Box(
            modifier =
                Modifier
                    .fillMaxWidth()
                    .aspectRatio(1.8f),
        ) {
            val coverImage = event.coverImage
            if (coverImage != null) {
                AsyncImage(
                    model = resolveImageUrl(coverImage),
                    contentDescription = event.title,
                    modifier = Modifier.fillMaxSize(),
                    contentScale = ContentScale.Crop,
                )
            }

            // Gradient overlay
            Box(
                modifier =
                    Modifier
                        .fillMaxSize()
                        .background(
                            Brush.verticalGradient(
                                colors =
                                    listOf(
                                        Color.Black.copy(alpha = 0.3f),
                                        Color.Transparent,
                                        Color.Black.copy(alpha = 0.65f),
                                    ),
                                startY = 0f,
                                endY = Float.POSITIVE_INFINITY,
                            ),
                        ),
            )

            // Navigation controls at top
            Row(
                modifier =
                    Modifier
                        .fillMaxWidth()
                        .align(Alignment.TopStart)
                        .padding(horizontal = 4.dp, vertical = 4.dp),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                IconButton(onClick = onBack) {
                    Icon(
                        imageVector = Icons.AutoMirrored.Filled.ArrowBack,
                        contentDescription = "Wstecz",
                        tint = Color.White,
                    )
                }

                Spacer(modifier = Modifier.weight(1f))

                IconButton(onClick = { }) {
                    Icon(
                        imageVector = Icons.Filled.BookmarkBorder,
                        contentDescription = "Zapisz",
                        tint = Color.White,
                    )
                }
                IconButton(onClick = { }) {
                    Icon(
                        imageVector = Icons.Filled.Share,
                        contentDescription = "Udostępnij",
                        tint = Color.White,
                    )
                }
                IconButton(onClick = { }) {
                    Icon(
                        imageVector = Icons.Filled.MoreVert,
                        contentDescription = "Więcej",
                        tint = Color.White,
                    )
                }
            }

            // Title at bottom of image on gradient
            Text(
                text = event.title,
                style = MaterialTheme.typography.headlineMedium,
                fontWeight = FontWeight.ExtraBold,
                color = Color.White,
                modifier =
                    Modifier
                        .align(Alignment.BottomStart)
                        .padding(horizontal = PoziomkiTheme.spacing.md, vertical = PoziomkiTheme.spacing.sm),
            )
        }

        Spacer(modifier = Modifier.height(PoziomkiTheme.spacing.sm))

        Column(modifier = Modifier.padding(horizontal = PoziomkiTheme.spacing.sm)) {
            // Creator
            event.creator?.let { creator ->
                Row(
                    verticalAlignment = Alignment.CenterVertically,
                    modifier = Modifier.clickable { onNavigateToProfile(creator.id) },
                ) {
                    UserAvatar(
                        picture = creator.profilePicture,
                        displayName = creator.name,
                        size = 40.dp,
                    )
                    Spacer(modifier = Modifier.width(8.dp))
                    Text(
                        text = creator.name,
                        style = MaterialTheme.typography.bodyLarge,
                        fontWeight = FontWeight.SemiBold,
                        color = Primary,
                    )
                }
                Spacer(modifier = Modifier.height(PoziomkiTheme.spacing.xs))
            }

            // Date
            Row(verticalAlignment = Alignment.CenterVertically) {
                Icon(
                    imageVector = Icons.Filled.CalendarMonth,
                    contentDescription = null,
                    modifier = Modifier.size(18.dp),
                    tint = TextSecondary,
                )
                Spacer(modifier = Modifier.width(6.dp))
                Text(
                    text = formatEventDateFull(event.startsAt),
                    style = MaterialTheme.typography.bodyMedium,
                    color = TextSecondary,
                )
            }

            // Location
            val location = event.location
            if (location != null) {
                Spacer(modifier = Modifier.height(2.dp))
                Row(verticalAlignment = Alignment.CenterVertically) {
                    Icon(
                        imageVector = Icons.Filled.LocationOn,
                        contentDescription = null,
                        modifier = Modifier.size(18.dp),
                        tint = TextSecondary,
                    )
                    Spacer(modifier = Modifier.width(6.dp))
                    Text(
                        text = location,
                        style = MaterialTheme.typography.bodyMedium,
                        color = TextSecondary,
                    )
                }
            }

            Spacer(modifier = Modifier.height(2.dp))

            // Participants
            Row(verticalAlignment = Alignment.CenterVertically) {
                Icon(
                    imageVector = Icons.Filled.Groups,
                    contentDescription = null,
                    modifier = Modifier.size(18.dp),
                    tint = TextSecondary,
                )
                Spacer(modifier = Modifier.width(6.dp))
                Text(
                    text =
                        pluralizePolish(
                            event.attendeesCount,
                            "uczestnik",
                            "uczestników",
                            "uczestników",
                        ),
                    style = MaterialTheme.typography.bodyMedium,
                    color = TextSecondary,
                )
            }
        }
    }
}
