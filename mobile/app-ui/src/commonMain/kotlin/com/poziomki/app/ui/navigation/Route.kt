package com.poziomki.app.ui.navigation

import kotlinx.serialization.Serializable

sealed interface Route {
    // Auth
    @Serializable data object AuthGraph : Route

    @Serializable data class Login(
        val prefillEmail: String? = null,
    ) : Route

    @Serializable data object Register : Route

    @Serializable data class Verify(
        val email: String,
    ) : Route

    // Onboarding
    @Serializable data object OnboardingGraph : Route

    @Serializable data object BasicInfo : Route

    @Serializable data object Interests : Route

    @Serializable data object ProfileSetup : Route

    // Main (tabs)
    @Serializable data object MainGraph : Route

    @Serializable data object Explore : Route

    @Serializable data object Events : Route

    @Serializable data object Messages : Route

    @Serializable data object ProfileTab : Route

    // Detail screens
    @Serializable data class EventDetail(
        val id: String,
    ) : Route

    @Serializable data object EventCreate : Route

    @Serializable data class EventEdit(
        val id: String,
    ) : Route

    @Serializable data class ProfileView(
        val id: String,
    ) : Route

    @Serializable data object ProfileEdit : Route

    @Serializable data object Privacy : Route

    @Serializable data class Chat(
        val id: String,
        val title: String? = null,
        val directUserId: String? = null,
        val directProfileId: String? = null,
    ) : Route

    @Serializable data object NewChat : Route
}
