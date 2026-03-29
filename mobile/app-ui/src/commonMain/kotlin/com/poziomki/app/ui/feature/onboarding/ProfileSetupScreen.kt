package com.poziomki.app.ui.feature.onboarding

import androidx.compose.foundation.Image
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.BasicTextField
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.Text
import androidx.compose.material3.rememberModalBottomSheetState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.SolidColor
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.adamglin.PhosphorIcons
import com.adamglin.phosphoricons.Bold
import com.adamglin.phosphoricons.bold.ArrowUpRight
import com.adamglin.phosphoricons.bold.PencilSimple
import com.adamglin.phosphoricons.bold.User
import com.poziomki.app.ui.designsystem.components.AppButton
import com.poziomki.app.ui.designsystem.components.ButtonVariant
import com.poziomki.app.ui.designsystem.components.ConfirmDialog
import com.poziomki.app.ui.designsystem.components.OnboardingLayout
import com.poziomki.app.ui.designsystem.theme.AppTheme
import com.poziomki.app.ui.designsystem.theme.Black
import com.poziomki.app.ui.designsystem.theme.Border
import com.poziomki.app.ui.designsystem.theme.MontserratFamily
import com.poziomki.app.ui.designsystem.theme.NunitoFamily
import com.poziomki.app.ui.designsystem.theme.Primary
import com.poziomki.app.ui.designsystem.theme.Surface
import com.poziomki.app.ui.designsystem.theme.SurfaceElevated
import com.poziomki.app.ui.designsystem.theme.TextMuted
import com.poziomki.app.ui.designsystem.theme.TextPrimary
import com.poziomki.app.ui.designsystem.theme.TextSecondary
import com.poziomki.app.ui.shared.decodeImageBytes
import com.poziomki.app.ui.shared.rememberMultiImagePicker
import com.poziomki.app.ui.shared.rememberSingleImagePicker
import org.koin.compose.viewmodel.koinViewModel

private const val BIO_MAX_LENGTH = 300

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun ProfileSetupScreen(
    onComplete: () -> Unit,
    onBack: () -> Unit,
    viewModel: OnboardingViewModel = koinViewModel(),
) {
    val state by viewModel.state.collectAsState()
    var showAvatarPicker by remember { mutableStateOf(false) }
    var showProfilePreview by remember { mutableStateOf(false) }
    var showClearAllDialog by remember { mutableStateOf(false) }
    val sheetState = rememberModalBottomSheetState(skipPartiallyExpanded = true)

    val avatarImagePicker =
        rememberSingleImagePicker { bytes ->
            if (bytes != null) viewModel.setAvatarImage(bytes)
        }
    val galleryImagePicker =
        rememberMultiImagePicker { images ->
            if (images.isNotEmpty()) viewModel.addGalleryImages(images)
        }

    val displayAvatarBytes = state.avatarImageBytes ?: state.galleryImages.firstOrNull()
    val selectedTags = state.availableTags.filter { it.id in state.selectedTagIds }

    OnboardingLayout(
        currentStep = 3,
        totalSteps = 3,
        showBack = true,
        onBack = onBack,
        footer = {
            state.error?.let { error ->
                Text(
                    text = error,
                    fontFamily = NunitoFamily,
                    fontWeight = FontWeight.Medium,
                    fontSize = 14.sp,
                    color = MaterialTheme.colorScheme.error,
                    modifier = Modifier.padding(bottom = AppTheme.spacing.sm),
                )
            }
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.End,
            ) {
                AppButton(
                    text = "potwierd\u017a",
                    onClick = { viewModel.createProfile(onComplete) },
                    loading = state.isLoading,
                    variant = ButtonVariant.PRIMARY,
                )
            }
        },
    ) {
        ProfileSetupContent(
            state = state,
            displayAvatarBytes = displayAvatarBytes,
            onAvatarClick = { showAvatarPicker = true },
            onPreviewClick = { showProfilePreview = true },
            onBioChange = { viewModel.updateBio(it) },
        )
    }

    if (showAvatarPicker) {
        ModalBottomSheet(
            onDismissRequest = { showAvatarPicker = false },
            sheetState = sheetState,
            containerColor = SurfaceElevated,
            dragHandle = {
                Box(
                    modifier =
                        Modifier
                            .padding(top = 12.dp, bottom = 8.dp)
                            .width(40.dp)
                            .height(4.dp)
                            .clip(RoundedCornerShape(50))
                            .background(TextMuted),
                )
            },
        ) {
            AvatarPickerContent(
                galleryImages = state.galleryImages,
                onPickGalleryImages = { galleryImagePicker() },
                onRemoveGalleryImage = { viewModel.removeGalleryImage(it) },
                onPickAvatarImage = { avatarImagePicker() },
                onClearAll = { showClearAllDialog = true },
            )
        }
    }

    if (showProfilePreview) {
        ProfilePreviewDialog(
            name = state.name,
            program = state.program,
            bio = state.bio,
            tags = selectedTags,
            selectedAvatar = state.selectedAvatar,
            avatarImageBytes = state.avatarImageBytes,
            galleryImages = state.galleryImages,
            onDismiss = { showProfilePreview = false },
        )
    }

    if (showClearAllDialog) {
        ConfirmDialog(
            title = "wyczy\u015b\u0107 wszystko",
            message = "czy na pewno chcesz usun\u0105\u0107 avatar i wszystkie zdj\u0119cia?",
            confirmText = "wyczy\u015b\u0107",
            isDestructive = true,
            onConfirm = {
                viewModel.clearAll()
                showAvatarPicker = false
                showClearAllDialog = false
            },
            onDismiss = { showClearAllDialog = false },
        )
    }
}

@Composable
private fun ProfileSetupContent(
    state: OnboardingState,
    displayAvatarBytes: ByteArray?,
    onAvatarClick: () -> Unit,
    onPreviewClick: () -> Unit,
    onBioChange: (String) -> Unit,
) {
    Column(
        modifier =
            Modifier
                .padding(horizontal = AppTheme.spacing.lg)
                .padding(bottom = AppTheme.spacing.lg),
    ) {
        Text(
            text = "tw\u00f3j profil",
            style = MaterialTheme.typography.headlineMedium,
            color = TextPrimary,
        )

        Spacer(modifier = Modifier.height(AppTheme.spacing.lg))

        // Profile card — matching main app style
        ProfilePreviewCard(
            state = state,
            displayAvatarBytes = displayAvatarBytes,
            onAvatarClick = onAvatarClick,
            onPreviewClick = onPreviewClick,
        )

        Spacer(modifier = Modifier.height(AppTheme.spacing.lg))

        // Bio section
        Text(
            text = "bio",
            fontFamily = MontserratFamily,
            fontWeight = FontWeight.ExtraBold,
            fontSize = 16.sp,
            color = TextPrimary,
            modifier = Modifier.padding(start = 4.dp, bottom = 8.dp),
        )

        BioInput(bio = state.bio, onBioChange = onBioChange)
    }
}

@Composable
private fun ProfilePreviewCard(
    state: OnboardingState,
    displayAvatarBytes: ByteArray?,
    onAvatarClick: () -> Unit,
    onPreviewClick: () -> Unit,
) {
    val cardShape = RoundedCornerShape(20.dp)
    val backgroundBrush =
        Brush.linearGradient(
            colors = listOf(Color(0xFF161C26), Color(0xFF080B10)),
            start = Offset(0f, 0f),
            end = Offset(Float.POSITIVE_INFINITY, Float.POSITIVE_INFINITY),
        )

    Box(
        modifier =
            Modifier
                .fillMaxWidth()
                .clip(cardShape)
                .border(1.dp, Border, cardShape)
                .background(backgroundBrush)
                .clickable(onClick = onPreviewClick),
    ) {
        Row(verticalAlignment = Alignment.CenterVertically) {
            EditableAvatar(
                displayAvatarBytes = displayAvatarBytes,
                selectedAvatar = state.selectedAvatar,
                onClick = onAvatarClick,
            )
            Spacer(modifier = Modifier.width(12.dp))
            CardInfoColumn(
                name = state.name,
                program = state.program,
                modifier = Modifier.weight(1f),
            )
            Icon(
                PhosphorIcons.Bold.ArrowUpRight,
                contentDescription = null,
                modifier =
                    Modifier
                        .padding(top = 12.dp, end = 12.dp)
                        .size(20.dp)
                        .align(Alignment.Top),
                tint = TextMuted,
            )
        }
    }
}

@Composable
private fun CardInfoColumn(
    name: String,
    program: String,
    modifier: Modifier = Modifier,
) {
    Column(modifier = modifier.padding(vertical = 16.dp)) {
        Text(
            text = name.ifBlank { "imi\u0119" },
            fontFamily = MontserratFamily,
            fontWeight = FontWeight.ExtraBold,
            fontSize = 20.sp,
            color = TextPrimary,
        )
        if (program.isNotBlank()) {
            Spacer(modifier = Modifier.height(2.dp))
            Text(
                text = program,
                fontFamily = NunitoFamily,
                fontWeight = FontWeight.Normal,
                fontSize = 14.sp,
                color = TextSecondary,
            )
        }
    }
}

@Composable
private fun EditableAvatar(
    displayAvatarBytes: ByteArray?,
    selectedAvatar: String?,
    onClick: () -> Unit,
) {
    Box(
        modifier =
            Modifier
                .size(98.dp)
                .clickable(
                    interactionSource = remember { MutableInteractionSource() },
                    indication = null,
                    onClick = onClick,
                ),
        contentAlignment = Alignment.TopStart,
    ) {
        Box(
            modifier = Modifier.padding(start = 4.dp, top = 4.dp).size(90.dp),
            contentAlignment = Alignment.Center,
        ) {
            when {
                displayAvatarBytes != null -> {
                    val bitmap = remember(displayAvatarBytes) { decodeImageBytes(displayAvatarBytes) }
                    if (bitmap != null) {
                        Image(
                            bitmap = bitmap,
                            contentDescription = null,
                            modifier = Modifier.size(90.dp),
                            contentScale = ContentScale.Crop,
                        )
                    }
                }

                selectedAvatar != null -> {
                    Text(text = selectedAvatar, fontSize = 42.sp, textAlign = TextAlign.Center)
                }

                else -> {
                    Icon(PhosphorIcons.Bold.User, null, Modifier.size(42.dp), tint = TextMuted)
                }
            }
        }
        Box(
            modifier =
                Modifier
                    .offset(x = 74.dp, y = 74.dp)
                    .size(24.dp)
                    .clip(CircleShape)
                    .background(Primary),
            contentAlignment = Alignment.Center,
        ) {
            Icon(PhosphorIcons.Bold.PencilSimple, null, Modifier.size(14.dp), tint = Black)
        }
    }
}

@Composable
private fun BioInput(
    bio: String,
    onBioChange: (String) -> Unit,
) {
    Box(
        modifier =
            Modifier
                .fillMaxWidth()
                .height(120.dp)
                .background(Surface, RoundedCornerShape(AppTheme.radius.lg))
                .border(1.dp, Border, RoundedCornerShape(AppTheme.radius.lg))
                .padding(AppTheme.spacing.md),
    ) {
        BasicTextField(
            value = bio,
            onValueChange = { if (it.length <= BIO_MAX_LENGTH) onBioChange(it) },
            modifier = Modifier.fillMaxWidth(),
            textStyle =
                TextStyle(
                    fontFamily = NunitoFamily,
                    fontWeight = FontWeight.Normal,
                    fontSize = 16.sp,
                    color = TextPrimary,
                    lineHeight = 22.sp,
                ),
            cursorBrush = SolidColor(TextPrimary),
            decorationBox = { innerTextField ->
                Box {
                    if (bio.isEmpty()) {
                        Text(
                            text = "opowiedz co\u015b o sobie, swoich pasjach...",
                            fontFamily = NunitoFamily,
                            fontWeight = FontWeight.Normal,
                            fontSize = 16.sp,
                            color = TextMuted,
                            lineHeight = 22.sp,
                        )
                    }
                    innerTextField()
                }
            },
        )
    }

    Text(
        text = "${bio.length}/$BIO_MAX_LENGTH",
        fontFamily = NunitoFamily,
        fontWeight = FontWeight.Medium,
        fontSize = 12.sp,
        color = TextMuted,
        modifier =
            Modifier
                .fillMaxWidth()
                .padding(top = 4.dp, end = 4.dp),
        textAlign = TextAlign.End,
    )
}
