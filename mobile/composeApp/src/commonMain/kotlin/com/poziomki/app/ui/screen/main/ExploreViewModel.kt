package com.poziomki.app.ui.screen.main

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import com.poziomki.app.api.ApiResult
import com.poziomki.app.api.ApiService
import com.poziomki.app.api.MatchProfile
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch

data class ExploreState(
    val profiles: List<MatchProfile> = emptyList(),
    val isLoading: Boolean = false,
    val error: String? = null,
)

class ExploreViewModel(
    private val apiService: ApiService,
) : ViewModel() {
    private val _state = MutableStateFlow(ExploreState())
    val state: StateFlow<ExploreState> = _state.asStateFlow()

    init {
        loadProfiles()
    }

    fun loadProfiles() {
        viewModelScope.launch {
            _state.value = _state.value.copy(isLoading = true)
            when (val result = apiService.getMatchingProfiles()) {
                is ApiResult.Success -> _state.value = ExploreState(profiles = result.data)
                is ApiResult.Error -> _state.value = ExploreState(error = result.message)
            }
        }
    }
}
