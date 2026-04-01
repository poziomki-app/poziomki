package com.poziomki.app.ui.feature.profile

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import com.poziomki.app.data.CacheManager
import com.poziomki.app.data.repository.SettingsRepository
import com.poziomki.app.network.ApiResult
import com.poziomki.app.network.ApiService
import com.poziomki.app.session.SessionManager
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.first
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
)

class PrivacyViewModel(
    private val apiService: ApiService,
    private val sessionManager: SessionManager,
    private val cacheManager: CacheManager,
    private val settingsRepository: SettingsRepository,
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
                    cacheManager.clearAll()
                    sessionManager.clearSession()
                    _state.value =
                        _state.value.copy(
                            isChangingPassword = false,
                            passwordSuccessMessage = "Hasło zostało zmienione. Zaloguj się ponownie.",
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
}
