package com.poziomki.app.ui.screen.main

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.BasicTextField
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Search
import androidx.compose.material.icons.filled.Tune
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.VerticalDivider
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.SolidColor
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.poziomki.app.ui.component.ProfileCard
import com.poziomki.app.ui.theme.Border
import com.poziomki.app.ui.theme.NunitoFamily
import com.poziomki.app.ui.theme.PoziomkiTheme
import com.poziomki.app.ui.theme.TextMuted
import com.poziomki.app.ui.theme.TextPrimary
import com.poziomki.app.ui.theme.TextSecondary
import org.koin.compose.viewmodel.koinViewModel
import com.poziomki.app.ui.theme.Surface as SurfaceColor
import com.poziomki.app.ui.theme.SurfaceElevated

@OptIn(ExperimentalLayoutApi::class)
@Composable
fun ExploreScreen(
    onNavigateToProfile: (String) -> Unit,
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
        // Header
        Text(
            text = "poznaj",
            style = MaterialTheme.typography.headlineMedium,
            color = TextPrimary,
            modifier =
                Modifier.padding(
                    horizontal = PoziomkiTheme.spacing.lg,
                    vertical = PoziomkiTheme.spacing.md,
                ),
        )

        // Search bar
        Surface(
            modifier =
                Modifier
                    .fillMaxWidth()
                    .padding(horizontal = PoziomkiTheme.spacing.md)
                    .height(48.dp),
            shape = RoundedCornerShape(28.dp),
            color = SurfaceColor,
            border = androidx.compose.foundation.BorderStroke(1.dp, Border),
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
                    contentDescription = "Search",
                    modifier = Modifier.size(22.dp),
                    tint = TextMuted,
                )
                Spacer(modifier = Modifier.width(12.dp))
                Box(modifier = Modifier.weight(1f)) {
                    if (state.query.isEmpty()) {
                        Text(
                            text = "szukaj os\u00f3b, wydarze\u0144...",
                            fontFamily = NunitoFamily,
                            color = TextMuted,
                        )
                    }
                    BasicTextField(
                        value = state.query,
                        onValueChange = viewModel::updateQuery,
                        singleLine = true,
                        textStyle =
                            TextStyle(
                                fontFamily = NunitoFamily,
                                color = TextPrimary,
                                fontSize = 15.sp,
                            ),
                        cursorBrush = SolidColor(MaterialTheme.colorScheme.primary),
                        modifier = Modifier.fillMaxWidth(),
                    )
                }
                VerticalDivider(
                    modifier = Modifier.height(20.dp),
                    thickness = 1.dp,
                    color = Border,
                )
                Spacer(modifier = Modifier.width(12.dp))
                Icon(
                    Icons.Filled.Tune,
                    contentDescription = "Filter",
                    modifier = Modifier.size(22.dp),
                    tint = TextMuted,
                )
            }
        }

        Spacer(modifier = Modifier.height(PoziomkiTheme.spacing.md))

        if (isSearchActive) {
            // Search results
            when {
                state.isSearching -> {
                    Box(Modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
                        CircularProgressIndicator(color = MaterialTheme.colorScheme.primary)
                    }
                }

                state.searchResults != null -> {
                    val results = state.searchResults!!
                    val hasResults =
                        results.profiles.isNotEmpty() ||
                            results.events.isNotEmpty() ||
                            results.tags.isNotEmpty()

                    if (!hasResults) {
                        Box(Modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
                            Text(
                                "brak wynik\u00f3w",
                                fontFamily = NunitoFamily,
                                color = TextSecondary,
                            )
                        }
                    } else {
                        LazyColumn(
                            modifier =
                                Modifier
                                    .fillMaxSize()
                                    .padding(horizontal = PoziomkiTheme.spacing.md),
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
                                        program = profile.program,
                                        profilePicture = profile.profilePicture,
                                        tags = profile.tags.map { tagName ->
                                            com.poziomki.app.api.Tag(
                                                id = "",
                                                name = tagName,
                                                scope = "interest",
                                            )
                                        },
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
                                                .clickable { },
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
                state.isLoading -> {
                    Box(Modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
                        CircularProgressIndicator(color = MaterialTheme.colorScheme.primary)
                    }
                }

                state.profiles.isEmpty() -> {
                    Box(Modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
                        Text(
                            "brak profili do wy\u015bwietlenia",
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
                        verticalArrangement = Arrangement.spacedBy(PoziomkiTheme.spacing.sm),
                    ) {
                        items(state.profiles, key = { it.id }) { profile ->
                            ProfileCard(
                                name = profile.name,
                                program = profile.program,
                                profilePicture = profile.profilePicture,
                                tags = profile.tags,
                                onClick = { onNavigateToProfile(profile.id) },
                            )
                        }
                    }
                }
            }
        }
    }
}
