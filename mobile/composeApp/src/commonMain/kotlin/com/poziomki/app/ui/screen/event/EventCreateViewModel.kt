package com.poziomki.app.ui.screen.event

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import com.poziomki.app.api.ApiResult
import com.poziomki.app.api.CreateEventRequest
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
    val isLoading: Boolean = false,
    val error: String? = null,
)

class EventCreateViewModel(
    private val eventRepository: EventRepository,
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

    fun updateStartsAt(startsAt: String) {
        _state.value = _state.value.copy(startsAt = startsAt)
    }

    fun createEvent(onCreated: () -> Unit) {
        val s = _state.value
        if (s.title.isBlank() || s.startsAt.isBlank()) return

        viewModelScope.launch {
            _state.value = s.copy(isLoading = true)
            val request =
                CreateEventRequest(
                    title = s.title,
                    description = s.description.ifBlank { null },
                    location = s.location.ifBlank { null },
                    startsAt = s.startsAt,
                )
            when (eventRepository.createEvent(request)) {
                is ApiResult.Success -> {
                    onCreated()
                }

                is ApiResult.Error -> {
                    _state.value = _state.value.copy(isLoading = false, error = "Failed to create event")
                }
            }
        }
    }
}
