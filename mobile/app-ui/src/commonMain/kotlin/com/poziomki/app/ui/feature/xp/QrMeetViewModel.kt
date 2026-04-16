package com.poziomki.app.ui.feature.xp

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import com.poziomki.app.network.ApiResult
import com.poziomki.app.network.ApiService
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch

data class QrMeetState(
    val token: String? = null,
    val expiresAt: Long = 0L,
    val isLoadingToken: Boolean = false,
    val tokenError: String? = null,
    val scanInput: String = "",
    val isScanning: Boolean = false,
    val scanResult: ScanResult? = null,
    val scanError: String? = null,
)

sealed interface ScanResult {
    data class Awarded(
        val xpGained: Int,
    ) : ScanResult

    data object AlreadyScanned : ScanResult
}

class QrMeetViewModel(
    private val api: ApiService,
) : ViewModel() {
    private val _state = MutableStateFlow(QrMeetState())
    val state: StateFlow<QrMeetState> = _state.asStateFlow()

    init {
        loadToken()
    }

    fun loadToken() {
        viewModelScope.launch {
            _state.value = _state.value.copy(isLoadingToken = true, tokenError = null)
            when (val result = api.getXpToken()) {
                is ApiResult.Success -> {
                    _state.value =
                        _state.value.copy(
                            token = result.data.token,
                            expiresAt = result.data.expiresAt,
                            isLoadingToken = false,
                        )
                }

                is ApiResult.Error -> {
                    _state.value =
                        _state.value.copy(
                            isLoadingToken = false,
                            tokenError = "Nie udało się wygenerować kodu QR",
                        )
                }
            }
        }
    }

    fun onScanInputChange(value: String) {
        _state.value = _state.value.copy(scanInput = value, scanError = null, scanResult = null)
    }

    fun submitScan() {
        val token = _state.value.scanInput.trim()
        if (token.isBlank()) return

        viewModelScope.launch {
            _state.value = _state.value.copy(isScanning = true, scanError = null, scanResult = null)
            when (val result = api.scanXpToken(token)) {
                is ApiResult.Success -> {
                    val scanResult =
                        if (result.data.xpGained > 0) {
                            ScanResult.Awarded(result.data.xpGained)
                        } else {
                            ScanResult.AlreadyScanned
                        }
                    _state.value =
                        _state.value.copy(
                            isScanning = false,
                            scanResult = scanResult,
                            scanInput = "",
                        )
                }

                is ApiResult.Error -> {
                    _state.value =
                        _state.value.copy(
                            isScanning = false,
                            scanError = "Nieprawidłowy lub wygasły kod",
                        )
                }
            }
        }
    }

    fun clearScanResult() {
        _state.value = _state.value.copy(scanResult = null, scanError = null)
    }
}
