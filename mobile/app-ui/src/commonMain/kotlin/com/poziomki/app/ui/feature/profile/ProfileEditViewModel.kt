package com.poziomki.app.ui.feature.profile

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import com.poziomki.app.data.repository.ProfileRepository
import com.poziomki.app.data.repository.TagRepository
import com.poziomki.app.network.ApiResult
import com.poziomki.app.network.ApiService
import com.poziomki.app.network.Tag
import com.poziomki.app.network.UpdateProfileRequest
import com.poziomki.app.ui.designsystem.components.SnackbarType
import kotlinx.coroutines.FlowPreview
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.debounce
import kotlinx.coroutines.flow.distinctUntilChanged
import kotlinx.coroutines.flow.filter
import kotlinx.coroutines.launch
import kotlinx.serialization.json.JsonNull
import kotlinx.serialization.json.JsonPrimitive

data class ProfileEditState(
    val isLoading: Boolean = false,
    val isSaving: Boolean = false,
    val isUploading: Boolean = false,
    val isBioImageUploading: Boolean = false,
    val profileId: String = "",
    val name: String = "",
    val bio: String = "",
    val status: String = "",
    val program: String = "",
    val images: List<String> = emptyList(),
    val allTags: List<Tag> = emptyList(),
    val selectedTags: List<Tag> = emptyList(),
    val interestQuery: String = "",
    val activityQuery: String = "",
    val interestSearchResults: List<Tag> = emptyList(),
    val activitySearchResults: List<Tag> = emptyList(),
    val isSearchingInterests: Boolean = false,
    val isSearchingActivities: Boolean = false,
    val isCreatingTag: Boolean = false,
    val gradientStart: String? = null,
    val gradientEnd: String? = null,
    val snackbarMessage: String? = null,
    val snackbarType: SnackbarType = SnackbarType.ERROR,
)

@OptIn(FlowPreview::class)
class ProfileEditViewModel(
    private val profileRepository: ProfileRepository,
    private val tagRepository: TagRepository,
    private val apiService: ApiService,
) : ViewModel() {
    private val _state = MutableStateFlow(ProfileEditState())
    val state: StateFlow<ProfileEditState> = _state.asStateFlow()

    private val interestSearchFlow = MutableStateFlow("")
    private val activitySearchFlow = MutableStateFlow("")
    private val removedImages = mutableListOf<String>()

    init {
        loadData()
        setupDebouncedSearch()
    }

    private fun setupDebouncedSearch() {
        viewModelScope.launch {
            interestSearchFlow
                .debounce(300)
                .distinctUntilChanged()
                .collect { query ->
                    if (query.length >= 2) {
                        _state.value = _state.value.copy(isSearchingInterests = true)
                        val results = tagRepository.searchTags("interest", query)
                        _state.value =
                            _state.value.copy(
                                interestSearchResults = results,
                                isSearchingInterests = false,
                            )
                    } else {
                        _state.value =
                            _state.value.copy(
                                interestSearchResults = emptyList(),
                                isSearchingInterests = false,
                            )
                    }
                }
        }
        viewModelScope.launch {
            activitySearchFlow
                .debounce(300)
                .distinctUntilChanged()
                .collect { query ->
                    if (query.length >= 2) {
                        _state.value = _state.value.copy(isSearchingActivities = true)
                        val results = tagRepository.searchTags("activity", query)
                        _state.value =
                            _state.value.copy(
                                activitySearchResults = results,
                                isSearchingActivities = false,
                            )
                    } else {
                        _state.value =
                            _state.value.copy(
                                activitySearchResults = emptyList(),
                                isSearchingActivities = false,
                            )
                    }
                }
        }
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
            tagRepository.refreshTags("activity")

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
                            status = profile.status ?: "",
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
        val visibleLength = bio.replace(Regex("""!\[\]\([^)]*\)"""), "").length
        if (visibleLength <= 1500) {
            _state.value = _state.value.copy(bio = bio)
        }
    }

    fun updateProgram(program: String) {
        _state.value = _state.value.copy(program = program)
    }

    fun updateStatus(status: String) {
        if (status.length <= 160) {
            _state.value = _state.value.copy(status = status)
        }
    }

    fun clearStatus() {
        _state.value = _state.value.copy(status = "")
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
        interestSearchFlow.value = query.trim()
    }

    fun updateActivityQuery(query: String) {
        _state.value = _state.value.copy(activityQuery = query)
        activitySearchFlow.value = query.trim()
    }

    fun createAndAddTag(
        name: String,
        scope: String,
    ) {
        viewModelScope.launch {
            _state.value = _state.value.copy(isCreatingTag = true)
            when (val result = tagRepository.createTag(name.trim(), scope)) {
                is ApiResult.Success -> {
                    addTag(result.data)
                    if (scope == "interest") {
                        updateInterestQuery("")
                    } else {
                        updateActivityQuery("")
                    }
                }

                is ApiResult.Error -> {
                    if (result.code == "CONFLICT") {
                        // Tag already exists — try to find and add it
                        val existing = tagRepository.searchTags(scope, name.trim())
                        existing.firstOrNull { it.name.equals(name.trim(), ignoreCase = true) }?.let {
                            addTag(it)
                            if (scope == "interest") updateInterestQuery("") else updateActivityQuery("")
                        }
                    } else {
                        _state.value =
                            _state.value.copy(
                                snackbarMessage = "nie uda\u0142o si\u0119 utworzy\u0107 tagu",
                                snackbarType = SnackbarType.ERROR,
                            )
                    }
                }
            }
            _state.value = _state.value.copy(isCreatingTag = false)
        }
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
            removedImages.add(current[index])
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
                    _state.value = _state.value.copy(bio = newBio)
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
                    status = s.status.trim(),
                    program = s.program.ifBlank { null },
                    profilePicture = s.images.firstOrNull()?.let { JsonPrimitive(it) } ?: JsonNull,
                    images = s.images,
                    tagIds = s.selectedTags.map { it.id },
                    gradientStart = s.gradientStart ?: "",
                    gradientEnd = s.gradientEnd ?: "",
                )
            when (val result = profileRepository.updateProfile(s.profileId, request)) {
                is ApiResult.Success -> {
                    for (imageUrl in removedImages) {
                        val filename = imageUrl.substringAfterLast("/").substringBefore("?")
                        if (filename.isNotEmpty()) {
                            launch { apiService.deleteUpload(filename) }
                        }
                    }
                    removedImages.clear()
                    onSuccess()
                }

                is ApiResult.Error -> {
                    val message =
                        if (result.code == "BIO_CONTENT_REJECTED") {
                            // Server returns a Polish, category-aware
                            // sentence \u2014 surface it verbatim instead
                            // of the generic save-failed snackbar.
                            result.message
                        } else {
                            "nie uda\u0142o si\u0119 zapisa\u0107 profilu"
                        }
                    _state.value =
                        _state.value.copy(
                            snackbarMessage = message,
                            snackbarType = SnackbarType.ERROR,
                        )
                }
            }
            _state.value = _state.value.copy(isSaving = false)
        }
    }
}
