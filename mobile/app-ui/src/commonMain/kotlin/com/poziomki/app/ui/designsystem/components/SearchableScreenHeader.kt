package com.poziomki.app.ui.designsystem.components

import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.RowScope
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.text.BasicTextField
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.focus.FocusRequester
import androidx.compose.ui.focus.focusRequester
import androidx.compose.ui.graphics.SolidColor
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.adamglin.PhosphorIcons
import com.adamglin.phosphoricons.Bold
import com.adamglin.phosphoricons.bold.MagnifyingGlass
import com.adamglin.phosphoricons.bold.SlidersHorizontal
import com.adamglin.phosphoricons.bold.X
import com.poziomki.app.ui.designsystem.theme.NunitoFamily
import com.poziomki.app.ui.designsystem.theme.PoziomkiTheme
import com.poziomki.app.ui.designsystem.theme.Primary
import com.poziomki.app.ui.designsystem.theme.TextMuted
import com.poziomki.app.ui.designsystem.theme.TextPrimary

@Composable
@Suppress("LongParameterList", "LongMethod")
fun SearchableScreenHeader(
    title: String,
    searchQuery: String,
    onSearchQueryChange: (String) -> Unit,
    searchActive: Boolean,
    onSearchActiveChange: (Boolean) -> Unit,
    modifier: Modifier = Modifier,
    placeholder: String = "szukaj",
    filterActive: Boolean = false,
    onFilterClick: (() -> Unit)? = null,
    actions: @Composable RowScope.() -> Unit = {},
) {
    val focusRequester = remember { FocusRequester() }
    val textFieldStyle =
        TextStyle(
            fontFamily = NunitoFamily,
            color = TextPrimary,
            fontSize = 15.sp,
        )
    LaunchedEffect(searchActive) {
        if (searchActive) focusRequester.requestFocus()
    }

    val rowModifier =
        modifier
            .fillMaxWidth()
            .padding(PoziomkiTheme.spacing.md)
            .height(48.dp)
    Row(
        modifier = rowModifier,
        verticalAlignment = Alignment.CenterVertically,
    ) {
        if (searchActive) {
            IconButton(onClick = {
                onSearchQueryChange("")
                onSearchActiveChange(false)
            }) {
                Icon(
                    PhosphorIcons.Bold.X,
                    contentDescription = "Zamknij wyszukiwanie",
                    tint = TextPrimary,
                )
            }
            Box(modifier = Modifier.weight(1f)) {
                if (searchQuery.isEmpty()) {
                    Text(
                        text = placeholder,
                        fontFamily = NunitoFamily,
                        color = TextMuted,
                    )
                }
                BasicTextField(
                    value = searchQuery,
                    onValueChange = onSearchQueryChange,
                    singleLine = true,
                    textStyle = textFieldStyle,
                    cursorBrush = SolidColor(MaterialTheme.colorScheme.primary),
                    modifier = Modifier.fillMaxWidth().focusRequester(focusRequester),
                )
            }
            if (onFilterClick != null) {
                IconButton(onClick = onFilterClick, modifier = Modifier.size(48.dp)) {
                    Icon(
                        PhosphorIcons.Bold.SlidersHorizontal,
                        contentDescription = "Filtruj",
                        modifier = Modifier.size(22.dp),
                        tint = if (filterActive) Primary else TextMuted,
                    )
                }
            }
        } else {
            Text(
                text = title,
                style = MaterialTheme.typography.headlineMedium,
                color = TextPrimary,
                modifier = Modifier.weight(1f).padding(start = PoziomkiTheme.spacing.sm),
            )
            IconButton(onClick = { onSearchActiveChange(true) }) {
                Icon(
                    PhosphorIcons.Bold.MagnifyingGlass,
                    contentDescription = "szukaj",
                    tint = TextPrimary,
                )
            }
            if (onFilterClick != null) {
                IconButton(onClick = onFilterClick) {
                    Icon(
                        PhosphorIcons.Bold.SlidersHorizontal,
                        contentDescription = "Filtruj",
                        tint = if (filterActive) Primary else TextPrimary,
                    )
                }
            }
            actions()
        }
    }
}
