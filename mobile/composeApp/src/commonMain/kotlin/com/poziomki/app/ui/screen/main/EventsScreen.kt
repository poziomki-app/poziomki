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
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.BookmarkBorder
import androidx.compose.material.icons.filled.CalendarMonth
import androidx.compose.material.icons.filled.Edit
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Snackbar
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.pulltorefresh.PullToRefreshBox
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
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
import com.poziomki.app.ui.component.EmptyView
import com.poziomki.app.ui.component.FilterTabs
import com.poziomki.app.ui.component.LoadingView
import com.poziomki.app.ui.component.PoziomkiSearchBar
import com.poziomki.app.ui.component.ScreenHeader
import com.poziomki.app.ui.component.StackedAvatars
import com.poziomki.app.ui.theme.Background
import com.poziomki.app.ui.theme.Border
import com.poziomki.app.ui.theme.NunitoFamily
import com.poziomki.app.ui.theme.Overlay
import com.poziomki.app.ui.theme.PoziomkiTheme
import com.poziomki.app.ui.theme.SurfaceElevated
import com.poziomki.app.ui.theme.TextMuted
import com.poziomki.app.ui.theme.TextPrimary
import com.poziomki.app.ui.theme.TextSecondary
import com.poziomki.app.util.TimeFilter
import com.poziomki.app.util.formatEventDate
import com.poziomki.app.util.pluralizePolish
import com.poziomki.app.util.resolveImageUrl
import kotlinx.coroutines.delay
import org.koin.compose.viewmodel.koinViewModel

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun EventsScreen(
    onNavigateToEventDetail: (String) -> Unit,
    onNavigateToEventCreate: () -> Unit,
    viewModel: EventsViewModel = koinViewModel(),
) {
    val state by viewModel.state.collectAsState()
    var searchQuery by remember { mutableStateOf("") }

    val timeFilterTabs =
        listOf(
            TimeFilter.ALL to "polecane",
            TimeFilter.NEARBY to "w pobliżu",
            TimeFilter.TODAY to "dzisiaj",
            TimeFilter.TOMORROW to "jutro",
        )

    Column(
        modifier =
            Modifier
                .fillMaxSize()
                .background(Background),
    ) {
        ScreenHeader(title = "wydarzenia") {
            IconButton(onClick = onNavigateToEventCreate) {
                Icon(
                    Icons.Filled.Edit,
                    contentDescription = "Utwórz wydarzenie",
                    modifier = Modifier.size(22.dp),
                    tint = TextSecondary,
                )
            }
        }

        PoziomkiSearchBar(
            query = searchQuery,
            onQueryChange = { searchQuery = it },
            placeholder = "szukaj wydarzeń...",
        )

        Spacer(modifier = Modifier.height(PoziomkiTheme.spacing.md))

        FilterTabs(
            tabs = timeFilterTabs,
            selected = state.activeFilter,
            onSelect = { viewModel.setTimeFilter(it) },
        )

        Spacer(modifier = Modifier.height(PoziomkiTheme.spacing.md))

        // Content
        Box(modifier = Modifier.fillMaxSize()) {
            when {
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
                        LazyColumn(
                            modifier =
                                Modifier
                                    .fillMaxSize()
                                    .padding(horizontal = PoziomkiTheme.spacing.md),
                            verticalArrangement = Arrangement.spacedBy(PoziomkiTheme.spacing.md),
                        ) {
                            if (state.events.isEmpty()) {
                                item {
                                    EmptyView("brak wydarzeń")
                                }
                            } else {
                                items(state.events, key = { it.id }) { event ->
                                    EventCard(
                                        event = event,
                                        onClick = { onNavigateToEventDetail(event.id) },
                                    )
                                }
                            }
                            item { Spacer(modifier = Modifier.height(PoziomkiTheme.spacing.md)) }
                        }
                    }
                }
            }

            // Refresh error snackbar
            state.refreshError?.let { error ->
                Snackbar(
                    modifier =
                        Modifier
                            .align(Alignment.BottomCenter)
                            .padding(PoziomkiTheme.spacing.md),
                ) {
                    Text(text = error)
                }
                LaunchedEffect(error) {
                    delay(3000)
                    viewModel.clearRefreshError()
                }
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
        color = Background,
        border = BorderStroke(1.dp, Border),
    ) {
        val coverImage = event.coverImage
        Box(
            modifier = Modifier.clickable(onClick = onClick),
        ) {
            // Cover image / placeholder — fills the card
            if (coverImage != null) {
                AsyncImage(
                    model = resolveImageUrl(coverImage),
                    contentDescription = event.title,
                    modifier =
                        Modifier
                            .fillMaxWidth()
                            .aspectRatio(1.4f),
                    contentScale = ContentScale.Crop,
                )
            } else {
                Box(
                    modifier =
                        Modifier
                            .fillMaxWidth()
                            .aspectRatio(1.4f)
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

            // Bottom gradient for text readability
            Box(
                modifier =
                    Modifier
                        .align(Alignment.BottomCenter)
                        .fillMaxWidth()
                        .fillMaxHeight(0.75f)
                        .background(
                            Brush.verticalGradient(
                                colors =
                                    listOf(
                                        Color.Transparent,
                                        Background.copy(alpha = 0.5f),
                                        Background.copy(alpha = 0.9f),
                                        Background,
                                    ),
                            ),
                        ),
            )

            // Content overlaid at bottom
            Column(
                modifier =
                    Modifier
                        .align(Alignment.BottomStart)
                        .padding(
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

                // Creator
                event.creator?.let { creator ->
                    Text(
                        text = "od ${creator.name}",
                        fontFamily = NunitoFamily,
                        fontWeight = FontWeight.Normal,
                        fontSize = 15.sp,
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
                                avatarSize = 36.dp,
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
                            fontSize = 15.sp,
                            color = TextPrimary,
                        )
                    }
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
                        .background(Overlay),
                contentAlignment = Alignment.Center,
            ) {
                Icon(
                    Icons.Filled.BookmarkBorder,
                    contentDescription = "Zapisz",
                    modifier = Modifier.size(22.dp),
                    tint = TextPrimary,
                )
            }
        }
    }
}
