package com.poziomki.app.ui.feature.onboarding

import androidx.compose.runtime.Composable
import androidx.compose.ui.window.Dialog
import androidx.compose.ui.window.DialogProperties
import com.poziomki.app.network.Tag
import com.poziomki.app.ui.designsystem.components.ProfileImage
import com.poziomki.app.ui.designsystem.components.ProfilePreview

@Suppress("LongParameterList")
@Composable
fun ProfilePreviewDialog(
    name: String,
    program: String,
    bio: String,
    tags: List<Tag>,
    galleryImages: List<ByteArray>,
    onDismiss: () -> Unit,
) {
    val images = galleryImages.map { ProfileImage.Bytes(it) }

    Dialog(
        onDismissRequest = onDismiss,
        properties =
            DialogProperties(
                usePlatformDefaultWidth = false,
                decorFitsSystemWindows = false,
            ),
    ) {
        ProfilePreview(
            name = name,
            program = program,
            bio = bio,
            tags = tags,
            images = images,
            onClose = onDismiss,
        )
    }
}
