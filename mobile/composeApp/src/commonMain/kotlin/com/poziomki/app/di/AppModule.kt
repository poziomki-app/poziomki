package com.poziomki.app.di

import com.poziomki.app.ui.screen.auth.AuthViewModel
import com.poziomki.app.ui.screen.event.EventDetailViewModel
import com.poziomki.app.ui.screen.main.EventsViewModel
import com.poziomki.app.ui.screen.main.ExploreViewModel
import com.poziomki.app.ui.screen.main.ProfileViewModel
import com.poziomki.app.ui.screen.onboarding.OnboardingViewModel
import com.poziomki.app.ui.screen.profile.ProfileEditViewModel
import com.poziomki.app.ui.screen.profile.ProfileViewViewModel
import org.koin.core.module.dsl.viewModelOf
import org.koin.dsl.module

val appModule =
    module {
        viewModelOf(::AuthViewModel)
        viewModelOf(::OnboardingViewModel)
        viewModelOf(::ExploreViewModel)
        viewModelOf(::EventsViewModel)
        viewModelOf(::ProfileViewModel)
        viewModelOf(::EventDetailViewModel)
        viewModelOf(::ProfileEditViewModel)
        viewModelOf(::ProfileViewViewModel)
    }
