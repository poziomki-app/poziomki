package com.poziomki.app.ui.feature.home

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import com.poziomki.app.data.CacheManager
import com.poziomki.app.data.repository.ProfileRepository
import com.poziomki.app.network.ApiService
import com.poziomki.app.network.Profile
import com.poziomki.app.network.Tag
import com.poziomki.app.session.SessionManager
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch

data class ProfileState(
    val profile: Profile? = null,
    val tags: List<Tag> = emptyList(),
    val isLoading: Boolean = true,
    val isRefreshing: Boolean = false,
    val error: String? = null,
    val refreshError: String? = null,
)

class ProfileViewModel(
    private val profileRepository: ProfileRepository,
    private val apiService: ApiService,
    private val sessionManager: SessionManager,
    private val cacheManager: CacheManager,
) : ViewModel() {
    private val _state = MutableStateFlow(ProfileState())
    val state: StateFlow<ProfileState> = _state.asStateFlow()

    init {
        observeProfile()
        loadProfile()
    }

    private fun observeProfile() {
        viewModelScope.launch {
            profileRepository.observeOwnProfileWithTags().collect { profileWithTags ->
                if (profileWithTags != null) {
                    _state.value =
                        _state.value.copy(
                            profile =
                                Profile(
                                    id = profileWithTags.id,
                                    userId = profileWithTags.userId,
                                    name = profileWithTags.name,
                                    bio = profileWithTags.bio,
                                    age = profileWithTags.age,
                                    profilePicture = profileWithTags.profilePicture,
                                    images = profileWithTags.images,
                                    program = profileWithTags.program,
                                    gradientStart = profileWithTags.gradientStart,
                                    gradientEnd = profileWithTags.gradientEnd,
                                ),
                            tags = profileWithTags.tags,
                            isLoading = false,
                        )
                }
            }
        }
    }

    fun loadProfile(showLoading: Boolean = false) {
        viewModelScope.launch {
            if (showLoading) {
                _state.value = _state.value.copy(isLoading = true)
            }
            val success = profileRepository.refreshOwnProfile()
            if (!success && _state.value.profile != null) {
                _state.value = _state.value.copy(refreshError = "Nie udało się odświeżyć profilu")
            }
            _state.value = _state.value.copy(isLoading = false)
        }
    }

    fun pullToRefresh() {
        viewModelScope.launch {
            _state.value = _state.value.copy(isRefreshing = true)
            val success = profileRepository.refreshOwnProfile(forceRefresh = true)
            if (!success && _state.value.profile != null) {
                _state.value = _state.value.copy(refreshError = "Nie udało się odświeżyć profilu")
            }
            _state.value = _state.value.copy(isRefreshing = false)
        }
    }

    fun clearRefreshError() {
        _state.value = _state.value.copy(refreshError = null)
    }

    fun signOut() {
        viewModelScope.launch {
            apiService.signOut()
            cacheManager.clearAll()
            sessionManager.clearSession()
        }
    }
}
