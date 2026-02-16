package com.poziomki.app.ui.screen.main

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import com.poziomki.app.api.ApiResult
import com.poziomki.app.api.ApiService
import com.poziomki.app.api.MatchProfile
import com.poziomki.app.api.SearchResults
import com.poziomki.app.data.repository.MatchProfileRepository
import kotlinx.coroutines.Job
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch

data class ExploreState(
    val profiles: List<MatchProfile> = emptyList(),
    val isLoading: Boolean = true,
    val isRefreshing: Boolean = false,
    val error: String? = null,
    val refreshError: String? = null,
    val query: String = "",
    val searchResults: SearchResults? = null,
    val isSearching: Boolean = false,
)

class ExploreViewModel(
    private val matchProfileRepository: MatchProfileRepository,
    private val apiService: ApiService,
) : ViewModel() {
    private val _state = MutableStateFlow(ExploreState())
    val state: StateFlow<ExploreState> = _state.asStateFlow()
    private var searchJob: Job? = null

    init {
        observeProfiles()
        refreshProfiles()
    }

    private fun observeProfiles() {
        viewModelScope.launch {
            matchProfileRepository.observeProfiles().collect { profiles ->
                _state.value =
                    _state.value.copy(
                        profiles = profiles,
                        isLoading = if (profiles.isNotEmpty()) false else _state.value.isLoading,
                    )
            }
        }
    }

    private fun refreshProfiles() {
        viewModelScope.launch {
            val success = matchProfileRepository.refreshProfiles()
            if (!success && _state.value.profiles.isNotEmpty()) {
                _state.value = _state.value.copy(refreshError = "Nie udało się odświeżyć profili")
            }
            _state.value = _state.value.copy(isLoading = false)
        }
    }

    fun pullToRefresh() {
        viewModelScope.launch {
            _state.value = _state.value.copy(isRefreshing = true)
            val success = matchProfileRepository.refreshProfiles(forceRefresh = true)
            if (!success && _state.value.profiles.isNotEmpty()) {
                _state.value = _state.value.copy(refreshError = "Nie udało się odświeżyć profili")
            }
            _state.value = _state.value.copy(isRefreshing = false)
        }
    }

    fun clearRefreshError() {
        _state.value = _state.value.copy(refreshError = null)
    }

    fun updateQuery(query: String) {
        _state.value = _state.value.copy(query = query)
        searchJob?.cancel()

        if (query.length < 2) {
            _state.value = _state.value.copy(searchResults = null, isSearching = false)
            return
        }

        searchJob =
            viewModelScope.launch {
                delay(300)
                _state.value = _state.value.copy(isSearching = true)
                when (val result = apiService.search(query)) {
                    is ApiResult.Success -> {
                        _state.value =
                            _state.value.copy(
                                searchResults = result.data,
                                isSearching = false,
                            )
                    }

                    is ApiResult.Error -> {
                        _state.value =
                            _state.value.copy(
                                searchResults = null,
                                isSearching = false,
                            )
                    }
                }
            }
    }
}
