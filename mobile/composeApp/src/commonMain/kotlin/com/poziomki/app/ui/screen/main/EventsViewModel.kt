package com.poziomki.app.ui.screen.main

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import com.poziomki.app.api.Event
import com.poziomki.app.data.repository.EventRepository
import com.poziomki.app.util.TimeFilter
import com.poziomki.app.util.matchesTimeFilter
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch

data class EventsState(
    val allEvents: List<Event> = emptyList(),
    val events: List<Event> = emptyList(),
    val isLoading: Boolean = true,
    val isRefreshing: Boolean = false,
    val error: String? = null,
    val refreshError: String? = null,
    val searchQuery: String = "",
    val activeFilter: TimeFilter = TimeFilter.ALL,
)

class EventsViewModel(
    private val eventRepository: EventRepository,
) : ViewModel() {
    private val _state = MutableStateFlow(EventsState())
    val state: StateFlow<EventsState> = _state.asStateFlow()

    init {
        observeEvents()
        refreshEvents()
    }

    private fun observeEvents() {
        viewModelScope.launch {
            eventRepository.observeEvents().collect { events ->
                _state.value =
                    _state.value.copy(
                        allEvents = events,
                        isLoading = false,
                    )
                filterEvents()
            }
        }
    }

    fun refreshEvents(showLoading: Boolean = false) {
        viewModelScope.launch {
            if (showLoading) {
                _state.value = _state.value.copy(isLoading = true)
            }
            val success = eventRepository.refreshEvents()
            if (!success && _state.value.allEvents.isNotEmpty()) {
                _state.value = _state.value.copy(refreshError = "Nie udało się odświeżyć wydarzeń")
            }
            _state.value = _state.value.copy(isLoading = false)
        }
    }

    fun pullToRefresh() {
        viewModelScope.launch {
            _state.value = _state.value.copy(isRefreshing = true)
            val success = eventRepository.refreshEvents(forceRefresh = true)
            if (!success && _state.value.allEvents.isNotEmpty()) {
                _state.value = _state.value.copy(refreshError = "Nie udało się odświeżyć wydarzeń")
            }
            _state.value = _state.value.copy(isRefreshing = false)
        }
    }

    fun clearRefreshError() {
        _state.value = _state.value.copy(refreshError = null)
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
