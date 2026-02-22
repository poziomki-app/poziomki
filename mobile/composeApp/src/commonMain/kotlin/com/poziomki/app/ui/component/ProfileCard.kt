package com.poziomki.app.ui.component

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.IntrinsicSize
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Icon
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.key
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.drawWithContent
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.BlendMode
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.CompositingStrategy
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import coil3.compose.AsyncImage
import com.adamglin.PhosphorIcons
import com.adamglin.phosphoricons.Regular
import com.adamglin.phosphoricons.regular.ArrowUpRight
import com.poziomki.app.ui.theme.Border
import com.poziomki.app.ui.theme.MontserratFamily
import com.poziomki.app.ui.theme.NunitoFamily
import com.poziomki.app.ui.theme.TextMuted
import com.poziomki.app.ui.theme.TextPrimary
import com.poziomki.app.ui.theme.TextSecondary
import com.poziomki.app.util.resolveImageUrl

@Composable
fun ProfileCard(
    name: String,
    program: String?,
    profilePicture: String?,
    gradientStart: String? = null,
    gradientEnd: String? = null,
    onClick: () -> Unit,
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
            Modifier
                .fillMaxWidth()
                .clip(cardShape)
                .border(1.dp, Border, cardShape)
                .background(backgroundBrush)
                .clickable(onClick = onClick),
    ) {
        Row(
            modifier = Modifier.height(IntrinsicSize.Min),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            // Photo — edge-to-edge left, full card height
            if (profilePicture != null) {
                key(profilePicture) {
                    Box(
                        modifier =
                            Modifier
                                .fillMaxHeight()
                                .width(photoSize)
                                .graphicsLayer(compositingStrategy = CompositingStrategy.Offscreen)
                                .drawWithContent {
                                    drawContent()
                                    drawRect(
                                        brush =
                                            Brush.horizontalGradient(
                                                colorStops =
                                                    arrayOf(
                                                        0f to Color.Black,
                                                        0.6f to Color.Black,
                                                        1f to Color.Transparent,
                                                    ),
                                            ),
                                        blendMode = BlendMode.DstIn,
                                    )
                                },
                    ) {
                        AsyncImage(
                            model = resolveImageUrl(profilePicture),
                            contentDescription = null,
                            modifier = Modifier.matchParentSize(),
                            contentScale = ContentScale.Crop,
                        )
                    }
                }
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
                PhosphorIcons.Regular.ArrowUpRight,
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
