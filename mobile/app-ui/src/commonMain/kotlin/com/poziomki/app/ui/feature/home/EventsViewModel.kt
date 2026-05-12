package com.poziomki.app.ui.feature.home

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import com.poziomki.app.data.repository.EventRepository
import com.poziomki.app.location.LocationProvider
import com.poziomki.app.location.LocationResult
import com.poziomki.app.network.ApiResult
import com.poziomki.app.network.Event
import com.poziomki.app.ui.shared.TimeFilter
import com.poziomki.app.ui.shared.matchesTimeFilter
import kotlinx.coroutines.Job
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch
import kotlin.math.abs

data class EventsState(
    val allEvents: List<Event> = emptyList(),
    val recommendedEvents: List<Event> = emptyList(),
    val nearbyEvents: List<Event> = emptyList(),
    val events: List<Event> = emptyList(),
    val isLoading: Boolean = true,
    val isRefreshing: Boolean = false,
    val error: String? = null,
    val refreshError: String? = null,
    val syncError: String? = null,
    val searchQuery: String = "",
    val activeFilter: TimeFilter = TimeFilter.ALL,
    val userLat: Double? = null,
    val userLng: Double? = null,
    val selectedNearbyEventId: String? = null,
    val isLocationPermissionDenied: Boolean = false,
    val selectedCategories: Set<String> = emptySet(),
    val showTagFilter: Boolean = false,
)

class EventsViewModel(
    private val eventRepository: EventRepository,
    private val apiService: com.poziomki.app.network.ApiService,
    private val locationProvider: LocationProvider,
) : ViewModel() {
    private val _state = MutableStateFlow(EventsState())
    val state: StateFlow<EventsState> = _state.asStateFlow()

    private var locationJob: Job? = null

    init {
        observeEvents()
        refreshEvents()
        loadRecommendedEvents()
        observeSyncErrors()
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
                x.maxAttendees == y.maxAttendees &&
                x.isAttending == y.isAttending &&
                x.attendeesPreview == y.attendeesPreview &&
                x.creator == y.creator
        }
    }

    private fun loadRecommendedEvents(forceRefresh: Boolean = false) {
        viewModelScope.launch {
            val location =
                if (locationProvider.isPermissionGranted()) {
                    locationProvider.getCurrentLocation()
                } else {
                    null
                }
            val recommended =
                eventRepository.fetchRecommendedEvents(
                    lat = location?.latitude,
                    lng = location?.longitude,
                    forceRefresh = forceRefresh,
                )
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
            loadRecommendedEvents(forceRefresh = true)
            if (_state.value.activeFilter == TimeFilter.NEARBY) {
                val lat = _state.value.userLat ?: FALLBACK_LAT
                val lng = _state.value.userLng ?: FALLBACK_LNG
                loadNearbyEvents(lat, lng)
            }
            _state.value = _state.value.copy(isRefreshing = false)
        }
    }

    fun selectNearbyEvent(id: String) {
        _state.value = _state.value.copy(selectedNearbyEventId = id)
    }

    fun clearRefreshError() {
        _state.value = _state.value.copy(refreshError = null)
    }

    fun clearSyncError() {
        _state.value = _state.value.copy(syncError = null)
    }

    private fun observeSyncErrors() {
        viewModelScope.launch {
            eventRepository.syncErrors.collect { error ->
                _state.value = _state.value.copy(syncError = error)
            }
        }
    }

    fun setSearchQuery(query: String) {
        _state.value = _state.value.copy(searchQuery = query)
        filterEvents()
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
            startLocationTracking()
        } else {
            stopLocationTracking()
        }
        filterEvents()
    }

    private fun startLocationTracking() {
        if (!locationProvider.isPermissionGranted()) {
            _state.value = _state.value.copy(isLocationPermissionDenied = true)
            // Seed map at Warsaw so it renders, but keep user dot hidden.
            if (_state.value.nearbyEvents.isEmpty()) {
                loadNearbyEvents(FALLBACK_LAT, FALLBACK_LNG)
            }
            return
        }
        _state.value = _state.value.copy(isLocationPermissionDenied = false)
        if (locationJob?.isActive == true) return
        locationJob =
            viewModelScope.launch {
                // Best-effort one-shot first so the dot appears as soon as possible
                // (cached fix, fused last-known, etc.).
                val initial = locationProvider.getCurrentLocation()
                if (initial != null) {
                    onLocationFix(initial)
                } else {
                    // No initial fix — still load nearby around Warsaw so map isn't blank.
                    if (_state.value.nearbyEvents.isEmpty()) {
                        loadNearbyEvents(FALLBACK_LAT, FALLBACK_LNG)
                    }
                }
                locationProvider.locationUpdates().collect { onLocationFix(it) }
            }
    }

    private fun stopLocationTracking() {
        locationJob?.cancel()
        locationJob = null
    }

    private fun onLocationFix(loc: LocationResult) {
        val prevLat = _state.value.userLat
        val prevLng = _state.value.userLng
        _state.value = _state.value.copy(userLat = loc.latitude, userLng = loc.longitude)
        val movedSignificantly =
            prevLat == null ||
                prevLng == null ||
                abs(prevLat - loc.latitude) > RELOAD_DEG_THRESHOLD ||
                abs(prevLng - loc.longitude) > RELOAD_DEG_THRESHOLD
        if (movedSignificantly) {
            loadNearbyEvents(loc.latitude, loc.longitude)
        }
    }

    fun retryNearby() {
        _state.value = _state.value.copy(isLocationPermissionDenied = false)
        startLocationTracking()
    }

    private companion object {
        // Warsaw — used when device location can't be obtained despite permission.
        const val FALLBACK_LAT = 52.2297
        const val FALLBACK_LNG = 21.0122

        // ~111 m at the equator — reload nearby events when user has moved this far.
        const val RELOAD_DEG_THRESHOLD = 0.001
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

    fun toggleSave(eventId: String) {
        viewModelScope.launch {
            val event = _state.value.events.find { it.id == eventId } ?: return@launch
            if (event.isSaved) {
                eventRepository.unsaveEvent(eventId)
            } else {
                eventRepository.saveEvent(eventId)
            }
        }
    }

    fun toggleCategoryFilter(category: String) {
        val current = _state.value.selectedCategories
        _state.value =
            _state.value.copy(
                selectedCategories = if (category in current) current - category else current + category,
            )
        filterEvents()
    }

    fun clearCategoryFilters() {
        _state.value = _state.value.copy(selectedCategories = emptySet())
        filterEvents()
    }

    fun toggleShowTagFilter() {
        _state.value = _state.value.copy(showTagFilter = !_state.value.showTagFilter)
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
                val matchesTags =
                    current.selectedCategories.isEmpty() ||
                        event.tags.any { it.category in current.selectedCategories }
                matchesSearch && matchesTime && matchesTags
            }
        _state.value = current.copy(events = filtered)
    }
}
