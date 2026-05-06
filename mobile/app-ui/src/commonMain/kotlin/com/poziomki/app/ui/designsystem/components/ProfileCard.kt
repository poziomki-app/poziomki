package com.poziomki.app.ui.designsystem.components

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
import androidx.compose.foundation.layout.aspectRatio
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.RoundedCornerShape
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
import com.poziomki.app.network.Tag
import com.poziomki.app.ui.designsystem.theme.Border
import com.poziomki.app.ui.designsystem.theme.MontserratFamily
import com.poziomki.app.ui.designsystem.theme.NunitoFamily
import com.poziomki.app.ui.designsystem.theme.Primary
import com.poziomki.app.ui.designsystem.theme.PrimaryMuted
import com.poziomki.app.ui.designsystem.theme.TextPrimary
import com.poziomki.app.ui.designsystem.theme.TextSecondary
import com.poziomki.app.ui.shared.resolveImageUrl

@OptIn(ExperimentalLayoutApi::class)
@Composable
fun ProfileCard(
    name: String,
    profilePicture: String?,
    gradientStart: String? = null,
    gradientEnd: String? = null,
    matchingTags: List<Tag> = emptyList(),
    program: String? = null,
    onClick: () -> Unit,
    modifier: Modifier = Modifier,
) {
    val cardShape = RoundedCornerShape(20.dp)
    val cardHeight = 88.dp

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
                .height(cardHeight)
                .clip(cardShape)
                .border(1.dp, Border, cardShape)
                .background(backgroundBrush)
                .clickable(onClick = onClick),
    ) {
        Row(
            modifier = Modifier.fillMaxSize(),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            // Photo — square that fills the full card height
            if (profilePicture != null) {
                AsyncImage(
                    model = resolveImageUrl(profilePicture),
                    contentDescription = null,
                    modifier = Modifier.fillMaxHeight().aspectRatio(1f),
                    contentScale = ContentScale.Crop,
                )
            } else {
                Box(modifier = Modifier.padding(16.dp)) {
                    UserAvatar(
                        picture = null,
                        displayName = name,
                        size = cardHeight - 32.dp,
                    )
                }
            }

            Spacer(modifier = Modifier.width(12.dp))

            // Content column
            Column(
                modifier =
                    Modifier
                        .weight(1f)
                        .padding(vertical = 12.dp),
                verticalArrangement = Arrangement.Center,
            ) {
                Text(
                    text = name,
                    fontFamily = MontserratFamily,
                    fontWeight = FontWeight.ExtraBold,
                    fontSize = 19.sp,
                    color = TextPrimary,
                )

                if (!program.isNullOrBlank()) {
                    Text(
                        text = program,
                        fontFamily = NunitoFamily,
                        fontWeight = FontWeight.Normal,
                        fontSize = 12.sp,
                        lineHeight = 14.sp,
                        color = TextSecondary,
                    )
                }

                if (matchingTags.isNotEmpty()) {
                    Spacer(modifier = Modifier.height(4.dp))
                    // Activities first (stronger signal: shared IRL action),
                    // then interests (talking points).
                    val orderedTags =
                        matchingTags.sortedByDescending { it.scope == "activity" }
                    FlowRow(
                        horizontalArrangement = Arrangement.spacedBy(4.dp),
                        verticalArrangement = Arrangement.spacedBy(4.dp),
                    ) {
                        orderedTags.forEach { tag ->
                            val isActivity = tag.scope == "activity"
                            Text(
                                text = tag.name.lowercase(),
                                fontFamily = NunitoFamily,
                                fontWeight = if (isActivity) FontWeight.SemiBold else FontWeight.Medium,
                                fontSize = 11.sp,
                                lineHeight = 12.sp,
                                color = if (isActivity) PrimaryMuted else TextSecondary,
                                modifier =
                                    Modifier
                                        .clip(RoundedCornerShape(50))
                                        .background(
                                            if (isActivity) {
                                                Primary.copy(alpha = 0.18f)
                                            } else {
                                                Color.White.copy(alpha = 0.06f)
                                            },
                                        )
                                        .padding(horizontal = 7.dp, vertical = 1.dp),
                            )
                        }
                    }
                }
            }
        }
    }
}
