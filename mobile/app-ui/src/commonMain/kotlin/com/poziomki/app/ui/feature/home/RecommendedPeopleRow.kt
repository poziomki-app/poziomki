package com.poziomki.app.ui.feature.home

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyRow
import androidx.compose.foundation.lazy.items
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.drawBehind
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.StrokeCap
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.poziomki.app.network.MatchProfile
import com.poziomki.app.ui.designsystem.components.SectionLabel
import com.poziomki.app.ui.designsystem.components.UserAvatar
import com.poziomki.app.ui.designsystem.theme.Border
import com.poziomki.app.ui.designsystem.theme.NunitoFamily
import com.poziomki.app.ui.designsystem.theme.PoziomkiTheme
import com.poziomki.app.ui.designsystem.theme.Primary
import com.poziomki.app.ui.designsystem.theme.TextPrimary

@Composable
fun RecommendedPeopleRow(
    profiles: List<MatchProfile>,
    onProfileClick: (String) -> Unit,
) {
    if (profiles.isEmpty()) return

    Column {
        SectionLabel(
            text = "polecane osoby",
            modifier = Modifier.padding(horizontal = PoziomkiTheme.spacing.lg),
        )

        LazyRow(
            contentPadding = PaddingValues(horizontal = PoziomkiTheme.spacing.lg),
            horizontalArrangement = Arrangement.spacedBy(12.dp),
        ) {
            items(profiles, key = { "rec-${it.id}" }) { profile ->
                RecommendedPersonItem(
                    name = profile.name,
                    profilePicture = profile.profilePicture,
                    score = profile.score,
                    onClick = { onProfileClick(profile.id) },
                )
            }
        }
    }
}

@Composable
private fun RecommendedPersonItem(
    name: String,
    profilePicture: String?,
    score: Double,
    onClick: () -> Unit,
) {
    val avatarSize = 56.dp
    val strokeWidth = 3.dp
    val ringPadding = 3.dp
    val totalSize = avatarSize + (strokeWidth + ringPadding) * 2
    val progress = (score / 100.0).coerceIn(0.0, 1.0).toFloat()
    val firstName = name.split(" ").first()

    Column(
        horizontalAlignment = Alignment.CenterHorizontally,
        modifier =
            Modifier
                .width(totalSize + 4.dp)
                .clickable(onClick = onClick),
    ) {
        Box(
            contentAlignment = Alignment.Center,
            modifier =
                Modifier
                    .size(totalSize)
                    .drawBehind {
                        val stroke =
                            Stroke(
                                width = strokeWidth.toPx(),
                                cap = StrokeCap.Round,
                            )
                        val arcOffset = strokeWidth.toPx() / 2f
                        val arcSize =
                            Size(
                                size.width - strokeWidth.toPx(),
                                size.height - strokeWidth.toPx(),
                            )
                        val topLeft = Offset(arcOffset, arcOffset)

                        // Background track
                        drawArc(
                            color = Border,
                            startAngle = -90f,
                            sweepAngle = 360f,
                            useCenter = false,
                            topLeft = topLeft,
                            size = arcSize,
                            style = stroke,
                        )

                        // Foreground progress
                        if (progress > 0f) {
                            drawArc(
                                color = Primary,
                                startAngle = -90f,
                                sweepAngle = progress * 360f,
                                useCenter = false,
                                topLeft = topLeft,
                                size = arcSize,
                                style = stroke,
                            )
                        }
                    },
        ) {
            UserAvatar(
                picture = profilePicture,
                displayName = name,
                size = avatarSize,
            )
        }

        Text(
            text = firstName,
            fontFamily = NunitoFamily,
            fontWeight = FontWeight.Medium,
            fontSize = 12.sp,
            color = TextPrimary,
            maxLines = 1,
            overflow = TextOverflow.Ellipsis,
            textAlign = TextAlign.Center,
            modifier = Modifier.padding(top = 4.dp),
        )
    }
}
