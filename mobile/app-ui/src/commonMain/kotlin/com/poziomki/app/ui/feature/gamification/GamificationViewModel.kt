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

data class DailyTask(
    val id: String,
    val title: String,
    val description: String,
    val xp: Int = 5,
)

val DefaultDailyTasks: List<DailyTask> =
    listOf(
        DailyTask(
            id = "daily_login",
            title = "Zajrzyj na Poziomki",
            description = "Otwórz aplikację dzisiaj — +5 XP i +1 do streaka.",
        ),
        DailyTask(
            id = "say_hi",
            title = "Powiedz cześć",
            description = "Napisz do kogoś w czacie i rozpocznij rozmowę.",
        ),
        DailyTask(
            id = "attend_event",
            title = "Znajdź wydarzenie",
            description = "Odkryj jedno nowe wydarzenie w zakładce Wydarzenia.",
        ),
        DailyTask(
            id = "meet_irl",
            title = "Spotkaj kogoś na żywo",
            description = "Zeskanuj QR znajomego w realu — oboje dostajecie +25 XP.",
        ),
    )

data class GamificationState(
    val streakCurrent: Int = 0,
    val streakLongest: Int = 0,
    val xp: Int = 0,
    val tasks: List<DailyTask> = DefaultDailyTasks,
    val claimedTaskIds: Set<String> = emptySet(),
    val claimingTaskId: String? = null,
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

    fun claim(task: DailyTask) {
        if (task.id in _state.value.claimedTaskIds || _state.value.claimingTaskId != null) return
        viewModelScope.launch {
            _state.value = _state.value.copy(claimingTaskId = task.id)
            when (val r = xpRepository.claimTask(task.id)) {
                is ApiResult.Success -> {
                    _state.value =
                        _state.value.copy(
                            claimingTaskId = null,
                            claimedTaskIds = _state.value.claimedTaskIds + task.id,
                        )
                }

                is ApiResult.Error -> {
                    _state.value =
                        _state.value.copy(
                            claimingTaskId = null,
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
