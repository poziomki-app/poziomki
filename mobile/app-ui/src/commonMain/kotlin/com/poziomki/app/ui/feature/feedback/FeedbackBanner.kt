package com.poziomki.app.ui.feature.feedback

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.SwipeToDismissBox
import androidx.compose.material3.SwipeToDismissBoxValue
import androidx.compose.material3.rememberSwipeToDismissBoxState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.adamglin.PhosphorIcons
import com.adamglin.phosphoricons.Regular
import com.adamglin.phosphoricons.regular.ChatTeardropText
import com.poziomki.app.ui.designsystem.Text
import com.poziomki.app.ui.designsystem.theme.NunitoFamily
import com.poziomki.app.ui.designsystem.theme.Primary
import com.poziomki.app.ui.designsystem.theme.TextSecondary

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun FeedbackBanner(
    onClick: () -> Unit,
    onDismiss: () -> Unit,
) {
    val dismissState = rememberSwipeToDismissBoxState()

    LaunchedEffect(dismissState.currentValue) {
        if (dismissState.currentValue != SwipeToDismissBoxValue.Settled) {
            onDismiss()
        }
    }

    SwipeToDismissBox(
        state = dismissState,
        backgroundContent = {},
        modifier = Modifier.padding(horizontal = 16.dp, vertical = 6.dp),
    ) {
        val rowModifier =
            Modifier
                .fillMaxWidth()
                .clip(RoundedCornerShape(10.dp))
                .background(Primary.copy(alpha = 0.08f))
                .clickable(onClick = onClick)
                .padding(horizontal = 12.dp, vertical = 8.dp)
        Row(modifier = rowModifier, verticalAlignment = Alignment.CenterVertically) {
            Icon(
                imageVector = PhosphorIcons.Regular.ChatTeardropText,
                contentDescription = null,
                tint = Primary,
                modifier = Modifier.size(16.dp),
            )
            Box(modifier = Modifier.size(width = 8.dp, height = 1.dp))
            Text(
                text = "masz uwagi? zostaw opinię",
                fontFamily = NunitoFamily,
                fontWeight = FontWeight.Medium,
                fontSize = 13.sp,
                color = TextSecondary,
            )
        }
    }
}
