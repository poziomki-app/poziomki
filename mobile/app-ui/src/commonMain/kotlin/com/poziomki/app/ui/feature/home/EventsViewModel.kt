package com.poziomki.app.ui.feature.home

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import com.poziomki.app.data.repository.EventRepository
import com.poziomki.app.location.LocationProvider
import com.poziomki.app.network.ApiResult
import com.poziomki.app.network.Event
import com.poziomki.app.ui.shared.TimeFilter
import com.poziomki.app.ui.shared.matchesTimeFilter
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
    val userLat: Double? = null,
    val userLng: Double? = null,
    val selectedNearbyEventId: String? = null,
    val isLocationPermissionDenied: Boolean = false,
    val isLocationUnavailable: Boolean = false,
)

class EventsViewModel(
    private val eventRepository: EventRepository,
    private val apiService: com.poziomki.app.network.ApiService,
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
                val current = _state.value.allEvents
                if (eventsVisuallyEqual(current, events)) {
                    if (_state.value.isLoading) {
                        _state.value = _state.value.copy(isLoading = false)
                    }
                } else {
                    _state.value =
                        _state.value.copy(
                            allEvents = events,
                            isLoading = if (events.isNotEmpty()) false else _state.value.isLoading,
                        )
                    filterEvents()
                }
            }
        }
    }

    private fun eventsVisuallyEqual(
        a: List<Event>,
        b: List<Event>,
    ): Boolean {
        if (a.size != b.size) return false
        return a.indices.all { i ->
            val x = a[i]
            val y = b[i]
            x.id == y.id && x.title == y.title &&
                x.coverImage == y.coverImage &&
                x.startsAt == y.startsAt &&
                x.location == y.location &&
                x.attendeesCount == y.attendeesCount &&
                x.isAttending == y.isAttending
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
            if (_state.value.activeFilter == TimeFilter.NEARBY) {
                fetchNearbyIfPermitted()
            }
            _state.value = _state.value.copy(isRefreshing = false)
        }
    }

    fun retryNearby() {
        _state.value = _state.value.copy(isLocationPermissionDenied = false)
        fetchNearbyIfPermitted()
    }

    fun selectNearbyEvent(id: String) {
        _state.value = _state.value.copy(selectedNearbyEventId = id)
    }

    fun clearRefreshError() {
        _state.value = _state.value.copy(refreshError = null)
    }

    fun setSearchQuery(query: String) {
        _state.value = _state.value.copy(searchQuery = query)
        filterEvents()
    }

    fun toggleSaved(event: Event) {
        viewModelScope.launch {
            val updatedSaved = !event.isSaved
            val result =
                if (event.isSaved) {
                    eventRepository.unsaveEvent(event.id)
                } else {
                    eventRepository.saveEvent(event.id)
                }
            when (result) {
                is ApiResult.Success -> {
                    _state.value =
                        _state.value.copy(
                            recommendedEvents =
                                _state.value.recommendedEvents.map { recommendedEvent ->
                                    if (recommendedEvent.id == event.id) {
                                        recommendedEvent.copy(isSaved = updatedSaved)
                                    } else {
                                        recommendedEvent
                                    }
                                },
                        )
                    filterEvents()
                }

                is ApiResult.Error -> {
                    val msg =
                        if (event.isSaved) {
                            "Nie udało się usunąć zapisu"
                        } else {
                            "Nie udało się zapisać wydarzenia"
                        }
                    _state.value = _state.value.copy(refreshError = msg)
                }
            }
        }
    }

    fun setTimeFilter(filter: TimeFilter) {
        _state.value = _state.value.copy(activeFilter = filter)
        if (filter == TimeFilter.NEARBY) {
            // Seed nearbyEvents from allEvents so the map shows dots while API loads
            if (_state.value.nearbyEvents.isEmpty() && _state.value.allEvents.isNotEmpty()) {
                _state.value =
                    _state.value.copy(
                        nearbyEvents =
                            _state.value.allEvents.filter {
                                it.latitude != null && it.longitude != null
                            },
                    )
            }
            if (_state.value.userLat == null) {
                fetchNearbyIfPermitted()
            }
        }
        filterEvents()
    }

    private fun fetchNearbyIfPermitted() {
        if (!locationProvider.isPermissionGranted()) {
            _state.value = _state.value.copy(isLocationPermissionDenied = true)
            return
        }
        _state.value = _state.value.copy(isLocationPermissionDenied = false, isLocationUnavailable = false)
        viewModelScope.launch {
            val loc = locationProvider.getCurrentLocation()
            if (loc == null) {
                _state.value = _state.value.copy(isLocationUnavailable = true)
                return@launch
            }
            _state.value = _state.value.copy(userLat = loc.latitude, userLng = loc.longitude)
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
                    val nearby = result.data
                    val closest =
                        nearby
                            .filter { it.latitude != null && it.longitude != null }
                            .minByOrNull { distanceDeg(lat, lng, it.latitude!!, it.longitude!!) }
                    _state.value =
                        _state.value.copy(
                            nearbyEvents = nearby,
                            selectedNearbyEventId = closest?.id ?: _state.value.selectedNearbyEventId,
                        )
                    filterEvents()
                }

                is ApiResult.Error -> {}
            }
        }
    }

    private fun distanceDeg(
        lat1: Double,
        lng1: Double,
        lat2: Double,
        lng2: Double,
    ): Double {
        val dLat = lat1 - lat2
        val dLng = lng1 - lng2
        return dLat * dLat + dLng * dLng
    }

    private fun filterEvents() {
        val current = _state.value
        val source =
            when (current.activeFilter) {
                TimeFilter.ALL -> recommendedDisplayEvents(current.recommendedEvents, current.allEvents)
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

    private fun recommendedDisplayEvents(
        recommended: List<Event>,
        cached: List<Event>,
    ): List<Event> =
        if (recommended.isEmpty()) {
            cached
        } else {
            recommended.map { recommendedEvent ->
                cached.firstOrNull { it.id == recommendedEvent.id } ?: recommendedEvent
            }
        }
}
