package com.poziomki.app.ui.feature.onboarding

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import com.poziomki.app.data.repository.DegreeRepository
import com.poziomki.app.data.repository.TagRepository
import com.poziomki.app.network.ApiResult
import com.poziomki.app.network.ApiService
import com.poziomki.app.network.CreateProfileRequest
import com.poziomki.app.network.Degree
import com.poziomki.app.network.Tag
import com.poziomki.app.network.UpdateProfileRequest
import com.poziomki.app.session.SessionManager
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch
import kotlinx.serialization.Serializable
import kotlinx.serialization.encodeToString
import kotlinx.serialization.json.Json

data class OnboardingState(
    val name: String = "",
    val age: String = "",
    val program: String = "",
    val bio: String = "",
    val selectedTagIds: Set<String> = emptySet(),
    val availableTags: List<Tag> = emptyList(),
    val degreeSearchResults: List<Degree> = emptyList(),
    val selectedAvatar: String? = null,
    val avatarImageBytes: ByteArray? = null,
    val galleryImages: List<ByteArray> = emptyList(),
    val isLoading: Boolean = false,
    val error: String? = null,
)

class OnboardingViewModel(
    private val apiService: ApiService,
    private val sessionManager: SessionManager,
    private val tagRepository: TagRepository,
    private val degreeRepository: DegreeRepository,
) : ViewModel() {
    @Serializable
    private data class OnboardingDraft(
        val name: String,
        val age: String,
        val program: String,
        val bio: String,
        val selectedTagIds: Set<String>,
        val selectedAvatar: String?,
    )

    private data class MediaUploadResult(
        val avatarUrl: String?,
        val imageUrls: List<String>,
        val failures: Int,
    )

    private companion object {
        private const val MAX_MEDIA_BYTES = 900 * 1024
    }

    private val _state = MutableStateFlow(OnboardingState())
    val state: StateFlow<OnboardingState> = _state.asStateFlow()
    private val json = Json { ignoreUnknownKeys = true }

    private var allDegrees: List<Degree> = emptyList()
    private var createdProfileId: String? = null

    init {
        restoreDraft()
        loadTags()
        loadDegrees()
    }

    private fun restoreDraft() {
        viewModelScope.launch {
            val draftRaw = sessionManager.getOnboardingDraft() ?: return@launch
            val draft = runCatching { json.decodeFromString<OnboardingDraft>(draftRaw) }.getOrNull() ?: return@launch
            _state.value =
                _state.value.copy(
                    name = draft.name,
                    age = draft.age,
                    program = draft.program,
                    bio = draft.bio,
                    selectedTagIds = draft.selectedTagIds,
                    selectedAvatar = draft.selectedAvatar,
                )
            filterDegrees(draft.program)
        }
    }

    private fun persistDraft(state: OnboardingState) {
        val draft =
            OnboardingDraft(
                name = state.name,
                age = state.age,
                program = state.program,
                bio = state.bio,
                selectedTagIds = state.selectedTagIds,
                selectedAvatar = state.selectedAvatar,
            )
        viewModelScope.launch {
            sessionManager.saveOnboardingDraft(json.encodeToString(draft))
        }
    }

    private fun updateState(
        persist: Boolean = true,
        transform: (OnboardingState) -> OnboardingState,
    ) {
        val next = transform(_state.value)
        _state.value = next
        if (persist) persistDraft(next)
    }

    private fun loadTags() {
        viewModelScope.launch {
            tagRepository.ensureInterestSeedIfEmpty()
            // Observe cached tags
            launch {
                tagRepository.observeTags("interest").collect { tags ->
                    updateState(persist = false) { it.copy(availableTags = tags) }
                }
            }
            // Refresh from network
            tagRepository.refreshTags("interest")
        }
    }

    private fun loadDegrees() {
        viewModelScope.launch {
            degreeRepository.ensureLocalSeedIfEmpty()
            launch {
                degreeRepository.observeDegrees().collect { degrees ->
                    allDegrees = degrees
                    // Re-filter if there's an active query
                    val query = _state.value.program
                    if (query.isNotBlank()) {
                        filterDegrees(query)
                    }
                }
            }
            degreeRepository.refreshDegrees()
        }
    }

    private fun filterDegrees(query: String) {
        if (query.isBlank()) {
            updateState(persist = false) { it.copy(degreeSearchResults = emptyList()) }
            return
        }
        val filtered = allDegrees.filter { it.name.contains(query, ignoreCase = true) }
        updateState(persist = false) { it.copy(degreeSearchResults = filtered) }
    }

    fun updateName(name: String) {
        updateState { it.copy(name = name, error = null) }
    }

    fun updateAge(age: String) {
        updateState { it.copy(age = age, error = null) }
    }

    fun updateProgram(program: String) {
        updateState { it.copy(program = program, error = null) }
        filterDegrees(program)
    }

    fun updateBio(bio: String) {
        updateState { it.copy(bio = bio, error = null) }
    }

    fun selectAvatar(emoji: String) {
        updateState { it.copy(selectedAvatar = emoji, avatarImageBytes = null, error = null) }
    }

    fun setAvatarImage(bytes: ByteArray) {
        if (bytes.size > MAX_MEDIA_BYTES) {
            updateState { it.copy(error = "Avatar image is too large. Choose a smaller image.") }
            return
        }
        updateState { it.copy(avatarImageBytes = bytes, selectedAvatar = null, error = null) }
    }

    fun clearAvatar() {
        updateState { it.copy(selectedAvatar = null, avatarImageBytes = null, error = null) }
    }

    fun clearAll() {
        updateState {
            it.copy(
                selectedAvatar = null,
                avatarImageBytes = null,
                galleryImages = emptyList(),
                error = null,
            )
        }
    }

    fun addGalleryImages(images: List<ByteArray>) {
        val tooLarge = images.any { it.size > MAX_MEDIA_BYTES }
        if (tooLarge) {
            updateState { it.copy(error = "One or more photos are too large. Choose smaller images.") }
            return
        }
        val combined = (_state.value.galleryImages + images).take(6)
        updateState { it.copy(galleryImages = combined, error = null) }
    }

    fun removeGalleryImage(index: Int) {
        val current = _state.value.galleryImages.toMutableList()
        if (index in current.indices) {
            current.removeAt(index)
            updateState { it.copy(galleryImages = current, error = null) }
        }
    }

    fun toggleTag(tagId: String) {
        val current = _state.value.selectedTagIds
        updateState {
            it.copy(
                selectedTagIds = if (tagId in current) current - tagId else current + tagId,
                error = null,
            )
        }
    }

    fun createProfile(onComplete: () -> Unit) {
        val s = _state.value
        val ageInt = s.age.toIntOrNull() ?: 20

        viewModelScope.launch {
            updateState(persist = false) { it.copy(isLoading = true, error = null) }
            val profileId = ensureProfileExists(s, ageInt) ?: return@launch
            val mediaResult = uploadMedia(s)
            val mediaUpdated = updateProfileMedia(profileId, mediaResult.avatarUrl, mediaResult.imageUrls)
            if (!mediaUpdated) return@launch
            if (mediaResult.failures > 0) {
                val uploadError =
                    "Profile created, but ${mediaResult.failures} media upload(s) " +
                        "failed. Retry with smaller files or better network."
                updateState {
                    it.copy(
                        isLoading = false,
                        error = uploadError,
                    )
                }
                return@launch
            }

            sessionManager.saveOnboardingDraft(null)
            updateState(persist = false) { it.copy(isLoading = false, error = null) }
            onComplete()
        }
    }

    private suspend fun ensureProfileExists(
        state: OnboardingState,
        age: Int,
    ): String? {
        val existing = createdProfileId
        if (existing != null) return existing

        return when (
            val result =
                apiService.createProfile(
                    CreateProfileRequest(
                        name = state.name,
                        age = age,
                        bio = state.bio.ifBlank { null },
                        program = state.program.ifBlank { null },
                        tagIds = state.selectedTagIds.toList(),
                    ),
                )
        ) {
            is ApiResult.Success -> {
                val id = result.data.id
                createdProfileId = id
                sessionManager.saveProfileId(id)
                id
            }

            is ApiResult.Error -> {
                updateState { it.copy(isLoading = false, error = result.message) }
                null
            }
        }
    }

    private suspend fun uploadMedia(state: OnboardingState): MediaUploadResult {
        var avatarUrl: String? = state.selectedAvatar
        var failures = 0

        if (state.avatarImageBytes != null) {
            when (
                val result =
                    apiService.uploadImage(
                        state.avatarImageBytes,
                        "avatar.jpg",
                        "profile_picture",
                    )
            ) {
                is ApiResult.Success -> avatarUrl = result.data.url
                is ApiResult.Error -> failures++
            }
        }

        val imageUrls =
            state.galleryImages.mapIndexedNotNull { index, bytes ->
                when (val result = apiService.uploadImage(bytes, "photo_$index.jpg", "profile_gallery")) {
                    is ApiResult.Success -> {
                        result.data.url
                    }

                    is ApiResult.Error -> {
                        failures++
                        null
                    }
                }
            }

        return MediaUploadResult(
            avatarUrl = avatarUrl,
            imageUrls = imageUrls,
            failures = failures,
        )
    }

    private suspend fun updateProfileMedia(
        profileId: String,
        avatarUrl: String?,
        imageUrls: List<String>,
    ): Boolean {
        var success = true
        if (avatarUrl != null || imageUrls.isNotEmpty()) {
            val result =
                apiService.updateProfile(
                    profileId,
                    UpdateProfileRequest(
                        profilePicture = avatarUrl,
                        images = imageUrls.ifEmpty { null },
                    ),
                )
            if (result is ApiResult.Error) {
                updateState {
                    it.copy(
                        isLoading = false,
                        error = "Profile created, but media sync failed. Retry when online.",
                    )
                }
                success = false
            }
        }
        return success
    }

    fun clearError() {
        updateState { it.copy(error = null) }
    }
}
