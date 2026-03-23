package com.poziomki.app.ui.feature.auth

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import com.poziomki.app.network.ApiResult
import com.poziomki.app.network.ApiService
import com.poziomki.app.session.SessionManager
import kotlinx.coroutines.Job
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch

data class AuthUiState(
    val isLoading: Boolean = false,
    val error: String? = null,
    val otpResent: Boolean = false,
    val resendCooldownSeconds: Int = 0,
)

@Suppress("TooManyFunctions")
class AuthViewModel(
    private val apiService: ApiService,
    private val sessionManager: SessionManager,
) : ViewModel() {
    private companion object {
        private const val HTTP_NOT_FOUND = 404
        private const val RESEND_COOLDOWN_SECONDS = 30
    }

    private val _uiState = MutableStateFlow(AuthUiState())
    val uiState: StateFlow<AuthUiState> = _uiState.asStateFlow()

    private var cooldownJob: Job? = null
    private var pendingResetToken: String? = null

    private fun localizeAuthError(
        code: String,
        message: String,
    ): String =
        when {
            code == "CONFLICT" -> {
                "Masz ju\u017c konto \u2014 zaloguj si\u0119"
            }

            code == "UNAUTHORIZED" || message.contains("Authentication failed", ignoreCase = true) -> {
                "Nieprawid\u0142owy email lub has\u0142o"
            }

            code == "VALIDATION_ERROR" && message.contains("verification code", ignoreCase = true) -> {
                "Nieprawid\u0142owy kod weryfikacyjny"
            }

            code == "NETWORK_ERROR" -> {
                "Brak po\u0142\u0105czenia z internetem"
            }

            else -> {
                message
            }
        }

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
                                        _uiState.value =
                                            AuthUiState(
                                                error = localizeAuthError(profileResult.code, profileResult.message),
                                            )
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
                        _uiState.value =
                            AuthUiState(
                                error = localizeAuthError(result.code, result.message),
                            )
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
        onUserExists: (String) -> Unit = {},
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
                    if (result.code == "CONFLICT") {
                        _uiState.value = AuthUiState()
                        onUserExists(email)
                    } else {
                        _uiState.value =
                            AuthUiState(
                                error = localizeAuthError(result.code, result.message),
                            )
                    }
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
            _uiState.value = _uiState.value.copy(isLoading = true, error = null)
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
                    _uiState.value = _uiState.value.copy(isLoading = false, error = null)
                    onSuccess()
                }

                is ApiResult.Error -> {
                    _uiState.value =
                        _uiState.value.copy(
                            isLoading = false,
                            error = localizeAuthError(result.code, result.message),
                        )
                }
            }
        }
    }

    fun resendOtp(email: String) {
        if (_uiState.value.resendCooldownSeconds > 0) return

        viewModelScope.launch {
            when (apiService.resendOtp(email)) {
                is ApiResult.Success -> {
                    _uiState.value = _uiState.value.copy(otpResent = true)
                    // Auto-clear confirmation after 3s
                    launch {
                        delay(3_000)
                        _uiState.value = _uiState.value.copy(otpResent = false)
                    }
                }

                is ApiResult.Error -> { /* silently fail — user can retry */ }
            }
            // Start 30s cooldown
            startResendCooldown()
        }
    }

    private fun startResendCooldown() {
        cooldownJob?.cancel()
        cooldownJob =
            viewModelScope.launch {
                for (seconds in RESEND_COOLDOWN_SECONDS downTo 1) {
                    _uiState.value = _uiState.value.copy(resendCooldownSeconds = seconds)
                    delay(1_000)
                }
                _uiState.value = _uiState.value.copy(resendCooldownSeconds = 0)
            }
    }

    fun clearError() {
        _uiState.value = _uiState.value.copy(error = null)
    }

    fun forgotPassword(
        email: String,
        onSuccess: () -> Unit,
    ) {
        viewModelScope.launch {
            _uiState.value = AuthUiState(isLoading = true)
            when (val result = apiService.forgotPassword(email)) {
                is ApiResult.Success -> {
                    _uiState.value = AuthUiState()
                    onSuccess()
                }

                is ApiResult.Error -> {
                    _uiState.value =
                        AuthUiState(error = localizeAuthError(result.code, result.message))
                }
            }
        }
    }

    fun forgotPasswordVerify(
        email: String,
        otp: String,
        onSuccess: (String) -> Unit,
    ) {
        viewModelScope.launch {
            _uiState.value = _uiState.value.copy(isLoading = true, error = null)
            when (val result = apiService.forgotPasswordVerify(email, otp)) {
                is ApiResult.Success -> {
                    pendingResetToken = result.data.resetToken
                    _uiState.value = _uiState.value.copy(isLoading = false, error = null)
                    onSuccess(result.data.resetToken)
                }

                is ApiResult.Error -> {
                    _uiState.value =
                        _uiState.value.copy(
                            isLoading = false,
                            error = localizeAuthError(result.code, result.message),
                        )
                }
            }
        }
    }

    fun forgotPasswordResend(email: String) {
        if (_uiState.value.resendCooldownSeconds > 0) return

        viewModelScope.launch {
            when (apiService.forgotPasswordResend(email)) {
                is ApiResult.Success -> {
                    _uiState.value = _uiState.value.copy(otpResent = true)
                    launch {
                        delay(3_000)
                        _uiState.value = _uiState.value.copy(otpResent = false)
                    }
                }

                is ApiResult.Error -> { /* silently fail */ }
            }
            startResendCooldown()
        }
    }

    @Suppress("CyclomaticComplexMethod")
    fun resetPassword(
        email: String,
        newPassword: String,
        onSuccess: () -> Unit,
        onNeedsOnboarding: () -> Unit,
    ) {
        val resetToken = pendingResetToken ?: return
        viewModelScope.launch {
            pendingResetToken = null
            _uiState.value = AuthUiState(isLoading = true)
            when (val result = apiService.resetPassword(email, resetToken, newPassword)) {
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
                        when (val profileResult = apiService.getMyProfile()) {
                            is ApiResult.Success -> {
                                sessionManager.saveProfileId(profileResult.data.id)
                                _uiState.value = AuthUiState()
                                onSuccess()
                            }

                            is ApiResult.Error -> {
                                if (profileResult.code == "NOT_FOUND" ||
                                    profileResult.status == HTTP_NOT_FOUND
                                ) {
                                    _uiState.value = AuthUiState()
                                    onNeedsOnboarding()
                                } else {
                                    _uiState.value = AuthUiState()
                                    onSuccess()
                                }
                            }
                        }
                    } else {
                        _uiState.value = AuthUiState(error = "No user data in response")
                    }
                }

                is ApiResult.Error -> {
                    _uiState.value =
                        AuthUiState(error = localizeAuthError(result.code, result.message))
                }
            }
        }
    }
}
