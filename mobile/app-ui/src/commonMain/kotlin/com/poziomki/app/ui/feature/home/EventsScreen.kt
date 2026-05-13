@file:Suppress("TooManyFunctions")

package com.poziomki.app.ui.feature.home

import androidx.compose.animation.animateColorAsState
import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.aspectRatio
import androidx.compose.foundation.layout.fillMaxHeight
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
import androidx.compose.material3.BasicAlertDialog
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.pulltorefresh.PullToRefreshBox
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
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
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import coil3.compose.AsyncImage
import com.adamglin.PhosphorIcons
import com.adamglin.phosphoricons.Bold
import com.adamglin.phosphoricons.Fill
import com.adamglin.phosphoricons.bold.BookmarkSimple
import com.adamglin.phosphoricons.bold.CaretDown
import com.adamglin.phosphoricons.bold.CaretUp
import com.adamglin.phosphoricons.bold.Plus
import com.adamglin.phosphoricons.bold.SlidersHorizontal
import com.adamglin.phosphoricons.fill.BookmarkSimple
import com.adamglin.phosphoricons.fill.MapPin
import com.poziomki.app.network.Event
import com.poziomki.app.ui.designsystem.components.AppSnackbar
import com.poziomki.app.ui.designsystem.components.EmptyView
import com.poziomki.app.ui.designsystem.components.FilterTabs
import com.poziomki.app.ui.designsystem.components.LoadingView
import com.poziomki.app.ui.designsystem.components.SearchableScreenHeader
import com.poziomki.app.ui.designsystem.components.StackedAvatars
import com.poziomki.app.ui.designsystem.components.UserAvatar
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
import com.poziomki.app.ui.feature.onboarding.INTEREST_CATEGORIES
import com.poziomki.app.ui.navigation.LocalImmersive
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
    var searchActive by remember { mutableStateOf(false) }
    val requestLocationPermission =
        rememberLocationPermissionLauncher { granted ->
            if (granted) viewModel.retryNearby()
        }
    var hasAutoRequestedLocation by remember { mutableStateOf(false) }
    LaunchedEffect(state.activeFilter, state.isLocationPermissionDenied) {
        if (state.activeFilter == TimeFilter.NEARBY &&
            state.isLocationPermissionDenied &&
            !hasAutoRequestedLocation
        ) {
            hasAutoRequestedLocation = true
            requestLocationPermission()
        }
    }

    val timeFilterTabs =
        listOf(
            TimeFilter.ALL to "polecane",
            TimeFilter.NEARBY to "w pobliżu",
            TimeFilter.WEEK to "ten tydzień",
        )

    // The nearby tab gives the map the whole height (no navbar, no
    // searchbar) so the user can pan/zoom freely. Reset on leave.
    val immersive = LocalImmersive.current
    val isNearby = state.activeFilter == TimeFilter.NEARBY
    DisposableEffect(isNearby) {
        immersive.value = isNearby
        onDispose { immersive.value = false }
    }

    Column(
        modifier =
            Modifier
                .fillMaxSize()
                .background(Background),
    ) {
        SearchableScreenHeader(
            title = "wydarzenia",
            searchQuery = searchQuery,
            onSearchQueryChange = {
                searchQuery = it
                viewModel.setSearchQuery(it)
            },
            searchActive = searchActive,
            onSearchActiveChange = { searchActive = it },
        ) {
            androidx.compose.material3.IconButton(onClick = onNavigateToEventCreate) {
                Icon(
                    PhosphorIcons.Bold.Plus,
                    contentDescription = "Dodaj wydarzenie",
                    tint = TextPrimary,
                )
            }
            profileAvatarAction()
        }

        if (state.showTagFilter) {
            CategoryFilterDialog(
                selectedCategories = state.selectedCategories,
                onToggleCategory = { viewModel.toggleCategoryFilter(it) },
                onClear = { viewModel.clearCategoryFilters() },
                onDismiss = { viewModel.toggleShowTagFilter() },
            )
        }

        Row(
            modifier =
                Modifier
                    .fillMaxWidth()
                    .padding(start = 24.dp, end = PoziomkiTheme.spacing.md),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            FilterTabs(
                tabs = timeFilterTabs,
                selected = state.activeFilter,
                onSelect = { viewModel.setTimeFilter(it) },
                modifier =
                    Modifier
                        .weight(1f)
                        .padding(end = 12.dp),
            )
            androidx.compose.material3.IconButton(
                onClick = { viewModel.toggleShowTagFilter() },
            ) {
                Icon(
                    PhosphorIcons.Bold.SlidersHorizontal,
                    contentDescription = "Filtruj",
                    tint = if (state.selectedCategories.isNotEmpty()) Primary else TextPrimary,
                )
            }
        }

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

            // Sync error snackbar
            state.syncError?.let { error ->
                AppSnackbar(
                    message = error,
                    modifier =
                        Modifier
                            .align(Alignment.BottomCenter)
                            .padding(PoziomkiTheme.spacing.md),
                )
                LaunchedEffect(error) {
                    delay(3000)
                    viewModel.clearSyncError()
                }
            }
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
            // Cover image area (only when present) — empty placeholder
            // wastes space, so drop it entirely for cover-less events.
            val coverImage = event.coverImage
            if (coverImage != null) {
                Box {
                    AsyncImage(
                        model = resolveImageUrl(coverImage),
                        contentDescription = event.title,
                        modifier =
                            Modifier
                                .fillMaxWidth()
                                .aspectRatio(COVER_ASPECT_W / COVER_ASPECT_H),
                        contentScale = ContentScale.Crop,
                    )

                    BookmarkOverlay(
                        isSaved = event.isSaved,
                        onClick = onSaveClick,
                        modifier =
                            Modifier
                                .align(Alignment.TopEnd)
                                .padding(PoziomkiTheme.spacing.sm),
                    )
                }
            }

            // Metadata
            Box(modifier = Modifier.fillMaxWidth()) {
                Column(
                    modifier =
                        Modifier
                            .fillMaxWidth()
                            .padding(
                                start = PoziomkiTheme.spacing.md,
                                end = if (coverImage == null) 56.dp else PoziomkiTheme.spacing.md,
                                top = PoziomkiTheme.spacing.sm,
                                bottom = PoziomkiTheme.spacing.sm,
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

                    // Date/time + location
                    Row(verticalAlignment = Alignment.CenterVertically) {
                        Text(
                            text = formatEventDate(event.startsAt),
                            fontFamily = NunitoFamily,
                            fontWeight = FontWeight.Normal,
                            fontSize = 15.sp,
                            color = TextSecondary,
                        )
                        event.location?.let { location ->
                            Text(
                                text = " · ",
                                fontFamily = NunitoFamily,
                                fontSize = 15.sp,
                                color = TextMuted,
                            )
                            Icon(
                                PhosphorIcons.Fill.MapPin,
                                contentDescription = null,
                                modifier = Modifier.size(13.dp),
                                tint = TextMuted,
                            )
                            Spacer(modifier = Modifier.width(2.dp))
                            Text(
                                text = location,
                                fontFamily = NunitoFamily,
                                fontWeight = FontWeight.Normal,
                                fontSize = 14.sp,
                                color = TextMuted,
                                maxLines = 1,
                                modifier = Modifier.weight(1f, fill = false),
                            )
                        }
                    }

                    formatRecurrence(event.recurrenceRule)?.let { label ->
                        Spacer(modifier = Modifier.height(2.dp))
                        Text(
                            text = label,
                            fontFamily = NunitoFamily,
                            fontWeight = FontWeight.Medium,
                            fontSize = 13.sp,
                            color = TextMuted,
                        )
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

                if (coverImage == null) {
                    BookmarkOverlay(
                        isSaved = event.isSaved,
                        onClick = onSaveClick,
                        modifier =
                            Modifier
                                .align(Alignment.TopEnd)
                                .padding(PoziomkiTheme.spacing.sm),
                    )
                }
            }
        }
    }
}

@Composable
private fun BookmarkOverlay(
    isSaved: Boolean,
    onClick: () -> Unit,
    modifier: Modifier = Modifier,
) {
    Box(
        modifier =
            modifier
                .size(36.dp)
                .clip(CircleShape)
                .background(Overlay)
                .clickable(onClick = onClick),
        contentAlignment = Alignment.Center,
    ) {
        Icon(
            if (isSaved) PhosphorIcons.Fill.BookmarkSimple else PhosphorIcons.Bold.BookmarkSimple,
            contentDescription = if (isSaved) "Usuń z zapisanych" else "Zapisz",
            modifier = Modifier.size(22.dp),
            tint = if (isSaved) Primary else TextPrimary,
        )
    }
}

private fun formatRecurrence(rule: String?): String? {
    if (rule.isNullOrBlank()) return null
    val parts = mutableMapOf<String, String>()
    rule.split(';').forEach { segment ->
        val kv = segment.split('=', limit = 2)
        if (kv.size == 2) parts[kv[0]] = kv[1]
    }
    val freq = parts["FREQ"] ?: return null
    val interval = parts["INTERVAL"]?.toIntOrNull() ?: 1
    val base =
        when (freq) {
            "WEEKLY" -> if (interval == 1) "co tydzień" else "co $interval tyg."
            "MONTHLY" -> if (interval == 1) "co miesiąc" else "co $interval mies."
            "DAILY" -> if (interval == 1) "codziennie" else "co $interval dni"
            else -> return null
        }
    return "🔁 $base"
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
    val collapsedDays = remember { mutableStateMapOf<Long, Boolean>() }

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
    val coverImage = event.coverImage

    Box(
        modifier =
            Modifier
                .fillMaxWidth()
                .clip(cardShape)
                .border(1.dp, Border, cardShape)
                .background(SurfaceElevated)
                .clickable(onClick = onClick),
    ) {
        if (coverImage != null) {
            // Cover variant: fixed card height so the photo can flush
            // to the edges. 116dp fits title + date + location +
            // creator row comfortably without clipping.
            Row(
                modifier = Modifier.height(116.dp),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                AsyncImage(
                    model = resolveImageUrl(coverImage),
                    contentDescription = event.title,
                    modifier =
                        Modifier
                            .fillMaxHeight()
                            .aspectRatio(1f),
                    contentScale = ContentScale.Crop,
                )
                Spacer(modifier = Modifier.width(12.dp))
                EventRowContent(
                    event,
                    modifier =
                        Modifier
                            .weight(1f)
                            .padding(end = 16.dp, top = 10.dp, bottom = 10.dp),
                )
            }
        } else {
            // Compact variant: no left column, content drives height.
            EventRowContent(
                event,
                modifier =
                    Modifier
                        .fillMaxWidth()
                        .padding(horizontal = 16.dp, vertical = 12.dp),
            )
        }
    }
}

@Composable
private fun EventRowContent(
    event: Event,
    modifier: Modifier = Modifier,
) {
    Column(modifier = modifier) {
        Text(
            text = event.title,
            fontFamily = MontserratFamily,
            fontWeight = FontWeight.ExtraBold,
            fontSize = 18.sp,
            color = TextPrimary,
            maxLines = 1,
        )
        Spacer(modifier = Modifier.height(2.dp))
        Text(
            text = formatEventDate(event.startsAt),
            fontFamily = NunitoFamily,
            fontWeight = FontWeight.Normal,
            fontSize = 13.sp,
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
                    overflow = TextOverflow.Ellipsis,
                )
            }
        }
        event.creator?.let { creator ->
            Spacer(modifier = Modifier.height(6.dp))
            Row(verticalAlignment = Alignment.CenterVertically) {
                UserAvatar(
                    picture = creator.profilePicture,
                    displayName = creator.name,
                    size = 22.dp,
                )
                Spacer(modifier = Modifier.width(6.dp))
                Text(
                    text = "od ${creator.name}",
                    fontFamily = NunitoFamily,
                    fontWeight = FontWeight.Normal,
                    fontSize = 13.sp,
                    color = TextMuted,
                )
            }
        }
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun CategoryFilterDialog(
    selectedCategories: Set<String>,
    onToggleCategory: (String) -> Unit,
    onClear: () -> Unit,
    onDismiss: () -> Unit,
) {
    BasicAlertDialog(onDismissRequest = onDismiss) {
        Surface(
            shape = RoundedCornerShape(20.dp),
            color = SurfaceElevated,
        ) {
            Column(modifier = Modifier.padding(20.dp)) {
                Row(
                    modifier = Modifier.fillMaxWidth(),
                    horizontalArrangement = Arrangement.SpaceBetween,
                    verticalAlignment = Alignment.CenterVertically,
                ) {
                    Text(
                        text = "filtruj kategorie",
                        fontFamily = MontserratFamily,
                        fontWeight = FontWeight.ExtraBold,
                        fontSize = 18.sp,
                        color = TextPrimary,
                    )
                    if (selectedCategories.isNotEmpty()) {
                        Text(
                            text = "wyczyść",
                            fontFamily = NunitoFamily,
                            fontWeight = FontWeight.Medium,
                            fontSize = 13.sp,
                            color = Primary,
                            modifier = Modifier.clickable(onClick = onClear),
                        )
                    }
                }
                Spacer(modifier = Modifier.height(16.dp))
                INTEREST_CATEGORIES.forEach { category ->
                    CategoryFilterRow(
                        category = category,
                        selected = category.key in selectedCategories,
                        onClick = { onToggleCategory(category.key) },
                    )
                }
            }
        }
    }
}

@Composable
private fun CategoryFilterRow(
    category: com.poziomki.app.ui.feature.onboarding.InterestCategoryInfo,
    selected: Boolean,
    onClick: () -> Unit,
) {
    val bgColor by animateColorAsState(
        targetValue = if (selected) category.color.copy(alpha = 0.15f) else Color.Transparent,
    )
    Row(
        modifier =
            Modifier
                .fillMaxWidth()
                .clip(RoundedCornerShape(12.dp))
                .background(bgColor)
                .clickable(onClick = onClick)
                .padding(horizontal = 12.dp, vertical = 10.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Icon(
            imageVector = category.icon,
            contentDescription = null,
            modifier = Modifier.size(20.dp),
            tint = if (selected) category.color else TextMuted,
        )
        Spacer(modifier = Modifier.width(12.dp))
        Text(
            text = category.displayName,
            fontFamily = NunitoFamily,
            fontWeight = if (selected) FontWeight.Bold else FontWeight.Medium,
            fontSize = 15.sp,
            color = if (selected) category.color else TextPrimary,
            modifier = Modifier.weight(1f),
        )
        if (selected) {
            Box(
                modifier =
                    Modifier
                        .size(8.dp)
                        .background(category.color, CircleShape),
            )
        }
    }
}
