package com.poziomki.app.ui.screen.auth

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import com.poziomki.app.api.ApiResult
import com.poziomki.app.api.ApiService
import com.poziomki.app.session.SessionManager
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch

data class AuthUiState(
    val isLoading: Boolean = false,
    val error: String? = null,
)

class AuthViewModel(
    private val apiService: ApiService,
    private val sessionManager: SessionManager,
) : ViewModel() {
    private companion object {
        private const val HTTP_NOT_FOUND = 404
    }

    private val _uiState = MutableStateFlow(AuthUiState())
    val uiState: StateFlow<AuthUiState> = _uiState.asStateFlow()

    fun signIn(
        email: String,
        password: String,
        onSuccess: () -> Unit,
        onNeedsVerification: (String) -> Unit,
        onNeedsOnboarding: () -> Unit,
    ) {
        viewModelScope.launch {
            _uiState.value = AuthUiState(isLoading = true)
            when (val result = apiService.signIn(email, password)) {
                is ApiResult.Success -> {
                    val user = result.data.user
                    val token = result.data.token
                    if (user != null && !token.isNullOrBlank()) {
                        sessionManager.saveSession(
                            token = token,
                            userId = user.id,
                            email = user.email,
                            name = user.name,
                        )
                        // Check if user has completed onboarding (has a profile)
                        when (val profileResult = apiService.getMyProfile()) {
                            is ApiResult.Success -> {
                                sessionManager.saveProfileId(profileResult.data.id)
                                _uiState.value = AuthUiState()
                                onSuccess()
                            }

                            is ApiResult.Error -> {
                                if (profileResult.code == "NOT_FOUND" || profileResult.status == HTTP_NOT_FOUND) {
                                    _uiState.value = AuthUiState()
                                    onNeedsOnboarding()
                                } else {
                                    val cachedProfileId = sessionManager.getProfileId()
                                    if (cachedProfileId != null) {
                                        _uiState.value = AuthUiState()
                                        onSuccess()
                                    } else {
                                        _uiState.value = AuthUiState(error = profileResult.message)
                                    }
                                }
                            }
                        }
                    } else {
                        _uiState.value = AuthUiState(error = "No user data in response")
                    }
                }

                is ApiResult.Error -> {
                    if (result.code == "EMAIL_NOT_VERIFIED") {
                        _uiState.value = AuthUiState()
                        onNeedsVerification(email)
                    } else {
                        _uiState.value = AuthUiState(error = result.message)
                    }
                }
            }
        }
    }

    fun signUp(
        email: String,
        password: String,
        name: String,
        onSuccess: (String) -> Unit,
    ) {
        viewModelScope.launch {
            _uiState.value = AuthUiState(isLoading = true)
            when (val result = apiService.signUp(email, password, name)) {
                is ApiResult.Success -> {
                    val user = result.data.user
                    val token = result.data.token
                    if (user != null && !token.isNullOrBlank()) {
                        sessionManager.saveSession(
                            token = token,
                            userId = user.id,
                            email = user.email,
                            name = user.name,
                        )
                    }
                    _uiState.value = AuthUiState()
                    onSuccess(email)
                }

                is ApiResult.Error -> {
                    _uiState.value = AuthUiState(error = result.message)
                }
            }
        }
    }

    fun verifyOtp(
        email: String,
        otp: String,
        onSuccess: () -> Unit,
    ) {
        viewModelScope.launch {
            _uiState.value = AuthUiState(isLoading = true)
            when (val result = apiService.verifyOtp(email, otp)) {
                is ApiResult.Success -> {
                    val data = result.data
                    val user = data.user
                    val token = data.token
                    if (user != null && !token.isNullOrBlank()) {
                        sessionManager.saveSession(
                            token = token,
                            userId = user.id,
                            email = user.email,
                            name = user.name,
                        )
                    }
                    _uiState.value = AuthUiState()
                    onSuccess()
                }

                is ApiResult.Error -> {
                    _uiState.value = AuthUiState(error = result.message)
                }
            }
        }
    }

    fun resendOtp(email: String) {
        viewModelScope.launch {
            apiService.resendOtp(email)
        }
    }

    fun clearError() {
        _uiState.value = _uiState.value.copy(error = null)
    }
}
