package com.poziomki.app.ui.screen.main

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import com.poziomki.app.api.ApiResult
import com.poziomki.app.api.ApiService
import com.poziomki.app.api.Event
import com.poziomki.app.util.TimeFilter
import com.poziomki.app.util.matchesTimeFilter
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch

data class EventsState(
    val allEvents: List<Event> = emptyList(),
    val events: List<Event> = emptyList(),
    val isLoading: Boolean = false,
    val error: String? = null,
    val searchQuery: String = "",
    val activeFilter: TimeFilter = TimeFilter.ALL,
)

class EventsViewModel(
    private val apiService: ApiService,
) : ViewModel() {
    private val _state = MutableStateFlow(EventsState())
    val state: StateFlow<EventsState> = _state.asStateFlow()

    init {
        loadEvents()
    }

    fun loadEvents() {
        viewModelScope.launch {
            _state.value = _state.value.copy(isLoading = true)
            when (val result = apiService.getEvents()) {
                is ApiResult.Success -> {
                    _state.value =
                        _state.value.copy(
                            allEvents = result.data,
                            isLoading = false,
                            error = null,
                        )
                    filterEvents()
                }

                is ApiResult.Error -> {
                    _state.value =
                        _state.value.copy(
                            isLoading = false,
                            error = result.message,
                        )
                }
            }
        }
    }

    fun setSearchQuery(query: String) {
        _state.value = _state.value.copy(searchQuery = query)
        filterEvents()
    }

    fun setTimeFilter(filter: TimeFilter) {
        _state.value = _state.value.copy(activeFilter = filter)
        filterEvents()
    }

    private fun filterEvents() {
        val current = _state.value
        val filtered =
            current.allEvents.filter { event ->
                val matchesSearch =
                    current.searchQuery.isBlank() ||
                        event.title.contains(current.searchQuery, ignoreCase = true)
                val matchesTime = matchesTimeFilter(event.startsAt, current.activeFilter)
                matchesSearch && matchesTime
            }
        _state.value = current.copy(events = filtered)
    }
}
