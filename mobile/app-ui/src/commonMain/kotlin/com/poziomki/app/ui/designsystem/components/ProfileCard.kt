package com.poziomki.app.ui.designsystem.components

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Icon
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import coil3.compose.AsyncImage
import com.adamglin.PhosphorIcons
import com.adamglin.phosphoricons.Bold
import com.adamglin.phosphoricons.bold.ArrowUpRight
import com.poziomki.app.ui.designsystem.theme.Border
import com.poziomki.app.ui.designsystem.theme.MontserratFamily
import com.poziomki.app.ui.designsystem.theme.NunitoFamily
import com.poziomki.app.ui.designsystem.theme.TextMuted
import com.poziomki.app.ui.designsystem.theme.TextPrimary
import com.poziomki.app.ui.designsystem.theme.TextSecondary
import com.poziomki.app.ui.shared.resolveImageUrl

@Composable
fun ProfileCard(
    name: String,
    program: String?,
    profilePicture: String?,
    gradientStart: String? = null,
    gradientEnd: String? = null,
    onClick: () -> Unit,
    modifier: Modifier = Modifier,
) {
    val cardShape = RoundedCornerShape(20.dp)
    val photoSize = 90.dp

    val startColor = parseHexColor(gradientStart)
    val endColor = parseHexColor(gradientEnd)
    val hasProfileGradient = startColor != null && endColor != null

    val backgroundBrush =
        if (hasProfileGradient) {
            val darkStart = blendWithBackground(startColor, 0.18f)
            val darkEnd = blendWithBackground(endColor, 0.18f)
            Brush.linearGradient(
                colors = listOf(darkStart, darkEnd),
                start = Offset(0f, 0f),
                end = Offset(Float.POSITIVE_INFINITY, Float.POSITIVE_INFINITY),
            )
        } else {
            Brush.linearGradient(
                colors = listOf(Color(0xFF161C26), Color(0xFF080B10)),
                start = Offset(0f, 0f),
                end = Offset(Float.POSITIVE_INFINITY, Float.POSITIVE_INFINITY),
            )
        }

    Box(
        modifier =
            modifier
                .fillMaxWidth()
                .clip(cardShape)
                .border(1.dp, Border, cardShape)
                .background(backgroundBrush)
                .clickable(onClick = onClick),
    ) {
        Row(
            verticalAlignment = Alignment.CenterVertically,
        ) {
            // Photo — fixed square, ContentScale.Crop fills it
            if (profilePicture != null) {
                AsyncImage(
                    model = resolveImageUrl(profilePicture),
                    contentDescription = null,
                    modifier = Modifier.size(photoSize),
                    contentScale = ContentScale.Crop,
                )
            } else {
                Box(modifier = Modifier.padding(16.dp)) {
                    UserAvatar(
                        picture = null,
                        displayName = name,
                        size = photoSize - 32.dp,
                    )
                }
            }

            Spacer(modifier = Modifier.width(12.dp))

            // Content column
            Column(
                modifier =
                    Modifier
                        .weight(1f)
                        .padding(vertical = 16.dp),
            ) {
                Text(
                    text = name,
                    fontFamily = MontserratFamily,
                    fontWeight = FontWeight.ExtraBold,
                    fontSize = 20.sp,
                    color = TextPrimary,
                )

                if (program != null) {
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

            // Expand arrow top-right
            Icon(
                PhosphorIcons.Bold.ArrowUpRight,
                contentDescription = "View profile",
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
