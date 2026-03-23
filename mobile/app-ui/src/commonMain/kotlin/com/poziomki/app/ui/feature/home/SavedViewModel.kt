package com.poziomki.app.ui.feature.home

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import com.poziomki.app.data.repository.EventRepository
import com.poziomki.app.data.repository.ProfileRepository
import com.poziomki.app.network.Event
import com.poziomki.app.network.Profile
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch

data class SavedState(
    val events: List<Event> = emptyList(),
    val profiles: List<Profile> = emptyList(),
    val isLoading: Boolean = true,
    val isRefreshing: Boolean = false,
)

class SavedViewModel(
    private val eventRepository: EventRepository,
    private val profileRepository: ProfileRepository,
) : ViewModel() {
    private val _state = MutableStateFlow(SavedState())
    val state: StateFlow<SavedState> = _state.asStateFlow()

    init {
        observeSavedEvents()
        observeBookmarkedProfiles()
        refresh()
    }

    private fun observeSavedEvents() {
        viewModelScope.launch {
            eventRepository.observeSavedEvents().collect { events ->
                _state.update { it.copy(events = events) }
            }
        }
    }

    private fun observeBookmarkedProfiles() {
        viewModelScope.launch {
            profileRepository.observeBookmarkedProfiles().collect { profiles ->
                _state.update { it.copy(profiles = profiles) }
            }
        }
    }

    private fun refresh() {
        viewModelScope.launch {
            eventRepository.refreshSavedEvents()
            profileRepository.refreshBookmarkedProfiles()
            _state.update { it.copy(isLoading = false) }
        }
    }

    fun pullToRefresh() {
        _state.update { it.copy(isRefreshing = true) }
        viewModelScope.launch {
            eventRepository.refreshSavedEvents(forceRefresh = true)
            profileRepository.refreshBookmarkedProfiles()
            _state.update { it.copy(isRefreshing = false) }
        }
    }
}
