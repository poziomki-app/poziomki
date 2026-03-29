package com.poziomki.app.ui.feature.onboarding

import androidx.compose.animation.animateColorAsState
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.BasicTextField
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.derivedStateOf
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
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
import com.poziomki.app.ui.designsystem.components.AppButton
import com.poziomki.app.ui.designsystem.components.ButtonVariant
import com.poziomki.app.ui.designsystem.components.OnboardingLayout
import com.poziomki.app.ui.designsystem.theme.AppTheme
import com.poziomki.app.ui.designsystem.theme.MontserratFamily
import com.poziomki.app.ui.designsystem.theme.NunitoFamily
import com.poziomki.app.ui.designsystem.theme.Primary
import com.poziomki.app.ui.designsystem.theme.SurfaceElevated
import com.poziomki.app.ui.designsystem.theme.TextMuted
import com.poziomki.app.ui.designsystem.theme.TextPrimary
import org.koin.compose.viewmodel.koinViewModel

@Composable
fun InterestsScreen(
    onNext: () -> Unit,
    onBack: () -> Unit,
    viewModel: OnboardingViewModel = koinViewModel(),
) {
    val state by viewModel.state.collectAsState()
    var searchQuery by remember { mutableStateOf("") }
    val selectedCount = state.selectedTagIds.size
    val ready = selectedCount >= 3

    OnboardingLayout(
        currentStep = 2,
        totalSteps = 3,
        showBack = true,
        onBack = onBack,
        footer = {
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.End,
            ) {
                AppButton(
                    text = "dalej",
                    onClick = onNext,
                    enabled = ready,
                    variant = if (ready) ButtonVariant.PRIMARY else ButtonVariant.OUTLINE,
                )
            }
        },
    ) {
        InterestsContent(
            state = state,
            searchQuery = searchQuery,
            onSearchQueryChange = { searchQuery = it },
            onToggleTag = { viewModel.toggleTag(it) },
        )
    }
}

@Composable
private fun InterestsContent(
    state: OnboardingState,
    searchQuery: String,
    onSearchQueryChange: (String) -> Unit,
    onToggleTag: (String) -> Unit,
) {
    val selectableTags by remember(state.availableTags) {
        derivedStateOf { state.availableTags.filter { it.category != "root" } }
    }

    val filteredTags by remember(selectableTags, searchQuery) {
        derivedStateOf {
            if (searchQuery.isBlank()) {
                selectableTags
            } else {
                selectableTags.filter { it.name.contains(searchQuery, ignoreCase = true) }
            }
        }
    }

    val groupedTags by remember(filteredTags, searchQuery) {
        derivedStateOf {
            if (searchQuery.isBlank()) {
                INTEREST_CATEGORIES.mapNotNull { category ->
                    val tags = filteredTags.filter { it.category == category.key }
                    if (tags.isNotEmpty()) category to tags else null
                }
            } else {
                emptyList()
            }
        }
    }

    Column(
        modifier =
            Modifier
                .padding(horizontal = AppTheme.spacing.lg)
                .padding(bottom = AppTheme.spacing.md),
    ) {
        Text(
            text = "zainteresowania",
            style = MaterialTheme.typography.headlineMedium,
            color = TextPrimary,
        )
        Spacer(modifier = Modifier.height(AppTheme.spacing.sm))

        val countColor by animateColorAsState(
            targetValue = if (state.selectedTagIds.size >= 3) Primary else TextMuted,
        )
        Text(
            text = "${state.selectedTagIds.size} wybrano \u00B7 minimum 3",
            style = MaterialTheme.typography.bodySmall,
            color = countColor,
        )
        Spacer(modifier = Modifier.height(AppTheme.spacing.md))

        SearchBar(query = searchQuery, onQueryChange = onSearchQueryChange)
        Spacer(modifier = Modifier.height(AppTheme.spacing.lg))

        if (searchQuery.isNotBlank()) {
            SearchResults(filteredTags, state.selectedTagIds, onToggleTag)
        } else {
            CategoryList(groupedTags, state.selectedTagIds, onToggleTag)
        }
    }
}

@OptIn(ExperimentalLayoutApi::class)
@Composable
private fun SearchResults(
    tags: List<Tag>,
    selectedTagIds: Set<String>,
    onToggleTag: (String) -> Unit,
) {
    if (tags.isEmpty()) {
        Text(
            text = "brak wyników",
            style = MaterialTheme.typography.bodyMedium,
            color = TextMuted,
        )
    } else {
        FlowRow(
            horizontalArrangement = Arrangement.spacedBy(6.dp),
            verticalArrangement = Arrangement.spacedBy(6.dp),
        ) {
            tags.forEach { tag ->
                val color = CATEGORY_MAP[tag.category]?.color ?: Primary
                InterestChip(
                    label = tag.name,
                    selected = tag.id in selectedTagIds,
                    accentColor = color,
                    onClick = { onToggleTag(tag.id) },
                )
            }
        }
    }
}

@Composable
private fun CategoryList(
    groupedTags: List<Pair<InterestCategoryInfo, List<Tag>>>,
    selectedTagIds: Set<String>,
    onToggleTag: (String) -> Unit,
) {
    groupedTags.forEachIndexed { index, (category, tags) ->
        if (index > 0) {
            Spacer(modifier = Modifier.height(AppTheme.spacing.md))
        }
        CategorySection(
            category = category,
            tags = tags,
            selectedTagIds = selectedTagIds,
            onToggleTag = onToggleTag,
        )
    }
}

@OptIn(ExperimentalLayoutApi::class)
@Composable
private fun CategorySection(
    category: InterestCategoryInfo,
    tags: List<Tag>,
    selectedTagIds: Set<String>,
    onToggleTag: (String) -> Unit,
) {
    Column {
        // Section header
        Row(
            verticalAlignment = Alignment.CenterVertically,
            modifier = Modifier.padding(bottom = AppTheme.spacing.sm),
        ) {
            Icon(
                imageVector = category.icon,
                contentDescription = null,
                modifier = Modifier.size(18.dp),
                tint = category.color,
            )
            Spacer(modifier = Modifier.width(AppTheme.spacing.sm))
            Text(
                text = category.displayName,
                style =
                    TextStyle(
                        fontFamily = MontserratFamily,
                        fontWeight = FontWeight.ExtraBold,
                        fontSize = 14.sp,
                        brush =
                            Brush.horizontalGradient(
                                colors =
                                    listOf(
                                        category.color,
                                        Color(0xFF6B7280),
                                    ),
                            ),
                    ),
            )
        }

        Spacer(modifier = Modifier.height(AppTheme.spacing.sm))

        // Tag chips
        FlowRow(
            horizontalArrangement = Arrangement.spacedBy(6.dp),
            verticalArrangement = Arrangement.spacedBy(6.dp),
        ) {
            tags.forEach { tag ->
                InterestChip(
                    label = tag.name,
                    selected = tag.id in selectedTagIds,
                    accentColor = category.color,
                    onClick = { onToggleTag(tag.id) },
                )
            }
        }
    }
}

@Composable
private fun SearchBar(
    query: String,
    onQueryChange: (String) -> Unit,
    modifier: Modifier = Modifier,
) {
    Surface(
        modifier = modifier.fillMaxWidth(),
        shape = RoundedCornerShape(AppTheme.radius.lg),
        color = SurfaceElevated,
    ) {
        Row(
            modifier = Modifier.padding(horizontal = 14.dp, vertical = 12.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Icon(
                imageVector = PhosphorIcons.Bold.MagnifyingGlass,
                contentDescription = null,
                modifier = Modifier.size(18.dp),
                tint = TextMuted,
            )
            Spacer(modifier = Modifier.width(AppTheme.spacing.sm))
            Box(modifier = Modifier.weight(1f)) {
                if (query.isEmpty()) {
                    Text(
                        text = "szukaj...",
                        style = MaterialTheme.typography.bodyMedium,
                        color = TextMuted,
                    )
                }
                BasicTextField(
                    value = query,
                    onValueChange = onQueryChange,
                    singleLine = true,
                    textStyle =
                        TextStyle(
                            fontFamily = NunitoFamily,
                            fontSize = 14.sp,
                            color = TextPrimary,
                        ),
                    cursorBrush = SolidColor(Primary),
                )
            }
            if (query.isNotEmpty()) {
                Spacer(modifier = Modifier.width(AppTheme.spacing.sm))
                Icon(
                    imageVector = PhosphorIcons.Bold.X,
                    contentDescription = "Clear",
                    modifier =
                        Modifier
                            .size(18.dp)
                            .clickable { onQueryChange("") },
                    tint = TextMuted,
                )
            }
        }
    }
}

private val ChipShape = RoundedCornerShape(50)
private val ChipUnselected = Color(0xFF1A1F26)

@Composable
private fun InterestChip(
    label: String,
    selected: Boolean,
    accentColor: Color,
    onClick: () -> Unit,
    modifier: Modifier = Modifier,
) {
    val bgColor by animateColorAsState(
        targetValue = if (selected) accentColor else ChipUnselected,
    )
    val textColor by animateColorAsState(
        targetValue = if (selected) Color.White else Color(0xFFB0B8C4),
    )

    Box(
        modifier =
            modifier
                .clip(ChipShape)
                .background(bgColor)
                .clickable(onClick = onClick)
                .padding(horizontal = 10.dp, vertical = 4.dp),
        contentAlignment = Alignment.Center,
    ) {
        Text(
            text = label.lowercase(),
            fontFamily = NunitoFamily,
            fontWeight = FontWeight.Medium,
            fontSize = 12.sp,
            color = textColor,
        )
    }
}
