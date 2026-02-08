package com.poziomki.app.ui.screen.onboarding

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import com.poziomki.app.api.ApiResult
import com.poziomki.app.api.ApiService
import com.poziomki.app.api.CreateProfileRequest
import com.poziomki.app.api.Degree
import com.poziomki.app.api.Tag
import com.poziomki.app.api.UpdateProfileRequest
import com.poziomki.app.session.SessionManager
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch

data class OnboardingState(
    val name: String = "",
    val age: String = "",
    val program: String = "",
    val bio: String = "",
    val selectedTagIds: Set<String> = emptySet(),
    val availableTags: List<Tag> = emptyList(),
    val degrees: List<Degree> = emptyList(),
    val selectedAvatar: String? = null,
    val avatarImageBytes: ByteArray? = null,
    val galleryImages: List<ByteArray> = emptyList(),
    val isLoading: Boolean = false,
    val error: String? = null,
)

class OnboardingViewModel(
    private val apiService: ApiService,
    private val sessionManager: SessionManager,
) : ViewModel() {
    private val _state = MutableStateFlow(OnboardingState())
    val state: StateFlow<OnboardingState> = _state.asStateFlow()

    init {
        loadTags()
        loadDegrees()
    }

    private fun loadTags() {
        viewModelScope.launch {
            when (val result = apiService.getTags("interest")) {
                is ApiResult.Success -> {
                    _state.value = _state.value.copy(availableTags = result.data)
                }

                is ApiResult.Error -> {}
            }
        }
    }

    private fun loadDegrees() {
        viewModelScope.launch {
            when (val result = apiService.getDegrees()) {
                is ApiResult.Success -> {
                    _state.value = _state.value.copy(degrees = result.data)
                }

                is ApiResult.Error -> {}
            }
        }
    }

    fun updateName(name: String) {
        _state.value = _state.value.copy(name = name)
    }

    fun updateAge(age: String) {
        _state.value = _state.value.copy(age = age)
    }

    fun updateProgram(program: String) {
        _state.value = _state.value.copy(program = program)
    }

    fun updateBio(bio: String) {
        _state.value = _state.value.copy(bio = bio)
    }

    fun selectAvatar(emoji: String) {
        _state.value = _state.value.copy(selectedAvatar = emoji, avatarImageBytes = null)
    }

    fun setAvatarImage(bytes: ByteArray) {
        _state.value = _state.value.copy(avatarImageBytes = bytes, selectedAvatar = null)
    }

    fun clearAvatar() {
        _state.value = _state.value.copy(selectedAvatar = null, avatarImageBytes = null)
    }

    fun clearAll() {
        _state.value =
            _state.value.copy(
                selectedAvatar = null,
                avatarImageBytes = null,
                galleryImages = emptyList(),
            )
    }

    fun addGalleryImages(images: List<ByteArray>) {
        val current = _state.value.galleryImages
        val combined = (current + images).take(6)
        _state.value = _state.value.copy(galleryImages = combined)
    }

    fun removeGalleryImage(index: Int) {
        val current = _state.value.galleryImages.toMutableList()
        if (index in current.indices) {
            current.removeAt(index)
            _state.value = _state.value.copy(galleryImages = current)
        }
    }

    fun toggleTag(tagId: String) {
        val current = _state.value.selectedTagIds
        _state.value =
            _state.value.copy(
                selectedTagIds = if (tagId in current) current - tagId else current + tagId,
            )
    }

    fun createProfile(onComplete: () -> Unit) {
        val s = _state.value
        val ageInt = s.age.toIntOrNull() ?: return

        viewModelScope.launch {
            _state.value = s.copy(isLoading = true)
            val request =
                CreateProfileRequest(
                    name = s.name,
                    age = ageInt,
                    bio = s.bio.ifBlank { null },
                    program = s.program.ifBlank { null },
                    tagIds = s.selectedTagIds.toList(),
                )
            when (val result = apiService.createProfile(request)) {
                is ApiResult.Success -> {
                    val profileId = result.data.id
                    sessionManager.saveProfileId(profileId)

                    // Upload avatar image if selected from gallery
                    var avatarUrl: String? = s.selectedAvatar
                    if (s.avatarImageBytes != null) {
                        val uploadResult =
                            apiService.uploadImage(
                                s.avatarImageBytes,
                                "avatar.jpg",
                                "profile_picture",
                            )
                        if (uploadResult is ApiResult.Success) {
                            avatarUrl = uploadResult.data.url
                        }
                    }

                    // Upload gallery images
                    val imageUrls =
                        s.galleryImages.mapIndexedNotNull { i, bytes ->
                            when (val r = apiService.uploadImage(bytes, "photo_$i.jpg", "profile_gallery")) {
                                is ApiResult.Success -> r.data.url
                                is ApiResult.Error -> null
                            }
                        }

                    // Update profile with avatar and images
                    if (avatarUrl != null || imageUrls.isNotEmpty()) {
                        apiService.updateProfile(
                            profileId,
                            UpdateProfileRequest(
                                profilePicture = avatarUrl,
                                images = imageUrls.ifEmpty { null },
                            ),
                        )
                    }

                    _state.value = _state.value.copy(isLoading = false)
                    onComplete()
                }

                is ApiResult.Error -> {
                    _state.value = _state.value.copy(isLoading = false, error = result.message)
                }
            }
        }
    }

    fun clearError() {
        _state.value = _state.value.copy(error = null)
    }
}
