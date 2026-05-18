package com.poziomki.app.ui.feature.feedback

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import com.poziomki.app.network.ApiResult
import com.poziomki.app.network.ApiService
import com.poziomki.app.network.FeedbackRequest
import com.poziomki.app.session.AppPreferences
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.launchIn
import kotlinx.coroutines.flow.onEach
import kotlinx.coroutines.launch

data class FeedbackState(
    val showWelcome: Boolean = false,
    val bannerVisible: Boolean = false,
    val dialogOpen: Boolean = false,
    val rating: Int = 0,
    val message: String = "",
    val isSubmitting: Boolean = false,
    val submitted: Boolean = false,
    val error: String? = null,
)

class FeedbackViewModel(
    private val apiService: ApiService,
    private val appPreferences: AppPreferences,
) : ViewModel() {
    private val _state = MutableStateFlow(FeedbackState())
    val state: StateFlow<FeedbackState> = _state.asStateFlow()

    init {
        appPreferences.welcomeSeen
            .onEach { seen -> _state.value = _state.value.copy(showWelcome = !seen) }
            .launchIn(viewModelScope)
        appPreferences.feedbackBannerDismissed
            .onEach { dismissed -> _state.value = _state.value.copy(bannerVisible = !dismissed) }
            .launchIn(viewModelScope)
    }

    fun dismissWelcome() {
        _state.value = _state.value.copy(showWelcome = false)
        viewModelScope.launch { appPreferences.setWelcomeSeen(true) }
    }

    fun dismissBanner() {
        _state.value = _state.value.copy(bannerVisible = false)
        viewModelScope.launch { appPreferences.setFeedbackBannerDismissed(true) }
    }

    fun openDialog() {
        _state.value =
            _state.value.copy(
                dialogOpen = true,
                rating = 0,
                message = "",
                submitted = false,
                error = null,
            )
    }

    fun closeDialog() {
        _state.value = _state.value.copy(dialogOpen = false)
    }

    fun setRating(value: Int) {
        _state.value = _state.value.copy(rating = value, error = null)
    }

    fun setMessage(value: String) {
        _state.value = _state.value.copy(message = value)
    }

    fun submit(appVersion: String?) {
        val current = _state.value
        if (current.rating !in 1..5 || current.isSubmitting) return
        viewModelScope.launch {
            _state.value = current.copy(isSubmitting = true, error = null)
            val req =
                FeedbackRequest(
                    rating = current.rating,
                    message = current.message.trim().takeIf { it.isNotEmpty() },
                    appVersion = appVersion,
                )
            when (apiService.submitFeedback(req)) {
                is ApiResult.Success -> {
                    _state.value =
                        _state.value.copy(
                            isSubmitting = false,
                            submitted = true,
                            dialogOpen = false,
                            bannerVisible = false,
                        )
                    appPreferences.setFeedbackBannerDismissed(true)
                }

                is ApiResult.Error -> {
                    _state.value =
                        _state.value.copy(
                            isSubmitting = false,
                            error = "Nie udało się wysłać opinii. Spróbuj ponownie.",
                        )
                }
            }
        }
    }

    fun clearSubmitted() {
        _state.value = _state.value.copy(submitted = false)
    }
}
