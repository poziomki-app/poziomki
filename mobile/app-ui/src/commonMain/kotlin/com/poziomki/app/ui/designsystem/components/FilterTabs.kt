package com.poziomki.app.ui.designsystem.components

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.poziomki.app.ui.designsystem.Text
import com.poziomki.app.ui.designsystem.theme.NunitoFamily
import com.poziomki.app.ui.designsystem.theme.Primary
import com.poziomki.app.ui.designsystem.theme.SurfaceElevated
import com.poziomki.app.ui.designsystem.theme.TextMuted
import com.poziomki.app.ui.designsystem.theme.TextPrimary

enum class FilterTabsStyle { Dot, Pill }

@Composable
@Suppress("LongParameterList")
fun <T> FilterTabs(
    tabs: List<Pair<T, String>>,
    selected: T,
    onSelect: (T) -> Unit,
    modifier: Modifier = Modifier,
    horizontalArrangement: Arrangement.Horizontal = Arrangement.spacedBy(24.dp),
    style: FilterTabsStyle = FilterTabsStyle.Dot,
) {
    Row(
        modifier =
            modifier
                .fillMaxWidth()
                .padding(top = 12.dp, bottom = 16.dp),
        horizontalArrangement = horizontalArrangement,
        verticalAlignment = Alignment.CenterVertically,
    ) {
        tabs.forEach { (value, label) ->
            val isSelected = value == selected
            when (style) {
                FilterTabsStyle.Dot -> {
                    DotTab(label = label, isSelected = isSelected, onClick = { onSelect(value) })
                }

                FilterTabsStyle.Pill -> {
                    PillTab(label = label, isSelected = isSelected, onClick = { onSelect(value) })
                }
            }
        }
    }
}

@Composable
private fun DotTab(
    label: String,
    isSelected: Boolean,
    onClick: () -> Unit,
) {
    Row(
        modifier =
            Modifier
                .clickable(onClick = onClick)
                .padding(vertical = 4.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        if (isSelected) {
            Box(
                modifier =
                    Modifier
                        .size(6.dp)
                        .background(Primary, CircleShape),
            )
            Spacer(modifier = Modifier.width(6.dp))
        }
        Text(
            text = label,
            fontFamily = NunitoFamily,
            fontWeight = if (isSelected) FontWeight.Bold else FontWeight.Normal,
            fontSize = 16.sp,
            color = if (isSelected) TextPrimary else TextMuted,
        )
    }
}

@Composable
private fun PillTab(
    label: String,
    isSelected: Boolean,
    onClick: () -> Unit,
) {
    Text(
        text = label,
        fontFamily = NunitoFamily,
        fontWeight = if (isSelected) FontWeight.SemiBold else FontWeight.Normal,
        fontSize = 15.sp,
        color = if (isSelected) TextPrimary else TextMuted,
        modifier =
            Modifier
                .clickable(onClick = onClick)
                .background(
                    color = if (isSelected) Color(0xFF242424) else SurfaceElevated,
                    shape = RoundedCornerShape(50),
                ).padding(horizontal = 12.dp, vertical = 5.dp),
    )
}
