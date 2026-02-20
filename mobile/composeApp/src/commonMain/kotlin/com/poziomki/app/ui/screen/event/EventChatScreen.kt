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
import androidx.compose.foundation.layout.statusBarsPadding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.rememberLazyListState
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.filled.CalendarMonth
import androidx.compose.material.icons.filled.Check
import androidx.compose.material.icons.filled.Groups
import androidx.compose.material.icons.filled.MoreVert
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
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
import com.poziomki.app.ui.theme.TextSecondary
import com.poziomki.app.util.formatEventDateFull
import com.poziomki.app.util.pluralizePolish
import com.poziomki.app.util.resolveImageUrl
import org.koin.compose.viewmodel.koinViewModel

@Composable
@Suppress("FunctionNaming", "LongMethod")
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
        if (event.isAttending && event.conversationId == null && !eventState.isOpeningChat) {
            eventDetailViewModel.openEventChat { }
        }
    }

    // Map Matrix sender IDs → Poziomki profile picture URLs from event attendees.
    // Key: normalized user UUID (lowercase, no hyphens), matching the Matrix localpart
    // format "poziomki_{uuid}".
    val avatarOverrides =
        remember(eventState.attendees) {
            eventState.attendees
                .filter { !it.userId.isNullOrBlank() && !it.profilePicture.isNullOrBlank() }
                .associate { attendee ->
                    val key =
                        attendee.userId!!
                            .filter { it.isLetterOrDigit() }
                            .lowercase()
                    key to attendee.profilePicture!!
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
                avatarOverrides = avatarOverrides,
                headerContent = {
                    eventState.event?.let { event ->
                        eventChatHeader(
                            event = event,
                            onBack = onBack,
                            onNavigateToProfile = onNavigateToProfile,
                            onJoin = eventDetailViewModel::attendEvent,
                            onLeave = eventDetailViewModel::leaveEvent,
                        )
                    }
                },
            )
        }
    }
}

@Composable
@Suppress("LongMethod", "LongParameterList")
private fun eventChatHeader(
    event: Event,
    onBack: () -> Unit,
    onNavigateToProfile: (String) -> Unit,
    onJoin: () -> Unit,
    onLeave: () -> Unit,
) {
    var showMenu by remember { mutableStateOf(false) }

    // Cover image with overlaid navigation, title, and metadata
    Box(
        modifier =
            Modifier
                .fillMaxWidth()
                .aspectRatio(1f),
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

        // Gradient overlay — fades to Background for seamless transition
        Box(
            modifier =
                Modifier
                    .fillMaxSize()
                    .background(
                        Brush.verticalGradient(
                            colorStops =
                                arrayOf(
                                    0f to Color.Black.copy(alpha = 0.3f),
                                    0.2f to Color.Transparent,
                                    0.45f to Background.copy(alpha = 0.3f),
                                    0.65f to Background.copy(alpha = 0.65f),
                                    0.8f to Background.copy(alpha = 0.85f),
                                    1f to Background,
                                ),
                        ),
                    ),
        )

        // Navigation controls at top
        Row(
            modifier =
                Modifier
                    .fillMaxWidth()
                    .align(Alignment.TopStart)
                    .statusBarsPadding()
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
            Box {
                IconButton(onClick = { showMenu = true }) {
                    Icon(
                        imageVector = Icons.Filled.MoreVert,
                        contentDescription = "Więcej",
                        tint = Color.White,
                    )
                }
                DropdownMenu(
                    expanded = showMenu,
                    onDismissRequest = { showMenu = false },
                ) {
                    if (event.isAttending) {
                        DropdownMenuItem(
                            text = { Text("Opuść wydarzenie") },
                            onClick = {
                                showMenu = false
                                onLeave()
                            },
                        )
                    } else {
                        DropdownMenuItem(
                            text = { Text("Dołącz do wydarzenia") },
                            onClick = {
                                showMenu = false
                                onJoin()
                            },
                        )
                    }
                }
            }
            if (event.isAttending) {
                Icon(
                    imageVector = Icons.Filled.Check,
                    contentDescription = "Dołączono",
                    tint = Primary,
                    modifier = Modifier.size(20.dp),
                )
            }
        }

        // Title and metadata overlaid on gradient
        Column(
            modifier =
                Modifier
                    .align(Alignment.BottomStart)
                    .padding(horizontal = PoziomkiTheme.spacing.md, vertical = PoziomkiTheme.spacing.sm),
        ) {
            // Title
            Text(
                text = event.title,
                style = MaterialTheme.typography.headlineMedium,
                fontWeight = FontWeight.ExtraBold,
                color = Color.White,
            )

            Spacer(modifier = Modifier.height(PoziomkiTheme.spacing.xs))

            // Creator
            event.creator?.let { creator ->
                Row(
                    verticalAlignment = Alignment.CenterVertically,
                    modifier = Modifier.clickable { onNavigateToProfile(creator.id) },
                ) {
                    UserAvatar(
                        picture = creator.profilePicture,
                        displayName = creator.name,
                        size = 36.dp,
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
