package com.poziomki.app.ui.feature.profile

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import com.poziomki.app.data.CacheManager
import com.poziomki.app.network.ApiResult
import com.poziomki.app.network.ApiService
import com.poziomki.app.session.SessionManager
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch

data class PrivacyState(
    val isExporting: Boolean = false,
    val isDeleting: Boolean = false,
    val isChangingPassword: Boolean = false,
    val exportedJson: String? = null,
    val passwordSuccessMessage: String? = null,
    val error: String? = null,
)

class PrivacyViewModel(
    private val apiService: ApiService,
    private val sessionManager: SessionManager,
    private val cacheManager: CacheManager,
) : ViewModel() {
    private val _state = MutableStateFlow(PrivacyState())
    val state: StateFlow<PrivacyState> = _state.asStateFlow()

    fun exportData() {
        viewModelScope.launch {
            _state.value = _state.value.copy(isExporting = true, error = null)
            when (val result = apiService.exportData()) {
                is ApiResult.Success -> {
                    _state.value =
                        _state.value.copy(
                            isExporting = false,
                            exportedJson = result.data.toString(),
                        )
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

    fun clearExport() {
        _state.value = _state.value.copy(exportedJson = null)
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
