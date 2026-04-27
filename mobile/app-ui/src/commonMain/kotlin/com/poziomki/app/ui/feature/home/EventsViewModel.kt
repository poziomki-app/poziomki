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
import kotlinx.datetime.Clock
import kotlinx.datetime.Instant
import kotlin.math.PI
import kotlin.math.atan2
import kotlin.math.cos
import kotlin.math.sin
import kotlin.math.sqrt

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
    val isLocationUnavailable: Boolean = false,
    val dismissedEventIds: Set<String> = emptySet(),
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
            _state.value = _state.value.copy(recommendedEvents = recommended, dismissedEventIds = emptySet())
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
                    val sortedNearby =
                        result.data
                            .filter { it.latitude != null && it.longitude != null }
                            .map { it to distanceMeters(lat, lng, it.latitude!!, it.longitude!!) }
                            .filter { (_, d) -> d <= radiusM }
                            .sortedBy { (_, d) -> d }
                            .map { (event, _) -> event }
                    _state.value =
                        _state.value.copy(
                            nearbyEvents = sortedNearby,
                            selectedNearbyEventId =
                                sortedNearby.firstOrNull()?.id ?: _state.value.selectedNearbyEventId,
                        )
                    filterEvents()
                }

                is ApiResult.Error -> {}
            }
        }
    }

    private fun distanceMeters(
        lat1: Double,
        lng1: Double,
        lat2: Double,
        lng2: Double,
    ): Double {
        val earthRadiusM = 6_371_000.0
        val dLat = (lat2 - lat1).toRadians()
        val dLng = (lng2 - lng1).toRadians()
        val a =
            sin(dLat / 2) * sin(dLat / 2) +
                cos(lat1.toRadians()) * cos(lat2.toRadians()) *
                sin(dLng / 2) * sin(dLng / 2)
        val c = 2 * atan2(sqrt(a), sqrt(1 - a))
        return earthRadiusM * c
    }

    private fun Double.toRadians(): Double = this * PI / 180.0

    fun onSwipeFeedback(
        eventId: String,
        feedback: String,
    ) {
        val removedFromRecommended = _state.value.recommendedEvents.find { it.id == eventId }
        _state.value =
            _state.value.copy(
                dismissedEventIds = _state.value.dismissedEventIds + eventId,
                recommendedEvents = _state.value.recommendedEvents.filter { it.id != eventId },
            )
        filterEvents()

        // Only send feedback for events the recommendation engine actually surfaced
        if (removedFromRecommended == null) return

        viewModelScope.launch {
            val result = apiService.postEventFeedback(eventId, feedback)
            if (result is ApiResult.Error) {
                _state.value =
                    _state.value.copy(
                        dismissedEventIds = _state.value.dismissedEventIds - eventId,
                        recommendedEvents = _state.value.recommendedEvents + removedFromRecommended,
                    )
                filterEvents()
            }
        }
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
        val now = Clock.System.now()
        val filtered =
            source.filter { event ->
                val notFinished = !event.hasFinished(now)
                val notDismissed =
                    current.activeFilter != TimeFilter.ALL || event.id !in current.dismissedEventIds
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
                notFinished && notDismissed && matchesSearch && matchesTime && matchesTags
            }
        _state.value = current.copy(events = filtered)
    }

    private fun Event.hasFinished(now: Instant): Boolean {
        val end = endsAt ?: startsAt
        val endInstant = runCatching { Instant.parse(end) }.getOrNull() ?: return false
        return endInstant < now
    }
}
