package com.poziomki.app.ui.screen.main

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
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
import androidx.compose.ui.unit.dp
import com.poziomki.app.ui.component.ProfileCard
import com.poziomki.app.ui.theme.Border
import com.poziomki.app.ui.theme.NunitoFamily
import com.poziomki.app.ui.theme.PoziomkiTheme
import com.poziomki.app.ui.theme.TextMuted
import com.poziomki.app.ui.theme.TextPrimary
import com.poziomki.app.ui.theme.TextSecondary
import org.koin.compose.viewmodel.koinViewModel
import com.poziomki.app.ui.theme.Surface as SurfaceColor

@Composable
fun ExploreScreen(
    onNavigateToProfile: (String) -> Unit,
    viewModel: ExploreViewModel = koinViewModel(),
) {
    val state by viewModel.state.collectAsState()

    Column(
        modifier =
            Modifier
                .fillMaxSize()
                .background(MaterialTheme.colorScheme.background),
    ) {
        // Header
        Text(
            text = "poznaj",
            style = MaterialTheme.typography.headlineLarge,
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
                Text(
                    text = "szukaj osób...",
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
                    contentDescription = "Filter",
                    modifier = Modifier.size(22.dp),
                    tint = TextMuted,
                )
            }
        }

        Spacer(modifier = Modifier.height(PoziomkiTheme.spacing.md))

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
