package com.poziomki.app.ui.feature.home

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Tab
import androidx.compose.material3.TabRow
import androidx.compose.material3.TabRowDefaults
import androidx.compose.material3.TabRowDefaults.tabIndicatorOffset
import androidx.compose.material3.Text
import androidx.compose.material3.pulltorefresh.PullToRefreshBox
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.poziomki.app.network.Event
import com.poziomki.app.network.Profile
import com.poziomki.app.ui.designsystem.components.EmptyView
import com.poziomki.app.ui.designsystem.components.LoadingView
import com.poziomki.app.ui.designsystem.components.ScreenHeader
import com.poziomki.app.ui.designsystem.components.UserAvatar
import com.poziomki.app.ui.designsystem.theme.Background
import com.poziomki.app.ui.designsystem.theme.NunitoFamily
import com.poziomki.app.ui.designsystem.theme.Primary
import com.poziomki.app.ui.designsystem.theme.TextMuted
import com.poziomki.app.ui.designsystem.theme.TextPrimary
import org.koin.compose.viewmodel.koinViewModel

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun SavedScreen(
    onBack: () -> Unit,
    onNavigateToEventDetail: (String) -> Unit,
    onNavigateToProfileView: (String) -> Unit,
    viewModel: SavedViewModel = koinViewModel(),
) {
    val state by viewModel.state.collectAsState()
    var selectedTab by rememberSaveable { mutableIntStateOf(0) }

    Column(
        modifier =
            Modifier
                .fillMaxSize()
                .background(MaterialTheme.colorScheme.background),
    ) {
        ScreenHeader(title = "zapisane", onBack = onBack)

        TabRow(
            selectedTabIndex = selectedTab,
            containerColor = Background,
            contentColor = TextPrimary,
            indicator = { tabPositions ->
                if (selectedTab < tabPositions.size) {
                    TabRowDefaults.SecondaryIndicator(
                        Modifier.tabIndicatorOffset(tabPositions[selectedTab]),
                        color = Primary,
                    )
                }
            },
        ) {
            Tab(
                selected = selectedTab == 0,
                onClick = { selectedTab = 0 },
                text = {
                    Text(
                        text = "wydarzenia",
                        fontFamily = NunitoFamily,
                        fontWeight = FontWeight.SemiBold,
                    )
                },
            )
            Tab(
                selected = selectedTab == 1,
                onClick = { selectedTab = 1 },
                text = {
                    Text(
                        text = "osoby",
                        fontFamily = NunitoFamily,
                        fontWeight = FontWeight.SemiBold,
                    )
                },
            )
        }

        when {
            state.isLoading -> {
                LoadingView()
            }

            else -> {
                PullToRefreshBox(
                    isRefreshing = state.isRefreshing,
                    onRefresh = { viewModel.pullToRefresh() },
                    modifier = Modifier.fillMaxSize(),
                ) {
                    when (selectedTab) {
                        0 -> SavedEventsList(state.events, onNavigateToEventDetail)
                        1 -> SavedPeopleList(state.profiles, onNavigateToProfileView)
                    }
                }
            }
        }
    }
}

@Composable
private fun SavedEventsList(
    events: List<Event>,
    onNavigateToEventDetail: (String) -> Unit,
) {
    if (events.isEmpty()) {
        EmptyView("brak zapisanych wydarzeń")
    } else {
        LazyColumn(modifier = Modifier.fillMaxSize()) {
            items(events, key = { it.id }) { event ->
                SavedEventRow(event, onClick = { onNavigateToEventDetail(event.id) })
            }
        }
    }
}

@Composable
private fun SavedEventRow(
    event: Event,
    onClick: () -> Unit,
) {
    Row(
        modifier =
            Modifier
                .fillMaxWidth()
                .clickable(onClick = onClick)
                .padding(horizontal = 16.dp, vertical = 12.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Column(modifier = Modifier.weight(1f)) {
            Text(
                text = event.title,
                fontFamily = NunitoFamily,
                fontWeight = FontWeight.SemiBold,
                fontSize = 16.sp,
                color = TextPrimary,
                maxLines = 1,
                overflow = TextOverflow.Ellipsis,
            )
            event.location?.let { loc ->
                Text(
                    text = loc,
                    fontFamily = NunitoFamily,
                    fontSize = 14.sp,
                    color = TextMuted,
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis,
                )
            }
        }
    }
}

@Composable
private fun SavedPeopleList(
    profiles: List<Profile>,
    onNavigateToProfileView: (String) -> Unit,
) {
    if (profiles.isEmpty()) {
        EmptyView("brak zapisanych osób")
    } else {
        LazyColumn(modifier = Modifier.fillMaxSize()) {
            items(profiles, key = { it.id }) { profile ->
                SavedProfileRow(profile, onClick = { onNavigateToProfileView(profile.id) })
            }
        }
    }
}

@Composable
private fun SavedProfileRow(
    profile: Profile,
    onClick: () -> Unit,
) {
    Row(
        modifier =
            Modifier
                .fillMaxWidth()
                .clickable(onClick = onClick)
                .padding(horizontal = 16.dp, vertical = 10.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        UserAvatar(
            picture = profile.profilePicture,
            displayName = profile.name,
            size = 44.dp,
        )
        Spacer(modifier = Modifier.width(12.dp))
        Column {
            Text(
                text = profile.name,
                fontFamily = NunitoFamily,
                fontWeight = FontWeight.SemiBold,
                fontSize = 16.sp,
                color = TextPrimary,
            )
            profile.program?.let { prog ->
                Spacer(modifier = Modifier.height(2.dp))
                Text(
                    text = prog,
                    fontFamily = NunitoFamily,
                    fontSize = 14.sp,
                    color = TextMuted,
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis,
                )
            }
        }
    }
}
