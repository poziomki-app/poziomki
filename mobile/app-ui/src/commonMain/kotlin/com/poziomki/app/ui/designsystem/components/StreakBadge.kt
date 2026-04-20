package com.poziomki.app.ui.designsystem.components

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Icon
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.adamglin.PhosphorIcons
import com.adamglin.phosphoricons.Fill
import com.adamglin.phosphoricons.fill.Flame
import com.poziomki.app.ui.designsystem.theme.Secondary
import com.poziomki.app.ui.designsystem.theme.SecondaryLight
import kotlin.math.max

@Composable
fun StreakBadge(
    streak: Int,
    modifier: Modifier = Modifier,
    onClick: (() -> Unit)? = null,
) {
    val display = max(1, streak)
    Row(
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.Center,
        modifier =
            modifier
                .clip(RoundedCornerShape(14.dp))
                .background(SecondaryLight.copy(alpha = 0.55f))
                .border(1.dp, Secondary.copy(alpha = 0.35f), RoundedCornerShape(14.dp))
                .let { if (onClick != null) it.clickable(onClick = onClick) else it }
                .padding(horizontal = 8.dp, vertical = 4.dp),
    ) {
        Icon(
            PhosphorIcons.Fill.Flame,
            contentDescription = "Streak",
            tint = Secondary,
            modifier = Modifier.size(16.dp),
        )
        Spacer(Modifier.width(3.dp))
        Text(
            text = display.toString(),
            color = Color(0xFFFFE9A8),
            fontWeight = FontWeight.ExtraBold,
            fontSize = 13.sp,
        )
    }
}
