package com.poziomki.app.ui.feature.xp

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import com.poziomki.app.network.ApiResult
import com.poziomki.app.network.ApiService
import com.poziomki.app.network.WeatherInfo
import com.poziomki.app.network.WeatherService
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch

data class DailyTask(
    val id: String,
    val emoji: String,
    val label: String,
) {
    companion object {
        val all =
            listOf(
                DailyTask("tree_photo", "📸", "Zrób zdjęcie drzewa"),
                DailyTask("drink_water", "💧", "Wypij szklankę wody"),
                DailyTask("walk_outside", "🚶", "Wyjdź na spacer"),
                DailyTask("say_hi", "👋", "Przywitaj się z nieznajomym"),
                DailyTask("read_something", "📖", "Przeczytaj coś przez 10 minut"),
                DailyTask("stretch", "🧘", "Rozciągnij się przez 5 minut"),
                DailyTask("coffee_break", "☕", "Zrób sobie przerwę bez telefonu"),
                DailyTask("compliment", "💬", "Powiedz komuś komplement"),
            )
    }
}

data class TasksState(
    val weather: WeatherInfo? = null,
    val isLoadingWeather: Boolean = true,
    val completedTaskIds: Set<String> = emptySet(),
    val claimingTaskId: String? = null,
    val lastXpMessage: String? = null,
)

class TasksViewModel(
    private val api: ApiService,
    private val weatherService: WeatherService,
) : ViewModel() {
    private val _state = MutableStateFlow(TasksState())
    val state: StateFlow<TasksState> = _state.asStateFlow()

    init {
        loadWeather()
    }

    private fun loadWeather() {
        viewModelScope.launch {
            val weather = weatherService.getWarsawWeather()
            _state.update { it.copy(weather = weather, isLoadingWeather = false) }
        }
    }

    fun claimTask(taskId: String) {
        if (_state.value.claimingTaskId != null) return
        viewModelScope.launch {
            _state.update { it.copy(claimingTaskId = taskId) }
            val message =
                when (val result = api.claimTask(taskId)) {
                    is ApiResult.Success -> {
                        _state.update { it.copy(completedTaskIds = it.completedTaskIds + taskId) }
                        if (result.data.xpGained > 0) "+${result.data.xpGained} XP!" else "już dzisiaj wykonano"
                    }

                    is ApiResult.Error -> {
                        null
                    }
                }
            _state.update { it.copy(claimingTaskId = null, lastXpMessage = message) }
        }
    }

    fun clearXpMessage() {
        _state.update { it.copy(lastXpMessage = null) }
    }
}
