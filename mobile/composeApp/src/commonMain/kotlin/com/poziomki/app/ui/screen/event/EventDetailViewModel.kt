package com.poziomki.app.ui.screen.event

import androidx.lifecycle.SavedStateHandle
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import androidx.navigation.toRoute
import com.poziomki.app.api.ApiResult
import com.poziomki.app.api.ApiService
import com.poziomki.app.api.Event
import com.poziomki.app.api.EventAttendee
import com.poziomki.app.ui.navigation.Route
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch

data class EventDetailState(
    val event: Event? = null,
    val attendees: List<EventAttendee> = emptyList(),
    val isLoading: Boolean = false,
    val error: String? = null,
)

class EventDetailViewModel(
    savedStateHandle: SavedStateHandle,
    private val apiService: ApiService,
) : ViewModel() {
    private val route = savedStateHandle.toRoute<Route.EventDetail>()
    private val eventId = route.id

    private val _state = MutableStateFlow(EventDetailState())
    val state: StateFlow<EventDetailState> = _state.asStateFlow()

    init {
        loadEvent()
    }

    private fun loadEvent() {
        viewModelScope.launch {
            _state.value = EventDetailState(isLoading = true)
            when (val result = apiService.getEvent(eventId)) {
                is ApiResult.Success -> {
                    _state.value = _state.value.copy(event = result.data, isLoading = false)
                    loadAttendees()
                }

                is ApiResult.Error -> {
                    _state.value = EventDetailState(error = result.message)
                }
            }
        }
    }

    private fun loadAttendees() {
        viewModelScope.launch {
            when (val result = apiService.getEventAttendees(eventId)) {
                is ApiResult.Success -> {
                    _state.value = _state.value.copy(attendees = result.data)
                }

                is ApiResult.Error -> {}
            }
        }
    }

    fun attendEvent() {
        viewModelScope.launch {
            when (apiService.attendEvent(eventId)) {
                is ApiResult.Success -> {
                    loadEvent()
                }

                is ApiResult.Error -> {}
            }
        }
    }

    fun leaveEvent() {
        viewModelScope.launch {
            when (apiService.leaveEvent(eventId)) {
                is ApiResult.Success -> {
                    loadEvent()
                }

                is ApiResult.Error -> {}
            }
        }
    }
}
