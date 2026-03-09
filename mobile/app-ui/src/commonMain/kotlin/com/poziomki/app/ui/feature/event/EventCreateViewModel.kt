package com.poziomki.app.ui.feature.event

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import com.poziomki.app.data.repository.EventRepository
import com.poziomki.app.network.ApiResult
import com.poziomki.app.network.ApiService
import com.poziomki.app.network.CreateEventRequest
import com.poziomki.app.network.GeocodingService
import com.poziomki.app.network.Tag
import com.poziomki.app.network.UpdateEventRequest
import kotlinx.coroutines.FlowPreview
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.debounce
import kotlinx.coroutines.flow.distinctUntilChanged
import kotlinx.coroutines.flow.firstOrNull
import kotlinx.coroutines.launch

data class EventCreateState(
    val title: String = "",
    val description: String = "",
    val location: String = "",
    val startsAt: String = "",
    val endsAt: String = "",
    val coverImageUrl: String? = null,
    val coverImageBytes: ByteArray? = null,
    val isUploadingCover: Boolean = false,
    val latitude: Double? = null,
    val longitude: Double? = null,
    val selectedTags: List<Tag> = emptyList(),
    val tagQuery: String = "",
    val tagSearchResults: List<Tag> = emptyList(),
    val suggestedTags: List<Tag> = emptyList(),
    val isSearchingTags: Boolean = false,
    val isSuggestingTags: Boolean = false,
    val isCreatingTag: Boolean = false,
    val isLoading: Boolean = false,
    val error: String? = null,
    val eventId: String? = null,
)

@OptIn(FlowPreview::class)
class EventCreateViewModel(
    private val eventRepository: EventRepository,
    private val apiService: ApiService,
    private val geocodingService: GeocodingService,
) : ViewModel() {
    private val _state = MutableStateFlow(EventCreateState())
    val state: StateFlow<EventCreateState> = _state.asStateFlow()

    private val tagSearchFlow = MutableStateFlow("")
    private val tagSuggestionFlow = MutableStateFlow("")

    init {
        setupDebouncedSearch()
    }

    private fun setupDebouncedSearch() {
        viewModelScope.launch {
            tagSearchFlow
                .debounce(300)
                .distinctUntilChanged()
                .collect { query ->
                    if (query.length < 2) {
                        _state.value = _state.value.copy(
                            tagSearchResults = emptyList(),
                            isSearchingTags = false,
                        )
                        return@collect
                    }

                    _state.value = _state.value.copy(isSearchingTags = true)
                    val results =
                        when (val result = apiService.searchTags("event", query)) {
                            is ApiResult.Success -> result.data
                            is ApiResult.Error -> emptyList()
                        }
                    _state.value =
                        _state.value.copy(
                            tagSearchResults =
                                results.filterNot { tag ->
                                    _state.value.selectedTags.any { it.id == tag.id }
                                },
                            isSearchingTags = false,
                        )
                }
        }

        viewModelScope.launch {
            tagSuggestionFlow
                .debounce(500)
                .distinctUntilChanged()
                .collect { combined ->
                    if (combined.length < 3) {
                        _state.value = _state.value.copy(
                            suggestedTags = emptyList(),
                            isSuggestingTags = false,
                        )
                        return@collect
                    }

                    _state.value = _state.value.copy(isSuggestingTags = true)
                    val current = _state.value
                    val suggestions =
                        when (
                            val result = apiService.suggestTags(
                                scope = "event",
                                title = current.title,
                                description = current.description.ifBlank { null },
                            )
                        ) {
                            is ApiResult.Success -> result.data.map { it.tag }
                            is ApiResult.Error -> emptyList()
                        }
                    _state.value =
                        _state.value.copy(
                            suggestedTags =
                                suggestions.filterNot { tag ->
                                    _state.value.selectedTags.any { it.id == tag.id }
                                },
                            isSuggestingTags = false,
                        )
                }
        }
    }

    fun clearError() {
        _state.value = _state.value.copy(error = null)
    }

    fun updateTitle(title: String) {
        _state.value = _state.value.copy(title = title)
        refreshSuggestions()
    }

    fun updateDescription(description: String) {
        _state.value = _state.value.copy(description = description)
        refreshSuggestions()
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

    fun updateTagQuery(query: String) {
        _state.value = _state.value.copy(tagQuery = query)
        tagSearchFlow.value = query.trim()
    }

    fun addTag(tag: Tag) {
        val current = _state.value
        if (current.selectedTags.any { it.id == tag.id }) return
        _state.value =
            current.copy(
                selectedTags = current.selectedTags + tag,
                tagSearchResults = current.tagSearchResults.filterNot { it.id == tag.id },
                suggestedTags = current.suggestedTags.filterNot { it.id == tag.id },
                tagQuery = "",
            )
        tagSearchFlow.value = ""
    }

    fun removeTag(tag: Tag) {
        _state.value =
            _state.value.copy(
                selectedTags = _state.value.selectedTags.filterNot { it.id == tag.id },
            )
        refreshSuggestions()
    }

    fun dismissSuggestedTag(tag: Tag) {
        _state.value =
            _state.value.copy(
                suggestedTags = _state.value.suggestedTags.filterNot { it.id == tag.id },
            )
    }

    fun createAndAddTag(name: String) {
        viewModelScope.launch {
            val trimmed = name.trim()
            if (trimmed.isEmpty()) return@launch
            _state.value = _state.value.copy(isCreatingTag = true)
            when (val result = apiService.createTag(com.poziomki.app.network.CreateTagRequest(trimmed, "event"))) {
                is ApiResult.Success -> {
                    addTag(result.data)
                }

                is ApiResult.Error -> {
                    if (result.code == "CONFLICT") {
                        when (val searchResult = apiService.searchTags("event", trimmed)) {
                            is ApiResult.Success -> {
                                searchResult.data.firstOrNull {
                                    it.name.equals(trimmed, ignoreCase = true)
                                }?.let(::addTag)
                            }

                            is ApiResult.Error -> {
                                _state.value =
                                    _state.value.copy(error = "Nie udało się utworzyć tagu")
                            }
                        }
                    } else {
                        _state.value = _state.value.copy(error = "Nie udało się utworzyć tagu")
                    }
                }
            }
            _state.value = _state.value.copy(isCreatingTag = false)
        }
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
                        coverImageUrl = event.coverImage,
                        latitude = event.latitude,
                        longitude = event.longitude,
                        selectedTags = event.tags,
                        isLoading = false,
                        eventId = eventId,
                    )
                refreshSuggestions()
            } else {
                _state.value = _state.value.copy(isLoading = false, eventId = eventId)
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
                        endsAt = s.endsAt.ifBlank { null },
                        latitude = s.latitude,
                        longitude = s.longitude,
                        tagIds = s.selectedTags.map { it.id },
                    )
                when (eventRepository.updateEvent(eventId, request)) {
                    is ApiResult.Success -> onSaved()
                    is ApiResult.Error -> {
                        _state.value =
                            _state.value.copy(
                                isLoading = false,
                                error = "Nie udało się zaktualizować wydarzenia",
                            )
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
                        tagIds = s.selectedTags.map { it.id },
                    )
                when (eventRepository.createEvent(request)) {
                    is ApiResult.Success -> onSaved()
                    is ApiResult.Error -> {
                        _state.value =
                            _state.value.copy(
                                isLoading = false,
                                error = "Nie udało się utworzyć wydarzenia",
                            )
                    }
                }
            }
        }
    }

    private fun refreshSuggestions() {
        tagSuggestionFlow.value =
            buildString {
                append(_state.value.title.trim())
                val description = _state.value.description.trim()
                if (description.isNotEmpty()) {
                    append(' ')
                    append(description)
                }
            }.trim()
    }
}
