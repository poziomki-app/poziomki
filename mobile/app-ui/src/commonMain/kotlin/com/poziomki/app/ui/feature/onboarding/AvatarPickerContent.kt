package com.poziomki.app.ui.feature.onboarding

import androidx.compose.foundation.Image
import androidx.compose.foundation.background
import androidx.compose.foundation.border
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
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.adamglin.PhosphorIcons
import com.adamglin.phosphoricons.Bold
import com.adamglin.phosphoricons.bold.Plus
import com.adamglin.phosphoricons.bold.Trash
import com.adamglin.phosphoricons.bold.X
import com.poziomki.app.ui.designsystem.theme.Black
import com.poziomki.app.ui.designsystem.theme.Border
import com.poziomki.app.ui.designsystem.theme.Error
import com.poziomki.app.ui.designsystem.theme.NunitoFamily
import com.poziomki.app.ui.designsystem.theme.Overlay
import com.poziomki.app.ui.designsystem.theme.PoziomkiTheme
import com.poziomki.app.ui.designsystem.theme.Primary
import com.poziomki.app.ui.designsystem.theme.Secondary
import com.poziomki.app.ui.designsystem.theme.Surface
import com.poziomki.app.ui.designsystem.theme.TextMuted
import com.poziomki.app.ui.designsystem.theme.TextPrimary
import com.poziomki.app.ui.designsystem.theme.White
import com.poziomki.app.ui.shared.decodeImageBytes

internal val EMOJI_AVATARS =
    listOf(
        "\uD83E\uDD8A", // 🦊
        "\uD83D\uDC3C", // 🐼
        "\uD83E\uDD81", // 🦁
        "\uD83D\uDC28", // 🐨
        "\uD83D\uDC38", // 🐸
        "\uD83E\uDD84", // 🦄
        "\uD83D\uDC36", // 🐶
        "\uD83D\uDC31", // 🐱
        "\uD83D\uDC30", // 🐰
        "\uD83D\uDC3B", // 🐻
        "\uD83D\uDC35", // 🐵
        "\uD83E\uDD8B", // 🦋
        "\uD83D\uDDFF", // 🗿
        "\uD83D\uDC22", // 🐢
        "\uD83C\uDF53", // 🍓
    )

@OptIn(ExperimentalLayoutApi::class)
@Composable
internal fun AvatarPickerContent(
    selectedAvatar: String?,
    galleryImages: List<ByteArray>,
    onPickGalleryImages: () -> Unit,
    onRemoveGalleryImage: (Int) -> Unit,
    onSelectAvatar: (String) -> Unit,
    onClearAll: () -> Unit,
    hasContent: Boolean,
) {
    val nunito = NunitoFamily

    Column(
        modifier =
            Modifier
                .fillMaxWidth()
                .padding(horizontal = PoziomkiTheme.spacing.lg)
                .padding(bottom = PoziomkiTheme.spacing.xl),
    ) {
        // Photos section header
        Text(
            text = "twoje zdj\u0119cia (${galleryImages.size}/6)",
            fontFamily = nunito,
            fontWeight = FontWeight.Bold,
            fontSize = 15.sp,
            color = TextPrimary,
        )

        // Photo thumbnails
        if (galleryImages.isNotEmpty()) {
            Spacer(modifier = Modifier.height(PoziomkiTheme.spacing.sm))
            FlowRow(
                horizontalArrangement = Arrangement.spacedBy(8.dp),
                verticalArrangement = Arrangement.spacedBy(8.dp),
            ) {
                galleryImages.forEachIndexed { index, imageBytes ->
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
                        // Remove button
                        Box(
                            modifier =
                                Modifier
                                    .align(Alignment.TopEnd)
                                    .padding(4.dp)
                                    .size(20.dp)
                                    .clip(CircleShape)
                                    .background(Overlay)
                                    .clickable { onRemoveGalleryImage(index) },
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
            }
        }

        Spacer(modifier = Modifier.height(PoziomkiTheme.spacing.md))

        // "+ dodaj kolejne" button — filled cyan
        if (galleryImages.size < 6) {
            Row(
                modifier =
                    Modifier
                        .fillMaxWidth()
                        .clip(RoundedCornerShape(PoziomkiTheme.radius.md))
                        .background(Primary)
                        .clickable(onClick = onPickGalleryImages)
                        .padding(vertical = 14.dp),
                horizontalArrangement = Arrangement.Center,
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Icon(
                    imageVector = PhosphorIcons.Bold.Plus,
                    contentDescription = null,
                    tint = Black,
                    modifier = Modifier.size(20.dp),
                )
                Spacer(modifier = Modifier.width(8.dp))
                Text(
                    text = "dodaj kolejne",
                    fontFamily = nunito,
                    fontWeight = FontWeight.SemiBold,
                    fontSize = 15.sp,
                    color = Black,
                )
            }
        }

        Spacer(modifier = Modifier.height(PoziomkiTheme.spacing.lg))

        // "lub avatar" divider
        Row(
            modifier = Modifier.fillMaxWidth(),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Box(
                modifier =
                    Modifier
                        .weight(1f)
                        .height(1.dp)
                        .background(Border),
            )
            Text(
                text = "lub avatar",
                fontFamily = nunito,
                fontWeight = FontWeight.Medium,
                fontSize = 13.sp,
                color = TextMuted,
                modifier = Modifier.padding(horizontal = 12.dp),
            )
            Box(
                modifier =
                    Modifier
                        .weight(1f)
                        .height(1.dp)
                        .background(Border),
            )
        }

        Spacer(modifier = Modifier.height(PoziomkiTheme.spacing.lg))

        // Emoji avatar grid (6 columns)
        FlowRow(
            modifier = Modifier.fillMaxWidth(),
            horizontalArrangement = Arrangement.SpaceEvenly,
            verticalArrangement = Arrangement.spacedBy(12.dp),
            maxItemsInEachRow = 6,
        ) {
            EMOJI_AVATARS.forEach { emoji ->
                val isSelected = emoji == selectedAvatar
                Box(
                    modifier =
                        Modifier
                            .size(52.dp)
                            .clip(CircleShape)
                            .then(
                                if (isSelected) {
                                    Modifier.border(2.5.dp, Secondary, CircleShape)
                                } else {
                                    Modifier.border(1.dp, Border, CircleShape)
                                },
                            ).background(Surface, CircleShape)
                            .clickable { onSelectAvatar(emoji) },
                    contentAlignment = Alignment.Center,
                ) {
                    Text(
                        text = emoji,
                        fontSize = 26.sp,
                        textAlign = TextAlign.Center,
                    )
                }
            }
        }

        // "Remove all" button
        if (hasContent) {
            Spacer(modifier = Modifier.height(PoziomkiTheme.spacing.lg))

            Row(
                modifier =
                    Modifier
                        .fillMaxWidth()
                        .clip(RoundedCornerShape(PoziomkiTheme.radius.md))
                        .clickable(onClick = onClearAll)
                        .padding(vertical = 12.dp),
                horizontalArrangement = Arrangement.Center,
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Icon(
                    imageVector = PhosphorIcons.Bold.Trash,
                    contentDescription = null,
                    tint = Error,
                    modifier = Modifier.size(18.dp),
                )
                Spacer(modifier = Modifier.width(6.dp))
                Text(
                    text = "usu\u0144 wszystko",
                    fontFamily = nunito,
                    fontWeight = FontWeight.SemiBold,
                    fontSize = 14.sp,
                    color = Error,
                )
            }
        }
    }
}
