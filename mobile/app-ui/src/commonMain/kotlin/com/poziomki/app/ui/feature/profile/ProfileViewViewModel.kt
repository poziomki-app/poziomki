package com.poziomki.app.ui.feature.profile

import androidx.lifecycle.SavedStateHandle
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import androidx.navigation.toRoute
import com.poziomki.app.data.repository.ProfileRepository
import com.poziomki.app.network.ProfileWithTags
import com.poziomki.app.session.SessionManager
import com.poziomki.app.ui.navigation.Route
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.launch

data class ProfileViewState(
    val profile: ProfileWithTags? = null,
    val isLoading: Boolean = false,
    val error: String? = null,
    val isOwnProfile: Boolean = false,
)

class ProfileViewViewModel(
    savedStateHandle: SavedStateHandle,
    private val profileRepository: ProfileRepository,
    private val sessionManager: SessionManager,
) : ViewModel() {
    private val route = savedStateHandle.toRoute<Route.ProfileView>()
    private val profileId = route.id

    private val _state = MutableStateFlow(ProfileViewState())
    val state: StateFlow<ProfileViewState> = _state.asStateFlow()

    init {
        observeProfile()
        refreshProfile()
        checkOwnProfile()
    }

    private fun checkOwnProfile() {
        viewModelScope.launch {
            val ownProfileId = sessionManager.profileId.first()
            if (ownProfileId == profileId) {
                _state.value = _state.value.copy(isOwnProfile = true)
            }
        }
    }

    private fun observeProfile() {
        viewModelScope.launch {
            profileRepository.observeProfile(profileId).collect { profile ->
                if (profile != null) {
                    _state.value = _state.value.copy(profile = profile, isLoading = false)
                }
            }
        }
    }

    private fun refreshProfile() {
        viewModelScope.launch {
            _state.value = ProfileViewState(isLoading = true)
            profileRepository.refreshProfile(profileId)
        }
    }
}
