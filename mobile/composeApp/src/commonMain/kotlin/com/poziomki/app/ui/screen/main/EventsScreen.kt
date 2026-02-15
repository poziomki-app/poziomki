package com.poziomki.app.ui.screen.main

import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
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
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.BookmarkBorder
import androidx.compose.material.icons.filled.CalendarMonth
import androidx.compose.material.icons.filled.Edit
import androidx.compose.material.icons.filled.Search
import androidx.compose.material.icons.filled.Tune
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.VerticalDivider
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import coil3.compose.AsyncImage
import com.poziomki.app.api.Event
import com.poziomki.app.ui.component.StackedAvatars
import com.poziomki.app.ui.theme.Background
import com.poziomki.app.ui.theme.Border
import com.poziomki.app.ui.theme.NunitoFamily
import com.poziomki.app.ui.theme.PoziomkiTheme
import com.poziomki.app.ui.theme.Primary
import com.poziomki.app.ui.theme.SurfaceElevated
import com.poziomki.app.ui.theme.TextMuted
import com.poziomki.app.ui.theme.TextPrimary
import com.poziomki.app.ui.theme.TextSecondary
import com.poziomki.app.util.TimeFilter
import com.poziomki.app.util.formatEventDate
import com.poziomki.app.util.pluralizePolish
import com.poziomki.app.util.resolveImageUrl
import org.koin.compose.viewmodel.koinViewModel
import com.poziomki.app.ui.theme.Surface as SurfaceColor

@Composable
fun EventsScreen(
    onNavigateToEventDetail: (String) -> Unit,
    onNavigateToEventCreate: () -> Unit,
    viewModel: EventsViewModel = koinViewModel(),
) {
    val state by viewModel.state.collectAsState()

    Column(
        modifier =
            Modifier
                .fillMaxSize()
                .background(Background),
    ) {
        // Header row
        Row(
            modifier =
                Modifier
                    .fillMaxWidth()
                    .padding(
                        start = PoziomkiTheme.spacing.lg,
                        end = PoziomkiTheme.spacing.sm,
                        top = PoziomkiTheme.spacing.md,
                        bottom = PoziomkiTheme.spacing.sm,
                    ),
            horizontalArrangement = Arrangement.SpaceBetween,
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Text(
                text = "wydarzenia",
                style = MaterialTheme.typography.headlineMedium,
                color = TextPrimary,
            )
            IconButton(onClick = onNavigateToEventCreate) {
                Icon(
                    Icons.Filled.Edit,
                    contentDescription = "Utwórz wydarzenie",
                    modifier = Modifier.size(22.dp),
                    tint = TextSecondary,
                )
            }
        }

        // Search bar
        Surface(
            modifier =
                Modifier
                    .fillMaxWidth()
                    .padding(horizontal = PoziomkiTheme.spacing.md)
                    .height(48.dp),
            shape = RoundedCornerShape(28.dp),
            color = SurfaceColor,
            border = BorderStroke(1.dp, Border),
        ) {
            Row(
                modifier =
                    Modifier
                        .fillMaxSize()
                        .padding(horizontal = 16.dp),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Icon(
                    Icons.Filled.Search,
                    contentDescription = "Szukaj",
                    modifier = Modifier.size(22.dp),
                    tint = TextMuted,
                )
                Spacer(modifier = Modifier.width(12.dp))
                Text(
                    text = "szukaj wydarzeń...",
                    fontFamily = NunitoFamily,
                    color = TextMuted,
                    modifier = Modifier.weight(1f),
                )
                VerticalDivider(
                    modifier = Modifier.height(20.dp),
                    thickness = 1.dp,
                    color = Border,
                )
                Spacer(modifier = Modifier.width(12.dp))
                Icon(
                    Icons.Filled.Tune,
                    contentDescription = "Filtruj",
                    modifier = Modifier.size(22.dp),
                    tint = TextMuted,
                )
            }
        }

        Spacer(modifier = Modifier.height(PoziomkiTheme.spacing.md))

        // Time filter chips — centered
        TimeFilterRow(
            activeFilter = state.activeFilter,
            onFilterSelected = { viewModel.setTimeFilter(it) },
        )

        Spacer(modifier = Modifier.height(PoziomkiTheme.spacing.md))

        // Content
        when {
            state.isLoading -> {
                Box(Modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
                    CircularProgressIndicator(color = Primary)
                }
            }

            state.error != null -> {
                Box(Modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
                    Text(
                        state.error ?: "",
                        fontFamily = NunitoFamily,
                        color = TextSecondary,
                    )
                }
            }

            state.events.isEmpty() -> {
                Box(Modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
                    Text(
                        "brak wydarzeń",
                        fontFamily = NunitoFamily,
                        color = TextSecondary,
                    )
                }
            }

            else -> {
                LazyColumn(
                    modifier =
                        Modifier
                            .fillMaxSize()
                            .padding(horizontal = PoziomkiTheme.spacing.md),
                    verticalArrangement = Arrangement.spacedBy(PoziomkiTheme.spacing.md),
                ) {
                    items(state.events, key = { it.id }) { event ->
                        EventCard(
                            event = event,
                            onClick = { onNavigateToEventDetail(event.id) },
                        )
                    }
                    item { Spacer(modifier = Modifier.height(PoziomkiTheme.spacing.md)) }
                }
            }
        }
    }
}

@Composable
private fun TimeFilterRow(
    activeFilter: TimeFilter,
    onFilterSelected: (TimeFilter) -> Unit,
) {
    val filters =
        listOf(
            TimeFilter.ALL to "polecane",
            TimeFilter.TODAY to "dzisiaj",
            TimeFilter.TOMORROW to "jutro",
            TimeFilter.WEEK to "ten tydzień",
        )

    Row(
        modifier =
            Modifier
                .fillMaxWidth()
                .padding(horizontal = PoziomkiTheme.spacing.md),
        horizontalArrangement = Arrangement.Center,
        verticalAlignment = Alignment.CenterVertically,
    ) {
        filters.forEachIndexed { index, (filter, label) ->
            val isActive = filter == activeFilter
            Row(
                modifier =
                    Modifier
                        .clickable { onFilterSelected(filter) }
                        .padding(horizontal = 12.dp, vertical = 4.dp),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                if (isActive) {
                    Box(
                        modifier =
                            Modifier
                                .size(6.dp)
                                .background(Primary, CircleShape),
                    )
                    Spacer(modifier = Modifier.width(6.dp))
                }
                Text(
                    text = label,
                    fontFamily = NunitoFamily,
                    fontWeight = if (isActive) FontWeight.Bold else FontWeight.Normal,
                    fontSize = 14.sp,
                    color = if (isActive) TextPrimary else TextMuted,
                )
            }
            if (index < filters.lastIndex) {
                Spacer(modifier = Modifier.width(8.dp))
            }
        }
    }
}

@Composable
private fun EventCard(
    event: Event,
    onClick: () -> Unit,
) {
    val cardShape = RoundedCornerShape(PoziomkiTheme.componentSizes.cardRadius)

    Surface(
        modifier = Modifier.fillMaxWidth(),
        shape = cardShape,
        color = Color.Transparent,
        border = BorderStroke(1.dp, Border),
    ) {
        Column(
            modifier =
                Modifier
                    .background(
                        Brush.verticalGradient(
                            colors = listOf(SurfaceElevated, Background),
                        ),
                    ).clickable(onClick = onClick),
        ) {
            // Cover image with bookmark overlay
            val coverImage = event.coverImage
            Box {
                if (coverImage != null) {
                    AsyncImage(
                        model = resolveImageUrl(coverImage),
                        contentDescription = event.title,
                        modifier =
                            Modifier
                                .fillMaxWidth()
                                .aspectRatio(1.6f)
                                .clip(
                                    RoundedCornerShape(
                                        topStart = PoziomkiTheme.componentSizes.cardRadius,
                                        topEnd = PoziomkiTheme.componentSizes.cardRadius,
                                    ),
                                ),
                        contentScale = ContentScale.Crop,
                    )
                } else {
                    Box(
                        modifier =
                            Modifier
                                .fillMaxWidth()
                                .aspectRatio(1.6f)
                                .background(SurfaceElevated),
                        contentAlignment = Alignment.Center,
                    ) {
                        Icon(
                            Icons.Filled.CalendarMonth,
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
                            .size(32.dp)
                            .clip(CircleShape)
                            .background(Color.Black.copy(alpha = 0.4f)),
                    contentAlignment = Alignment.Center,
                ) {
                    Icon(
                        Icons.Filled.BookmarkBorder,
                        contentDescription = "Zapisz",
                        modifier = Modifier.size(18.dp),
                        tint = TextPrimary,
                    )
                }
            }

            // Content area
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
                    style = MaterialTheme.typography.titleSmall,
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
                    fontSize = 13.sp,
                    color = TextSecondary,
                )

                // Creator
                event.creator?.let { creator ->
                    Text(
                        text = "od ${creator.name}",
                        fontFamily = NunitoFamily,
                        fontWeight = FontWeight.Normal,
                        fontSize = 13.sp,
                        color = TextMuted,
                    )
                }

                Spacer(modifier = Modifier.height(6.dp))

                // Attendees row
                if (event.attendeesCount > 0) {
                    Row(verticalAlignment = Alignment.CenterVertically) {
                        if (event.attendeesPreview.isNotEmpty()) {
                            StackedAvatars(
                                imageUrls = event.attendeesPreview.map { it.profilePicture },
                            )
                            Spacer(modifier = Modifier.width(PoziomkiTheme.spacing.sm))
                        }
                        Text(
                            text =
                                pluralizePolish(
                                    event.attendeesCount,
                                    "osoba",
                                    "osoby",
                                    "osób",
                                ),
                            fontFamily = NunitoFamily,
                            fontWeight = FontWeight.Bold,
                            fontSize = 13.sp,
                            color = TextPrimary,
                        )
                    }
                }
            }
        }
    }
}
