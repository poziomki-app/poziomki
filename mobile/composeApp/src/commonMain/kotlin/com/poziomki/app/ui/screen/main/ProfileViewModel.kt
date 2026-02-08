package com.poziomki.app.ui.screen.main

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import com.poziomki.app.api.ApiService
import com.poziomki.app.api.Profile
import com.poziomki.app.api.Tag
import com.poziomki.app.data.CacheManager
import com.poziomki.app.data.repository.ProfileRepository
import com.poziomki.app.session.SessionManager
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch

data class ProfileState(
    val profile: Profile? = null,
    val tags: List<Tag> = emptyList(),
    val isLoading: Boolean = false,
    val error: String? = null,
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
                                ),
                            tags = profileWithTags.tags,
                            isLoading = false,
                        )
                }
            }
        }
    }

    fun loadProfile() {
        viewModelScope.launch {
            _state.value = _state.value.copy(isLoading = true)
            profileRepository.refreshOwnProfile()
        }
    }

    fun signOut() {
        viewModelScope.launch {
            apiService.signOut()
            cacheManager.clearAll()
            sessionManager.clearSession()
        }
    }
}
