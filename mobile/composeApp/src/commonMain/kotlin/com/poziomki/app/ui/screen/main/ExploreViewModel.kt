package com.poziomki.app.ui.screen.main

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import com.poziomki.app.api.ApiResult
import com.poziomki.app.api.ApiService
import com.poziomki.app.api.MatchProfile
import com.poziomki.app.api.SearchResults
import kotlinx.coroutines.Job
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch

data class ExploreState(
    val profiles: List<MatchProfile> = emptyList(),
    val isLoading: Boolean = false,
    val error: String? = null,
    val query: String = "",
    val searchResults: SearchResults? = null,
    val isSearching: Boolean = false,
)

class ExploreViewModel(
    private val apiService: ApiService,
) : ViewModel() {
    private val _state = MutableStateFlow(ExploreState())
    val state: StateFlow<ExploreState> = _state.asStateFlow()
    private var searchJob: Job? = null

    init {
        loadProfiles()
    }

    fun loadProfiles() {
        viewModelScope.launch {
            _state.value = _state.value.copy(isLoading = true)
            when (val result = apiService.getMatchingProfiles()) {
                is ApiResult.Success -> _state.value = _state.value.copy(profiles = result.data, isLoading = false)
                is ApiResult.Error -> _state.value = _state.value.copy(error = result.message, isLoading = false)
            }
        }
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
                    is ApiResult.Success ->
                        _state.value =
                            _state.value.copy(
                                searchResults = result.data,
                                isSearching = false,
                            )
                    is ApiResult.Error ->
                        _state.value =
                            _state.value.copy(
                                searchResults = null,
                                isSearching = false,
                            )
                }
            }
    }
}
