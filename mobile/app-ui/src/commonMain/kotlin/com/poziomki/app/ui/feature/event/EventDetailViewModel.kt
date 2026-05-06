package com.poziomki.app.ui.feature.event

import androidx.lifecycle.SavedStateHandle
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import androidx.navigation.toRoute
import com.poziomki.app.data.repository.EventRepository
import com.poziomki.app.data.repository.ProfileRepository
import com.poziomki.app.network.ApiResult
import com.poziomki.app.network.Event
import com.poziomki.app.network.EventAttendee
import com.poziomki.app.ui.designsystem.components.SnackbarType
import com.poziomki.app.ui.navigation.Route
import kotlinx.coroutines.TimeoutCancellationException
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.combine
import kotlinx.coroutines.launch
import kotlinx.coroutines.withTimeout

data class EventDetailState(
    val event: Event? = null,
    val attendees: List<EventAttendee> = emptyList(),
    val isLoading: Boolean = true,
    val isOpeningChat: Boolean = false,
    val chatOpenError: String? = null,
    val isUpdatingAttendance: Boolean = false,
    val isCreator: Boolean = false,
    val snackbarMessage: String? = null,
    val snackbarType: SnackbarType = SnackbarType.ERROR,
)

@Suppress("TooManyFunctions")
class EventDetailViewModel(
    savedStateHandle: SavedStateHandle,
    private val eventRepository: EventRepository,
    private val profileRepository: ProfileRepository,
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
            eventRepository
                .observeEvent(eventId)
                .combine(profileRepository.observeOwnProfile()) { event, profile ->
                    val isCreator = event?.creator?.id != null && event.creator?.id == profile?.id
                    _state.value = _state.value.copy(event = event, isLoading = false, isCreator = isCreator)
                }.collect {}
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
            _state.value = _state.value.copy(isLoading = true)
            val success = eventRepository.refreshEvent(eventId)
            eventRepository.refreshAttendees(eventId)
            if (!success) {
                _state.value = _state.value.copy(isLoading = false)
            }
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
                            snackbarMessage = "nie udało się dołączyć do wydarzenia",
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

    fun approveAttendee(profileId: String) {
        viewModelScope.launch {
            when (eventRepository.approveAttendee(eventId, profileId)) {
                is ApiResult.Success -> {
                    eventRepository.refreshAttendees(eventId)
                    eventRepository.refreshEvent(eventId, forceRefresh = true)
                }

                is ApiResult.Error -> {
                    _state.value =
                        _state.value.copy(
                            snackbarMessage = "nie uda\u0142o si\u0119 zaakceptowa\u0107 uczestnika",
                            snackbarType = SnackbarType.ERROR,
                        )
                }
            }
        }
    }

    fun rejectAttendee(profileId: String) {
        viewModelScope.launch {
            when (eventRepository.rejectAttendee(eventId, profileId)) {
                is ApiResult.Success -> {
                    eventRepository.refreshAttendees(eventId)
                }

                is ApiResult.Error -> {
                    _state.value =
                        _state.value.copy(
                            snackbarMessage = "nie uda\u0142o si\u0119 odrzuci\u0107 uczestnika",
                            snackbarType = SnackbarType.ERROR,
                        )
                }
            }
        }
    }

    fun deleteEvent(onDeleted: () -> Unit) {
        viewModelScope.launch {
            when (eventRepository.deleteEvent(eventId)) {
                is ApiResult.Success -> {
                    onDeleted()
                }

                is ApiResult.Error -> {
                    _state.value =
                        _state.value.copy(
                            snackbarMessage = "nie udało się usunąć wydarzenia",
                            snackbarType = SnackbarType.ERROR,
                        )
                }
            }
        }
    }

    fun openEventChat() {
        val currentEvent = _state.value.event ?: return
        if (_state.value.isOpeningChat) return
        if (!currentEvent.isAttending) {
            _state.value =
                _state.value.copy(
                    snackbarMessage = "Najpierw dołącz do wydarzenia, aby otworzyć czat",
                    snackbarType = SnackbarType.ERROR,
                )
            return
        }

        viewModelScope.launch {
            _state.value = _state.value.copy(isOpeningChat = true, chatOpenError = null)

            val result =
                runCatching {
                    withTimeout(EVENT_CHAT_OPEN_TIMEOUT_MS) {
                        eventRepository.ensureEventRoom(eventId = eventId)
                    }
                }

            val errorMessage: String? =
                result.fold(
                    onSuccess = { roomResult ->
                        roomResult.exceptionOrNull()?.let { throwable ->
                            throwable.message ?: "Nie udało się otworzyć czatu wydarzenia"
                        }
                    },
                    onFailure = { throwable ->
                        if (throwable is TimeoutCancellationException) {
                            "Nie udało się otworzyć czatu, spróbuj ponownie"
                        } else {
                            throwable.message ?: "Nie udało się otworzyć czatu wydarzenia"
                        }
                    },
                )

            _state.value =
                _state.value.copy(
                    isOpeningChat = false,
                    chatOpenError = errorMessage,
                    snackbarMessage = errorMessage ?: _state.value.snackbarMessage,
                    snackbarType = if (errorMessage != null) SnackbarType.ERROR else _state.value.snackbarType,
                )
        }
    }

    fun retryOpenEventChat() {
        _state.value = _state.value.copy(chatOpenError = null)
        openEventChat()
    }

    companion object {
        private const val EVENT_CHAT_OPEN_TIMEOUT_MS = 8_000L
    }
}
