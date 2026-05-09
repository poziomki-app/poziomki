package com.poziomki.app.ui.feature.profile

import androidx.lifecycle.SavedStateHandle
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import androidx.navigation.toRoute
import com.poziomki.app.data.repository.ProfileRepository
import com.poziomki.app.data.repository.SettingsRepository
import com.poziomki.app.network.ApiResult
import com.poziomki.app.network.ApiService
import com.poziomki.app.network.ProfileWithTags
import com.poziomki.app.session.SessionManager
import com.poziomki.app.ui.navigation.Route
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch

data class ProfileViewState(
    val profile: ProfileWithTags? = null,
    val isLoading: Boolean = true,
    val isOwnProfile: Boolean = false,
    val isBookmarked: Boolean = false,
    val showOwnProgram: Boolean = true,
    val ownTagIds: Set<String> = emptySet(),
)

class ProfileViewViewModel(
    savedStateHandle: SavedStateHandle,
    private val profileRepository: ProfileRepository,
    private val sessionManager: SessionManager,
    private val settingsRepository: SettingsRepository,
    private val apiService: ApiService,
) : ViewModel() {
    private val route = savedStateHandle.toRoute<Route.ProfileView>()
    private val profileId = route.id

    private val _state = MutableStateFlow(ProfileViewState())
    val state: StateFlow<ProfileViewState> = _state.asStateFlow()

    private var bookmarkInFlight = false

    init {
        observeProfile()
        refreshProfile()
        checkOwnProfile()
        observeOwnTags()
    }

    private fun observeOwnTags() {
        viewModelScope.launch {
            val ownProfileId = sessionManager.profileId.first() ?: return@launch
            profileRepository.observeProfile(ownProfileId).collect { ownProfile ->
                if (ownProfile != null) {
                    _state.update {
                        it.copy(ownTagIds = ownProfile.tags.mapTo(mutableSetOf()) { t -> t.id })
                    }
                }
            }
        }
    }

    private fun checkOwnProfile() {
        viewModelScope.launch {
            val ownProfileId = sessionManager.profileId.first()
            if (ownProfileId == profileId) {
                _state.update { it.copy(isOwnProfile = true) }
                observeOwnPrivacy()
            }
        }
    }

    private fun observeOwnPrivacy() {
        viewModelScope.launch {
            val userId = sessionManager.userId.first() ?: return@launch
            settingsRepository.observeSettings(userId).collect { settings ->
                val show = settings?.privacy_show_program?.let { it != 0L } ?: true
                _state.update { it.copy(showOwnProgram = show) }
            }
        }
    }

    private fun observeProfile() {
        viewModelScope.launch {
            profileRepository.observeProfile(profileId).collect { profile ->
                if (profile != null) {
                    _state.update {
                        it.copy(
                            profile = profile,
                            isLoading = false,
                            isBookmarked = profile.isBookmarked,
                        )
                    }
                }
            }
        }
    }

    private fun refreshProfile() {
        viewModelScope.launch {
            val success = profileRepository.refreshProfile(profileId)
            if (!success) {
                _state.update { it.copy(isLoading = false) }
            }
        }
    }

    fun toggleBookmark() {
        if (bookmarkInFlight) return
        bookmarkInFlight = true
        val current = _state.value.isBookmarked
        val newValue = !current
        viewModelScope.launch {
            // Optimistic local update
            profileRepository.updateBookmarked(profileId, newValue)
            val result =
                if (current) {
                    apiService.unbookmarkProfile(profileId)
                } else {
                    apiService.bookmarkProfile(profileId)
                }
            if (result is ApiResult.Error) {
                // Revert on failure
                profileRepository.updateBookmarked(profileId, current)
            }
            bookmarkInFlight = false
        }
    }
}
