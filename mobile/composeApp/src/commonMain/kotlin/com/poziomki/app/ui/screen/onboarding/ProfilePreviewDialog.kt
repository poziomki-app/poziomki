package com.poziomki.app.ui.screen.onboarding

import androidx.compose.runtime.Composable
import androidx.compose.ui.window.Dialog
import androidx.compose.ui.window.DialogProperties
import com.poziomki.app.api.Tag
import com.poziomki.app.ui.component.ProfileImage
import com.poziomki.app.ui.component.ProfilePreview

@Composable
fun ProfilePreviewDialog(
    name: String,
    program: String,
    bio: String,
    tags: List<Tag>,
    selectedAvatar: String?,
    avatarImageBytes: ByteArray?,
    galleryImages: List<ByteArray>,
    onDismiss: () -> Unit,
) {
    val images =
        buildList {
            if (avatarImageBytes != null) add(ProfileImage.Bytes(avatarImageBytes))
            galleryImages.forEach { add(ProfileImage.Bytes(it)) }
        }

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
            emojiAvatar = selectedAvatar,
            onClose = onDismiss,
        )
    }
}
