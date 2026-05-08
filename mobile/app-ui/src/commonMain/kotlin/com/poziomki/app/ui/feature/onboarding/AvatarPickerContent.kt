package com.poziomki.app.ui.feature.onboarding

import androidx.compose.foundation.Image
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Icon
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.adamglin.PhosphorIcons
import com.adamglin.phosphoricons.Bold
import com.adamglin.phosphoricons.bold.Camera
import com.adamglin.phosphoricons.bold.Plus
import com.adamglin.phosphoricons.bold.X
import com.poziomki.app.ui.designsystem.components.AppButton
import com.poziomki.app.ui.designsystem.components.ButtonVariant
import com.poziomki.app.ui.designsystem.theme.AppTheme
import com.poziomki.app.ui.designsystem.theme.MontserratFamily
import com.poziomki.app.ui.designsystem.theme.Overlay
import com.poziomki.app.ui.designsystem.theme.TextPrimary
import com.poziomki.app.ui.designsystem.theme.White
import com.poziomki.app.ui.shared.decodeImageBytes

@OptIn(ExperimentalLayoutApi::class)
@Composable
internal fun AvatarPickerContent(
    galleryImages: List<ByteArray>,
    onPickGalleryImages: () -> Unit,
    onRemoveGalleryImage: (Int) -> Unit,
    onPickAvatarImage: () -> Unit,
) {
    Column(
        modifier =
            Modifier
                .fillMaxWidth()
                .padding(horizontal = AppTheme.spacing.lg)
                .padding(bottom = AppTheme.spacing.xl),
    ) {
        // Photos section header
        Text(
            text = "twoje zdj\u0119cia (${galleryImages.size}/6)",
            fontFamily = MontserratFamily,
            fontWeight = FontWeight.ExtraBold,
            fontSize = 15.sp,
            color = TextPrimary,
        )

        // Photo thumbnails
        if (galleryImages.isNotEmpty()) {
            Spacer(modifier = Modifier.height(AppTheme.spacing.sm))
            FlowRow(
                horizontalArrangement = Arrangement.spacedBy(8.dp),
                verticalArrangement = Arrangement.spacedBy(8.dp),
            ) {
                galleryImages.forEachIndexed { index, imageBytes ->
                    PhotoThumbnail(
                        imageBytes = imageBytes,
                        onRemove = { onRemoveGalleryImage(index) },
                    )
                }
            }
        }

        Spacer(modifier = Modifier.height(AppTheme.spacing.md))

        // Add photos button
        if (galleryImages.size < 6) {
            AppButton(
                text = "dodaj zdj\u0119cia",
                onClick = onPickGalleryImages,
                icon = PhosphorIcons.Bold.Plus,
                modifier = Modifier.fillMaxWidth(),
            )
        }

        Spacer(modifier = Modifier.height(AppTheme.spacing.sm))

        // Take photo with camera
        AppButton(
            text = "zr\u00f3b zdj\u0119cie",
            onClick = onPickAvatarImage,
            icon = PhosphorIcons.Bold.Camera,
            variant = ButtonVariant.SECONDARY,
            modifier = Modifier.fillMaxWidth(),
        )
    }
}

@Composable
private fun PhotoThumbnail(
    imageBytes: ByteArray,
    onRemove: () -> Unit,
) {
    Box(
        modifier =
            Modifier
                .size(80.dp)
                .clip(RoundedCornerShape(12.dp)),
    ) {
        val bitmap = remember(imageBytes) { decodeImageBytes(imageBytes) }
        if (bitmap != null) {
            Image(
                bitmap = bitmap,
                contentDescription = null,
                modifier =
                    Modifier
                        .size(80.dp)
                        .clip(RoundedCornerShape(12.dp)),
                contentScale = ContentScale.Crop,
            )
        }
        Box(
            modifier =
                Modifier
                    .align(Alignment.TopEnd)
                    .padding(4.dp)
                    .size(20.dp)
                    .clip(CircleShape)
                    .background(Overlay)
                    .clickable(onClick = onRemove),
            contentAlignment = Alignment.Center,
        ) {
            Icon(
                imageVector = PhosphorIcons.Bold.X,
                contentDescription = "Usu\u0144",
                tint = White,
                modifier = Modifier.size(14.dp),
            )
        }
    }
}
