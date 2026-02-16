package com.poziomki.app.ui.screen.main

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import com.poziomki.app.api.ApiResult
import com.poziomki.app.api.Event
import com.poziomki.app.data.repository.EventRepository
import com.poziomki.app.location.LocationProvider
import com.poziomki.app.util.TimeFilter
import com.poziomki.app.util.matchesTimeFilter
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch

data class EventsState(
    val allEvents: List<Event> = emptyList(),
    val recommendedEvents: List<Event> = emptyList(),
    val nearbyEvents: List<Event> = emptyList(),
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
    private val apiService: com.poziomki.app.api.ApiService,
    private val locationProvider: LocationProvider,
) : ViewModel() {
    private val _state = MutableStateFlow(EventsState())
    val state: StateFlow<EventsState> = _state.asStateFlow()

    init {
        observeEvents()
        refreshEvents()
        loadRecommendedEvents()
    }

    private fun observeEvents() {
        viewModelScope.launch {
            eventRepository.observeEvents().collect { events ->
                _state.value =
                    _state.value.copy(
                        allEvents = events,
                        isLoading = if (events.isNotEmpty()) false else _state.value.isLoading,
                    )
                filterEvents()
            }
        }
    }

    private fun loadRecommendedEvents() {
        viewModelScope.launch {
            val recommended = eventRepository.fetchRecommendedEvents()
            _state.value = _state.value.copy(recommendedEvents = recommended)
            filterEvents()
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
            loadRecommendedEvents()
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
        if (filter == TimeFilter.NEARBY && _state.value.nearbyEvents.isEmpty()) {
            fetchNearbyIfPermitted()
        }
        filterEvents()
    }

    private fun fetchNearbyIfPermitted() {
        if (!locationProvider.isPermissionGranted()) return
        viewModelScope.launch {
            val loc = locationProvider.getCurrentLocation() ?: return@launch
            loadNearbyEvents(loc.latitude, loc.longitude)
        }
    }

    fun loadNearbyEvents(
        lat: Double,
        lng: Double,
        radiusM: Int = 10_000,
    ) {
        viewModelScope.launch {
            when (val result = apiService.getMatchingEvents(lat = lat, lng = lng, radiusM = radiusM)) {
                is ApiResult.Success -> {
                    _state.value = _state.value.copy(nearbyEvents = result.data)
                    filterEvents()
                }

                is ApiResult.Error -> {}
            }
        }
    }

    private fun filterEvents() {
        val current = _state.value
        val source =
            when (current.activeFilter) {
                TimeFilter.ALL -> current.recommendedEvents.ifEmpty { current.allEvents }
                TimeFilter.NEARBY -> current.nearbyEvents
                else -> current.allEvents
            }
        val filtered =
            source.filter { event ->
                val matchesSearch =
                    current.searchQuery.isBlank() ||
                        event.title.contains(current.searchQuery, ignoreCase = true)
                val matchesTime =
                    current.activeFilter == TimeFilter.ALL ||
                        current.activeFilter == TimeFilter.NEARBY ||
                        matchesTimeFilter(event.startsAt, current.activeFilter)
                matchesSearch && matchesTime
            }
        _state.value = current.copy(events = filtered)
    }
}
