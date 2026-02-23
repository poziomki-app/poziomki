package com.poziomki.app.ui.feature.chat

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import com.poziomki.app.data.repository.MatchProfileRepository
import com.poziomki.app.ui.feature.chat.model.NewChatUiState
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch

class NewChatViewModel(
    private val matchProfileRepository: MatchProfileRepository,
) : ViewModel() {
    private val _uiState = MutableStateFlow(NewChatUiState(isLoading = true))
    val uiState: StateFlow<NewChatUiState> = _uiState.asStateFlow()

    init {
        observeProfiles()
        refreshProfiles()
    }

    private fun observeProfiles() {
        viewModelScope.launch {
            matchProfileRepository.observeProfiles().collect { profiles ->
                _uiState.value =
                    _uiState.value.copy(
                        profiles = profiles,
                        isLoading = if (profiles.isNotEmpty()) false else _uiState.value.isLoading,
                    )
            }
        }
    }

    private fun refreshProfiles() {
        viewModelScope.launch {
            matchProfileRepository.refreshProfiles()
            _uiState.value = _uiState.value.copy(isLoading = false)
        }
    }
}
