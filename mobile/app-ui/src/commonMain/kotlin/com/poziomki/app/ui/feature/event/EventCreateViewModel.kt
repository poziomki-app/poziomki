package com.poziomki.app.ui.feature.event

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import com.poziomki.app.data.repository.EventRepository
import com.poziomki.app.network.ApiResult
import com.poziomki.app.network.ApiService
import com.poziomki.app.network.CreateEventRequest
import com.poziomki.app.network.GeocodingService
import com.poziomki.app.network.UpdateEventRequest
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.firstOrNull
import kotlinx.coroutines.launch

data class EventCreateState(
    val title: String = "",
    val description: String = "",
    val location: String = "",
    val startsAt: String = "",
    val endsAt: String = "",
    val attendeeLimit: String = "",
    val attendeeLimitError: String? = null,
    val coverImageUrl: String? = null,
    val coverImageBytes: ByteArray? = null,
    val isUploadingCover: Boolean = false,
    val latitude: Double? = null,
    val longitude: Double? = null,
    val isLoading: Boolean = false,
    val error: String? = null,
    val eventId: String? = null,
)

class EventCreateViewModel(
    private val eventRepository: EventRepository,
    private val apiService: ApiService,
    private val geocodingService: GeocodingService,
) : ViewModel() {
    private val _state = MutableStateFlow(EventCreateState())
    val state: StateFlow<EventCreateState> = _state.asStateFlow()

    fun clearError() {
        _state.value = _state.value.copy(error = null)
    }

    fun updateTitle(title: String) {
        _state.value = _state.value.copy(title = title)
    }

    fun updateDescription(description: String) {
        _state.value = _state.value.copy(description = description)
    }

    fun updateLocation(location: String) {
        _state.value = _state.value.copy(location = location)
    }

    fun updateLocationWithCoordinates(
        name: String,
        lat: Double,
        lng: Double,
    ) {
        _state.value = _state.value.copy(location = name, latitude = lat, longitude = lng)
    }

    suspend fun searchLocation(query: String) = geocodingService.search(query)

    fun updateStartsAt(startsAt: String) {
        _state.value = _state.value.copy(startsAt = startsAt)
    }

    fun updateEndsAt(endsAt: String) {
        _state.value = _state.value.copy(endsAt = endsAt)
    }

    fun updateAttendeeLimit(attendeeLimit: String) {
        val normalized = attendeeLimit.filter(Char::isDigit).take(5)
        val error =
            when {
                normalized.isBlank() -> null
                normalized.toIntOrNull() == null -> "Podaj poprawny limit"
                normalized.toInt() <= 0 -> "Limit musi być większy od 0"
                else -> null
            }
        _state.value = _state.value.copy(attendeeLimit = normalized, attendeeLimitError = error)
    }

    fun uploadCoverImage(bytes: ByteArray) {
        _state.value = _state.value.copy(coverImageBytes = bytes, isUploadingCover = true)
        viewModelScope.launch {
            when (val result = apiService.uploadImage(bytes, "event_cover.jpg", "event_cover")) {
                is ApiResult.Success -> {
                    _state.value =
                        _state.value.copy(
                            coverImageUrl = result.data.url,
                            isUploadingCover = false,
                        )
                }

                is ApiResult.Error -> {
                    _state.value =
                        _state.value.copy(
                            isUploadingCover = false,
                            error = "Nie udało się przesłać zdjęcia",
                        )
                }
            }
        }
    }

    fun removeCoverImage() {
        _state.value = _state.value.copy(coverImageUrl = null, coverImageBytes = null)
    }

    fun loadEvent(eventId: String) {
        viewModelScope.launch {
            _state.value = _state.value.copy(isLoading = true, eventId = eventId)
            eventRepository.refreshEvent(eventId)
            val event = eventRepository.observeEvent(eventId).firstOrNull()
            if (event != null) {
                _state.value =
                    _state.value.copy(
                        title = event.title,
                        description = event.description ?: "",
                        location = event.location ?: "",
                        startsAt = event.startsAt,
                        endsAt = event.endsAt ?: "",
                        attendeeLimit = event.maxAttendees?.toString().orEmpty(),
                        coverImageUrl = event.coverImage,
                        latitude = event.latitude,
                        longitude = event.longitude,
                        isLoading = false,
                        eventId = eventId,
                    )
            } else {
                _state.value = _state.value.copy(isLoading = false, eventId = eventId)
            }
        }
    }

    fun saveEvent(onSaved: () -> Unit) {
        val s = _state.value
        if (s.title.isBlank() || s.startsAt.isBlank()) return
        val maxAttendees =
            when {
                s.attendeeLimit.isBlank() -> null
                s.attendeeLimit.toIntOrNull() == null -> {
                    _state.value = s.copy(attendeeLimitError = "Podaj poprawny limit")
                    return
                }
                s.attendeeLimit.toInt() <= 0 -> {
                    _state.value = s.copy(attendeeLimitError = "Limit musi być większy od 0")
                    return
                }
                else -> s.attendeeLimit.toInt()
            }

        viewModelScope.launch {
            _state.value = s.copy(isLoading = true, error = null)
            val eventId = s.eventId
            if (eventId != null) {
                val request =
                    UpdateEventRequest(
                        title = s.title,
                        description = s.description.ifBlank { null },
                        coverImage = s.coverImageUrl,
                        location = s.location.ifBlank { null },
                        startsAt = s.startsAt,
                        endsAt = s.endsAt.ifBlank { null },
                        latitude = s.latitude,
                        longitude = s.longitude,
                        maxAttendees = maxAttendees,
                    )
                when (eventRepository.updateEvent(eventId, request)) {
                    is ApiResult.Success -> {
                        onSaved()
                    }

                    is ApiResult.Error -> {
                        _state.value = _state.value.copy(isLoading = false, error = "Nie udało się zaktualizować wydarzenia")
                    }
                }
            } else {
                val request =
                    CreateEventRequest(
                        title = s.title,
                        description = s.description.ifBlank { null },
                        coverImage = s.coverImageUrl,
                        location = s.location.ifBlank { null },
                        startsAt = s.startsAt,
                        endsAt = s.endsAt.ifBlank { null },
                        latitude = s.latitude,
                        longitude = s.longitude,
                        maxAttendees = maxAttendees,
                    )
                when (eventRepository.createEvent(request)) {
                    is ApiResult.Success -> {
                        onSaved()
                    }

                    is ApiResult.Error -> {
                        _state.value = _state.value.copy(isLoading = false, error = "Nie udało się utworzyć wydarzenia")
                    }
                }
            }
        }
    }
}
