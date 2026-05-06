package com.poziomki.app.ui.feature.home

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import com.poziomki.app.data.repository.MatchProfileRepository
import com.poziomki.app.data.repository.ProfileRepository
import com.poziomki.app.network.ApiResult
import com.poziomki.app.network.ApiService
import com.poziomki.app.network.MatchProfile
import com.poziomki.app.network.SearchResults
import com.poziomki.app.network.Tag
import kotlinx.coroutines.Job
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch

data class ExploreState(
    val profiles: List<MatchProfile> = emptyList(),
    val isLoading: Boolean = true,
    val isRefreshing: Boolean = false,
    val error: String? = null,
    val refreshError: String? = null,
    val query: String = "",
    val searchResults: SearchResults? = null,
    val isSearching: Boolean = false,
    val ownTags: List<Tag> = emptyList(),
) {
    companion object {
        const val RECOMMENDED_COUNT = 4
    }

    val recommendedProfiles get() = profiles.take(RECOMMENDED_COUNT)
    val remainingProfiles get() = profiles.drop(RECOMMENDED_COUNT)
}

class ExploreViewModel(
    private val matchProfileRepository: MatchProfileRepository,
    private val profileRepository: ProfileRepository,
    private val apiService: ApiService,
) : ViewModel() {
    private val _state = MutableStateFlow(ExploreState())
    val state: StateFlow<ExploreState> = _state.asStateFlow()
    private var searchJob: Job? = null

    init {
        observeProfiles()
        observeOwnTags()
        // Refresh own profile *before* the feed so the very first
        // emission already has ownTags hydrated. Without this the
        // initial cards on a fresh install render with empty matching
        // tags until the own-profile observer catches up.
        viewModelScope.launch {
            profileRepository.refreshOwnProfile()
            refreshFeed()
        }
    }

    private fun observeOwnTags() {
        viewModelScope.launch {
            profileRepository.observeOwnProfileWithTags().collect { own ->
                _state.value = _state.value.copy(ownTags = own?.tags ?: emptyList())
            }
        }
    }

    private fun observeProfiles() {
        viewModelScope.launch {
            matchProfileRepository.observeProfiles().collect { profiles ->
                val current = _state.value.profiles
                _state.value =
                    _state.value.copy(
                        profiles = if (profilesVisuallyEqual(current, profiles)) current else profiles,
                        isLoading = if (profiles.isNotEmpty()) false else _state.value.isLoading,
                    )
            }
        }
    }

    private fun profilesVisuallyEqual(
        a: List<MatchProfile>,
        b: List<MatchProfile>,
    ): Boolean {
        if (a.size != b.size) return false
        return a.indices.all { i ->
            val x = a[i]
            val y = b[i]
            x.id == y.id && x.name == y.name &&
                x.profilePicture == y.profilePicture &&
                x.program == y.program &&
                x.gradientStart == y.gradientStart &&
                x.gradientEnd == y.gradientEnd
        }
    }

    private suspend fun refreshFeed() {
        val success = matchProfileRepository.refreshProfiles()
        if (!success && _state.value.profiles.isNotEmpty()) {
            _state.value = _state.value.copy(refreshError = "Nie udało się odświeżyć profili")
        }
        _state.value = _state.value.copy(isLoading = false)
    }

    fun pullToRefresh() {
        viewModelScope.launch {
            _state.value = _state.value.copy(isRefreshing = true)
            val success = matchProfileRepository.refreshProfiles(forceRefresh = true)
            if (!success && _state.value.profiles.isNotEmpty()) {
                _state.value = _state.value.copy(refreshError = "Nie udało się odświeżyć profili")
            }
            _state.value = _state.value.copy(isRefreshing = false)
        }
    }

    fun clearRefreshError() {
        _state.value = _state.value.copy(refreshError = null)
    }

    fun updateQuery(query: String) {
        _state.value = _state.value.copy(query = query)
        searchJob?.cancel()

        if (query.length < 2) {
            _state.value = _state.value.copy(searchResults = null, isSearching = false)
            return
        }

        searchJob =
            viewModelScope.launch {
                delay(300)
                _state.value = _state.value.copy(isSearching = true)
                when (val result = apiService.search(query)) {
                    is ApiResult.Success -> {
                        _state.value =
                            _state.value.copy(
                                searchResults = result.data,
                                isSearching = false,
                            )
                    }

                    is ApiResult.Error -> {
                        _state.value =
                            _state.value.copy(
                                searchResults = null,
                                isSearching = false,
                            )
                    }
                }
            }
    }
}
