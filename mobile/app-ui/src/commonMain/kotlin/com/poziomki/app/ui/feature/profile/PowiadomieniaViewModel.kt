package com.poziomki.app.ui.feature.profile

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import com.poziomki.app.data.repository.SettingsRepository
import com.poziomki.app.session.SessionManager
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.launch

data class PowiadomieniaState(
    val masterEnabled: Boolean = true,
    val dms: Boolean = true,
    val eventChats: Boolean = false,
    val tagEvents: Boolean = false,
)

class PowiadomieniaViewModel(
    private val sessionManager: SessionManager,
    private val settingsRepository: SettingsRepository,
) : ViewModel() {
    private val _state = MutableStateFlow(PowiadomieniaState())
    val state: StateFlow<PowiadomieniaState> = _state.asStateFlow()

    init {
        viewModelScope.launch {
            val userId = sessionManager.userId.first() ?: return@launch
            settingsRepository.ensureDefaults(userId)
            val settings = settingsRepository.observeSettings(userId).first()
            if (settings != null) {
                _state.value =
                    PowiadomieniaState(
                        masterEnabled = settings.notifications_enabled != 0L,
                        dms = settings.notify_dms != 0L,
                        eventChats = settings.notify_event_chats != 0L,
                        tagEvents = settings.notify_tag_events != 0L,
                    )
            }
        }
    }

    fun toggleMaster(enabled: Boolean) {
        _state.value = _state.value.copy(masterEnabled = enabled)
        viewModelScope.launch {
            val userId = sessionManager.userId.first() ?: return@launch
            settingsRepository.updateNotifications(userId, enabled)
        }
    }

    fun toggleDms(enabled: Boolean) {
        _state.value = _state.value.copy(dms = enabled)
        viewModelScope.launch {
            val userId = sessionManager.userId.first() ?: return@launch
            settingsRepository.updateNotifyDms(userId, enabled)
        }
    }

    fun toggleEventChats(enabled: Boolean) {
        _state.value = _state.value.copy(eventChats = enabled)
        viewModelScope.launch {
            val userId = sessionManager.userId.first() ?: return@launch
            settingsRepository.updateNotifyEventChats(userId, enabled)
        }
    }

    fun toggleTagEvents(enabled: Boolean) {
        _state.value = _state.value.copy(tagEvents = enabled)
        viewModelScope.launch {
            val userId = sessionManager.userId.first() ?: return@launch
            settingsRepository.updateNotifyTagEvents(userId, enabled)
        }
    }
}
