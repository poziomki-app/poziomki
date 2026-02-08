package com.poziomki.app.ui.screen.profile

import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.asPaddingValues
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.navigationBars
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.Send
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.poziomki.app.ui.component.ProfileImage
import com.poziomki.app.ui.component.ProfilePreview
import com.poziomki.app.ui.theme.Border
import com.poziomki.app.ui.theme.NunitoFamily
import com.poziomki.app.ui.theme.Primary
import com.poziomki.app.util.isImageUrl
import org.koin.compose.viewmodel.koinViewModel

@Composable
fun ProfileViewScreen(
    onBack: () -> Unit,
    onNavigateToChat: (String) -> Unit,
    viewModel: ProfileViewViewModel = koinViewModel(),
) {
    val state by viewModel.state.collectAsState()

    when {
        state.isLoading -> {
            Box(Modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
                CircularProgressIndicator(color = Primary)
            }
        }

        state.profile != null -> {
            val p = state.profile!!
            val images =
                buildList {
                    p.profilePicture?.let { if (isImageUrl(it)) add(ProfileImage.Url(it)) }
                    p.images.filter { isImageUrl(it) }.forEach { add(ProfileImage.Url(it)) }
                }
            val emoji = p.profilePicture?.takeUnless { isImageUrl(it) }

            Box(Modifier.fillMaxSize()) {
                ProfilePreview(
                    name = p.name,
                    program = p.program,
                    bio = p.bio,
                    tags = p.tags,
                    images = images,
                    emojiAvatar = emoji,
                    onClose = onBack,
                )

                val bottomInsets = WindowInsets.navigationBars.asPaddingValues().calculateBottomPadding()

                Surface(
                    modifier =
                        Modifier
                            .align(Alignment.BottomEnd)
                            .padding(
                                end = 16.dp,
                                bottom = bottomInsets + 8.dp,
                            ),
                    shape = RoundedCornerShape(28.dp),
                    color = Color.Transparent,
                    border = BorderStroke(1.dp, Border),
                ) {
                    Row(
                        modifier =
                            Modifier
                                .background(
                                    Brush.verticalGradient(
                                        colors =
                                            listOf(
                                                Color(0xFF1A2029),
                                                Color(0xFF161B22),
                                            ),
                                    ),
                                ).clickable { onNavigateToChat(p.userId) }
                                .padding(horizontal = 20.dp, vertical = 14.dp),
                        verticalAlignment = Alignment.CenterVertically,
                    ) {
                        Icon(
                            imageVector = Icons.AutoMirrored.Filled.Send,
                            contentDescription = null,
                            tint = Primary,
                            modifier = Modifier.size(20.dp),
                        )
                        Spacer(Modifier.width(8.dp))
                        Text(
                            text = "Wiadomość",
                            fontFamily = NunitoFamily,
                            fontWeight = FontWeight.SemiBold,
                            fontSize = 15.sp,
                            color = Color.White,
                        )
                    }
                }
            }
        }

        else -> {
            Box(Modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
                Text(
                    text = state.error ?: "nie znaleziono profilu",
                    fontFamily = NunitoFamily,
                    color = MaterialTheme.colorScheme.error,
                )
            }
        }
    }
}
