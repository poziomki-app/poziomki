package com.poziomki.app.ui.designsystem.components

import androidx.compose.foundation.Image
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.size
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.poziomki.app.ui.designsystem.theme.MontserratFamily
import com.poziomki.app.ui.designsystem.theme.TextPrimary
import org.jetbrains.compose.resources.painterResource
import poziomki_mobile.composeapp.generated.resources.Res
import poziomki_mobile.composeapp.generated.resources.strawberry_logo

@Composable
fun PoziomkiLogo(
    size: Dp = 48.dp,
    modifier: Modifier = Modifier,
) {
    val fontSize = size.value.sp
    val imageSize = size * 0.88f
    val montserrat = MontserratFamily

    Row(
        modifier = modifier,
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Text(
            text = "p",
            fontFamily = montserrat,
            fontSize = fontSize,
            color = TextPrimary,
        )
        Image(
            painter = painterResource(Res.drawable.strawberry_logo),
            contentDescription = null,
            modifier =
                Modifier
                    .size(imageSize)
                    .offset(x = (-2).dp, y = (size.value * 0.1f).dp),
        )
        Text(
            text = "ziomki",
            fontFamily = montserrat,
            fontSize = fontSize,
            color = TextPrimary,
        )
    }
}
