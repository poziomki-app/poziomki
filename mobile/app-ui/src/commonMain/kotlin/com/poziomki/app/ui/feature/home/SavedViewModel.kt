package com.poziomki.app.ui.feature.home

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import com.poziomki.app.data.repository.EventRepository
import com.poziomki.app.network.ApiResult
import com.poziomki.app.network.ApiService
import com.poziomki.app.network.Event
import com.poziomki.app.network.Profile
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.IO
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext

data class SavedState(
    val events: List<Event> = emptyList(),
    val profiles: List<Profile> = emptyList(),
    val isLoading: Boolean = true,
    val isRefreshing: Boolean = false,
)

class SavedViewModel(
    private val eventRepository: EventRepository,
    private val apiService: ApiService,
) : ViewModel() {
    private val _state = MutableStateFlow(SavedState())
    val state: StateFlow<SavedState> = _state.asStateFlow()

    init {
        observeSavedEvents()
        refresh()
    }

    private fun observeSavedEvents() {
        viewModelScope.launch {
            eventRepository.observeSavedEvents().collect { events ->
                _state.value = _state.value.copy(events = events)
            }
        }
    }

    private fun refresh() {
        viewModelScope.launch {
            eventRepository.refreshSavedEvents()
            loadBookmarkedProfiles()
            _state.value = _state.value.copy(isLoading = false)
        }
    }

    fun pullToRefresh() {
        _state.value = _state.value.copy(isRefreshing = true)
        viewModelScope.launch {
            eventRepository.refreshSavedEvents(forceRefresh = true)
            loadBookmarkedProfiles()
            _state.value = _state.value.copy(isRefreshing = false)
        }
    }

    private suspend fun loadBookmarkedProfiles() {
        withContext(Dispatchers.IO) {
            when (val result = apiService.getBookmarkedProfiles()) {
                is ApiResult.Success -> {
                    _state.value = _state.value.copy(profiles = result.data)
                }

                is ApiResult.Error -> {}
            }
        }
    }
}
