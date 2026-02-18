package com.poziomki.app.ui.screen.profile

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import com.poziomki.app.api.ApiResult
import com.poziomki.app.api.ApiService
import com.poziomki.app.api.Tag
import com.poziomki.app.api.UpdateProfileRequest
import com.poziomki.app.data.repository.ProfileRepository
import com.poziomki.app.data.repository.TagRepository
import com.poziomki.app.ui.component.SnackbarType
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch

data class ProfileEditState(
    val isLoading: Boolean = false,
    val isSaving: Boolean = false,
    val isUploading: Boolean = false,
    val isBioImageUploading: Boolean = false,
    val profileId: String = "",
    val name: String = "",
    val bio: String = "",
    val program: String = "",
    val images: List<String> = emptyList(),
    val allTags: List<Tag> = emptyList(),
    val selectedTags: List<Tag> = emptyList(),
    val interestQuery: String = "",
    val activityQuery: String = "",
    val gradientStart: String? = null,
    val gradientEnd: String? = null,
    val snackbarMessage: String? = null,
    val snackbarType: SnackbarType = SnackbarType.ERROR,
)

class ProfileEditViewModel(
    private val profileRepository: ProfileRepository,
    private val tagRepository: TagRepository,
    private val apiService: ApiService,
) : ViewModel() {
    private val _state = MutableStateFlow(ProfileEditState())
    val state: StateFlow<ProfileEditState> = _state.asStateFlow()

    init {
        loadData()
    }

    private fun loadData() {
        viewModelScope.launch {
            _state.value = _state.value.copy(isLoading = true)

            // Observe tags from cache and trigger refresh
            launch {
                tagRepository.observeTags().collect { tags ->
                    _state.value = _state.value.copy(allTags = tags)
                }
            }
            tagRepository.refreshTags()

            // Load profile from cache or network
            profileRepository.refreshOwnProfile()
            profileRepository.observeOwnProfileWithTags().collect { profile ->
                if (profile != null) {
                    _state.value =
                        _state.value.copy(
                            isLoading = false,
                            profileId = profile.id,
                            name = profile.name,
                            bio = profile.bio ?: "",
                            program = profile.program ?: "",
                            images = profile.images,
                            selectedTags = profile.tags,
                            gradientStart = profile.gradientStart,
                            gradientEnd = profile.gradientEnd,
                        )
                }
            }
        }
    }

    fun updateBio(bio: String) {
        if (bio.length <= 1500) {
            _state.value = _state.value.copy(bio = bio)
        }
    }

    fun updateProgram(program: String) {
        _state.value = _state.value.copy(program = program)
    }

    fun clearProgram() {
        _state.value = _state.value.copy(program = "")
    }

    fun updateGradient(
        start: String?,
        end: String?,
    ) {
        _state.value = _state.value.copy(gradientStart = start, gradientEnd = end)
    }

    fun updateInterestQuery(query: String) {
        _state.value = _state.value.copy(interestQuery = query)
    }

    fun updateActivityQuery(query: String) {
        _state.value = _state.value.copy(activityQuery = query)
    }

    fun addTag(tag: Tag) {
        val current = _state.value.selectedTags
        if (current.none { it.id == tag.id }) {
            _state.value = _state.value.copy(selectedTags = current + tag)
        }
    }

    fun removeTag(tag: Tag) {
        _state.value =
            _state.value.copy(
                selectedTags = _state.value.selectedTags.filter { it.id != tag.id },
            )
    }

    fun removeImage(index: Int) {
        val current = _state.value.images.toMutableList()
        if (index in current.indices) {
            current.removeAt(index)
            _state.value = _state.value.copy(images = current)
        }
    }

    fun clearSnackbar() {
        _state.value = _state.value.copy(snackbarMessage = null)
    }

    fun uploadAndAddImage(bytes: ByteArray) {
        viewModelScope.launch {
            _state.value = _state.value.copy(isUploading = true)
            when (val result = apiService.uploadImage(bytes, "profile_image.jpg")) {
                is ApiResult.Success -> {
                    val current = _state.value.images
                    _state.value = _state.value.copy(images = current + result.data.url)
                }

                is ApiResult.Error -> {
                    _state.value =
                        _state.value.copy(
                            snackbarMessage = "nie uda\u0142o si\u0119 przes\u0142a\u0107 zdj\u0119cia",
                            snackbarType = SnackbarType.ERROR,
                        )
                }
            }
            _state.value = _state.value.copy(isUploading = false)
        }
    }

    fun uploadBioImage(bytes: ByteArray) {
        viewModelScope.launch {
            _state.value = _state.value.copy(isBioImageUploading = true)
            when (val result = apiService.uploadImage(bytes, "bio_image.jpg")) {
                is ApiResult.Success -> {
                    val marker = "![](${result.data.url})"
                    val currentBio = _state.value.bio
                    val newBio = if (currentBio.isBlank()) marker else "$currentBio\n$marker"
                    if (newBio.length <= 1500) {
                        _state.value = _state.value.copy(bio = newBio)
                    }
                }

                is ApiResult.Error -> {
                    _state.value =
                        _state.value.copy(
                            snackbarMessage = "nie uda\u0142o si\u0119 przes\u0142a\u0107 zdj\u0119cia",
                            snackbarType = SnackbarType.ERROR,
                        )
                }
            }
            _state.value = _state.value.copy(isBioImageUploading = false)
        }
    }

    fun save(onSuccess: () -> Unit) {
        viewModelScope.launch {
            _state.value = _state.value.copy(isSaving = true)
            val s = _state.value
            val request =
                UpdateProfileRequest(
                    bio = s.bio.ifBlank { null },
                    program = s.program.ifBlank { null },
                    profilePicture = s.images.firstOrNull(),
                    images = s.images,
                    tagIds = s.selectedTags.map { it.id },
                    gradientStart = s.gradientStart ?: "",
                    gradientEnd = s.gradientEnd ?: "",
                )
            when (profileRepository.updateProfile(s.profileId, request)) {
                is ApiResult.Success -> {
                    onSuccess()
                }

                is ApiResult.Error -> {
                    _state.value =
                        _state.value.copy(
                            snackbarMessage = "nie uda\u0142o si\u0119 zapisa\u0107 profilu",
                            snackbarType = SnackbarType.ERROR,
                        )
                }
            }
            _state.value = _state.value.copy(isSaving = false)
        }
    }
}
