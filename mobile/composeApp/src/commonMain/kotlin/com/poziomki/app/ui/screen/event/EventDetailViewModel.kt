package com.poziomki.app.ui.screen.event

import androidx.lifecycle.SavedStateHandle
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import androidx.navigation.toRoute
import com.poziomki.app.api.ApiResult
import com.poziomki.app.api.Event
import com.poziomki.app.api.EventAttendee
import com.poziomki.app.data.repository.EventRepository
import com.poziomki.app.ui.component.SnackbarType
import com.poziomki.app.ui.navigation.Route
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch

data class EventDetailState(
    val event: Event? = null,
    val attendees: List<EventAttendee> = emptyList(),
    val isLoading: Boolean = false,
    val isOpeningChat: Boolean = false,
    val isUpdatingAttendance: Boolean = false,
    val error: String? = null,
    val snackbarMessage: String? = null,
    val snackbarType: SnackbarType = SnackbarType.ERROR,
)

class EventDetailViewModel(
    savedStateHandle: SavedStateHandle,
    private val eventRepository: EventRepository,
) : ViewModel() {
    private val route = savedStateHandle.toRoute<Route.EventDetail>()
    private val eventId = route.id

    private val _state = MutableStateFlow(EventDetailState())
    val state: StateFlow<EventDetailState> = _state.asStateFlow()

    init {
        observeEvent()
        observeAttendees()
        refreshData()
    }

    private fun observeEvent() {
        viewModelScope.launch {
            eventRepository.observeEvent(eventId).collect { event ->
                _state.value = _state.value.copy(event = event, isLoading = false)
            }
        }
    }

    private fun observeAttendees() {
        viewModelScope.launch {
            eventRepository.observeAttendees(eventId).collect { attendees ->
                _state.value = _state.value.copy(attendees = attendees)
            }
        }
    }

    private fun refreshData() {
        viewModelScope.launch {
            _state.value = EventDetailState(isLoading = true)
            eventRepository.refreshEvent(eventId)
            eventRepository.refreshAttendees(eventId)
        }
    }

    fun clearSnackbar() {
        _state.value = _state.value.copy(snackbarMessage = null)
    }

    fun attendEvent() {
        if (_state.value.isUpdatingAttendance) return
        viewModelScope.launch {
            _state.value = _state.value.copy(isUpdatingAttendance = true)
            when (eventRepository.attendEvent(eventId)) {
                is ApiResult.Success -> {
                    eventRepository.refreshAttendees(eventId)
                }

                is ApiResult.Error -> {
                    _state.value =
                        _state.value.copy(
                            snackbarMessage = "nie uda\u0142o si\u0119 zapisa\u0107 na wydarzenie",
                            snackbarType = SnackbarType.ERROR,
                        )
                }
            }
            _state.value = _state.value.copy(isUpdatingAttendance = false)
        }
    }

    fun leaveEvent() {
        if (_state.value.isUpdatingAttendance) return
        viewModelScope.launch {
            _state.value = _state.value.copy(isUpdatingAttendance = true)
            when (eventRepository.leaveEvent(eventId)) {
                is ApiResult.Success -> {
                    eventRepository.refreshAttendees(eventId)
                }

                is ApiResult.Error -> {
                    _state.value =
                        _state.value.copy(
                            snackbarMessage = "nie uda\u0142o si\u0119 opu\u015bci\u0107 wydarzenia",
                            snackbarType = SnackbarType.ERROR,
                        )
                }
            }
            _state.value = _state.value.copy(isUpdatingAttendance = false)
        }
    }

    fun openEventChat(onNavigateToChat: (String) -> Unit) {
        val currentEvent = _state.value.event ?: return
        if (_state.value.isOpeningChat) return

        viewModelScope.launch {
            _state.value = _state.value.copy(isOpeningChat = true, error = null)

            val roomResult =
                eventRepository.ensureEventRoom(
                    eventId = eventId,
                    fallbackName = currentEvent.title,
                    attendeeUserIds = _state.value.attendees.mapNotNull { it.userId },
                )

            roomResult
                .onSuccess { roomId ->
                    _state.value = _state.value.copy(isOpeningChat = false)
                    onNavigateToChat(roomId)
                }.onFailure { throwable ->
                    _state.value =
                        _state.value.copy(
                            isOpeningChat = false,
                            error = throwable.message ?: "Nie udalo sie otworzyc czatu wydarzenia",
                        )
                }
        }
    }
}
