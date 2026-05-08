package com.poziomki.app.ui.feature.profile

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
import androidx.compose.foundation.layout.defaultMinSize
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.imePadding
import androidx.compose.foundation.layout.navigationBars
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.statusBars
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.BasicTextField
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
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
import androidx.compose.ui.draw.drawBehind
import androidx.compose.ui.focus.FocusRequester
import androidx.compose.ui.focus.focusRequester
import androidx.compose.ui.geometry.CornerRadius
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.PathEffect
import androidx.compose.ui.graphics.SolidColor
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.text.AnnotatedString
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.OffsetMapping
import androidx.compose.ui.text.input.TransformedText
import androidx.compose.ui.text.input.VisualTransformation
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.compose.ui.window.Dialog
import androidx.compose.ui.window.DialogProperties
import coil3.compose.AsyncImage
import com.adamglin.PhosphorIcons
import com.adamglin.phosphoricons.Bold
import com.adamglin.phosphoricons.bold.Image
import com.adamglin.phosphoricons.bold.MagnifyingGlass
import com.adamglin.phosphoricons.bold.PencilSimple
import com.adamglin.phosphoricons.bold.Plus
import com.adamglin.phosphoricons.bold.SlidersHorizontal
import com.adamglin.phosphoricons.bold.X
import com.poziomki.app.network.Tag
import com.poziomki.app.ui.designsystem.components.AppButton
import com.poziomki.app.ui.designsystem.components.AppSnackbar
import com.poziomki.app.ui.designsystem.components.ButtonVariant
import com.poziomki.app.ui.designsystem.components.PoziomkiTextField
import com.poziomki.app.ui.designsystem.components.ScreenHeader
import com.poziomki.app.ui.designsystem.components.SectionLabel
import com.poziomki.app.ui.designsystem.theme.Background
import com.poziomki.app.ui.designsystem.theme.Border
import com.poziomki.app.ui.designsystem.theme.MontserratFamily
import com.poziomki.app.ui.designsystem.theme.NunitoFamily
import com.poziomki.app.ui.designsystem.theme.Overlay
import com.poziomki.app.ui.designsystem.theme.PoziomkiTheme
import com.poziomki.app.ui.designsystem.theme.Primary
import com.poziomki.app.ui.designsystem.theme.PrimaryLight
import com.poziomki.app.ui.designsystem.theme.TextMuted
import com.poziomki.app.ui.designsystem.theme.TextPrimary
import com.poziomki.app.ui.designsystem.theme.TextSecondary
import com.poziomki.app.ui.designsystem.theme.White
import com.poziomki.app.ui.shared.rememberSingleImagePicker
import com.poziomki.app.ui.shared.resolveImageUrl
import org.koin.compose.viewmodel.koinViewModel
import com.poziomki.app.ui.designsystem.theme.Surface as SurfaceColor

@Composable
fun ProfileEditScreen(
    onBack: () -> Unit,
    viewModel: ProfileEditViewModel = koinViewModel(),
) {
    val state by viewModel.state.collectAsState()
    val nunito = NunitoFamily
    var showGradientPicker by remember { mutableStateOf(false) }
    var showBioEditor by remember { mutableStateOf(false) }

    val imagePicker =
        rememberSingleImagePicker { bytes ->
            if (bytes != null) {
                viewModel.uploadAndAddImage(bytes)
            }
        }

    val bioImagePicker =
        rememberSingleImagePicker { bytes ->
            if (bytes != null) {
                viewModel.uploadBioImage(bytes)
            }
        }

    val navBarBottom = WindowInsets.navigationBars.asPaddingValues().calculateBottomPadding()

    Box(modifier = Modifier.fillMaxSize()) {
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

                // --- imię ---
                SectionLabel("imię")

                PoziomkiTextField(
                    value = state.name,
                    onValueChange = { viewModel.updateName(it) },
                    placeholder = "imię",
                    modifier = Modifier.fillMaxWidth(),
                )

                Spacer(modifier = Modifier.height(PoziomkiTheme.spacing.lg))

                // --- bio ---
                SectionLabel("bio")

                Box(
                    modifier =
                        Modifier
                            .fillMaxWidth()
                            .defaultMinSize(minHeight = 48.dp)
                            .background(SurfaceColor, RoundedCornerShape(PoziomkiTheme.radius.md))
                            .border(1.dp, Border, RoundedCornerShape(PoziomkiTheme.radius.md))
                            .clip(RoundedCornerShape(PoziomkiTheme.radius.md))
                            .clickable { showBioEditor = true }
                            .padding(horizontal = 16.dp, vertical = 12.dp),
                ) {
                    if (state.bio.isBlank()) {
                        Text(
                            text = "napisz coś o sobie...",
                            fontFamily = nunito,
                            fontWeight = FontWeight.Normal,
                            fontSize = 16.sp,
                            color = TextMuted,
                        )
                    } else {
                        val bioImageRegex = remember { Regex("""!\[\]\((.*?)\)""") }
                        val textOnly =
                            remember(state.bio) {
                                state.bio.replace(bioImageRegex, "").trim()
                            }
                        val imageUrls =
                            remember(state.bio) {
                                bioImageRegex.findAll(state.bio).map { it.groupValues[1] }.toList()
                            }
                        Column(verticalArrangement = Arrangement.spacedBy(6.dp)) {
                            if (textOnly.isNotEmpty()) {
                                Text(
                                    text = textOnly,
                                    fontFamily = nunito,
                                    fontWeight = FontWeight.Normal,
                                    fontSize = 16.sp,
                                    color = TextPrimary,
                                    maxLines = 3,
                                    overflow = TextOverflow.Ellipsis,
                                )
                            }
                            if (imageUrls.isNotEmpty()) {
                                Row(horizontalArrangement = Arrangement.spacedBy(6.dp)) {
                                    imageUrls.take(4).forEach { url ->
                                        AsyncImage(
                                            model = resolveImageUrl(url),
                                            contentDescription = null,
                                            modifier =
                                                Modifier
                                                    .size(48.dp)
                                                    .clip(RoundedCornerShape(8.dp)),
                                            contentScale = ContentScale.Crop,
                                        )
                                    }
                                }
                            }
                        }
                    }
                }

                Spacer(modifier = Modifier.height(PoziomkiTheme.spacing.lg))

                // --- kolor profilu ---
                SectionLabel("kolor profilu")

                GradientCircle(
                    gradientStart = state.gradientStart,
                    gradientEnd = state.gradientEnd,
                    size = 28.dp,
                    onClick = { showGradientPicker = true },
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
                            PhosphorIcons.Bold.X,
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
                    searchResults = state.interestSearchResults,
                    isSearching = state.isSearchingInterests,
                    selectedTags = state.selectedTags.filter { it.scope == "interest" },
                    onAddTag = {
                        viewModel.addTag(it)
                        viewModel.updateInterestQuery("")
                    },
                    onRemoveTag = { viewModel.removeTag(it) },
                    onCreateTag = { name -> viewModel.createAndAddTag(name, "interest") },
                    isCreatingTag = state.isCreatingTag,
                )

                Spacer(modifier = Modifier.height(PoziomkiTheme.spacing.lg))

                // --- aktywności ---
                TagSection(
                    label = "aktywności",
                    query = state.activityQuery,
                    onQueryChange = { viewModel.updateActivityQuery(it) },
                    searchPlaceholder = "szukaj aktywności...",
                    searchResults = state.activitySearchResults,
                    isSearching = state.isSearchingActivities,
                    selectedTags = state.selectedTags.filter { it.scope == "activity" },
                    onAddTag = {
                        viewModel.addTag(it)
                        viewModel.updateActivityQuery("")
                    },
                    onRemoveTag = { viewModel.removeTag(it) },
                    onCreateTag = { name -> viewModel.createAndAddTag(name, "activity") },
                    isCreatingTag = state.isCreatingTag,
                )

                Spacer(modifier = Modifier.height(PoziomkiTheme.spacing.xl))

                // Save button
                Box(modifier = Modifier.fillMaxWidth(), contentAlignment = Alignment.CenterEnd) {
                    AppButton(
                        text = "zapisz",
                        onClick = { viewModel.save(onBack) },
                        variant = ButtonVariant.PRIMARY,
                        loading = state.isSaving,
                    )
                }

                Spacer(modifier = Modifier.height(navBarBottom + PoziomkiTheme.spacing.xl))
            }
        }

        // Snackbar for save/upload errors
        state.snackbarMessage?.let { message ->
            AppSnackbar(
                message = message,
                type = state.snackbarType,
                modifier =
                    Modifier
                        .align(Alignment.BottomCenter)
                        .padding(PoziomkiTheme.spacing.md),
            )
            LaunchedEffect(message) {
                kotlinx.coroutines.delay(3000)
                viewModel.clearSnackbar()
            }
        }
    }

    if (showBioEditor) {
        BioEditorDialog(
            bio = state.bio,
            isBioImageUploading = state.isBioImageUploading,
            onBioChange = { viewModel.updateBio(it.take(1500)) },
            onAddImage = { bioImagePicker() },
            onDismiss = { showBioEditor = false },
        )
    }

    if (showGradientPicker) {
        GradientPickerDialog(
            name = state.name,
            program = state.program,
            bio = state.bio,
            initialStart = state.gradientStart,
            initialEnd = state.gradientEnd,
            onSave = { start, end ->
                viewModel.updateGradient(start, end)
                showGradientPicker = false
            },
            onDismiss = { showGradientPicker = false },
        )
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
                        PhosphorIcons.Bold.X,
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
                    PhosphorIcons.Bold.Plus,
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
                PhosphorIcons.Bold.PencilSimple,
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
    searchResults: List<Tag>,
    isSearching: Boolean,
    selectedTags: List<Tag>,
    onAddTag: (Tag) -> Unit,
    onRemoveTag: (Tag) -> Unit,
    onCreateTag: ((String) -> Unit)? = null,
    isCreatingTag: Boolean = false,
) {
    val nunito = NunitoFamily
    val filtered =
        searchResults.filter { tag ->
            selectedTags.none { it.id == tag.id }
        }
    val trimmedQuery = query.trim()
    val hasExactMatch =
        searchResults.any { it.name.equals(trimmedQuery, ignoreCase = true) } ||
            selectedTags.any { it.name.equals(trimmedQuery, ignoreCase = true) }

    SectionLabel(label)

    TagSearchBar(
        query = query,
        onQueryChange = onQueryChange,
        placeholder = searchPlaceholder,
    )

    if (query.isNotBlank()) {
        Spacer(modifier = Modifier.height(PoziomkiTheme.spacing.sm))

        if (isSearching) {
            CircularProgressIndicator(
                color = Primary,
                modifier = Modifier.size(20.dp),
                strokeWidth = 2.dp,
            )
        } else {
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

                // "Create new" option when no exact match
                if (onCreateTag != null && trimmedQuery.length >= 2 && !hasExactMatch) {
                    CreateTagChip(
                        name = trimmedQuery,
                        isCreating = isCreatingTag,
                        onClick = { onCreateTag(trimmedQuery) },
                    )
                }
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
private fun CreateTagChip(
    name: String,
    isCreating: Boolean,
    onClick: () -> Unit,
) {
    val nunito = NunitoFamily
    val shape = RoundedCornerShape(8.dp)

    Row(
        modifier =
            Modifier
                .background(Color.Transparent, shape)
                .border(1.dp, Primary, shape)
                .clip(shape)
                .clickable(enabled = !isCreating, onClick = onClick)
                .padding(horizontal = 10.dp, vertical = 5.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        if (isCreating) {
            CircularProgressIndicator(
                color = Primary,
                modifier = Modifier.size(12.dp),
                strokeWidth = 1.5.dp,
            )
        } else {
            Icon(
                PhosphorIcons.Bold.Plus,
                contentDescription = null,
                tint = Primary,
                modifier = Modifier.size(12.dp),
            )
        }
        Spacer(modifier = Modifier.width(4.dp))
        Text(
            text = "dodaj \"$name\"",
            fontFamily = nunito,
            fontWeight = FontWeight.Medium,
            fontSize = 12.sp,
            color = Primary,
            lineHeight = 14.sp,
        )
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
            PhosphorIcons.Bold.MagnifyingGlass,
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
            PhosphorIcons.Bold.SlidersHorizontal,
            contentDescription = null,
            tint = TextMuted,
            modifier = Modifier.size(20.dp),
        )
    }
}

private data class GradientPreset(
    val start: String,
    val end: String,
    val label: String,
)

private val gradientPresets =
    listOf(
        GradientPreset("FF6B6B", "FFA07A", "sunset"),
        GradientPreset("00F5FF", "FF00FF", "neon"),
        GradientPreset("667EEA", "764BA2", "ocean"),
        GradientPreset("00B09B", "96C93D", "mint"),
        GradientPreset("FF416C", "FF4B2B", "fire"),
        GradientPreset("C471F5", "FA71CD", "lavender"),
        GradientPreset("2193B0", "6DD5ED", "sky"),
        GradientPreset("F093FB", "F5576C", "coral"),
        GradientPreset("0F2027", "2C5364", "night"),
        GradientPreset("FFD89B", "19547B", "peach"),
        GradientPreset("8E2DE2", "4A00E0", "berry"),
        GradientPreset("FC5C7D", "6A82FB", "candy"),
        GradientPreset("FF0080", "7928CA", "ultraviolet"),
        GradientPreset("F7971E", "FFD200", "golden"),
        GradientPreset("00C9FF", "92FE9D", "aurora"),
        GradientPreset("FC466B", "3F5EFB", "electric"),
        GradientPreset("3A1C71", "D76D77", "cosmic"),
        GradientPreset("11998E", "38EF7D", "emerald"),
        GradientPreset("FDC830", "F37335", "citrus"),
        GradientPreset("C33764", "1D2671", "twilight"),
        GradientPreset("ED4264", "FFEDBC", "flamingo"),
        GradientPreset("654EA3", "EAAFC8", "dream"),
        GradientPreset("F953C6", "B91D73", "magenta"),
        GradientPreset("00B4DB", "0083B0", "arctic"),
    )

private fun parseHex(hex: String): Color = runCatching { Color(("FF$hex").toLong(16).toInt()) }.getOrDefault(Color.Gray)

private object BioImageVisualTransformation : VisualTransformation {
    private val imageRegex = Regex("""!\[\]\([^)]*\)""")
    private const val PLACEHOLDER = " "

    override fun filter(text: AnnotatedString): TransformedText {
        val src = text.text
        val matches = imageRegex.findAll(src).toList()
        if (matches.isEmpty()) return TransformedText(text, OffsetMapping.Identity)

        val sb = StringBuilder()
        var srcPos = 0
        val o2t = IntArray(src.length + 1)

        for (match in matches) {
            while (srcPos < match.range.first) {
                o2t[srcPos] = sb.length
                sb.append(src[srcPos])
                srcPos++
            }
            val pStart = sb.length
            sb.append(PLACEHOLDER)
            for (i in match.range) {
                o2t[i] = pStart
            }
            srcPos = match.range.last + 1
        }
        while (srcPos < src.length) {
            o2t[srcPos] = sb.length
            sb.append(src[srcPos])
            srcPos++
        }
        o2t[src.length] = sb.length

        val transformed = sb.toString()
        val tLen = transformed.length
        val t2o = IntArray(tLen + 1)
        var oSearch = 0
        for (tIdx in 0..tLen) {
            while (oSearch < src.length && o2t[oSearch] < tIdx) oSearch++
            t2o[tIdx] = oSearch
        }

        return TransformedText(
            AnnotatedString(transformed),
            object : OffsetMapping {
                override fun originalToTransformed(offset: Int) = o2t[offset.coerceIn(0, src.length)]

                override fun transformedToOriginal(offset: Int) = t2o[offset.coerceIn(0, tLen)]
            },
        )
    }
}

@Composable
private fun BioEditorDialog(
    bio: String,
    isBioImageUploading: Boolean,
    onBioChange: (String) -> Unit,
    onAddImage: () -> Unit,
    onDismiss: () -> Unit,
) {
    val nunito = NunitoFamily
    val focusRequester = remember { FocusRequester() }

    LaunchedEffect(Unit) {
        focusRequester.requestFocus()
    }

    Dialog(
        onDismissRequest = onDismiss,
        properties = DialogProperties(usePlatformDefaultWidth = false, decorFitsSystemWindows = false),
    ) {
        Box(
            modifier =
                Modifier
                    .fillMaxSize()
                    .imePadding()
                    .background(Background),
        ) {
            Column(
                modifier = Modifier.fillMaxSize(),
            ) {
                // Top bar
                Spacer(modifier = Modifier.height(WindowInsets.statusBars.asPaddingValues().calculateTopPadding()))
                ScreenHeader(
                    title = "bio",
                    onBack = onDismiss,
                )

                // Text area — fills available space
                BasicTextField(
                    value = bio,
                    onValueChange = onBioChange,
                    modifier =
                        Modifier
                            .fillMaxWidth()
                            .weight(1f)
                            .focusRequester(focusRequester)
                            .padding(horizontal = PoziomkiTheme.spacing.lg, vertical = PoziomkiTheme.spacing.md),
                    textStyle =
                        androidx.compose.ui.text.TextStyle(
                            fontFamily = nunito,
                            fontWeight = FontWeight.Normal,
                            fontSize = 16.sp,
                            color = TextPrimary,
                            lineHeight = 24.sp,
                        ),
                    cursorBrush = SolidColor(Primary),
                    visualTransformation = BioImageVisualTransformation,
                    decorationBox = { innerTextField ->
                        Box {
                            if (bio.isEmpty()) {
                                Text(
                                    text = "napisz coś o sobie...",
                                    fontFamily = nunito,
                                    fontWeight = FontWeight.Normal,
                                    fontSize = 16.sp,
                                    color = TextMuted,
                                )
                            }
                            innerTextField()
                        }
                    },
                )

                // Image previews
                val bioImageMatches =
                    remember(bio) {
                        Regex("""!\[\]\((.*?)\)""").findAll(bio).toList()
                    }
                if (bioImageMatches.isNotEmpty()) {
                    Row(
                        modifier =
                            Modifier
                                .fillMaxWidth()
                                .horizontalScroll(rememberScrollState())
                                .padding(horizontal = PoziomkiTheme.spacing.md, vertical = 6.dp),
                        horizontalArrangement = Arrangement.spacedBy(8.dp),
                    ) {
                        bioImageMatches.forEachIndexed { index, match ->
                            val url = match.groupValues[1]
                            Box(modifier = Modifier.size(72.dp)) {
                                AsyncImage(
                                    model = resolveImageUrl(url),
                                    contentDescription = "Zdjęcie ${index + 1}",
                                    modifier =
                                        Modifier
                                            .fillMaxSize()
                                            .clip(RoundedCornerShape(10.dp)),
                                    contentScale = ContentScale.Crop,
                                )
                                Surface(
                                    modifier =
                                        Modifier
                                            .align(Alignment.TopEnd)
                                            .padding(2.dp)
                                            .size(22.dp)
                                            .clickable {
                                                val start = match.range.first
                                                var end = match.range.last + 1
                                                if (end < bio.length && bio[end] == '\n') end++
                                                onBioChange(bio.removeRange(start, end))
                                            },
                                    shape = CircleShape,
                                    color = Color.Black.copy(alpha = 0.6f),
                                ) {
                                    Box(contentAlignment = Alignment.Center) {
                                        Icon(
                                            PhosphorIcons.Bold.X,
                                            contentDescription = "Usuń zdjęcie",
                                            tint = Color.White,
                                            modifier = Modifier.size(14.dp),
                                        )
                                    }
                                }
                            }
                        }
                    }
                }

                // Bottom toolbar — image button + char counter
                Row(
                    modifier =
                        Modifier
                            .fillMaxWidth()
                            .background(SurfaceColor)
                            .padding(horizontal = PoziomkiTheme.spacing.md, vertical = 8.dp),
                    verticalAlignment = Alignment.CenterVertically,
                ) {
                    if (isBioImageUploading) {
                        CircularProgressIndicator(
                            color = Primary,
                            modifier = Modifier.size(20.dp),
                            strokeWidth = 2.dp,
                        )
                    } else {
                        Icon(
                            PhosphorIcons.Bold.Image,
                            contentDescription = "Dodaj zdjęcie",
                            tint = TextMuted,
                            modifier =
                                Modifier
                                    .size(20.dp)
                                    .clickable(onClick = onAddImage),
                        )
                    }
                    Spacer(modifier = Modifier.weight(1f))
                    val visibleLength =
                        remember(bio) {
                            bio.replace(Regex("""!\[\]\([^)]*\)"""), "").length
                        }
                    Text(
                        text = "$visibleLength/1500",
                        fontFamily = nunito,
                        fontWeight = FontWeight.Normal,
                        fontSize = 12.sp,
                        color =
                            if (visibleLength > 1400) {
                                com.poziomki.app.ui.designsystem.theme.Error
                            } else {
                                TextMuted
                            },
                    )
                    AppButton(
                        text = "zapisz",
                        onClick = onDismiss,
                        variant = ButtonVariant.PRIMARY,
                        modifier = Modifier.padding(start = 12.dp),
                    )
                }
            }
        }
    }
}

@Composable
private fun GradientCircle(
    gradientStart: String?,
    gradientEnd: String?,
    size: androidx.compose.ui.unit.Dp,
    isSelected: Boolean = false,
    onClick: () -> Unit,
) {
    val hasGradient = gradientStart != null && gradientEnd != null
    val bgModifier =
        if (gradientStart != null && gradientEnd != null) {
            Modifier.background(
                Brush.linearGradient(
                    colors = listOf(parseHex(gradientStart), parseHex(gradientEnd)),
                ),
                CircleShape,
            )
        } else {
            Modifier.background(SurfaceColor, CircleShape)
        }

    Box(
        modifier =
            Modifier
                .size(size)
                .clip(CircleShape)
                .border(
                    if (isSelected) 2.dp else 1.dp,
                    if (isSelected) Primary else Border,
                    CircleShape,
                ).then(bgModifier)
                .clickable(onClick = onClick),
        contentAlignment = Alignment.Center,
    ) {
        if (!hasGradient) {
            Text(
                text = "—",
                fontFamily = NunitoFamily,
                fontWeight = FontWeight.Medium,
                fontSize = (size.value * 0.3f).sp,
                color = TextMuted,
            )
        }
    }
}

private fun blendWithBg(
    color: Color,
    amount: Float,
): Color =
    Color(
        red = Background.red * (1f - amount) + color.red * amount,
        green = Background.green * (1f - amount) + color.green * amount,
        blue = Background.blue * (1f - amount) + color.blue * amount,
        alpha = 1f,
    )

@OptIn(ExperimentalLayoutApi::class)
@Composable
private fun GradientPickerDialog(
    name: String,
    program: String,
    bio: String,
    initialStart: String?,
    initialEnd: String?,
    onSave: (String?, String?) -> Unit,
    onDismiss: () -> Unit,
) {
    val nunito = NunitoFamily
    val montserrat = MontserratFamily

    var selectedStart by remember { mutableStateOf(initialStart) }
    var selectedEnd by remember { mutableStateOf(initialEnd) }

    // Build darkened preview background from currently selected gradient
    val previewStart = selectedStart?.let { blendWithBg(parseHex(it), 0.18f) }
    val previewEnd = selectedEnd?.let { blendWithBg(parseHex(it), 0.18f) }
    val previewBg =
        if (previewStart != null && previewEnd != null) {
            Modifier.background(
                Brush.verticalGradient(colors = listOf(previewStart, previewEnd)),
                RoundedCornerShape(20.dp),
            )
        } else {
            Modifier.background(SurfaceColor, RoundedCornerShape(20.dp))
        }

    val navBarBottom = WindowInsets.navigationBars.asPaddingValues().calculateBottomPadding()

    Dialog(
        onDismissRequest = onDismiss,
        properties = DialogProperties(usePlatformDefaultWidth = false, decorFitsSystemWindows = false),
    ) {
        Column(
            modifier =
                Modifier
                    .fillMaxSize()
                    .background(Background)
                    .padding(horizontal = PoziomkiTheme.spacing.lg),
        ) {
            // Top bar with close
            Spacer(modifier = Modifier.height(WindowInsets.statusBars.asPaddingValues().calculateTopPadding()))
            ScreenHeader(
                title = "kolor profilu",
                onBack = onDismiss,
            )

            Spacer(modifier = Modifier.height(PoziomkiTheme.spacing.md))

            // Mini profile preview
            Box(
                modifier =
                    Modifier
                        .fillMaxWidth()
                        .then(previewBg)
                        .padding(PoziomkiTheme.spacing.lg),
            ) {
                Column {
                    Text(
                        text = name.ifBlank { "imię" },
                        fontFamily = montserrat,
                        fontWeight = FontWeight.ExtraBold,
                        fontSize = 22.sp,
                        color = TextPrimary,
                    )
                    if (program.isNotBlank()) {
                        Text(
                            text = program,
                            fontFamily = nunito,
                            fontWeight = FontWeight.Normal,
                            fontSize = 14.sp,
                            color = TextSecondary,
                        )
                    }
                    if (bio.isNotBlank()) {
                        Spacer(modifier = Modifier.height(4.dp))
                        Text(
                            text = bio,
                            fontFamily = nunito,
                            fontWeight = FontWeight.Normal,
                            fontSize = 14.sp,
                            color = TextSecondary,
                            maxLines = 3,
                            overflow = TextOverflow.Ellipsis,
                        )
                    }
                }
            }

            Spacer(modifier = Modifier.height(PoziomkiTheme.spacing.lg))

            // Circles
            val dotSize = 40.dp

            FlowRow(
                horizontalArrangement = Arrangement.spacedBy(16.dp),
                verticalArrangement = Arrangement.spacedBy(16.dp),
            ) {
                // "brak" option
                GradientCircle(
                    gradientStart = null,
                    gradientEnd = null,
                    size = dotSize,
                    isSelected = selectedStart == null && selectedEnd == null,
                    onClick = {
                        selectedStart = null
                        selectedEnd = null
                    },
                )

                gradientPresets.forEach { preset ->
                    GradientCircle(
                        gradientStart = preset.start,
                        gradientEnd = preset.end,
                        size = dotSize,
                        isSelected = selectedStart == preset.start && selectedEnd == preset.end,
                        onClick = {
                            selectedStart = preset.start
                            selectedEnd = preset.end
                        },
                    )
                }
            }

            Spacer(modifier = Modifier.weight(1f))

            // Save button
            AppButton(
                text = "zapisz",
                onClick = { onSave(selectedStart, selectedEnd) },
                variant = ButtonVariant.PRIMARY,
            )

            Spacer(modifier = Modifier.height(navBarBottom + PoziomkiTheme.spacing.xl))
        }
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
                PhosphorIcons.Bold.X,
                contentDescription = "Usuń",
                tint = textColor,
                modifier = Modifier.size(12.dp),
            )
        }
    }
}
