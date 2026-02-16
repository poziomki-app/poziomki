package com.poziomki.app.ui.screen.profile

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
import androidx.compose.foundation.layout.imePadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.navigationBars
import androidx.compose.foundation.layout.statusBars
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.BasicTextField
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Add
import androidx.compose.material.icons.filled.Close
import androidx.compose.material.icons.filled.Edit
import androidx.compose.material.icons.filled.Search
import androidx.compose.material.icons.filled.Tune
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.drawBehind
import androidx.compose.ui.geometry.CornerRadius
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.PathEffect
import androidx.compose.ui.graphics.SolidColor
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import coil3.compose.AsyncImage
import com.poziomki.app.api.Tag
import com.poziomki.app.ui.component.ScreenHeader
import com.poziomki.app.ui.component.ButtonVariant
import com.poziomki.app.ui.component.PoziomkiButton
import com.poziomki.app.ui.component.PoziomkiTextField
import com.poziomki.app.ui.component.SectionLabel
import com.poziomki.app.ui.theme.Border
import com.poziomki.app.ui.theme.NunitoFamily
import com.poziomki.app.ui.theme.Overlay
import com.poziomki.app.ui.theme.PoziomkiTheme
import com.poziomki.app.ui.theme.Primary
import com.poziomki.app.ui.theme.PrimaryLight
import com.poziomki.app.ui.theme.TextMuted
import com.poziomki.app.ui.theme.TextPrimary
import com.poziomki.app.ui.theme.TextSecondary
import com.poziomki.app.ui.theme.White
import com.poziomki.app.util.rememberSingleImagePicker
import com.poziomki.app.util.resolveImageUrl
import org.koin.compose.viewmodel.koinViewModel
import com.poziomki.app.ui.theme.Surface as SurfaceColor

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

    val navBarBottom = WindowInsets.navigationBars.asPaddingValues().calculateBottomPadding()

    Column(
        modifier =
            Modifier
                .fillMaxSize()
                .background(MaterialTheme.colorScheme.background)
                .imePadding(),
    ) {
        // Top bar
        val statusBarPadding = WindowInsets.statusBars.asPaddingValues().calculateTopPadding()
        ScreenHeader(
            title = "edytuj profil",
            onBack = onBack,
            modifier = Modifier.padding(top = statusBarPadding),
        )

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
            SectionLabel("zdjęcia")


            ImageGalleryRow(
                images = state.images,
                isUploading = state.isUploading,
                onRemoveImage = { viewModel.removeImage(it) },
                onAddImage = { imagePicker() },
            )

            Spacer(modifier = Modifier.height(PoziomkiTheme.spacing.lg))

            // --- bio ---
            SectionLabel("bio")


            PoziomkiTextField(
                value = state.bio,
                onValueChange = { viewModel.updateBio(it) },
                placeholder = "Napisz coś o sobie...",
                singleLine = false,
                maxLines = 5,
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
            SectionLabel("kierunek")


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
            TagSection(
                label = "zainteresowania",
                query = state.interestQuery,
                onQueryChange = { viewModel.updateInterestQuery(it) },
                searchPlaceholder = "szukaj zainteresowań...",
                allTags = state.allTags.filter { it.scope == "interest" },
                selectedTags = state.selectedTags.filter { it.scope == "interest" },
                onAddTag = {
                    viewModel.addTag(it)
                    viewModel.updateInterestQuery("")
                },
                onRemoveTag = { viewModel.removeTag(it) },
            )

            Spacer(modifier = Modifier.height(PoziomkiTheme.spacing.lg))

            // --- aktywności ---
            TagSection(
                label = "aktywności",
                query = state.activityQuery,
                onQueryChange = { viewModel.updateActivityQuery(it) },
                searchPlaceholder = "szukaj aktywności...",
                allTags = state.allTags.filter { it.scope == "activity" },
                selectedTags = state.selectedTags.filter { it.scope == "activity" },
                onAddTag = {
                    viewModel.addTag(it)
                    viewModel.updateActivityQuery("")
                },
                onRemoveTag = { viewModel.removeTag(it) },
            )

            Spacer(modifier = Modifier.height(PoziomkiTheme.spacing.xl))

            // Save button
            PoziomkiButton(
                text = "zapisz",
                onClick = { viewModel.save(onBack) },
                variant = ButtonVariant.PRIMARY,
                loading = state.isSaving,
            )

            Spacer(modifier = Modifier.height(navBarBottom + PoziomkiTheme.spacing.xl))
        }
    }
}

@Composable
private fun ImageGalleryRow(
    images: List<String>,
    isUploading: Boolean,
    onRemoveImage: (Int) -> Unit,
    onAddImage: () -> Unit,
) {
    val nunito = NunitoFamily
    val imageWidth = 90.dp
    val imageHeight = 120.dp

    Row(
        modifier = Modifier.horizontalScroll(rememberScrollState()),
        horizontalArrangement = Arrangement.spacedBy(PoziomkiTheme.spacing.sm),
    ) {
        images.forEachIndexed { index, imageUrl ->
            Box(
                modifier =
                    Modifier
                        .size(width = imageWidth, height = imageHeight)
                        .clip(RoundedCornerShape(12.dp)),
            ) {
                AsyncImage(
                    model = resolveImageUrl(imageUrl),
                    contentDescription = "Zdjęcie ${index + 1}",
                    modifier =
                        Modifier
                            .size(width = imageWidth, height = imageHeight)
                            .clip(RoundedCornerShape(12.dp)),
                    contentScale = ContentScale.Crop,
                )
                // X overlay
                Box(
                    modifier =
                        Modifier
                            .align(Alignment.TopEnd)
                            .padding(4.dp)
                            .size(22.dp)
                            .background(Overlay, RoundedCornerShape(50))
                            .clickable { onRemoveImage(index) },
                    contentAlignment = Alignment.Center,
                ) {
                    Icon(
                        Icons.Filled.Close,
                        contentDescription = "Usuń",
                        tint = White,
                        modifier = Modifier.size(14.dp),
                    )
                }
            }
        }

        // Upload loading placeholder
        if (isUploading) {
            Box(
                modifier =
                    Modifier
                        .size(width = imageWidth, height = imageHeight)
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

        // Add button — dashed border
        val dashedBorderColor = Border
        val cornerRadiusDp = 12.dp
        Box(
            modifier =
                Modifier
                    .size(width = imageWidth, height = imageHeight)
                    .clip(RoundedCornerShape(cornerRadiusDp))
                    .drawBehind {
                        val strokeWidth = 1.5.dp.toPx()
                        val dash = 8.dp.toPx()
                        val gap = 6.dp.toPx()
                        drawRoundRect(
                            color = dashedBorderColor,
                            style =
                                Stroke(
                                    width = strokeWidth,
                                    pathEffect = PathEffect.dashPathEffect(floatArrayOf(dash, gap), 0f),
                                ),
                            cornerRadius = CornerRadius(cornerRadiusDp.toPx()),
                        )
                    }.clickable(enabled = !isUploading) { onAddImage() },
            contentAlignment = Alignment.Center,
        ) {
            Column(horizontalAlignment = Alignment.CenterHorizontally) {
                Icon(
                    Icons.Filled.Add,
                    contentDescription = "Dodaj zdjęcie",
                    tint = Primary,
                    modifier = Modifier.size(28.dp),
                )
                Text(
                    text = "dodaj",
                    fontFamily = nunito,
                    fontWeight = FontWeight.Medium,
                    fontSize = 12.sp,
                    color = Primary,
                )
            }
        }

        // Edit icon
        Box(
            modifier = Modifier.size(width = imageWidth, height = imageHeight),
            contentAlignment = Alignment.Center,
        ) {
            Icon(
                Icons.Filled.Edit,
                contentDescription = "Edytuj zdjęcia",
                tint = TextMuted,
                modifier = Modifier.size(24.dp),
            )
        }
    }
}

@OptIn(ExperimentalLayoutApi::class)
@Composable
private fun TagSection(
    label: String,
    query: String,
    onQueryChange: (String) -> Unit,
    searchPlaceholder: String,
    allTags: List<Tag>,
    selectedTags: List<Tag>,
    onAddTag: (Tag) -> Unit,
    onRemoveTag: (Tag) -> Unit,
) {
    val nunito = NunitoFamily
    val filtered =
        allTags.filter { tag ->
            selectedTags.none { it.id == tag.id } &&
                (query.isBlank() || tag.name.contains(query, ignoreCase = true))
        }

    SectionLabel(label)

    TagSearchBar(
        query = query,
        onQueryChange = onQueryChange,
        placeholder = searchPlaceholder,
    )

    if (query.isNotBlank() && filtered.isNotEmpty()) {
        Spacer(modifier = Modifier.height(PoziomkiTheme.spacing.sm))
        FlowRow(
            horizontalArrangement = Arrangement.spacedBy(PoziomkiTheme.spacing.sm),
            verticalArrangement = Arrangement.spacedBy(PoziomkiTheme.spacing.sm),
        ) {
            filtered.take(10).forEach { tag ->
                TagChip(
                    tag = tag,
                    selected = false,
                    onClick = { onAddTag(tag) },
                )
            }
        }
    }

    if (selectedTags.isNotEmpty()) {
        Spacer(modifier = Modifier.height(PoziomkiTheme.spacing.sm))
        FlowRow(
            horizontalArrangement = Arrangement.spacedBy(PoziomkiTheme.spacing.sm),
            verticalArrangement = Arrangement.spacedBy(PoziomkiTheme.spacing.sm),
        ) {
            selectedTags.forEach { tag ->
                TagChip(
                    tag = tag,
                    selected = true,
                    onClick = { onRemoveTag(tag) },
                )
            }
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
        Box(
            modifier = Modifier.weight(1f).padding(vertical = 10.dp),
            contentAlignment = Alignment.CenterStart,
        ) {
            if (query.isEmpty()) {
                Text(
                    text = placeholder,
                    fontFamily = nunito,
                    color = TextMuted,
                    fontSize = 14.sp,
                )
            }
            BasicTextField(
                value = query,
                onValueChange = onQueryChange,
                textStyle =
                    androidx.compose.ui.text.TextStyle(
                        fontFamily = nunito,
                        fontWeight = FontWeight.Normal,
                        fontSize = 14.sp,
                        color = TextPrimary,
                    ),
                singleLine = true,
                cursorBrush = SolidColor(Primary),
                modifier = Modifier.fillMaxWidth(),
            )
        }
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
    val shape = RoundedCornerShape(8.dp)

    Row(
        modifier =
            Modifier
                .background(bgColor, shape)
                .border(1.dp, borderColor, shape)
                .clip(shape)
                .clickable(onClick = onClick)
                .padding(horizontal = 10.dp, vertical = 5.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Text(
            text = "${tag.emoji ?: ""} ${tag.name}".trim(),
            fontFamily = nunito,
            fontWeight = FontWeight.Medium,
            fontSize = 12.sp,
            color = textColor,
            lineHeight = 14.sp,
        )
        if (selected) {
            Spacer(modifier = Modifier.width(4.dp))
            Icon(
                Icons.Filled.Close,
                contentDescription = "Usuń",
                tint = textColor,
                modifier = Modifier.size(12.dp),
            )
        }
    }
}
