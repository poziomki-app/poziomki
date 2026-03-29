package com.poziomki.app.ui.designsystem.components

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.defaultMinSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.BasicTextField
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.autofill.ContentType
import androidx.compose.ui.graphics.SolidColor
import androidx.compose.ui.semantics.contentType
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.text.input.PasswordVisualTransformation
import androidx.compose.ui.text.input.VisualTransformation
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.adamglin.PhosphorIcons
import com.adamglin.phosphoricons.Bold
import com.adamglin.phosphoricons.bold.Eye
import com.adamglin.phosphoricons.bold.EyeSlash
import com.poziomki.app.ui.designsystem.theme.Border
import com.poziomki.app.ui.designsystem.theme.Error
import com.poziomki.app.ui.designsystem.theme.MontserratFamily
import com.poziomki.app.ui.designsystem.theme.PoziomkiTheme
import com.poziomki.app.ui.designsystem.theme.Surface
import com.poziomki.app.ui.designsystem.theme.TextMuted
import com.poziomki.app.ui.designsystem.theme.TextPrimary

@Suppress("LongParameterList", "LongMethod")
@Composable
fun PoziomkiTextField(
    value: String,
    onValueChange: (String) -> Unit,
    modifier: Modifier = Modifier,
    label: String? = null,
    placeholder: String? = null,
    error: String? = null,
    singleLine: Boolean = true,
    maxLines: Int = if (singleLine) 1 else Int.MAX_VALUE,
    minLines: Int = 1,
    keyboardOptions: KeyboardOptions = KeyboardOptions.Default,
    visualTransformation: VisualTransformation = VisualTransformation.None,
    trailingContent: @Composable (() -> Unit)? = null,
    contentType: ContentType? = null,
) {
    val nunito = MontserratFamily
    val shape = RoundedCornerShape(PoziomkiTheme.radius.md)
    val borderColor = if (error != null) Error else Border

    Column(modifier = modifier) {
        if (label != null) {
            Text(
                text = label,
                fontFamily = nunito,
                fontWeight = FontWeight.SemiBold,
                fontSize = 14.sp,
                color = TextPrimary,
                modifier = Modifier.padding(start = 4.dp, bottom = 8.dp),
            )
        }

        val heightModifier =
            if (singleLine) {
                Modifier.height(PoziomkiTheme.componentSizes.inputHeight)
            } else {
                Modifier.defaultMinSize(minHeight = PoziomkiTheme.componentSizes.inputHeight)
            }

        Box(
            modifier =
                Modifier
                    .fillMaxWidth()
                    .then(heightModifier)
                    .background(Surface, shape)
                    .border(1.dp, borderColor, shape),
        ) {
            Row(
                modifier = Modifier.matchParentSize(),
                verticalAlignment = if (singleLine) Alignment.CenterVertically else Alignment.Top,
            ) {
                BasicTextField(
                    value = value,
                    onValueChange = onValueChange,
                    modifier =
                        Modifier
                            .weight(1f)
                            .padding(horizontal = 16.dp, vertical = if (singleLine) 0.dp else 12.dp)
                            .then(
                                if (contentType != null) {
                                    Modifier.semantics { this.contentType = contentType }
                                } else {
                                    Modifier
                                },
                            ),
                    singleLine = singleLine,
                    maxLines = maxLines,
                    minLines = minLines,
                    textStyle =
                        TextStyle(
                            fontFamily = nunito,
                            fontWeight = FontWeight.Normal,
                            fontSize = 16.sp,
                            color = TextPrimary,
                        ),
                    keyboardOptions = keyboardOptions,
                    visualTransformation = visualTransformation,
                    cursorBrush = SolidColor(TextPrimary),
                    decorationBox = { innerTextField ->
                        Box {
                            if (value.isEmpty() && placeholder != null) {
                                Text(
                                    text = placeholder,
                                    fontFamily = nunito,
                                    fontWeight = FontWeight.Normal,
                                    fontSize = 16.sp,
                                    color = TextMuted,
                                )
                            }
                            innerTextField()
                        }
                    },
                )
                if (trailingContent != null) {
                    trailingContent()
                }
            }
        }

        if (error != null) {
            Text(
                text = error,
                fontFamily = nunito,
                fontWeight = FontWeight.Normal,
                fontSize = 12.sp,
                color = Error,
                modifier = Modifier.padding(start = 4.dp, top = 4.dp),
            )
        }
    }
}

@Suppress("LongParameterList")
@Composable
fun PoziomkiPasswordField(
    value: String,
    onValueChange: (String) -> Unit,
    modifier: Modifier = Modifier,
    label: String? = null,
    placeholder: String? = null,
    error: String? = null,
    keyboardOptions: KeyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Password),
    contentType: ContentType? = null,
) {
    var passwordVisible by remember { mutableStateOf(false) }

    PoziomkiTextField(
        value = value,
        onValueChange = onValueChange,
        modifier = modifier,
        label = label,
        placeholder = placeholder,
        error = error,
        keyboardOptions = keyboardOptions,
        visualTransformation =
            if (passwordVisible) {
                VisualTransformation.None
            } else {
                PasswordVisualTransformation()
            },
        contentType = contentType,
        trailingContent = {
            IconButton(onClick = { passwordVisible = !passwordVisible }) {
                Icon(
                    imageVector =
                        if (passwordVisible) {
                            PhosphorIcons.Bold.Eye
                        } else {
                            PhosphorIcons.Bold.EyeSlash
                        },
                    contentDescription = if (passwordVisible) "Hide password" else "Show password",
                    tint = TextMuted,
                )
            }
        },
    )
}
