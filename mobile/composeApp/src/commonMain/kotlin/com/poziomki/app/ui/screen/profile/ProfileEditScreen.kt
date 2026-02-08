package com.poziomki.app.ui.screen.profile

import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.asPaddingValues
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.statusBars
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.filled.Add
import androidx.compose.material.icons.filled.Close
import androidx.compose.material.icons.filled.Search
import androidx.compose.material.icons.filled.Tune
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.TextField
import androidx.compose.material3.TextFieldDefaults
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import coil3.compose.AsyncImage
import com.poziomki.app.api.Tag
import com.poziomki.app.ui.theme.Border
import com.poziomki.app.ui.theme.NunitoFamily
import com.poziomki.app.ui.theme.PoziomkiTheme
import com.poziomki.app.ui.theme.Primary
import com.poziomki.app.ui.theme.PrimaryLight
import com.poziomki.app.ui.theme.TextMuted
import com.poziomki.app.ui.theme.TextPrimary
import com.poziomki.app.ui.theme.TextSecondary
import com.poziomki.app.util.rememberSingleImagePicker
import com.poziomki.app.util.resolveImageUrl
import org.koin.compose.viewmodel.koinViewModel
import com.poziomki.app.ui.theme.Surface as SurfaceColor

@OptIn(ExperimentalLayoutApi::class)
@Composable
fun ProfileEditScreen(
    onBack: () -> Unit,
    viewModel: ProfileEditViewModel = koinViewModel(),
) {
    val state by viewModel.state.collectAsState()
    val nunito = NunitoFamily

    val imagePicker =
        rememberSingleImagePicker { bytes ->
            if (bytes != null) {
                viewModel.uploadAndAddImage(bytes)
            }
        }

    Column(
        modifier =
            Modifier
                .fillMaxSize()
                .background(MaterialTheme.colorScheme.background),
    ) {
        // Top bar
        val statusBarPadding = WindowInsets.statusBars.asPaddingValues().calculateTopPadding()
        Row(
            modifier =
                Modifier
                    .fillMaxWidth()
                    .padding(
                        start = PoziomkiTheme.spacing.sm,
                        end = PoziomkiTheme.spacing.sm,
                        top = statusBarPadding + PoziomkiTheme.spacing.sm,
                        bottom = PoziomkiTheme.spacing.sm,
                    ),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            IconButton(onClick = onBack) {
                Icon(
                    Icons.AutoMirrored.Filled.ArrowBack,
                    contentDescription = "Wstecz",
                    tint = TextPrimary,
                )
            }
            Text(
                text = "edytuj profil",
                fontFamily = com.poziomki.app.ui.theme.MontserratFamily,
                fontWeight = FontWeight.ExtraBold,
                fontSize = 20.sp,
                color = TextPrimary,
            )
        }

        if (state.isLoading) {
            Box(Modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
                CircularProgressIndicator(color = Primary)
            }
            return
        }

        Column(
            modifier =
                Modifier
                    .fillMaxSize()
                    .verticalScroll(rememberScrollState())
                    .padding(horizontal = PoziomkiTheme.spacing.lg),
        ) {
            // --- zdjęcia ---
            Text(
                text = "zdjęcia",
                fontFamily = nunito,
                fontWeight = FontWeight.SemiBold,
                fontSize = 14.sp,
                color = Primary,
            )
            Spacer(modifier = Modifier.height(PoziomkiTheme.spacing.sm))

            Row(
                modifier = Modifier.horizontalScroll(rememberScrollState()),
                horizontalArrangement = Arrangement.spacedBy(PoziomkiTheme.spacing.sm),
            ) {
                state.images.forEachIndexed { index, imageUrl ->
                    Box(
                        modifier =
                            Modifier
                                .size(80.dp)
                                .clip(RoundedCornerShape(12.dp)),
                    ) {
                        AsyncImage(
                            model = resolveImageUrl(imageUrl),
                            contentDescription = "Zdjęcie ${index + 1}",
                            modifier =
                                Modifier
                                    .size(80.dp)
                                    .clip(RoundedCornerShape(12.dp)),
                            contentScale = ContentScale.Crop,
                        )
                        // X overlay
                        Box(
                            modifier =
                                Modifier
                                    .align(Alignment.TopEnd)
                                    .padding(4.dp)
                                    .size(20.dp)
                                    .background(Color.Black.copy(alpha = 0.6f), RoundedCornerShape(50))
                                    .clickable { viewModel.removeImage(index) },
                            contentAlignment = Alignment.Center,
                        ) {
                            Icon(
                                Icons.Filled.Close,
                                contentDescription = "Usuń",
                                tint = Color.White,
                                modifier = Modifier.size(14.dp),
                            )
                        }
                    }
                }

                // Upload loading placeholder
                if (state.isUploading) {
                    Box(
                        modifier =
                            Modifier
                                .size(80.dp)
                                .background(SurfaceColor, RoundedCornerShape(12.dp))
                                .border(1.dp, Border, RoundedCornerShape(12.dp)),
                        contentAlignment = Alignment.Center,
                    ) {
                        CircularProgressIndicator(
                            color = Primary,
                            modifier = Modifier.size(24.dp),
                            strokeWidth = 2.dp,
                        )
                    }
                }

                // Add button
                Box(
                    modifier =
                        Modifier
                            .size(80.dp)
                            .border(
                                width = 1.dp,
                                color = Border,
                                shape = RoundedCornerShape(12.dp),
                            ).clip(RoundedCornerShape(12.dp))
                            .clickable(enabled = !state.isUploading) { imagePicker() },
                    contentAlignment = Alignment.Center,
                ) {
                    Column(horizontalAlignment = Alignment.CenterHorizontally) {
                        Icon(
                            Icons.Filled.Add,
                            contentDescription = "Dodaj zdjęcie",
                            tint = TextMuted,
                            modifier = Modifier.size(24.dp),
                        )
                        Text(
                            text = "dodaj",
                            fontFamily = nunito,
                            fontWeight = FontWeight.Medium,
                            fontSize = 11.sp,
                            color = TextMuted,
                        )
                    }
                }
            }

            Spacer(modifier = Modifier.height(PoziomkiTheme.spacing.lg))

            // --- bio ---
            Text(
                text = "bio",
                fontFamily = nunito,
                fontWeight = FontWeight.SemiBold,
                fontSize = 14.sp,
                color = Primary,
            )
            Spacer(modifier = Modifier.height(PoziomkiTheme.spacing.sm))

            TextField(
                value = state.bio,
                onValueChange = { viewModel.updateBio(it) },
                modifier =
                    Modifier
                        .fillMaxWidth()
                        .border(1.dp, Border, RoundedCornerShape(16.dp)),
                shape = RoundedCornerShape(16.dp),
                colors =
                    TextFieldDefaults.colors(
                        focusedContainerColor = SurfaceColor,
                        unfocusedContainerColor = SurfaceColor,
                        focusedTextColor = TextPrimary,
                        unfocusedTextColor = TextPrimary,
                        focusedIndicatorColor = Color.Transparent,
                        unfocusedIndicatorColor = Color.Transparent,
                        cursorColor = Primary,
                    ),
                textStyle =
                    androidx.compose.ui.text.TextStyle(
                        fontFamily = nunito,
                        fontWeight = FontWeight.Normal,
                        fontSize = 16.sp,
                    ),
                maxLines = 5,
                placeholder = {
                    Text(
                        text = "Napisz coś o sobie...",
                        fontFamily = nunito,
                        color = TextMuted,
                        fontSize = 16.sp,
                    )
                },
            )
            Text(
                text = "${state.bio.length}/500",
                fontFamily = nunito,
                fontWeight = FontWeight.Normal,
                fontSize = 12.sp,
                color = TextMuted,
                modifier =
                    Modifier
                        .fillMaxWidth()
                        .padding(top = 4.dp),
                textAlign = TextAlign.End,
            )

            Spacer(modifier = Modifier.height(PoziomkiTheme.spacing.md))

            // --- kierunek ---
            Text(
                text = "kierunek",
                fontFamily = nunito,
                fontWeight = FontWeight.SemiBold,
                fontSize = 14.sp,
                color = Primary,
            )
            Spacer(modifier = Modifier.height(PoziomkiTheme.spacing.sm))

            Row(
                modifier =
                    Modifier
                        .fillMaxWidth()
                        .background(SurfaceColor, RoundedCornerShape(50))
                        .border(1.dp, Border, RoundedCornerShape(50))
                        .padding(horizontal = 16.dp, vertical = 12.dp),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Text(
                    text = state.program.ifEmpty { "Wybierz kierunek" },
                    fontFamily = nunito,
                    fontWeight = FontWeight.Normal,
                    fontSize = 16.sp,
                    color = if (state.program.isEmpty()) TextMuted else TextPrimary,
                    modifier = Modifier.weight(1f),
                )
                if (state.program.isNotEmpty()) {
                    Icon(
                        Icons.Filled.Close,
                        contentDescription = "Wyczyść",
                        tint = TextMuted,
                        modifier =
                            Modifier
                                .size(20.dp)
                                .clickable { viewModel.clearProgram() },
                    )
                }
            }

            Spacer(modifier = Modifier.height(PoziomkiTheme.spacing.lg))

            // --- zainteresowania ---
            val interestTags = state.allTags.filter { it.scope == "interest" }
            val selectedInterests = state.selectedTags.filter { it.scope == "interest" }
            val filteredInterests =
                interestTags.filter { tag ->
                    selectedInterests.none { it.id == tag.id } &&
                        (state.interestQuery.isBlank() || tag.name.contains(state.interestQuery, ignoreCase = true))
                }

            Text(
                text = "zainteresowania",
                fontFamily = nunito,
                fontWeight = FontWeight.SemiBold,
                fontSize = 14.sp,
                color = Primary,
            )
            Spacer(modifier = Modifier.height(PoziomkiTheme.spacing.sm))

            TagSearchBar(
                query = state.interestQuery,
                onQueryChange = { viewModel.updateInterestQuery(it) },
                placeholder = "szukaj zainteresowań...",
            )

            if (state.interestQuery.isNotBlank() && filteredInterests.isNotEmpty()) {
                Spacer(modifier = Modifier.height(PoziomkiTheme.spacing.sm))
                FlowRow(
                    horizontalArrangement = Arrangement.spacedBy(PoziomkiTheme.spacing.sm),
                    verticalArrangement = Arrangement.spacedBy(PoziomkiTheme.spacing.sm),
                ) {
                    filteredInterests.take(10).forEach { tag ->
                        TagChip(
                            tag = tag,
                            selected = false,
                            onClick = {
                                viewModel.addTag(tag)
                                viewModel.updateInterestQuery("")
                            },
                        )
                    }
                }
            }

            if (selectedInterests.isNotEmpty()) {
                Spacer(modifier = Modifier.height(PoziomkiTheme.spacing.sm))
                FlowRow(
                    horizontalArrangement = Arrangement.spacedBy(PoziomkiTheme.spacing.sm),
                    verticalArrangement = Arrangement.spacedBy(PoziomkiTheme.spacing.sm),
                ) {
                    selectedInterests.forEach { tag ->
                        TagChip(
                            tag = tag,
                            selected = true,
                            onClick = { viewModel.removeTag(tag) },
                        )
                    }
                }
            }

            Spacer(modifier = Modifier.height(PoziomkiTheme.spacing.lg))

            // --- aktywności ---
            val activityTags = state.allTags.filter { it.scope == "activity" }
            val selectedActivities = state.selectedTags.filter { it.scope == "activity" }
            val filteredActivities =
                activityTags.filter { tag ->
                    selectedActivities.none { it.id == tag.id } &&
                        (state.activityQuery.isBlank() || tag.name.contains(state.activityQuery, ignoreCase = true))
                }

            Text(
                text = "aktywności",
                fontFamily = nunito,
                fontWeight = FontWeight.SemiBold,
                fontSize = 14.sp,
                color = Primary,
            )
            Spacer(modifier = Modifier.height(PoziomkiTheme.spacing.sm))

            TagSearchBar(
                query = state.activityQuery,
                onQueryChange = { viewModel.updateActivityQuery(it) },
                placeholder = "szukaj aktywności...",
            )

            if (state.activityQuery.isNotBlank() && filteredActivities.isNotEmpty()) {
                Spacer(modifier = Modifier.height(PoziomkiTheme.spacing.sm))
                FlowRow(
                    horizontalArrangement = Arrangement.spacedBy(PoziomkiTheme.spacing.sm),
                    verticalArrangement = Arrangement.spacedBy(PoziomkiTheme.spacing.sm),
                ) {
                    filteredActivities.take(10).forEach { tag ->
                        TagChip(
                            tag = tag,
                            selected = false,
                            onClick = {
                                viewModel.addTag(tag)
                                viewModel.updateActivityQuery("")
                            },
                        )
                    }
                }
            }

            if (selectedActivities.isNotEmpty()) {
                Spacer(modifier = Modifier.height(PoziomkiTheme.spacing.sm))
                FlowRow(
                    horizontalArrangement = Arrangement.spacedBy(PoziomkiTheme.spacing.sm),
                    verticalArrangement = Arrangement.spacedBy(PoziomkiTheme.spacing.sm),
                ) {
                    selectedActivities.forEach { tag ->
                        TagChip(
                            tag = tag,
                            selected = true,
                            onClick = { viewModel.removeTag(tag) },
                        )
                    }
                }
            }

            Spacer(modifier = Modifier.height(PoziomkiTheme.spacing.xl))

            // Save button
            androidx.compose.material3.Surface(
                modifier =
                    Modifier
                        .fillMaxWidth()
                        .clip(RoundedCornerShape(50))
                        .clickable(enabled = !state.isSaving) { viewModel.save(onBack) },
                shape = RoundedCornerShape(50),
                color = Primary,
            ) {
                Box(
                    modifier =
                        Modifier
                            .fillMaxWidth()
                            .padding(vertical = 14.dp),
                    contentAlignment = Alignment.Center,
                ) {
                    if (state.isSaving) {
                        CircularProgressIndicator(
                            color = Color.Black,
                            modifier = Modifier.size(20.dp),
                            strokeWidth = 2.dp,
                        )
                    } else {
                        Text(
                            text = "zapisz",
                            fontFamily = nunito,
                            fontWeight = FontWeight.SemiBold,
                            fontSize = 16.sp,
                            color = Color.Black,
                        )
                    }
                }
            }

            Spacer(modifier = Modifier.height(PoziomkiTheme.spacing.xl))
        }
    }
}

@Composable
private fun TagSearchBar(
    query: String,
    onQueryChange: (String) -> Unit,
    placeholder: String,
) {
    val nunito = NunitoFamily

    Row(
        modifier =
            Modifier
                .fillMaxWidth()
                .background(SurfaceColor, RoundedCornerShape(50))
                .border(1.dp, Border, RoundedCornerShape(50))
                .padding(horizontal = 12.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Icon(
            Icons.Filled.Search,
            contentDescription = null,
            tint = TextMuted,
            modifier = Modifier.size(20.dp),
        )
        Spacer(modifier = Modifier.width(8.dp))
        TextField(
            value = query,
            onValueChange = onQueryChange,
            modifier = Modifier.weight(1f),
            colors =
                TextFieldDefaults.colors(
                    focusedContainerColor = Color.Transparent,
                    unfocusedContainerColor = Color.Transparent,
                    focusedTextColor = TextPrimary,
                    unfocusedTextColor = TextPrimary,
                    focusedIndicatorColor = Color.Transparent,
                    unfocusedIndicatorColor = Color.Transparent,
                    cursorColor = Primary,
                ),
            textStyle =
                androidx.compose.ui.text.TextStyle(
                    fontFamily = nunito,
                    fontWeight = FontWeight.Normal,
                    fontSize = 14.sp,
                ),
            singleLine = true,
            placeholder = {
                Text(
                    text = placeholder,
                    fontFamily = nunito,
                    color = TextMuted,
                    fontSize = 14.sp,
                )
            },
        )
        Box(
            modifier =
                Modifier
                    .width(1.dp)
                    .height(24.dp)
                    .background(Border),
        )
        Spacer(modifier = Modifier.width(8.dp))
        Icon(
            Icons.Filled.Tune,
            contentDescription = null,
            tint = TextMuted,
            modifier = Modifier.size(20.dp),
        )
    }
}

@Composable
private fun TagChip(
    tag: Tag,
    selected: Boolean,
    onClick: () -> Unit,
) {
    val nunito = NunitoFamily
    val bgColor = if (selected) PrimaryLight else Color.Transparent
    val borderColor = if (selected) Primary else Border
    val textColor = if (selected) Primary else TextSecondary

    Row(
        modifier =
            Modifier
                .background(bgColor, RoundedCornerShape(50))
                .border(1.dp, borderColor, RoundedCornerShape(50))
                .clip(RoundedCornerShape(50))
                .clickable(onClick = onClick)
                .padding(horizontal = 12.dp, vertical = 6.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Text(
            text = "${tag.emoji ?: ""} ${tag.name}".trim(),
            fontFamily = nunito,
            fontWeight = FontWeight.Medium,
            fontSize = 14.sp,
            color = textColor,
        )
        if (selected) {
            Spacer(modifier = Modifier.width(4.dp))
            Icon(
                Icons.Filled.Close,
                contentDescription = "Usuń",
                tint = textColor,
                modifier = Modifier.size(16.dp),
            )
        }
    }
}
