package com.poziomki.app.ui.designsystem.components

import androidx.compose.foundation.Image
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.size
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.FilterQuality
import androidx.compose.ui.graphics.painter.BitmapPainter
import androidx.compose.ui.unit.dp
import com.poziomki.app.ui.designsystem.Text
import com.poziomki.app.ui.designsystem.theme.NunitoFamily
import com.poziomki.app.ui.designsystem.theme.Primary
import com.poziomki.app.ui.designsystem.theme.TextSecondary
import org.jetbrains.compose.resources.DrawableResource
import org.jetbrains.compose.resources.imageResource
import poziomki_mobile.app_ui.generated.resources.Res
import poziomki_mobile.app_ui.generated.resources.doodle_splash

@Composable
fun LoadingView(modifier: Modifier = Modifier) {
    Box(modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
        CircularProgressIndicator(color = Primary)
    }
}

@Composable
fun EmptyView(
    message: String,
    modifier: Modifier = Modifier,
    illustration: DrawableResource? = null,
) {
    Box(modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
        Column(
            horizontalAlignment = Alignment.CenterHorizontally,
            verticalArrangement = Arrangement.spacedBy(20.dp),
        ) {
            if (illustration != null) {
                Box(contentAlignment = Alignment.Center) {
                    Image(
                        painter = highQualityPainter(Res.drawable.doodle_splash),
                        contentDescription = null,
                        alpha = 0.6f,
                        modifier = Modifier.size(260.dp, 170.dp),
                    )
                    Image(
                        painter = highQualityPainter(illustration),
                        contentDescription = null,
                        modifier = Modifier.size(160.dp),
                    )
                }
            }
            Text(
                text = message,
                fontFamily = NunitoFamily,
                color = TextSecondary,
            )
        }
    }
}

// Use FilterQuality.High so scaled doodle PNGs render with a proper bilinear
// resampler instead of the default Low which leaves nearest-neighbour pixel
// stair-stepping on the sketchy outlines.
@Composable
private fun highQualityPainter(resource: DrawableResource): BitmapPainter =
    BitmapPainter(imageResource(resource), filterQuality = FilterQuality.High)
