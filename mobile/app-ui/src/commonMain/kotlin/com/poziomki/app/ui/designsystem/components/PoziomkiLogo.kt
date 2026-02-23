package com.poziomki.app.ui.designsystem.components

import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.offset
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.poziomki.app.ui.designsystem.theme.MontserratFamily
import com.poziomki.app.ui.designsystem.theme.TextPrimary

@Composable
fun PoziomkiLogo(
    size: Dp = 48.dp,
    modifier: Modifier = Modifier,
) {
    val fontSize = size.value.sp
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
        Text(
            text = "🍓",
            fontSize = (size.value * 0.7f).sp,
            modifier = Modifier.offset(x = (-2).dp, y = (size.value * 0.1f).dp),
        )
        Text(
            text = "ziomki",
            fontFamily = montserrat,
            fontSize = fontSize,
            color = TextPrimary,
        )
    }
}
