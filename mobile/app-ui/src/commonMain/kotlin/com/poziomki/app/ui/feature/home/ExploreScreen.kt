package com.poziomki.app.ui.feature.home

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.BasicTextField
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.BasicAlertDialog
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
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
import androidx.compose.ui.graphics.SolidColor
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.adamglin.PhosphorIcons
import com.adamglin.phosphoricons.Bold
import com.adamglin.phosphoricons.bold.MagnifyingGlass
import com.adamglin.phosphoricons.bold.X
import com.poziomki.app.network.Tag
import com.poziomki.app.ui.designsystem.components.AppSnackbar
import com.poziomki.app.ui.designsystem.components.EmptyView
import com.poziomki.app.ui.designsystem.components.LoadingView
import com.poziomki.app.ui.designsystem.components.ProfileCard
import com.poziomki.app.ui.designsystem.components.SearchableScreenHeader
import com.poziomki.app.ui.designsystem.theme.MontserratFamily
import com.poziomki.app.ui.designsystem.theme.NunitoFamily
import com.poziomki.app.ui.designsystem.theme.PoziomkiTheme
import com.poziomki.app.ui.designsystem.theme.Primary
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
    var searchActive by remember { mutableStateOf(false) }

    Column(
        modifier =
            Modifier
                .fillMaxSize()
                .background(MaterialTheme.colorScheme.background),
    ) {
        SearchableScreenHeader(
            title = "poznaj",
            searchQuery = state.query,
            onSearchQueryChange = viewModel::updateQuery,
            searchActive = searchActive,
            onSearchActiveChange = { searchActive = it },
            filterActive = state.isFilterActive,
            onFilterClick = { viewModel.toggleShowTagFilter() },
        ) {
            profileAvatarAction()
        }

        if (state.showTagFilter) {
            ExploreTagFilterDialog(
                ownTags = state.ownTags,
                selectedTags = state.selectedTags,
                searchQuery = state.tagFilterQuery,
                searchResults = state.tagFilterSearchResults,
                isSearching = state.isSearchingFilterTags,
                onSearchQueryChange = viewModel::updateTagFilterQuery,
                onToggleTag = viewModel::toggleTagFilter,
                onClear = viewModel::clearTagFilters,
                onDismiss = { viewModel.toggleShowTagFilter() },
            )
        }

        if (state.isFilterActive) {
            ActiveTagFilterRow(
                selectedTags = state.selectedTags,
                onRemove = viewModel::toggleTagFilter,
                onClear = viewModel::clearTagFilters,
            )
        }

        Spacer(modifier = Modifier.height(PoziomkiTheme.spacing.md))

        val isFilterOnlyMode = !isSearchActive && state.isFilterActive
        Box(modifier = Modifier.fillMaxSize()) {
            if (isFilterOnlyMode) {
                val results = state.filteredProfiles
                if (results.isEmpty()) {
                    EmptyView("brak osób pasujących do filtra")
                } else {
                    LazyColumn(
                        modifier =
                            Modifier
                                .fillMaxSize()
                                .padding(horizontal = PoziomkiTheme.spacing.md),
                        contentPadding = PaddingValues(bottom = LocalNavBarPadding.current),
                        verticalArrangement = Arrangement.spacedBy(PoziomkiTheme.spacing.sm),
                    ) {
                        item {
                            Text(
                                text = "osoby",
                                style = MaterialTheme.typography.titleSmall,
                                color = TextSecondary,
                                modifier = Modifier.padding(vertical = 4.dp),
                            )
                        }
                        items(results, key = { it.id }) { profile ->
                            val ownIds = state.ownTags.mapTo(mutableSetOf()) { it.id }
                            ProfileCard(
                                name = profile.name,
                                profilePicture = profile.profilePicture,
                                gradientStart = profile.gradientStart,
                                gradientEnd = profile.gradientEnd,
                                matchingTags = profile.tags.filter { it.id in ownIds },
                                onClick = { onNavigateToProfile(profile.id) },
                            )
                        }
                    }
                }
            } else if (isSearchActive) {
                // Search results
                when {
                    state.isSearching -> {
                        LoadingView()
                    }

                    state.searchResults != null -> {
                        val raw = state.searchResults!!
                        val selectedIds = state.selectedTagIds
                        val results =
                            if (selectedIds.isEmpty()) {
                                raw
                            } else {
                                raw.copy(
                                    profiles = raw.profiles.filter { p -> p.tags.any { it in selectedIds } },
                                )
                            }
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

@OptIn(ExperimentalLayoutApi::class, ExperimentalMaterial3Api::class)
@Composable
@Suppress("LongParameterList", "LongMethod")
private fun ExploreTagFilterDialog(
    ownTags: List<Tag>,
    selectedTags: List<Tag>,
    searchQuery: String,
    searchResults: List<Tag>,
    isSearching: Boolean,
    onSearchQueryChange: (String) -> Unit,
    onToggleTag: (Tag) -> Unit,
    onClear: () -> Unit,
    onDismiss: () -> Unit,
) {
    val selectedIds = selectedTags.mapTo(mutableSetOf()) { it.id }
    val ownInterests = ownTags.filter { it.scope == "interest" }.sortedBy { it.name }
    val ownActivities = ownTags.filter { it.scope == "activity" }.sortedBy { it.name }
    val ownIds = ownTags.mapTo(mutableSetOf()) { it.id }
    val customSelected = selectedTags.filter { it.id !in ownIds }
    val searchHits = searchResults.filter { it.id !in ownIds }

    BasicAlertDialog(onDismissRequest = onDismiss) {
        Surface(
            shape = RoundedCornerShape(20.dp),
            color = MaterialTheme.colorScheme.surface,
        ) {
            Column(modifier = Modifier.padding(20.dp)) {
                Row(
                    modifier = Modifier.fillMaxWidth(),
                    horizontalArrangement = Arrangement.SpaceBetween,
                    verticalAlignment = Alignment.CenterVertically,
                ) {
                    Text(
                        text = "filtruj po tagach",
                        fontFamily = MontserratFamily,
                        fontWeight = FontWeight.ExtraBold,
                        fontSize = 18.sp,
                        color = TextPrimary,
                    )
                    if (selectedTags.isNotEmpty()) {
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
                Spacer(modifier = Modifier.height(12.dp))
                TagFilterSearchField(
                    query = searchQuery,
                    onQueryChange = onSearchQueryChange,
                )
                Spacer(modifier = Modifier.height(12.dp))
                Column(
                    modifier =
                        Modifier
                            .heightIn(max = 420.dp)
                            .verticalScroll(rememberScrollState()),
                ) {
                    if (searchQuery.length >= 2) {
                        TagFilterSection(
                            label = if (isSearching) "szukam..." else "wyniki",
                            tags = searchHits,
                            selectedIds = selectedIds,
                            onToggleTag = onToggleTag,
                            emptyText = if (isSearching) null else "brak wyników",
                        )
                    } else {
                        if (customSelected.isNotEmpty()) {
                            TagFilterSection(
                                label = "wybrane",
                                tags = customSelected,
                                selectedIds = selectedIds,
                                onToggleTag = onToggleTag,
                            )
                        }
                        TagFilterSection(
                            label = "twoje zainteresowania",
                            tags = ownInterests,
                            selectedIds = selectedIds,
                            onToggleTag = onToggleTag,
                            emptyText = "brak zainteresowań w profilu",
                        )
                        TagFilterSection(
                            label = "twoje aktywności",
                            tags = ownActivities,
                            selectedIds = selectedIds,
                            onToggleTag = onToggleTag,
                            emptyText = "brak aktywności w profilu",
                        )
                    }
                }
            }
        }
    }
}

@Composable
private fun TagFilterSearchField(
    query: String,
    onQueryChange: (String) -> Unit,
) {
    Surface(
        modifier =
            Modifier
                .fillMaxWidth()
                .heightIn(min = 40.dp),
        shape = RoundedCornerShape(20.dp),
        color = SurfaceElevated,
    ) {
        Row(
            modifier = Modifier.padding(horizontal = 12.dp, vertical = 8.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Icon(
                PhosphorIcons.Bold.MagnifyingGlass,
                contentDescription = null,
                modifier = Modifier.size(16.dp),
                tint = TextMuted,
            )
            Spacer(modifier = Modifier.width(8.dp))
            Box(modifier = Modifier.weight(1f)) {
                if (query.isEmpty()) {
                    Text(
                        text = "szukaj",
                        fontFamily = NunitoFamily,
                        color = TextMuted,
                        fontSize = 14.sp,
                    )
                }
                BasicTextField(
                    value = query,
                    onValueChange = onQueryChange,
                    singleLine = true,
                    textStyle =
                        TextStyle(
                            fontFamily = NunitoFamily,
                            color = TextPrimary,
                            fontSize = 14.sp,
                        ),
                    cursorBrush = SolidColor(Primary),
                    modifier = Modifier.fillMaxWidth(),
                )
            }
        }
    }
}

@OptIn(ExperimentalLayoutApi::class)
@Composable
private fun TagFilterSection(
    label: String,
    tags: List<Tag>,
    selectedIds: Set<String>,
    onToggleTag: (Tag) -> Unit,
    emptyText: String? = null,
) {
    Text(
        text = label,
        style = MaterialTheme.typography.titleSmall,
        color = TextSecondary,
        modifier = Modifier.padding(vertical = 4.dp),
    )
    if (tags.isEmpty()) {
        if (emptyText != null) {
            Text(
                text = emptyText,
                fontFamily = NunitoFamily,
                color = TextMuted,
                fontSize = 13.sp,
                modifier = Modifier.padding(vertical = 4.dp),
            )
        }
    } else {
        FlowRow(
            horizontalArrangement = Arrangement.spacedBy(8.dp),
            verticalArrangement = Arrangement.spacedBy(8.dp),
            modifier = Modifier.padding(bottom = 8.dp),
        ) {
            tags.forEach { tag ->
                TagFilterChip(
                    tag = tag,
                    selected = tag.id in selectedIds,
                    onClick = { onToggleTag(tag) },
                )
            }
        }
    }
    Spacer(modifier = Modifier.height(4.dp))
}

@Composable
private fun TagFilterChip(
    tag: Tag,
    selected: Boolean,
    onClick: () -> Unit,
) {
    val bg = if (selected) Primary else SurfaceElevated
    val fg = if (selected) MaterialTheme.colorScheme.onPrimary else TextPrimary
    Surface(
        shape = RoundedCornerShape(16.dp),
        color = bg,
        modifier = Modifier.clickable(onClick = onClick),
    ) {
        Text(
            text = "${tag.emoji ?: ""} ${tag.name}".trim(),
            fontFamily = NunitoFamily,
            fontWeight = if (selected) FontWeight.Bold else FontWeight.Medium,
            color = fg,
            fontSize = 13.sp,
            modifier = Modifier.padding(horizontal = 12.dp, vertical = 8.dp),
        )
    }
}

@OptIn(ExperimentalLayoutApi::class)
@Composable
private fun ActiveTagFilterRow(
    selectedTags: List<Tag>,
    onRemove: (Tag) -> Unit,
    onClear: () -> Unit,
) {
    Row(
        modifier =
            Modifier
                .fillMaxWidth()
                .padding(start = 24.dp, end = 16.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        FlowRow(
            modifier = Modifier.weight(1f),
            horizontalArrangement = Arrangement.spacedBy(6.dp),
            verticalArrangement = Arrangement.spacedBy(4.dp),
        ) {
            selectedTags.forEach { tag ->
                Surface(
                    shape = RoundedCornerShape(12.dp),
                    color = Primary,
                    modifier = Modifier.clickable { onRemove(tag) },
                ) {
                    Text(
                        text = "${tag.emoji ?: ""} ${tag.name} ×".trim(),
                        fontFamily = NunitoFamily,
                        fontWeight = FontWeight.Medium,
                        color = MaterialTheme.colorScheme.onPrimary,
                        fontSize = 11.sp,
                        modifier = Modifier.padding(horizontal = 8.dp, vertical = 1.dp),
                    )
                }
            }
        }
        IconButton(
            onClick = onClear,
            modifier = Modifier.size(48.dp),
        ) {
            Icon(
                PhosphorIcons.Bold.X,
                contentDescription = "Wyczyść filtry",
                modifier = Modifier.size(18.dp),
                tint = Primary,
            )
        }
    }
}
