package com.poziomki.app.ui.feature.home

import androidx.compose.animation.animateColorAsState
import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.aspectRatio
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.SwipeToDismissBox
import androidx.compose.material3.SwipeToDismissBoxState
import androidx.compose.material3.SwipeToDismissBoxValue
import androidx.compose.material3.Text
import androidx.compose.material3.pulltorefresh.PullToRefreshBox
import androidx.compose.material3.rememberSwipeToDismissBoxState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateMapOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import coil3.compose.AsyncImage
import com.adamglin.PhosphorIcons
import com.adamglin.phosphoricons.Bold
import com.adamglin.phosphoricons.Fill
import com.adamglin.phosphoricons.bold.ArrowSquareOut
import com.adamglin.phosphoricons.bold.BookmarkSimple
import com.adamglin.phosphoricons.bold.CaretDown
import com.adamglin.phosphoricons.bold.CaretUp
import com.adamglin.phosphoricons.bold.PencilSimple
import com.adamglin.phosphoricons.bold.Plus
import com.adamglin.phosphoricons.bold.ThumbsDown
import com.adamglin.phosphoricons.bold.ThumbsUp
import com.adamglin.phosphoricons.fill.BookmarkSimple
import com.adamglin.phosphoricons.fill.CalendarDots
import com.adamglin.phosphoricons.fill.MapPin
import com.poziomki.app.network.Event
import com.poziomki.app.ui.designsystem.components.AppButton
import com.poziomki.app.ui.designsystem.components.AppSnackbar
import com.poziomki.app.ui.designsystem.components.ButtonVariant
import com.poziomki.app.ui.designsystem.components.EmptyView
import com.poziomki.app.ui.designsystem.components.FilterTabs
import com.poziomki.app.ui.designsystem.components.LoadingView
import com.poziomki.app.ui.designsystem.components.PoziomkiSearchBar
import com.poziomki.app.ui.designsystem.components.ScreenHeader
import com.poziomki.app.ui.designsystem.components.StackedAvatars
import com.poziomki.app.ui.designsystem.theme.Background
import com.poziomki.app.ui.designsystem.theme.Border
import com.poziomki.app.ui.designsystem.theme.MontserratFamily
import com.poziomki.app.ui.designsystem.theme.NunitoFamily
import com.poziomki.app.ui.designsystem.theme.Overlay
import com.poziomki.app.ui.designsystem.theme.PoziomkiTheme
import com.poziomki.app.ui.designsystem.theme.Primary
import com.poziomki.app.ui.designsystem.theme.SurfaceElevated
import com.poziomki.app.ui.designsystem.theme.TextMuted
import com.poziomki.app.ui.designsystem.theme.TextPrimary
import com.poziomki.app.ui.designsystem.theme.TextSecondary
import com.poziomki.app.ui.navigation.LocalNavBarPadding
import com.poziomki.app.ui.shared.TimeFilter
import com.poziomki.app.ui.shared.dayLabel
import com.poziomki.app.ui.shared.eventDateKey
import com.poziomki.app.ui.shared.formatEventDate
import com.poziomki.app.ui.shared.pluralizePolish
import com.poziomki.app.ui.shared.rememberLocationPermissionLauncher
import com.poziomki.app.ui.shared.resolveImageUrl
import kotlinx.coroutines.delay
import org.koin.compose.viewmodel.koinViewModel

@OptIn(ExperimentalMaterial3Api::class)
@Composable
@Suppress("LongMethod", "CyclomaticComplexMethod")
fun EventsScreen(
    onNavigateToEventDetail: (String) -> Unit,
    onNavigateToEventCreate: () -> Unit,
    onNavigateToProfile: (String) -> Unit = {},
    profileAvatarAction: @Composable () -> Unit = {},
    viewModel: EventsViewModel = koinViewModel(),
) {
    val state by viewModel.state.collectAsState()
    var searchQuery by remember { mutableStateOf("") }
    val requestLocationPermission =
        rememberLocationPermissionLauncher { granted ->
            if (granted) viewModel.retryNearby()
        }

    val timeFilterTabs =
        listOf(
            TimeFilter.ALL to "polecane",
            TimeFilter.NEARBY to "w pobliżu",
            TimeFilter.WEEK to "ten tydzień",
        )

    Column(
        modifier =
            Modifier
                .fillMaxSize()
                .background(Background),
    ) {
        ScreenHeader(title = "wydarzenia") {
            profileAvatarAction()
        }

        PoziomkiSearchBar(
            query = searchQuery,
            onQueryChange = { searchQuery = it },
            placeholder = "szukaj wydarzeń...",
        )

        FilterTabs(
            tabs = timeFilterTabs,
            selected = state.activeFilter,
            onSelect = { viewModel.setTimeFilter(it) },
        )

        // Content
        Box(modifier = Modifier.fillMaxSize()) {
            when {
                state.activeFilter == TimeFilter.NEARBY -> {
                    val nearbyDisplayEvents =
                        remember(state.nearbyEvents, state.allEvents) {
                            val nearbyGeo = state.nearbyEvents.filter { it.latitude != null && it.longitude != null }
                            if (nearbyGeo.isNotEmpty()) {
                                state.nearbyEvents
                            } else {
                                state.allEvents.filter { it.latitude != null && it.longitude != null }
                            }
                        }
                    PullToRefreshBox(
                        isRefreshing = state.isRefreshing,
                        onRefresh = { viewModel.pullToRefresh() },
                    ) {
                        NearbyEventsContent(
                            events = nearbyDisplayEvents,
                            selectedEventId = state.selectedNearbyEventId,
                            userLat = state.userLat,
                            userLng = state.userLng,
                            isPermissionDenied = state.isLocationPermissionDenied,
                            isLocationUnavailable = state.isLocationUnavailable,
                            onEventSelected = { viewModel.selectNearbyEvent(it) },
                            onEventClick = onNavigateToEventDetail,
                            onRequestPermission = requestLocationPermission,
                        )
                    }
                }

                state.isLoading && state.allEvents.isEmpty() -> {
                    LoadingView()
                }

                state.allEvents.isEmpty() -> {
                    EmptyView(state.error ?: "brak wydarzeń")
                }

                else -> {
                    PullToRefreshBox(
                        isRefreshing = state.isRefreshing,
                        onRefresh = { viewModel.pullToRefresh() },
                    ) {
                        if (state.activeFilter == TimeFilter.WEEK) {
                            WeekEventsContent(
                                events = state.events,
                                onEventClick = onNavigateToEventDetail,
                            )
                        } else {
                            val isRecommended = state.activeFilter == TimeFilter.ALL
                            LazyColumn(
                                modifier =
                                    Modifier
                                        .fillMaxSize()
                                        .padding(horizontal = PoziomkiTheme.spacing.md),
                                contentPadding = PaddingValues(bottom = LocalNavBarPadding.current),
                                verticalArrangement = Arrangement.spacedBy(PoziomkiTheme.spacing.md),
                            ) {
                                if (state.events.isEmpty()) {
                                    item {
                                        EmptyView("brak wydarzeń")
                                    }
                                } else {
                                    items(state.events, key = { it.id }) { event ->
                                        val creatorClick =
                                            event.creator?.let { c ->
                                                {
                                                    onNavigateToProfile(c.id)
                                                }
                                            }
                                        if (isRecommended) {
                                            SwipeableEventCard(
                                                event = event,
                                                onClick = { onNavigateToEventDetail(event.id) },
                                                onSaveClick = { viewModel.toggleSave(event.id) },
                                                onCreatorClick = creatorClick,
                                                onSwipeFeedback = { feedback ->
                                                    viewModel.onSwipeFeedback(event.id, feedback)
                                                },
                                            )
                                        } else {
                                            EventCard(
                                                event = event,
                                                onClick = { onNavigateToEventDetail(event.id) },
                                                onSaveClick = { viewModel.toggleSave(event.id) },
                                                onCreatorClick = creatorClick,
                                            )
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // FAB: create event
            AppButton(
                text = "nowe",
                onClick = onNavigateToEventCreate,
                variant = ButtonVariant.PRIMARY,
                icon = PhosphorIcons.Bold.Plus,
                modifier =
                    Modifier
                        .align(Alignment.BottomEnd)
                        .padding(
                            end = PoziomkiTheme.spacing.lg,
                            bottom = LocalNavBarPadding.current + 24.dp,
                        ),
            )

            // Refresh error snackbar
            state.refreshError?.let { error ->
                AppSnackbar(
                    message = error,
                    modifier =
                        Modifier
                            .align(Alignment.BottomCenter)
                            .padding(PoziomkiTheme.spacing.md),
                )
                LaunchedEffect(error) {
                    delay(3000)
                    viewModel.clearRefreshError()
                }
            }
        }
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun SwipeableEventCard(
    event: Event,
    onClick: () -> Unit,
    onSaveClick: () -> Unit = {},
    onCreatorClick: (() -> Unit)?,
    onSwipeFeedback: (String) -> Unit,
) {
    val dismissState = rememberSwipeToDismissBoxState()

    LaunchedEffect(dismissState.currentValue) {
        when (dismissState.currentValue) {
            SwipeToDismissBoxValue.StartToEnd -> onSwipeFeedback("more")
            SwipeToDismissBoxValue.EndToStart -> onSwipeFeedback("less")
            SwipeToDismissBoxValue.Settled -> Unit
        }
    }

    SwipeToDismissBox(
        state = dismissState,
        backgroundContent = {
            SwipeFeedbackBackground(dismissState)
        },
    ) {
        EventCard(
            event = event,
            onClick = onClick,
            onSaveClick = onSaveClick,
            onCreatorClick = onCreatorClick,
        )
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun SwipeFeedbackBackground(state: SwipeToDismissBoxState) {
    val direction = state.dismissDirection
    val color by animateColorAsState(
        when (state.targetValue) {
            SwipeToDismissBoxValue.StartToEnd -> Color(0xFF4CAF50).copy(alpha = 0.3f)
            SwipeToDismissBoxValue.EndToStart -> Color(0xFFE57373).copy(alpha = 0.3f)
            SwipeToDismissBoxValue.Settled -> Color.Transparent
        },
        label = "swipeBg",
    )
    val cardShape = RoundedCornerShape(PoziomkiTheme.componentSizes.cardRadius)
    Box(
        modifier =
            Modifier
                .fillMaxSize()
                .clip(cardShape)
                .background(color),
    ) {
        if (direction == SwipeToDismissBoxValue.StartToEnd) {
            Icon(
                PhosphorIcons.Bold.ThumbsUp,
                contentDescription = "Więcej takich",
                modifier =
                    Modifier
                        .align(Alignment.CenterStart)
                        .padding(start = 24.dp)
                        .size(28.dp),
                tint = Color(0xFF2E7D32),
            )
        } else if (direction == SwipeToDismissBoxValue.EndToStart) {
            Icon(
                PhosphorIcons.Bold.ThumbsDown,
                contentDescription = "Mniej takich",
                modifier =
                    Modifier
                        .align(Alignment.CenterEnd)
                        .padding(end = 24.dp)
                        .size(28.dp),
                tint = Color(0xFFC62828),
            )
        }
    }
}

private const val COVER_ASPECT_W = 16f
private const val COVER_ASPECT_H = 9f

@Composable
@Suppress("LongMethod")
private fun EventCard(
    event: Event,
    onClick: () -> Unit,
    onSaveClick: () -> Unit = {},
    onCreatorClick: (() -> Unit)? = null,
) {
    val cardShape = RoundedCornerShape(PoziomkiTheme.componentSizes.cardRadius)

    Surface(
        modifier =
            Modifier
                .fillMaxWidth()
                .clickable(onClick = onClick),
        shape = cardShape,
        color = SurfaceElevated,
        border = BorderStroke(1.dp, Border),
    ) {
        Column {
            // Cover image area with bookmark overlay
            Box {
                val coverImage = event.coverImage
                if (coverImage != null) {
                    AsyncImage(
                        model = resolveImageUrl(coverImage),
                        contentDescription = event.title,
                        modifier =
                            Modifier
                                .fillMaxWidth()
                                .aspectRatio(COVER_ASPECT_W / COVER_ASPECT_H),
                        contentScale = ContentScale.Crop,
                    )
                } else {
                    Box(
                        modifier =
                            Modifier
                                .fillMaxWidth()
                                .aspectRatio(COVER_ASPECT_W / COVER_ASPECT_H)
                                .background(Background),
                        contentAlignment = Alignment.Center,
                    ) {
                        Icon(
                            PhosphorIcons.Fill.CalendarDots,
                            contentDescription = null,
                            modifier = Modifier.size(48.dp),
                            tint = TextMuted,
                        )
                    }
                }

                // Bookmark overlay
                Box(
                    modifier =
                        Modifier
                            .align(Alignment.TopEnd)
                            .padding(PoziomkiTheme.spacing.sm)
                            .size(36.dp)
                            .clip(CircleShape)
                            .background(Overlay)
                            .clickable(onClick = onSaveClick),
                    contentAlignment = Alignment.Center,
                ) {
                    Icon(
                        if (event.isSaved) {
                            PhosphorIcons.Fill.BookmarkSimple
                        } else {
                            PhosphorIcons.Bold.BookmarkSimple
                        },
                        contentDescription = if (event.isSaved) "Usuń z zapisanych" else "Zapisz",
                        modifier = Modifier.size(22.dp),
                        tint = if (event.isSaved) Primary else TextPrimary,
                    )
                }
            }

            // Metadata below the image
            Column(
                modifier =
                    Modifier.padding(
                        horizontal = PoziomkiTheme.spacing.md,
                        vertical = PoziomkiTheme.spacing.sm,
                    ),
            ) {
                // Title
                Text(
                    text = event.title,
                    style = MaterialTheme.typography.titleMedium,
                    color = TextPrimary,
                    fontWeight = FontWeight.Bold,
                    maxLines = 2,
                )

                Spacer(modifier = Modifier.height(2.dp))

                // Date/time
                Text(
                    text = formatEventDate(event.startsAt),
                    fontFamily = NunitoFamily,
                    fontWeight = FontWeight.Normal,
                    fontSize = 15.sp,
                    color = TextSecondary,
                )

                // Location
                event.location?.let { location ->
                    Row(verticalAlignment = Alignment.CenterVertically) {
                        Icon(
                            PhosphorIcons.Fill.MapPin,
                            contentDescription = null,
                            modifier = Modifier.size(14.dp),
                            tint = TextMuted,
                        )
                        Spacer(modifier = Modifier.width(4.dp))
                        Text(
                            text = location,
                            fontFamily = NunitoFamily,
                            fontWeight = FontWeight.Normal,
                            fontSize = 14.sp,
                            color = TextMuted,
                            maxLines = 1,
                        )
                    }
                }

                // Tags
                if (event.tags.isNotEmpty()) {
                    Spacer(modifier = Modifier.height(4.dp))
                    @OptIn(ExperimentalLayoutApi::class)
                    FlowRow(
                        horizontalArrangement = Arrangement.spacedBy(4.dp),
                        verticalArrangement = Arrangement.spacedBy(4.dp),
                    ) {
                        val visibleTags = event.tags.take(3)
                        val overflow = event.tags.size - visibleTags.size
                        visibleTags.forEach { tag ->
                            Text(
                                text = "${tag.emoji.orEmpty()} ${tag.name}".trim(),
                                fontFamily = NunitoFamily,
                                fontWeight = FontWeight.Medium,
                                fontSize = 12.sp,
                                color = TextMuted,
                                modifier =
                                    Modifier
                                        .border(1.dp, Border, RoundedCornerShape(50))
                                        .padding(horizontal = 8.dp, vertical = 3.dp),
                            )
                        }
                        if (overflow > 0) {
                            Text(
                                text = "+$overflow",
                                fontFamily = NunitoFamily,
                                fontWeight = FontWeight.Medium,
                                fontSize = 12.sp,
                                color = TextMuted,
                                modifier =
                                    Modifier
                                        .border(1.dp, Border, RoundedCornerShape(50))
                                        .padding(horizontal = 8.dp, vertical = 3.dp),
                            )
                        }
                    }
                }

                Spacer(modifier = Modifier.height(6.dp))

                // Attendees row
                if (event.attendeesCount > 0 || event.maxAttendees != null) {
                    Row(verticalAlignment = Alignment.CenterVertically) {
                        if (event.attendeesPreview.isNotEmpty()) {
                            StackedAvatars(
                                imageUrls = event.attendeesPreview.map { it.profilePicture },
                                avatarSize = 36.dp,
                                modifier = onCreatorClick?.let { Modifier.clickable(onClick = it) } ?: Modifier,
                            )
                            Spacer(modifier = Modifier.width(PoziomkiTheme.spacing.sm))
                        }
                        Text(
                            text = event.attendeeUsageLabel(),
                            fontFamily = NunitoFamily,
                            fontWeight = FontWeight.Bold,
                            fontSize = 15.sp,
                            color = TextPrimary,
                        )
                    }
                }
            }
        }
    }
}

private fun Event.attendeeUsageLabel(): String =
    maxAttendees?.let { "$attendeesCount / $it" }
        ?: pluralizePolish(
            attendeesCount,
            "osoba",
            "osoby",
            "osób",
        )

@Composable
private fun WeekEventsContent(
    events: List<Event>,
    onEventClick: (String) -> Unit,
) {
    if (events.isEmpty()) {
        EmptyView("brak wydarzeń w tym tygodniu")
        return
    }

    val grouped =
        remember(events) {
            events
                .groupBy { eventDateKey(it.startsAt) }
                .toSortedMap()
        }
    val collapsedDays = remember { mutableStateMapOf<Int, Boolean>() }

    LazyColumn(
        modifier =
            Modifier
                .fillMaxSize()
                .padding(horizontal = PoziomkiTheme.spacing.md),
        contentPadding = PaddingValues(bottom = LocalNavBarPadding.current),
        verticalArrangement = Arrangement.spacedBy(PoziomkiTheme.spacing.sm),
    ) {
        grouped.forEach { (dateKey, dayEvents) ->
            val label = dayLabel(dayEvents.first().startsAt)
            val isCollapsed = collapsedDays[dateKey] == true

            item(key = "header_$dateKey") {
                Row(
                    modifier =
                        Modifier
                            .fillMaxWidth()
                            .clickable { collapsedDays[dateKey] = !isCollapsed }
                            .padding(
                                start = PoziomkiTheme.spacing.sm,
                                top = PoziomkiTheme.spacing.sm,
                                bottom = PoziomkiTheme.spacing.sm,
                            ),
                    verticalAlignment = Alignment.CenterVertically,
                ) {
                    Text(
                        text = label,
                        fontFamily = MontserratFamily,
                        fontWeight = FontWeight.ExtraBold,
                        fontSize = 18.sp,
                        color = TextPrimary,
                        modifier = Modifier.weight(1f),
                    )
                    Icon(
                        if (isCollapsed) PhosphorIcons.Bold.CaretDown else PhosphorIcons.Bold.CaretUp,
                        contentDescription = if (isCollapsed) "Rozwiń" else "Zwiń",
                        modifier = Modifier.size(24.dp),
                        tint = TextMuted,
                    )
                }
            }

            if (!isCollapsed) {
                items(dayEvents, key = { it.id }) { event ->
                    EventRow(
                        event = event,
                        onClick = { onEventClick(event.id) },
                    )
                }
            }
        }
    }
}

@Composable
internal fun EventRow(
    event: Event,
    onClick: () -> Unit,
) {
    val cardShape = RoundedCornerShape(20.dp)
    val photoSize = 90.dp

    Box(
        modifier =
            Modifier
                .fillMaxWidth()
                .clip(cardShape)
                .border(1.dp, Border, cardShape)
                .background(SurfaceElevated)
                .clickable(onClick = onClick),
    ) {
        Row(verticalAlignment = Alignment.CenterVertically) {
            val coverImage = event.coverImage
            if (coverImage != null) {
                AsyncImage(
                    model = resolveImageUrl(coverImage),
                    contentDescription = event.title,
                    modifier = Modifier.size(photoSize),
                    contentScale = ContentScale.Crop,
                )
            } else {
                Box(
                    modifier =
                        Modifier
                            .size(photoSize)
                            .background(Background),
                    contentAlignment = Alignment.Center,
                ) {
                    Icon(
                        PhosphorIcons.Fill.CalendarDots,
                        contentDescription = null,
                        modifier = Modifier.size(32.dp),
                        tint = TextMuted,
                    )
                }
            }

            Spacer(modifier = Modifier.width(12.dp))

            Column(
                modifier =
                    Modifier
                        .weight(1f)
                        .padding(vertical = 12.dp),
            ) {
                Text(
                    text = event.title,
                    fontFamily = MontserratFamily,
                    fontWeight = FontWeight.ExtraBold,
                    fontSize = 20.sp,
                    color = TextPrimary,
                    maxLines = 1,
                )
                Spacer(modifier = Modifier.height(2.dp))
                Text(
                    text = formatEventDate(event.startsAt),
                    fontFamily = NunitoFamily,
                    fontWeight = FontWeight.Normal,
                    fontSize = 14.sp,
                    color = TextSecondary,
                )
                event.location?.let { location ->
                    Row(verticalAlignment = Alignment.CenterVertically) {
                        Icon(
                            PhosphorIcons.Fill.MapPin,
                            contentDescription = null,
                            modifier = Modifier.size(12.dp),
                            tint = TextMuted,
                        )
                        Spacer(modifier = Modifier.width(3.dp))
                        Text(
                            text = location,
                            fontFamily = NunitoFamily,
                            fontWeight = FontWeight.Normal,
                            fontSize = 13.sp,
                            color = TextMuted,
                            maxLines = 1,
                        )
                    }
                }
                event.creator?.let { creator ->
                    Text(
                        text = "od ${creator.name}",
                        fontFamily = NunitoFamily,
                        fontWeight = FontWeight.Normal,
                        fontSize = 14.sp,
                        color = TextMuted,
                    )
                }
            }

            Icon(
                PhosphorIcons.Bold.ArrowSquareOut,
                contentDescription = "Otwórz",
                modifier =
                    Modifier
                        .padding(top = 12.dp, end = 12.dp)
                        .size(20.dp)
                        .align(Alignment.Top),
                tint = TextMuted,
            )
        }
    }
}
