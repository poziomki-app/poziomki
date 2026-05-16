package com.poziomki.app.ui.feature.profile

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import com.poziomki.app.data.CacheManager
import com.poziomki.app.data.repository.SettingsRepository
import com.poziomki.app.network.ApiResult
import com.poziomki.app.network.ApiService
import com.poziomki.app.session.AppPreferences
import com.poziomki.app.session.SessionManager
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.flow.launchIn
import kotlinx.coroutines.flow.onEach
import kotlinx.coroutines.launch

data class PrivacyState(
    val isExporting: Boolean = false,
    val isDeleting: Boolean = false,
    val isChangingPassword: Boolean = false,
    val exportBytes: ByteArray? = null,
    val exportSuccess: Boolean = false,
    val passwordSuccessMessage: String? = null,
    val error: String? = null,
    val discoverable: Boolean = true,
    val showProgram: Boolean = true,
    val screenshotsAllowed: Boolean = true,
    val currentEmail: String? = null,
    val pendingNewEmail: String? = null,
    val isRequestingEmailOtp: Boolean = false,
    val isConfirmingEmail: Boolean = false,
    val emailOtpSent: Boolean = false,
    val emailChangeError: String? = null,
    val emailChangeSuccess: String? = null,
)

@Suppress("TooManyFunctions")
class PrivacyViewModel(
    private val apiService: ApiService,
    private val sessionManager: SessionManager,
    private val cacheManager: CacheManager,
    private val settingsRepository: SettingsRepository,
    private val appPreferences: AppPreferences,
) : ViewModel() {
    private val _state = MutableStateFlow(PrivacyState())
    val state: StateFlow<PrivacyState> = _state.asStateFlow()

    init {
        viewModelScope.launch {
            val userId = sessionManager.userId.first() ?: return@launch
            settingsRepository.ensureDefaults(userId)
            val settings = settingsRepository.observeSettings(userId).first()
            if (settings != null) {
                _state.value =
                    _state.value.copy(
                        discoverable = settings.privacy_discoverable != 0L,
                        showProgram = settings.privacy_show_program != 0L,
                    )
            }
        }
        sessionManager.email
            .onEach { email ->
                _state.value = _state.value.copy(currentEmail = email)
            }.launchIn(viewModelScope)
        appPreferences.screenshotsAllowed
            .onEach { allowed ->
                _state.value = _state.value.copy(screenshotsAllowed = allowed)
            }.launchIn(viewModelScope)
    }

    fun toggleScreenshotsAllowed(enabled: Boolean) {
        _state.value = _state.value.copy(screenshotsAllowed = enabled)
        viewModelScope.launch {
            appPreferences.setScreenshotsAllowed(enabled)
        }
    }

    fun toggleDiscoverable(enabled: Boolean) {
        _state.value = _state.value.copy(discoverable = enabled)
        viewModelScope.launch {
            val userId = sessionManager.userId.first() ?: return@launch
            settingsRepository.updatePrivacy(userId, discoverable = enabled)
        }
    }

    fun toggleShowProgram(enabled: Boolean) {
        _state.value = _state.value.copy(showProgram = enabled)
        viewModelScope.launch {
            val userId = sessionManager.userId.first() ?: return@launch
            settingsRepository.updatePrivacy(userId, showProgram = enabled)
        }
    }

    fun exportData() {
        if (_state.value.isExporting || _state.value.exportSuccess || _state.value.exportBytes != null) return
        viewModelScope.launch {
            _state.value = _state.value.copy(isExporting = true, error = null)
            when (val result = apiService.exportData()) {
                is ApiResult.Success -> {
                    if (result.data.isEmpty()) {
                        _state.value =
                            _state.value.copy(
                                isExporting = false,
                                error = "Nie udało się pobrać danych",
                            )
                    } else {
                        _state.value =
                            _state.value.copy(
                                isExporting = false,
                                exportBytes = result.data,
                            )
                    }
                }

                is ApiResult.Error -> {
                    _state.value =
                        _state.value.copy(
                            isExporting = false,
                            error = "Nie udało się wyeksportować danych",
                        )
                }
            }
        }
    }

    fun onExportSaved() {
        _state.value = _state.value.copy(exportBytes = null, exportSuccess = true)
    }

    fun clearExportBytes() {
        _state.value = _state.value.copy(exportBytes = null)
    }

    fun changePassword(
        currentPassword: String,
        newPassword: String,
        confirmPassword: String,
        onPasswordChanged: () -> Unit,
    ) {
        when {
            currentPassword.isBlank() || newPassword.isBlank() || confirmPassword.isBlank() -> {
                _state.value =
                    _state.value.copy(
                        error = "Uzupełnij wszystkie pola hasła.",
                        passwordSuccessMessage = null,
                    )
                return
            }

            newPassword.length < 8 -> {
                _state.value =
                    _state.value.copy(
                        error = "Nowe hasło musi mieć co najmniej 8 znaków.",
                        passwordSuccessMessage = null,
                    )
                return
            }

            newPassword != confirmPassword -> {
                _state.value =
                    _state.value.copy(
                        error = "Nowe hasła nie są takie same.",
                        passwordSuccessMessage = null,
                    )
                return
            }
        }

        viewModelScope.launch {
            _state.value =
                _state.value.copy(
                    isChangingPassword = true,
                    error = null,
                    passwordSuccessMessage = null,
                )
            when (apiService.changePassword(currentPassword, newPassword)) {
                is ApiResult.Success -> {
                    _state.value =
                        _state.value.copy(
                            isChangingPassword = false,
                            passwordSuccessMessage = "Hasło zostało zmienione.",
                        )
                    onPasswordChanged()
                }

                is ApiResult.Error -> {
                    _state.value =
                        _state.value.copy(
                            isChangingPassword = false,
                            error = "Nie udało się zmienić hasła. Sprawdź aktualne hasło.",
                        )
                }
            }
        }
    }

    fun deleteAccount(
        password: String,
        onDeleted: () -> Unit,
    ) {
        if (password.isBlank()) return
        viewModelScope.launch {
            _state.value = _state.value.copy(isDeleting = true, error = null)
            when (apiService.deleteAccount(password)) {
                is ApiResult.Success -> {
                    cacheManager.clearAll()
                    sessionManager.clearSession()
                    _state.value = _state.value.copy(isDeleting = false)
                    onDeleted()
                }

                is ApiResult.Error -> {
                    _state.value =
                        _state.value.copy(
                            isDeleting = false,
                            error = "Nie udało się usunąć konta. Sprawdź hasło.",
                        )
                }
            }
        }
    }

    fun clearError() {
        _state.value = _state.value.copy(error = null)
    }

    fun clearPasswordSuccess() {
        _state.value = _state.value.copy(passwordSuccessMessage = null)
    }

    fun requestEmailChange(
        newEmail: String,
        currentPassword: String,
    ) {
        val trimmed = newEmail.trim().lowercase()
        if (trimmed.isBlank() || !trimmed.contains('@') || !trimmed.substringAfter('@').contains('.')) {
            _state.value = _state.value.copy(emailChangeError = "Nieprawidłowy adres email")
            return
        }
        if (currentPassword.isBlank()) {
            _state.value = _state.value.copy(emailChangeError = "Wpisz aktualne hasło")
            return
        }
        viewModelScope.launch {
            _state.value =
                _state.value.copy(
                    isRequestingEmailOtp = true,
                    emailChangeError = null,
                    pendingNewEmail = trimmed,
                )
            when (val result = apiService.requestEmailChange(trimmed, currentPassword)) {
                is ApiResult.Success -> {
                    _state.value =
                        _state.value.copy(
                            isRequestingEmailOtp = false,
                            emailOtpSent = true,
                            emailChangeError = null,
                        )
                }

                is ApiResult.Error -> {
                    _state.value =
                        _state.value.copy(
                            isRequestingEmailOtp = false,
                            emailOtpSent = false,
                            pendingNewEmail = null,
                            emailChangeError = mapEmailChangeError(result),
                        )
                }
            }
        }
    }

    fun confirmEmailChange(code: String) {
        val newEmail = _state.value.pendingNewEmail ?: return
        if (code.length != 6) {
            _state.value = _state.value.copy(emailChangeError = "Wpisz 6-cyfrowy kod")
            return
        }
        viewModelScope.launch {
            _state.value = _state.value.copy(isConfirmingEmail = true, emailChangeError = null)
            when (val result = apiService.confirmEmailChange(newEmail, code)) {
                is ApiResult.Success -> {
                    sessionManager.updateEmail(result.data.email)
                    _state.value =
                        _state.value.copy(
                            isConfirmingEmail = false,
                            emailOtpSent = false,
                            pendingNewEmail = null,
                            emailChangeError = null,
                            emailChangeSuccess = "email zmieniony",
                        )
                }

                is ApiResult.Error -> {
                    _state.value =
                        _state.value.copy(
                            isConfirmingEmail = false,
                            emailChangeError = mapEmailChangeError(result),
                        )
                }
            }
        }
    }

    fun cancelEmailChange() {
        _state.value =
            _state.value.copy(
                isRequestingEmailOtp = false,
                isConfirmingEmail = false,
                emailOtpSent = false,
                pendingNewEmail = null,
                emailChangeError = null,
            )
    }

    fun clearEmailChangeSuccess() {
        _state.value = _state.value.copy(emailChangeSuccess = null)
    }

    private fun mapEmailChangeError(error: ApiResult.Error): String =
        when (error.code) {
            "UNAUTHORIZED" -> "nieprawidłowe hasło"
            "EMAIL_TAKEN" -> "ten email jest już zajęty"
            "INVALID_OTP" -> "nieprawidłowy lub wygasły kod"
            "VALIDATION_ERROR" -> "nieprawidłowy adres email"
            "RATE_LIMITED" -> "poczekaj chwilę zanim spróbujesz ponownie"
            else -> "nie udało się zmienić emaila"
        }
}
