package com.poziomki.app.ui.screen.chat

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import com.poziomki.app.api.ApiResult
import com.poziomki.app.api.ApiService
import com.poziomki.app.ui.screen.chat.model.NewChatUiState
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch

class NewChatViewModel(
    private val apiService: ApiService,
) : ViewModel() {
    private val _uiState = MutableStateFlow(NewChatUiState())
    val uiState: StateFlow<NewChatUiState> = _uiState.asStateFlow()

    init {
        loadProfiles()
    }

    fun loadProfiles() {
        viewModelScope.launch {
            _uiState.value = _uiState.value.copy(isLoading = true, error = null)
            when (val result = apiService.getMatchingProfiles()) {
                is ApiResult.Success -> _uiState.value = NewChatUiState(profiles = result.data)
                is ApiResult.Error -> _uiState.value = NewChatUiState(error = result.message)
            }
        }
    }
}
