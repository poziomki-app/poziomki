package com.poziomki.app.ui.designsystem.components

import androidx.compose.foundation.layout.padding
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.SpanStyle
import androidx.compose.ui.text.buildAnnotatedString
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.withStyle
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.poziomki.app.ui.designsystem.theme.Accent
import com.poziomki.app.ui.designsystem.theme.MontserratFamily
import com.poziomki.app.ui.designsystem.theme.TextPrimary

@Composable
fun SectionLabel(
    text: String,
    modifier: Modifier = Modifier,
    color: Color = TextPrimary,
    required: Boolean = false,
) {
    val label =
        buildAnnotatedString {
            append(text)
            if (required) {
                withStyle(SpanStyle(color = Accent, fontWeight = FontWeight.Bold)) {
                    append(" *")
                }
            }
        }
    Text(
        text = label,
        fontFamily = MontserratFamily,
        fontWeight = FontWeight.SemiBold,
        fontSize = 14.sp,
        color = color,
        modifier = modifier.padding(bottom = 8.dp),
    )
}
