package com.poziomki.app.ui.component

import androidx.compose.material3.Snackbar
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.sp
import com.poziomki.app.ui.theme.Error
import com.poziomki.app.ui.theme.NunitoFamily
import com.poziomki.app.ui.theme.Success

enum class SnackbarType { SUCCESS, ERROR }

@Composable
fun PoziomkiSnackbar(
    message: String,
    type: SnackbarType = SnackbarType.ERROR,
    modifier: Modifier = Modifier,
) {
    val containerColor =
        when (type) {
            SnackbarType.ERROR -> Color(0xFF3D1F1F)
            SnackbarType.SUCCESS -> Color(0xFF1A3D25)
        }
    val contentColor =
        when (type) {
            SnackbarType.ERROR -> Error
            SnackbarType.SUCCESS -> Success
        }

    Snackbar(
        modifier = modifier,
        containerColor = containerColor,
    ) {
        Text(
            text = message,
            fontFamily = NunitoFamily,
            fontWeight = FontWeight.Medium,
            fontSize = 14.sp,
            color = contentColor,
        )
    }
}
