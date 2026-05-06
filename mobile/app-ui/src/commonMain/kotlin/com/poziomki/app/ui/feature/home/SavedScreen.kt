package com.poziomki.app.ui.feature.home

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.asPaddingValues
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.navigationBars
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.statusBars
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.pulltorefresh.PullToRefreshBox
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import com.poziomki.app.ui.designsystem.components.EmptyView
import com.poziomki.app.ui.designsystem.components.FilterTabs
import com.poziomki.app.ui.designsystem.components.LoadingView
import com.poziomki.app.ui.designsystem.components.ProfileCard
import com.poziomki.app.ui.designsystem.components.ScreenHeader
import com.poziomki.app.ui.designsystem.theme.PoziomkiTheme
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
    val tabs = listOf(0 to "wydarzenia", 1 to "osoby")

    Column(
        modifier =
            Modifier
                .fillMaxSize()
                .background(MaterialTheme.colorScheme.background),
    ) {
        val statusBarPadding = WindowInsets.statusBars.asPaddingValues().calculateTopPadding()
        val navBarBottom = WindowInsets.navigationBars.asPaddingValues().calculateBottomPadding()
        ScreenHeader(
            title = "zapisane",
            onBack = onBack,
            modifier = Modifier.padding(top = statusBarPadding),
        )

        FilterTabs(tabs = tabs, selected = selectedTab, onSelect = { selectedTab = it })

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
                        0 -> {
                            if (state.events.isEmpty()) {
                                EmptyView("brak zapisanych wydarzeń")
                            } else {
                                LazyColumn(
                                    modifier = Modifier.fillMaxSize().padding(horizontal = PoziomkiTheme.spacing.md),
                                    contentPadding = PaddingValues(bottom = navBarBottom + PoziomkiTheme.spacing.md),
                                    verticalArrangement = Arrangement.spacedBy(PoziomkiTheme.spacing.sm),
                                ) {
                                    items(state.events, key = { it.id }) { event ->
                                        EventRow(event = event, onClick = { onNavigateToEventDetail(event.id) })
                                    }
                                }
                            }
                        }

                        1 -> {
                            if (state.profiles.isEmpty()) {
                                EmptyView("brak zapisanych osób")
                            } else {
                                LazyColumn(
                                    modifier = Modifier.fillMaxSize().padding(horizontal = PoziomkiTheme.spacing.md),
                                    contentPadding = PaddingValues(bottom = navBarBottom + PoziomkiTheme.spacing.md),
                                    verticalArrangement = Arrangement.spacedBy(PoziomkiTheme.spacing.sm),
                                ) {
                                    items(state.profiles, key = { it.id }) { profile ->
                                        ProfileCard(
                                            name = profile.name,
                                            profilePicture = profile.profilePicture,
                                            gradientStart = profile.gradientStart,
                                            gradientEnd = profile.gradientEnd,
                                            onClick = { onNavigateToProfileView(profile.id) },
                                        )
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
