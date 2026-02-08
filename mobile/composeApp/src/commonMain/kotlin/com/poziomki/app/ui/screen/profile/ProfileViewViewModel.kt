package com.poziomki.app.ui.screen.profile

import androidx.lifecycle.SavedStateHandle
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import androidx.navigation.toRoute
import com.poziomki.app.api.ApiResult
import com.poziomki.app.api.ApiService
import com.poziomki.app.api.ProfileWithTags
import com.poziomki.app.ui.navigation.Route
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch

data class ProfileViewState(
    val profile: ProfileWithTags? = null,
    val isLoading: Boolean = false,
    val error: String? = null,
)

class ProfileViewViewModel(
    savedStateHandle: SavedStateHandle,
    private val apiService: ApiService,
) : ViewModel() {
    private val route = savedStateHandle.toRoute<Route.ProfileView>()
    private val profileId = route.id

    private val _state = MutableStateFlow(ProfileViewState())
    val state: StateFlow<ProfileViewState> = _state.asStateFlow()

    init {
        loadProfile()
    }

    private fun loadProfile() {
        viewModelScope.launch {
            _state.value = ProfileViewState(isLoading = true)
            when (val result = apiService.getProfileFull(profileId)) {
                is ApiResult.Success -> {
                    _state.value = ProfileViewState(profile = result.data)
                }

                is ApiResult.Error -> {
                    _state.value = ProfileViewState(error = result.message)
                }
            }
        }
    }
}
