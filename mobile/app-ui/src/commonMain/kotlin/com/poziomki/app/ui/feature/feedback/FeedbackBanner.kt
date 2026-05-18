package com.poziomki.app.ui.feature.feedback

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Icon
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.adamglin.PhosphorIcons
import com.adamglin.phosphoricons.Bold
import com.adamglin.phosphoricons.Regular
import com.adamglin.phosphoricons.bold.X
import com.adamglin.phosphoricons.regular.ChatTeardropText
import com.poziomki.app.ui.designsystem.Text
import com.poziomki.app.ui.designsystem.theme.NunitoFamily
import com.poziomki.app.ui.designsystem.theme.Primary
import com.poziomki.app.ui.designsystem.theme.TextPrimary
import com.poziomki.app.ui.designsystem.theme.TextSecondary

@Composable
fun FeedbackBanner(
    onClick: () -> Unit,
    onDismiss: () -> Unit,
) {
    val rowModifier =
        Modifier
            .fillMaxWidth()
            .padding(horizontal = 16.dp, vertical = 8.dp)
            .clip(RoundedCornerShape(12.dp))
            .background(Primary.copy(alpha = 0.12f))
            .clickable(onClick = onClick)
            .padding(horizontal = 12.dp, vertical = 10.dp)
    Row(
        modifier = rowModifier,
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Icon(
            imageVector = PhosphorIcons.Regular.ChatTeardropText,
            contentDescription = null,
            tint = Primary,
            modifier = Modifier.size(20.dp),
        )
        Column(
            modifier = Modifier.weight(1f).padding(horizontal = 12.dp),
        ) {
            Text(
                text = "Testujemy aplikację!",
                fontFamily = NunitoFamily,
                fontWeight = FontWeight.Bold,
                fontSize = 14.sp,
                color = TextPrimary,
            )
            Text(
                text = "Zostaw opinię — dotknij, aby otworzyć",
                fontFamily = NunitoFamily,
                fontSize = 12.sp,
                color = TextSecondary,
            )
        }
        Icon(
            imageVector = PhosphorIcons.Bold.X,
            contentDescription = "Zamknij",
            tint = TextSecondary,
            modifier = Modifier.size(20.dp).clickable(onClick = onDismiss),
        )
    }
}
