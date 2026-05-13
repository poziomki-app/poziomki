package com.poziomki.app.ui.feature.event

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import com.poziomki.app.data.repository.EventRepository
import com.poziomki.app.network.ApiResult
import com.poziomki.app.network.ApiService
import com.poziomki.app.network.CreateEventRequest
import com.poziomki.app.network.Tag
import com.poziomki.app.network.UpdateEventRequest
import kotlinx.coroutines.Job
import kotlinx.coroutines.delay
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
    val requiresApproval: Boolean = false,
    val visibility: String = "public",
    val selectedTags: List<Tag> = emptyList(),
    val tagSearchQuery: String = "",
    val tagSearchResults: List<Tag> = emptyList(),
    val isSearchingTags: Boolean = false,
    val isLoading: Boolean = false,
    val error: String? = null,
    val eventId: String? = null,
)

class EventCreateViewModel(
    private val eventRepository: EventRepository,
    private val apiService: ApiService,
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

    fun updateLocationWithCoordinates(
        name: String,
        lat: Double,
        lng: Double,
    ) {
        _state.value = _state.value.copy(location = name, latitude = lat, longitude = lng)
    }

    fun updateStartsAt(startsAt: String) {
        _state.value = _state.value.copy(startsAt = startsAt)
    }

    fun updateEndsAt(endsAt: String) {
        _state.value = _state.value.copy(endsAt = endsAt)
    }

    fun updateRequiresApproval(value: Boolean) {
        _state.value = _state.value.copy(requiresApproval = value)
    }

    fun updateVisibility(value: String) {
        _state.value = _state.value.copy(visibility = value)
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

    private var tagSearchJob: Job? = null

    fun updateTagSearch(query: String) {
        _state.value = _state.value.copy(tagSearchQuery = query)
        tagSearchJob?.cancel()
        if (query.isBlank()) {
            _state.value = _state.value.copy(tagSearchResults = emptyList(), isSearchingTags = false)
            return
        }
        tagSearchJob =
            viewModelScope.launch {
                delay(300)
                _state.value = _state.value.copy(isSearchingTags = true)
                when (val result = apiService.searchTags("event", query)) {
                    is ApiResult.Success -> {
                        _state.value =
                            _state.value.copy(tagSearchResults = result.data, isSearchingTags = false)
                    }

                    is ApiResult.Error -> {
                        _state.value = _state.value.copy(isSearchingTags = false)
                    }
                }
            }
    }

    fun addTag(tag: Tag) {
        val current = _state.value.selectedTags
        if (current.none { it.id == tag.id } && current.size < MAX_EVENT_TAGS) {
            _state.value =
                _state.value.copy(
                    selectedTags = current + tag,
                    tagSearchQuery = "",
                    tagSearchResults = emptyList(),
                )
        }
    }

    fun removeTag(tag: Tag) {
        _state.value = _state.value.copy(selectedTags = _state.value.selectedTags.filter { it.id != tag.id })
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
                        requiresApproval = event.requiresApproval,
                        visibility = event.visibility,
                        selectedTags = event.tags,
                        isLoading = false,
                        eventId = eventId,
                    )
            } else {
                _state.value = _state.value.copy(isLoading = false, eventId = eventId)
            }
        }
    }

    private fun parseAttendeeLimit(s: EventCreateState): Int? {
        if (s.attendeeLimit.isBlank()) return null
        val parsed = s.attendeeLimit.toIntOrNull()
        if (parsed == null) {
            _state.value = s.copy(attendeeLimitError = "Podaj poprawny limit")
            return null
        }
        if (parsed <= 0) {
            _state.value = s.copy(attendeeLimitError = "Limit musi być większy od 0")
            return null
        }
        return parsed
    }

    fun saveEvent(onSaved: () -> Unit) {
        val s = _state.value
        // Reject re-entry while a create/update is in flight. Without
        // this guard, isLoading is only set inside the launch block —
        // two synchronous taps both pass the validation prelude and
        // fire two POSTs, producing duplicate events on the server.
        if (s.isLoading) return
        if (s.title.isBlank() || s.startsAt.isBlank()) return
        if (s.attendeeLimit.isNotBlank() && s.attendeeLimit.toIntOrNull().let { it == null || it <= 0 }) {
            parseAttendeeLimit(s)
            return
        }
        val maxAttendees = if (s.attendeeLimit.isBlank()) null else s.attendeeLimit.toInt()

        // Flip isLoading synchronously so a follow-up tap that lands
        // before viewModelScope.launch is dispatched still sees the
        // guard above.
        _state.value = s.copy(isLoading = true, error = null)
        viewModelScope.launch {
            val eventId = s.eventId
            if (eventId != null) {
                submitUpdate(s, eventId, maxAttendees, onSaved)
            } else {
                submitCreate(s, maxAttendees, onSaved)
            }
        }
    }

    private suspend fun submitUpdate(
        s: EventCreateState,
        eventId: String,
        maxAttendees: Int?,
        onSaved: () -> Unit,
    ) {
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
                maxAttendees = UpdateEventRequest.maxAttendeesValue(maxAttendees),
                tagIds = s.selectedTags.map { it.id },
                requiresApproval = s.requiresApproval,
                visibility = s.visibility,
            )
        when (val result = eventRepository.updateEvent(eventId, request)) {
            is ApiResult.Success -> {
                onSaved()
            }

            is ApiResult.Error -> {
                _state.value =
                    _state.value.copy(
                        isLoading = false,
                        error = moderationAwareError(result, "Nie udało się zaktualizować wydarzenia"),
                    )
            }
        }
    }

    private suspend fun submitCreate(
        s: EventCreateState,
        maxAttendees: Int?,
        onSaved: () -> Unit,
    ) {
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
                tagIds = s.selectedTags.map { it.id },
                requiresApproval = if (s.requiresApproval) true else null,
                visibility = if (s.visibility != "public") s.visibility else null,
            )
        when (val result = eventRepository.createEvent(request)) {
            is ApiResult.Success -> {
                onSaved()
            }

            is ApiResult.Error -> {
                _state.value =
                    _state.value.copy(
                        isLoading = false,
                        error = moderationAwareError(result, "Nie udało się utworzyć wydarzenia"),
                    )
            }
        }
    }

    private fun moderationAwareError(
        result: ApiResult.Error,
        fallback: String,
    ): String =
        if (result.code == "EVENT_CONTENT_REJECTED" || result.code == "BIO_CONTENT_REJECTED") {
            // Server-side moderation rejection — its `message` is
            // already a user-facing Polish sentence with the
            // categories listed. Surface it verbatim.
            result.message
        } else {
            fallback
        }

    companion object {
        private const val MAX_EVENT_TAGS = 15
    }
}
