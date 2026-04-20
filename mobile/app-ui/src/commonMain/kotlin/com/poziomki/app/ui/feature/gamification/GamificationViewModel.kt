package com.poziomki.app.ui.feature.gamification

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import com.poziomki.app.data.repository.ProfileRepository
import com.poziomki.app.data.repository.XpRepository
import com.poziomki.app.network.ApiResult
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch

data class GamificationState(
    val streakCurrent: Int = 0,
    val streakLongest: Int = 0,
    val xp: Int = 0,
    val myToken: String? = null,
    val isLoadingToken: Boolean = false,
    val lastScanXp: Int? = null,
    val errorMessage: String? = null,
)

class GamificationViewModel(
    private val profileRepository: ProfileRepository,
    private val xpRepository: XpRepository,
) : ViewModel() {
    private val _state = MutableStateFlow(GamificationState())
    val state: StateFlow<GamificationState> = _state.asStateFlow()

    init {
        viewModelScope.launch {
            profileRepository.observeOwnProfile().collect { profile ->
                if (profile != null) {
                    _state.value =
                        _state.value.copy(
                            streakCurrent = profile.streakCurrent,
                            streakLongest = profile.streakLongest,
                            xp = profile.xp,
                        )
                }
            }
        }
        refreshToken()
        viewModelScope.launch { profileRepository.refreshOwnProfile(forceRefresh = true) }
    }

    fun refreshToken() {
        viewModelScope.launch {
            _state.value = _state.value.copy(isLoadingToken = true)
            when (val r = xpRepository.generateToken()) {
                is ApiResult.Success -> {
                    _state.value = _state.value.copy(myToken = r.data.token, isLoadingToken = false)
                }

                is ApiResult.Error -> {
                    _state.value =
                        _state.value.copy(
                            isLoadingToken = false,
                            errorMessage = r.message,
                        )
                }
            }
        }
    }

    fun onScanResult(token: String?) {
        if (token.isNullOrBlank()) return
        viewModelScope.launch {
            when (val r = xpRepository.scan(token)) {
                is ApiResult.Success -> {
                    _state.value = _state.value.copy(lastScanXp = r.data.xpGained)
                }

                is ApiResult.Error -> {
                    _state.value = _state.value.copy(errorMessage = r.message)
                }
            }
        }
    }

    fun clearMessage() {
        _state.value = _state.value.copy(errorMessage = null, lastScanXp = null)
    }
}
