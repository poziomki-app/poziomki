package com.poziomki.app.ui.screen.profile

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import com.poziomki.app.api.ApiResult
import com.poziomki.app.api.ApiService
import com.poziomki.app.api.Tag
import com.poziomki.app.api.UpdateProfileRequest
import com.poziomki.app.session.SessionManager
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.launch

data class ProfileEditState(
    val isLoading: Boolean = false,
    val isSaving: Boolean = false,
    val isUploading: Boolean = false,
    val profileId: String = "",
    val bio: String = "",
    val program: String = "",
    val images: List<String> = emptyList(),
    val allTags: List<Tag> = emptyList(),
    val selectedTags: List<Tag> = emptyList(),
    val interestQuery: String = "",
    val activityQuery: String = "",
)

class ProfileEditViewModel(
    private val apiService: ApiService,
    private val sessionManager: SessionManager,
) : ViewModel() {
    private val _state = MutableStateFlow(ProfileEditState())
    val state: StateFlow<ProfileEditState> = _state.asStateFlow()

    init {
        loadData()
    }

    private fun loadData() {
        viewModelScope.launch {
            _state.value = _state.value.copy(isLoading = true)
            when (val result = apiService.getMyProfile()) {
                is ApiResult.Success -> {
                    val profile = result.data
                    val fullResult = apiService.getProfileFull(profile.id)
                    val tags = if (fullResult is ApiResult.Success) fullResult.data.tags else emptyList()
                    val allTagsResult = apiService.getTags()
                    val allTags = if (allTagsResult is ApiResult.Success) allTagsResult.data else emptyList()

                    _state.value =
                        _state.value.copy(
                            isLoading = false,
                            profileId = profile.id,
                            bio = profile.bio ?: "",
                            program = profile.program ?: "",
                            images = profile.images,
                            selectedTags = tags,
                            allTags = allTags,
                        )
                }

                is ApiResult.Error -> {
                    _state.value = _state.value.copy(isLoading = false)
                }
            }
        }
    }

    fun updateBio(bio: String) {
        if (bio.length <= 500) {
            _state.value = _state.value.copy(bio = bio)
        }
    }

    fun updateProgram(program: String) {
        _state.value = _state.value.copy(program = program)
    }

    fun clearProgram() {
        _state.value = _state.value.copy(program = "")
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

    fun uploadAndAddImage(bytes: ByteArray) {
        viewModelScope.launch {
            _state.value = _state.value.copy(isUploading = true)
            println("POZIOMKI_UPLOAD: starting upload, bytes=${bytes.size}")
            when (val result = apiService.uploadImage(bytes, "profile_image.jpg")) {
                is ApiResult.Success -> {
                    println("POZIOMKI_UPLOAD: success, url=${result.data.url}")
                    val current = _state.value.images
                    _state.value = _state.value.copy(images = current + result.data.url)
                }

                is ApiResult.Error -> {
                    println("POZIOMKI_UPLOAD: error=${result.message} code=${result.code} status=${result.status}")
                }
            }
            _state.value = _state.value.copy(isUploading = false)
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
                    images = s.images,
                    tagIds = s.selectedTags.map { it.id },
                )
            when (apiService.updateProfile(s.profileId, request)) {
                is ApiResult.Success -> {
                    onSuccess()
                }

                is ApiResult.Error -> {}
            }
            _state.value = _state.value.copy(isSaving = false)
        }
    }
}
