package com.poziomki.app.ui.screen.main

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import com.poziomki.app.api.ApiResult
import com.poziomki.app.api.ApiService
import com.poziomki.app.api.Profile
import com.poziomki.app.api.Tag
import com.poziomki.app.session.SessionManager
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.launch

data class ProfileState(
    val profile: Profile? = null,
    val tags: List<Tag> = emptyList(),
    val isLoading: Boolean = false,
    val error: String? = null,
)

class ProfileViewModel(
    private val apiService: ApiService,
    private val sessionManager: SessionManager,
) : ViewModel() {
    private val _state = MutableStateFlow(ProfileState())
    val state: StateFlow<ProfileState> = _state.asStateFlow()

    init {
        loadProfile()
    }

    fun loadProfile() {
        viewModelScope.launch {
            _state.value = _state.value.copy(isLoading = true)
            when (val result = apiService.getMyProfile()) {
                is ApiResult.Success -> {
                    sessionManager.saveProfileId(result.data.id)
                    // Load tags separately via full profile
                    val fullResult = apiService.getProfileFull(result.data.id)
                    val tags = if (fullResult is ApiResult.Success) fullResult.data.tags else emptyList()
                    _state.value = ProfileState(profile = result.data, tags = tags)
                }

                is ApiResult.Error -> {
                    _state.value = ProfileState(error = result.message)
                }
            }
        }
    }

    fun signOut() {
        viewModelScope.launch {
            apiService.signOut()
            sessionManager.clearSession()
        }
    }
}
