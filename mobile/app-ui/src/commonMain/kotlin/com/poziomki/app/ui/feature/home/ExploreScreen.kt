package com.poziomki.app.ui.feature.home

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.pulltorefresh.PullToRefreshBox
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.poziomki.app.ui.designsystem.components.AppSnackbar
import com.poziomki.app.ui.designsystem.components.EmptyView
import com.poziomki.app.ui.designsystem.components.LoadingView
import com.poziomki.app.ui.designsystem.components.PoziomkiSearchBar
import com.poziomki.app.ui.designsystem.components.ProfileCard
import com.poziomki.app.ui.designsystem.components.ScreenHeader
import com.poziomki.app.ui.designsystem.theme.NunitoFamily
import com.poziomki.app.ui.designsystem.theme.PoziomkiTheme
import com.poziomki.app.ui.designsystem.theme.SurfaceElevated
import com.poziomki.app.ui.designsystem.theme.TextMuted
import com.poziomki.app.ui.designsystem.theme.TextPrimary
import com.poziomki.app.ui.designsystem.theme.TextSecondary
import com.poziomki.app.ui.navigation.LocalNavBarPadding
import kotlinx.coroutines.delay
import org.koin.compose.viewmodel.koinViewModel

@OptIn(ExperimentalLayoutApi::class, ExperimentalMaterial3Api::class)
@Composable
fun ExploreScreen(
    onNavigateToProfile: (String) -> Unit,
    onNavigateToEventDetail: (String) -> Unit = {},
    profileAvatarAction: @Composable () -> Unit = {},
    viewModel: ExploreViewModel = koinViewModel(),
) {
    val state by viewModel.state.collectAsState()
    val isSearchActive = state.query.length >= 2

    Column(
        modifier =
            Modifier
                .fillMaxSize()
                .background(MaterialTheme.colorScheme.background),
    ) {
        ScreenHeader(title = "poznaj") {
            profileAvatarAction()
        }

        PoziomkiSearchBar(
            query = state.query,
            onQueryChange = viewModel::updateQuery,
            placeholder = "szukaj os\u00f3b, wydarze\u0144...",
        )

        Spacer(modifier = Modifier.height(PoziomkiTheme.spacing.md))

        Box(modifier = Modifier.fillMaxSize()) {
            if (isSearchActive) {
                // Search results
                when {
                    state.isSearching -> {
                        LoadingView()
                    }

                    state.searchResults != null -> {
                        val results = state.searchResults!!
                        val hasResults =
                            results.profiles.isNotEmpty() ||
                                results.events.isNotEmpty() ||
                                results.tags.isNotEmpty()

                        if (!hasResults) {
                            EmptyView("brak wynik\u00f3w")
                        } else {
                            LazyColumn(
                                modifier =
                                    Modifier
                                        .fillMaxSize()
                                        .padding(horizontal = PoziomkiTheme.spacing.md),
                                contentPadding = PaddingValues(bottom = LocalNavBarPadding.current),
                                verticalArrangement = Arrangement.spacedBy(PoziomkiTheme.spacing.sm),
                            ) {
                                // Tags section
                                if (results.tags.isNotEmpty()) {
                                    item {
                                        Text(
                                            text = "tagi",
                                            style = MaterialTheme.typography.titleSmall,
                                            color = TextSecondary,
                                            modifier = Modifier.padding(vertical = 4.dp),
                                        )
                                        FlowRow(
                                            horizontalArrangement = Arrangement.spacedBy(8.dp),
                                            verticalArrangement = Arrangement.spacedBy(8.dp),
                                        ) {
                                            results.tags.forEach { tag ->
                                                Surface(
                                                    shape = RoundedCornerShape(16.dp),
                                                    color = SurfaceElevated,
                                                ) {
                                                    Text(
                                                        text = "${tag.emoji ?: ""} ${tag.name}".trim(),
                                                        fontFamily = NunitoFamily,
                                                        color = TextPrimary,
                                                        fontSize = 13.sp,
                                                        modifier = Modifier.padding(horizontal = 12.dp, vertical = 6.dp),
                                                    )
                                                }
                                            }
                                        }
                                    }
                                }

                                // Profiles section
                                if (results.profiles.isNotEmpty()) {
                                    item {
                                        Text(
                                            text = "osoby",
                                            style = MaterialTheme.typography.titleSmall,
                                            color = TextSecondary,
                                            modifier = Modifier.padding(vertical = 4.dp),
                                        )
                                    }
                                    items(results.profiles, key = { "p-${it.id}" }) { profile ->
                                        ProfileCard(
                                            name = profile.name,
                                            profilePicture = profile.profilePicture,
                                            matchingTags = state.ownTags.filter { it.id in profile.tags },
                                            program = profile.program,
                                            onClick = { onNavigateToProfile(profile.id) },
                                        )
                                    }
                                }

                                // Events section
                                if (results.events.isNotEmpty()) {
                                    item {
                                        Text(
                                            text = "wydarzenia",
                                            style = MaterialTheme.typography.titleSmall,
                                            color = TextSecondary,
                                            modifier = Modifier.padding(vertical = 4.dp),
                                        )
                                    }
                                    items(results.events, key = { "e-${it.id}" }) { event ->
                                        Surface(
                                            modifier =
                                                Modifier
                                                    .fillMaxWidth()
                                                    .clickable { onNavigateToEventDetail(event.id) },
                                            shape = RoundedCornerShape(12.dp),
                                            color = SurfaceElevated,
                                        ) {
                                            Column(modifier = Modifier.padding(12.dp)) {
                                                Text(
                                                    text = event.title,
                                                    style = MaterialTheme.typography.titleSmall,
                                                    color = TextPrimary,
                                                    fontFamily = NunitoFamily,
                                                )
                                                val loc = event.location
                                                if (loc != null) {
                                                    Text(
                                                        text = loc,
                                                        fontSize = 13.sp,
                                                        color = TextSecondary,
                                                        fontFamily = NunitoFamily,
                                                    )
                                                }
                                                Text(
                                                    text = event.creatorName,
                                                    fontSize = 12.sp,
                                                    color = TextMuted,
                                                    fontFamily = NunitoFamily,
                                                )
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            } else {
                // Default: profile recommendations
                when {
                    state.isLoading && state.profiles.isEmpty() -> {
                        LoadingView()
                    }

                    state.profiles.isEmpty() -> {
                        EmptyView("brak profili do wy\u015bwietlenia")
                    }

                    else -> {
                        PullToRefreshBox(
                            isRefreshing = state.isRefreshing,
                            onRefresh = { viewModel.pullToRefresh() },
                        ) {
                            LazyColumn(
                                modifier = Modifier.fillMaxSize(),
                                contentPadding = PaddingValues(bottom = LocalNavBarPadding.current),
                                verticalArrangement = Arrangement.spacedBy(PoziomkiTheme.spacing.sm),
                            ) {
                                if (state.profiles.size >= 7) {
                                    item(key = "recommended-row") {
                                        RecommendedPeopleRow(
                                            profiles = state.recommendedProfiles,
                                            onProfileClick = onNavigateToProfile,
                                        )
                                    }
                                }
                                val cardProfiles = if (state.profiles.size >= 7) state.remainingProfiles else state.profiles
                                items(cardProfiles, key = { it.id }) { profile ->
                                    val ownIds = state.ownTags.mapTo(mutableSetOf()) { it.id }
                                    ProfileCard(
                                        name = profile.name,
                                        profilePicture = profile.profilePicture,
                                        gradientStart = profile.gradientStart,
                                        gradientEnd = profile.gradientEnd,
                                        matchingTags = profile.tags.filter { it.id in ownIds },
                                        bio = profile.bio,
                                        onClick = { onNavigateToProfile(profile.id) },
                                        modifier = Modifier.padding(horizontal = PoziomkiTheme.spacing.md),
                                    )
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
        }
    }
}
