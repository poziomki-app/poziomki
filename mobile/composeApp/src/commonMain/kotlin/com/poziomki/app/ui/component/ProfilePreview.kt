package com.poziomki.app.ui.component

import androidx.compose.foundation.Image
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.aspectRatio
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.pager.HorizontalPager
import androidx.compose.foundation.pager.rememberPagerState
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Close
import androidx.compose.material.icons.filled.Person
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import coil3.compose.AsyncImage
import com.poziomki.app.api.Tag
import com.poziomki.app.ui.theme.Background
import com.poziomki.app.ui.theme.Black
import com.poziomki.app.ui.theme.Border
import com.poziomki.app.ui.theme.MontserratFamily
import com.poziomki.app.ui.theme.NunitoFamily
import com.poziomki.app.ui.theme.PoziomkiTheme
import com.poziomki.app.ui.theme.Surface
import com.poziomki.app.ui.theme.TextMuted
import com.poziomki.app.ui.theme.TextPrimary
import com.poziomki.app.ui.theme.TextSecondary
import com.poziomki.app.ui.theme.White
import com.poziomki.app.util.decodeImageBytes
import com.poziomki.app.util.resolveImageUrl

private fun parseHexColor(hex: String?): Color? {
    if (hex.isNullOrBlank()) return null
    val clean = hex.trimStart('#')
    if (clean.length != 6) return null
    return runCatching { Color(("FF$clean").toLong(16).toInt()) }.getOrNull()
}

private fun blendWithBackground(
    color: Color,
    amount: Float,
): Color {
    val bg = Background
    return Color(
        red = bg.red * (1f - amount) + color.red * amount,
        green = bg.green * (1f - amount) + color.green * amount,
        blue = bg.blue * (1f - amount) + color.blue * amount,
        alpha = 1f,
    )
}

sealed class ProfileImage {
    data class Bytes(
        val data: ByteArray,
    ) : ProfileImage()

    data class Url(
        val url: String,
    ) : ProfileImage()
}

@OptIn(ExperimentalLayoutApi::class)
@Composable
fun ProfilePreview(
    name: String,
    program: String?,
    bio: String?,
    tags: List<Tag>,
    images: List<ProfileImage>,
    emojiAvatar: String?,
    gradientStart: String? = null,
    gradientEnd: String? = null,
    onClose: () -> Unit,
    bottomContent: @Composable (() -> Unit)? = null,
) {
    val nunito = NunitoFamily
    val montserrat = MontserratFamily
    val startColor = parseHexColor(gradientStart)
    val endColor = parseHexColor(gradientEnd)
    val hasGradient = startColor != null && endColor != null
    val darkStart = startColor?.let { blendWithBackground(it, 0.18f) }
    val darkEnd = endColor?.let { blendWithBackground(it, 0.18f) }
    val pageBackground =
        if (hasGradient && darkStart != null && darkEnd != null) {
            Modifier.background(
                Brush.verticalGradient(
                    colors = listOf(darkStart, darkEnd),
                ),
            )
        } else {
            Modifier.background(Background)
        }

    Column(
        modifier =
            Modifier
                .fillMaxSize()
                .then(pageBackground)
                .verticalScroll(rememberScrollState()),
    ) {
        // Image carousel or avatar placeholder — rounded card with margin
        Box(
            modifier =
                Modifier
                    .fillMaxWidth()
                    .padding(start = 12.dp, end = 12.dp, top = 8.dp)
                    .aspectRatio(0.75f)
                    .clip(RoundedCornerShape(24.dp)),
        ) {
            if (images.isNotEmpty()) {
                val pagerState = rememberPagerState(pageCount = { images.size })
                HorizontalPager(
                    state = pagerState,
                    modifier = Modifier.fillMaxSize(),
                ) { page ->
                    when (val image = images[page]) {
                        is ProfileImage.Bytes -> {
                            val bitmap =
                                remember(image.data) {
                                    decodeImageBytes(image.data)
                                }
                            if (bitmap != null) {
                                Image(
                                    bitmap = bitmap,
                                    contentDescription = null,
                                    modifier = Modifier.fillMaxSize(),
                                    contentScale = ContentScale.Crop,
                                )
                            } else {
                                Box(
                                    modifier = Modifier.fillMaxSize().background(Surface),
                                    contentAlignment = Alignment.Center,
                                ) {
                                    Icon(
                                        imageVector = Icons.Filled.Person,
                                        contentDescription = null,
                                        modifier = Modifier.size(64.dp),
                                        tint = TextMuted,
                                    )
                                }
                            }
                        }

                        is ProfileImage.Url -> {
                            AsyncImage(
                                model = resolveImageUrl(image.url),
                                contentDescription = null,
                                modifier = Modifier.fillMaxSize(),
                                contentScale = ContentScale.Crop,
                            )
                        }
                    }
                }

                // Page indicators with subtle shadow gradient
                if (images.size > 1) {
                    // Shadow gradient behind indicators
                    Box(
                        modifier =
                            Modifier
                                .align(Alignment.BottomCenter)
                                .fillMaxWidth()
                                .height(48.dp)
                                .background(
                                    Brush.verticalGradient(
                                        colors =
                                            listOf(
                                                Color.Transparent,
                                                Black.copy(alpha = 0.3f),
                                            ),
                                    ),
                                ),
                    )
                    Row(
                        modifier =
                            Modifier
                                .align(Alignment.BottomCenter)
                                .padding(start = 20.dp, end = 20.dp, bottom = 14.dp),
                        horizontalArrangement = Arrangement.spacedBy(6.dp),
                    ) {
                        repeat(images.size) { index ->
                            val isActive = index == pagerState.currentPage
                            Box(
                                modifier =
                                    Modifier
                                        .weight(1f)
                                        .height(4.dp)
                                        .clip(RoundedCornerShape(4.dp))
                                        .background(
                                            if (isActive) White else White.copy(alpha = 0.35f),
                                        ),
                            )
                        }
                    }
                }
            } else if (emojiAvatar != null) {
                Box(
                    modifier = Modifier.fillMaxSize().background(Surface),
                    contentAlignment = Alignment.Center,
                ) {
                    Text(
                        text = emojiAvatar,
                        fontSize = 96.sp,
                        textAlign = TextAlign.Center,
                    )
                }
            } else {
                Box(
                    modifier = Modifier.fillMaxSize().background(Surface),
                    contentAlignment = Alignment.Center,
                ) {
                    Icon(
                        imageVector = Icons.Filled.Person,
                        contentDescription = null,
                        modifier = Modifier.size(80.dp),
                        tint = TextMuted,
                    )
                }
            }

            // Close button — positioned a bit lower
            IconButton(
                onClick = onClose,
                modifier =
                    Modifier
                        .align(Alignment.TopEnd)
                        .padding(top = 28.dp, end = 12.dp)
                        .size(40.dp)
                        .clip(CircleShape)
                        .background(Black.copy(alpha = 0.45f)),
            ) {
                Icon(
                    imageVector = Icons.Filled.Close,
                    contentDescription = "Zamknij",
                    tint = White,
                    modifier = Modifier.size(24.dp),
                )
            }
        }

        // Profile info
        Column(
            modifier =
                Modifier
                    .fillMaxWidth()
                    .padding(horizontal = PoziomkiTheme.spacing.lg)
                    .padding(top = PoziomkiTheme.spacing.lg, bottom = PoziomkiTheme.spacing.xl),
        ) {
            // Name
            Text(
                text = name.ifBlank { "imi\u0119" },
                fontFamily = montserrat,
                fontWeight = FontWeight.ExtraBold,
                fontSize = 28.sp,
                color = TextPrimary,
            )

            // Program
            if (!program.isNullOrBlank()) {
                Text(
                    text = program,
                    fontFamily = nunito,
                    fontWeight = FontWeight.Normal,
                    fontSize = 16.sp,
                    color = TextSecondary,
                )
            }

            // Bio
            if (!bio.isNullOrBlank()) {
                Spacer(modifier = Modifier.height(PoziomkiTheme.spacing.md))
                Text(
                    text = "bio",
                    fontFamily = montserrat,
                    fontWeight = FontWeight.ExtraBold,
                    fontSize = 16.sp,
                    color = TextPrimary,
                )
                Spacer(modifier = Modifier.height(4.dp))
                RichBio(bio = bio)
            }

            // Tags — compact
            if (tags.isNotEmpty()) {
                Spacer(modifier = Modifier.height(PoziomkiTheme.spacing.md))
                Text(
                    text = "zainteresowania",
                    fontFamily = montserrat,
                    fontWeight = FontWeight.ExtraBold,
                    fontSize = 16.sp,
                    color = TextPrimary,
                )
                Spacer(modifier = Modifier.height(PoziomkiTheme.spacing.sm))
                FlowRow(
                    horizontalArrangement = Arrangement.spacedBy(6.dp),
                    verticalArrangement = Arrangement.spacedBy(6.dp),
                ) {
                    tags.forEach { tag ->
                        Text(
                            text = "${tag.emoji ?: ""} ${tag.name}".trim(),
                            fontFamily = nunito,
                            fontWeight = FontWeight.Medium,
                            fontSize = 13.sp,
                            color = TextSecondary,
                            modifier =
                                Modifier
                                    .border(1.dp, Border, RoundedCornerShape(50))
                                    .padding(horizontal = 8.dp, vertical = 3.dp),
                        )
                    }
                }
            }

            // Bottom content slot (e.g. "send message" button)
            if (bottomContent != null) {
                Spacer(modifier = Modifier.height(PoziomkiTheme.spacing.lg))
                bottomContent()
            }
        }
    }
}

private val bioImageRegex = Regex("""!\[\]\((.*?)\)""")

@Composable
private fun RichBio(bio: String) {
    val nunito = NunitoFamily

    if (!bio.contains("![](")) {
        Text(
            text = bio,
            fontFamily = nunito,
            fontWeight = FontWeight.Normal,
            fontSize = 15.sp,
            color = TextPrimary,
            lineHeight = 22.sp,
        )
        return
    }

    val segments = remember(bio) { parseBioSegments(bio) }
    Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
        segments.forEach { segment ->
            when (segment) {
                is BioSegment.TextSegment -> {
                    if (segment.text.isNotBlank()) {
                        Text(
                            text = segment.text,
                            fontFamily = nunito,
                            fontWeight = FontWeight.Normal,
                            fontSize = 15.sp,
                            color = TextPrimary,
                            lineHeight = 22.sp,
                        )
                    }
                }

                is BioSegment.ImageSegment -> {
                    AsyncImage(
                        model = resolveImageUrl(segment.url),
                        contentDescription = null,
                        modifier =
                            Modifier
                                .fillMaxWidth()
                                .clip(RoundedCornerShape(12.dp)),
                        contentScale = ContentScale.FillWidth,
                    )
                }
            }
        }
    }
}

private sealed class BioSegment {
    data class TextSegment(
        val text: String,
    ) : BioSegment()

    data class ImageSegment(
        val url: String,
    ) : BioSegment()
}

private fun parseBioSegments(bio: String): List<BioSegment> {
    val segments = mutableListOf<BioSegment>()
    var lastIndex = 0
    bioImageRegex.findAll(bio).forEach { match ->
        val before = bio.substring(lastIndex, match.range.first).trim()
        if (before.isNotEmpty()) {
            segments.add(BioSegment.TextSegment(before))
        }
        val url = match.groupValues[1]
        if (url.isNotBlank()) {
            segments.add(BioSegment.ImageSegment(url))
        }
        lastIndex = match.range.last + 1
    }
    val after = bio.substring(lastIndex).trim()
    if (after.isNotEmpty()) {
        segments.add(BioSegment.TextSegment(after))
    }
    return segments
}
