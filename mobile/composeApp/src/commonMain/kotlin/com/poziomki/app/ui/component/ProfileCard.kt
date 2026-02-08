package com.poziomki.app.ui.component

import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.NorthEast
import androidx.compose.material.icons.filled.Person
import androidx.compose.material3.Icon
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import coil3.compose.AsyncImage
import com.poziomki.app.api.Tag
import com.poziomki.app.ui.theme.Border
import com.poziomki.app.ui.theme.MontserratFamily
import com.poziomki.app.ui.theme.NunitoFamily
import com.poziomki.app.ui.theme.TextMuted
import com.poziomki.app.ui.theme.TextPrimary
import com.poziomki.app.ui.theme.TextSecondary
import com.poziomki.app.util.isImageUrl
import com.poziomki.app.util.resolveImageUrl
import com.poziomki.app.ui.theme.Surface as SurfaceColor

@Composable
fun ProfileCard(
    name: String,
    program: String?,
    profilePicture: String?,
    tags: List<Tag>,
    maxVisibleTags: Int = 2,
    onClick: () -> Unit,
) {
    Surface(
        modifier =
            Modifier
                .fillMaxWidth()
                .clip(RoundedCornerShape(20.dp))
                .clickable(onClick = onClick),
        shape = RoundedCornerShape(20.dp),
        color = SurfaceColor,
        border = androidx.compose.foundation.BorderStroke(1.dp, Border),
    ) {
        Box(modifier = Modifier.padding(16.dp)) {
            Row(verticalAlignment = Alignment.CenterVertically) {
                // Avatar
                Surface(
                    modifier = Modifier.size(80.dp),
                    shape = CircleShape,
                    color = Border,
                ) {
                    when {
                        profilePicture != null && isImageUrl(profilePicture) -> {
                            AsyncImage(
                                model = resolveImageUrl(profilePicture),
                                contentDescription = name,
                                modifier =
                                    Modifier
                                        .size(80.dp)
                                        .clip(CircleShape),
                                contentScale = ContentScale.Crop,
                            )
                        }

                        profilePicture != null -> {
                            // Emoji avatar
                            Box(
                                modifier = Modifier.size(80.dp),
                                contentAlignment = Alignment.Center,
                            ) {
                                Text(
                                    text = profilePicture,
                                    fontSize = 36.sp,
                                )
                            }
                        }

                        else -> {
                            Box(
                                modifier = Modifier.size(80.dp),
                                contentAlignment = Alignment.Center,
                            ) {
                                Icon(
                                    Icons.Filled.Person,
                                    contentDescription = name,
                                    modifier = Modifier.size(40.dp),
                                    tint = TextMuted,
                                )
                            }
                        }
                    }
                }

                Spacer(modifier = Modifier.width(16.dp))

                // Content column
                Column(modifier = Modifier.weight(1f)) {
                    // Name
                    Text(
                        text = name,
                        fontFamily = MontserratFamily,
                        fontWeight = FontWeight.ExtraBold,
                        fontSize = 20.sp,
                        color = TextPrimary,
                    )

                    // Program
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

                    // Tags
                    if (tags.isNotEmpty()) {
                        Spacer(modifier = Modifier.height(8.dp))
                        Row(
                            horizontalArrangement = Arrangement.spacedBy(6.dp),
                            verticalAlignment = Alignment.CenterVertically,
                        ) {
                            tags.take(maxVisibleTags).forEach { tag ->
                                Text(
                                    text = tag.name,
                                    fontFamily = NunitoFamily,
                                    fontWeight = FontWeight.Medium,
                                    fontSize = 13.sp,
                                    color = TextSecondary,
                                    modifier =
                                        Modifier
                                            .border(
                                                1.dp,
                                                Border,
                                                RoundedCornerShape(50),
                                            ).padding(horizontal = 8.dp, vertical = 3.dp),
                                )
                            }
                            val overflow = tags.size - maxVisibleTags
                            if (overflow > 0) {
                                Text(
                                    text = "+$overflow",
                                    fontFamily = NunitoFamily,
                                    fontWeight = FontWeight.Medium,
                                    fontSize = 13.sp,
                                    color = TextMuted,
                                )
                            }
                        }
                    }
                }
            }

            // Expand arrow top-right
            Icon(
                Icons.Filled.NorthEast,
                contentDescription = "View profile",
                modifier =
                    Modifier
                        .size(20.dp)
                        .align(Alignment.TopEnd),
                tint = TextMuted,
            )
        }
    }
}
