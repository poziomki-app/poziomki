package com.poziomki.app.ui.designsystem.components

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.poziomki.app.network.Tag
import com.poziomki.app.ui.designsystem.theme.NunitoFamily
import com.poziomki.app.ui.designsystem.theme.Primary
import com.poziomki.app.ui.designsystem.theme.PrimaryMuted
import com.poziomki.app.ui.designsystem.theme.TextSecondary

enum class TagChipSize { SMALL, REGULAR }

@Composable
fun TagChip(
    tag: Tag,
    matching: Boolean = false,
    size: TagChipSize = TagChipSize.REGULAR,
    modifier: Modifier = Modifier,
) {
    val fontSize = if (size == TagChipSize.SMALL) 11.sp else 13.sp
    val lineHeight = if (size == TagChipSize.SMALL) 12.sp else 16.sp
    val hPad = if (size == TagChipSize.SMALL) 8.dp else 10.dp
    val vPad = if (size == TagChipSize.SMALL) 2.dp else 4.dp

    val bg = if (matching) Primary.copy(alpha = 0.18f) else Color.White.copy(alpha = 0.06f)
    val fg = if (matching) PrimaryMuted else TextSecondary
    val textModifier =
        modifier
            .clip(RoundedCornerShape(50))
            .background(bg)
            .padding(horizontal = hPad, vertical = vPad)

    Text(
        text = tag.name.lowercase(),
        fontFamily = NunitoFamily,
        fontWeight = if (matching) FontWeight.SemiBold else FontWeight.Medium,
        fontSize = fontSize,
        lineHeight = lineHeight,
        color = fg,
        modifier = textModifier,
    )
}
