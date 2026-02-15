package com.poziomki.app.ui.screen.event

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import com.poziomki.app.api.ApiResult
import com.poziomki.app.api.ApiService
import com.poziomki.app.api.CreateEventRequest
import com.poziomki.app.api.GeocodingService
import com.poziomki.app.api.UpdateEventRequest
import com.poziomki.app.data.repository.EventRepository
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch

data class EventCreateState(
    val title: String = "",
    val description: String = "",
    val location: String = "",
    val startsAt: String = "",
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

    fun updateTitle(title: String) {
        _state.value = _state.value.copy(title = title)
    }

    fun updateDescription(description: String) {
        _state.value = _state.value.copy(description = description)
    }

    fun updateLocation(location: String) {
        _state.value = _state.value.copy(location = location)
    }

    fun updateLocationWithCoordinates(name: String, lat: Double, lng: Double) {
        _state.value = _state.value.copy(location = name, latitude = lat, longitude = lng)
    }

    suspend fun searchLocation(query: String) = geocodingService.search(query)

    fun updateStartsAt(startsAt: String) {
        _state.value = _state.value.copy(startsAt = startsAt)
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
            eventRepository.observeEvent(eventId).collect { event ->
                if (event != null) {
                    _state.value =
                        _state.value.copy(
                            title = event.title,
                            description = event.description ?: "",
                            location = event.location ?: "",
                            startsAt = event.startsAt,
                            coverImageUrl = event.coverImage,
                            isLoading = false,
                            eventId = eventId,
                        )
                }
            }
        }
    }

    fun saveEvent(onSaved: () -> Unit) {
        val s = _state.value
        if (s.title.isBlank() || s.startsAt.isBlank()) return

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
                    )
                when (eventRepository.updateEvent(eventId, request)) {
                    is ApiResult.Success -> onSaved()
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
                    )
                when (eventRepository.createEvent(request)) {
                    is ApiResult.Success -> onSaved()
                    is ApiResult.Error -> {
                        _state.value = _state.value.copy(isLoading = false, error = "Nie udało się utworzyć wydarzenia")
                    }
                }
            }
        }
    }

    @Deprecated("Use saveEvent instead", replaceWith = ReplaceWith("saveEvent(onCreated)"))
    fun createEvent(onCreated: () -> Unit) = saveEvent(onCreated)
}
