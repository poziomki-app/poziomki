package com.poziomki.app.ui.screen.onboarding

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
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Edit
import androidx.compose.material.icons.filled.OpenInFull
import androidx.compose.material.icons.filled.Person
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
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
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.SolidColor
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.poziomki.app.ui.component.OnboardingLayout
import com.poziomki.app.ui.component.PoziomkiButton
import com.poziomki.app.ui.theme.Border
import com.poziomki.app.ui.theme.NunitoFamily
import com.poziomki.app.ui.theme.PoziomkiTheme
import com.poziomki.app.ui.theme.Primary
import com.poziomki.app.ui.theme.PrimaryLight
import com.poziomki.app.ui.theme.Surface
import com.poziomki.app.ui.theme.SurfaceElevated
import com.poziomki.app.ui.theme.TextMuted
import com.poziomki.app.ui.theme.TextPrimary
import com.poziomki.app.ui.theme.TextSecondary
import com.poziomki.app.util.decodeImageBytes
import com.poziomki.app.util.rememberMultiImagePicker
import com.poziomki.app.util.rememberSingleImagePicker
import org.koin.compose.viewmodel.koinViewModel

private const val BIO_MAX_LENGTH = 300
private const val TAG_PREVIEW_LIMIT = 2

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun ProfileSetupScreen(
    onComplete: () -> Unit,
    onBack: () -> Unit,
    viewModel: OnboardingViewModel = koinViewModel(),
) {
    val state by viewModel.state.collectAsState()
    val nunito = NunitoFamily
    var showAvatarPicker by remember { mutableStateOf(false) }
    var showProfilePreview by remember { mutableStateOf(false) }
    val sheetState = rememberModalBottomSheetState(skipPartiallyExpanded = true)

    // Image pickers
    val avatarImagePicker =
        rememberSingleImagePicker { bytes ->
            if (bytes != null) {
                viewModel.setAvatarImage(bytes)
            }
        }
    val galleryImagePicker =
        rememberMultiImagePicker { images ->
            if (images.isNotEmpty()) {
                viewModel.addGalleryImages(images)
            }
        }

    // Get selected tags for preview
    val selectedTags = state.availableTags.filter { it.id in state.selectedTagIds }

    // Resolve the avatar image to display: explicit avatar > first gallery image > emoji > placeholder
    val displayAvatarBytes = state.avatarImageBytes ?: state.galleryImages.firstOrNull()

    OnboardingLayout(
        currentStep = 3,
        totalSteps = 3,
        showBack = true,
        onBack = onBack,
        footer = {
            if (state.error != null) {
                Text(
                    text = state.error!!,
                    fontFamily = nunito,
                    fontWeight = FontWeight.Medium,
                    fontSize = 14.sp,
                    color = MaterialTheme.colorScheme.error,
                    modifier = Modifier.padding(bottom = PoziomkiTheme.spacing.sm),
                )
            }
            PoziomkiButton(
                text = "potwierd\u017a",
                onClick = { viewModel.createProfile(onComplete) },
                loading = state.isLoading,
            )
        },
    ) {
        Column(
            modifier =
                Modifier
                    .padding(horizontal = PoziomkiTheme.spacing.lg)
                    .padding(bottom = PoziomkiTheme.spacing.lg),
        ) {
            Text(
                text = "tw\u00f3j profil",
                style = MaterialTheme.typography.headlineMedium,
                color = TextPrimary,
            )

            Spacer(modifier = Modifier.height(PoziomkiTheme.spacing.lg))

            // Profile preview card
            Row(
                modifier =
                    Modifier
                        .fillMaxWidth()
                        .background(SurfaceElevated, RoundedCornerShape(PoziomkiTheme.radius.lg))
                        .border(
                            width = 1.5.dp,
                            color = PrimaryLight,
                            shape = RoundedCornerShape(PoziomkiTheme.radius.lg),
                        ).padding(PoziomkiTheme.spacing.md),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                // Avatar — no clip on outer box so edit badge isn't cropped
                Box(
                    modifier =
                        Modifier
                            .size(80.dp) // slightly larger to fit badge
                            .clickable(
                                interactionSource = remember { MutableInteractionSource() },
                                indication = null,
                            ) { showAvatarPicker = true },
                    contentAlignment = Alignment.TopStart,
                ) {
                    Box(
                        modifier =
                            Modifier
                                .size(72.dp)
                                .clip(RoundedCornerShape(16.dp))
                                .background(Surface),
                        contentAlignment = Alignment.Center,
                    ) {
                        when {
                            displayAvatarBytes != null -> {
                                val bitmap =
                                    remember(displayAvatarBytes) {
                                        decodeImageBytes(displayAvatarBytes)
                                    }
                                if (bitmap != null) {
                                    Image(
                                        bitmap = bitmap,
                                        contentDescription = null,
                                        modifier =
                                            Modifier
                                                .size(72.dp)
                                                .clip(RoundedCornerShape(16.dp)),
                                        contentScale = ContentScale.Crop,
                                    )
                                }
                            }

                            state.selectedAvatar != null -> {
                                Text(
                                    text = state.selectedAvatar!!,
                                    fontSize = 36.sp,
                                    textAlign = TextAlign.Center,
                                )
                            }

                            else -> {
                                Icon(
                                    imageVector = Icons.Filled.Person,
                                    contentDescription = null,
                                    modifier = Modifier.size(36.dp),
                                    tint = TextMuted,
                                )
                            }
                        }
                    }
                    // Cyan edit badge — positioned at bottom-end of 72dp avatar
                    Box(
                        modifier =
                            Modifier
                                .offset(x = 60.dp, y = 60.dp)
                                .size(24.dp)
                                .clip(CircleShape)
                                .background(Primary),
                        contentAlignment = Alignment.Center,
                    ) {
                        Icon(
                            imageVector = Icons.Filled.Edit,
                            contentDescription = "Zmie\u0144 avatar",
                            tint = Color.Black,
                            modifier = Modifier.size(14.dp),
                        )
                    }
                }

                Spacer(modifier = Modifier.width(PoziomkiTheme.spacing.sm))

                Column(modifier = Modifier.weight(1f)) {
                    Text(
                        text = state.name.ifBlank { "imi\u0119" },
                        style = MaterialTheme.typography.titleMedium,
                        color = TextPrimary,
                    )

                    if (selectedTags.isNotEmpty()) {
                        Spacer(modifier = Modifier.height(6.dp))
                        Row(
                            horizontalArrangement = Arrangement.spacedBy(4.dp),
                            verticalAlignment = Alignment.CenterVertically,
                        ) {
                            selectedTags.take(TAG_PREVIEW_LIMIT).forEach { tag ->
                                MiniTagChip(label = tag.name)
                            }
                            val remaining = selectedTags.size - TAG_PREVIEW_LIMIT
                            if (remaining > 0) {
                                Text(
                                    text = "+$remaining",
                                    fontFamily = nunito,
                                    fontWeight = FontWeight.Medium,
                                    fontSize = 12.sp,
                                    color = TextMuted,
                                )
                            }
                        }
                    } else if (state.program.isNotBlank()) {
                        Text(
                            text = state.program,
                            fontFamily = nunito,
                            fontWeight = FontWeight.Normal,
                            fontSize = 14.sp,
                            color = TextSecondary,
                        )
                    }
                }

                // Expand icon — opens full profile preview
                IconButton(
                    onClick = { showProfilePreview = true },
                    modifier = Modifier.size(36.dp),
                ) {
                    Icon(
                        imageVector = Icons.Filled.OpenInFull,
                        contentDescription = "Podgl\u0105d profilu",
                        tint = TextMuted,
                        modifier = Modifier.size(20.dp),
                    )
                }
            }

            Spacer(modifier = Modifier.height(PoziomkiTheme.spacing.lg))

            // Bio section
            Text(
                text = "bio",
                fontFamily = nunito,
                fontWeight = FontWeight.Bold,
                fontSize = 16.sp,
                color = TextPrimary,
                modifier = Modifier.padding(start = 4.dp, bottom = 8.dp),
            )

            Box(
                modifier =
                    Modifier
                        .fillMaxWidth()
                        .height(120.dp)
                        .background(Surface, RoundedCornerShape(PoziomkiTheme.radius.lg))
                        .border(1.dp, Border, RoundedCornerShape(PoziomkiTheme.radius.lg))
                        .padding(PoziomkiTheme.spacing.md),
            ) {
                BasicTextField(
                    value = state.bio,
                    onValueChange = { if (it.length <= BIO_MAX_LENGTH) viewModel.updateBio(it) },
                    modifier = Modifier.fillMaxWidth(),
                    textStyle =
                        TextStyle(
                            fontFamily = nunito,
                            fontWeight = FontWeight.Normal,
                            fontSize = 16.sp,
                            color = TextPrimary,
                            lineHeight = 22.sp,
                        ),
                    cursorBrush = SolidColor(TextPrimary),
                    decorationBox = { innerTextField ->
                        Box {
                            if (state.bio.isEmpty()) {
                                Text(
                                    text = "opowiedz co\u015b o sobie, swoich pasjach, co lubisz robi\u0107 w wolnym czasie...",
                                    fontFamily = nunito,
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

            // Character counter
            Text(
                text = "${state.bio.length}/$BIO_MAX_LENGTH",
                fontFamily = nunito,
                fontWeight = FontWeight.Medium,
                fontSize = 12.sp,
                color = TextMuted,
                modifier =
                    Modifier
                        .align(Alignment.End)
                        .padding(top = 4.dp, end = 4.dp),
            )
        }
    }

    // Avatar / photos picker bottom sheet
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
                selectedAvatar = state.selectedAvatar,
                galleryImages = state.galleryImages,
                onPickGalleryImages = {
                    galleryImagePicker()
                },
                onRemoveGalleryImage = { index ->
                    viewModel.removeGalleryImage(index)
                },
                onSelectAvatar = { emoji ->
                    viewModel.selectAvatar(emoji)
                    showAvatarPicker = false
                },
                onClearAll = {
                    viewModel.clearAll()
                    showAvatarPicker = false
                },
                hasContent = state.selectedAvatar != null || state.avatarImageBytes != null || state.galleryImages.isNotEmpty(),
            )
        }
    }

    // Profile preview dialog
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
}

@Composable
private fun MiniTagChip(
    label: String,
    modifier: Modifier = Modifier,
) {
    Text(
        text = label,
        fontFamily = NunitoFamily,
        fontWeight = FontWeight.Medium,
        fontSize = 11.sp,
        color = TextSecondary,
        modifier =
            modifier
                .border(1.dp, Border, RoundedCornerShape(50))
                .padding(horizontal = 6.dp, vertical = 1.dp),
    )
}
