package com.poziomki.app.di

import com.poziomki.app.ui.feature.auth.AuthViewModel
import com.poziomki.app.ui.feature.chat.ChatViewModel
import com.poziomki.app.ui.feature.chat.NewChatViewModel
import com.poziomki.app.ui.feature.event.EventCreateViewModel
import com.poziomki.app.ui.feature.event.EventDetailViewModel
import com.poziomki.app.ui.feature.home.EventsViewModel
import com.poziomki.app.ui.feature.home.ExploreViewModel
import com.poziomki.app.ui.feature.home.MessagesViewModel
import com.poziomki.app.ui.feature.home.ProfileViewModel
import com.poziomki.app.ui.feature.onboarding.OnboardingViewModel
import com.poziomki.app.ui.feature.profile.PrivacyViewModel
import com.poziomki.app.ui.feature.profile.ProfileEditViewModel
import com.poziomki.app.ui.feature.profile.ProfileViewViewModel
import org.koin.core.module.dsl.viewModelOf
import org.koin.dsl.module

val appModule =
    module {
        viewModelOf(::AuthViewModel)
        viewModelOf(::OnboardingViewModel)
        viewModelOf(::ExploreViewModel)
        viewModelOf(::EventsViewModel)
        viewModelOf(::MessagesViewModel)
        viewModelOf(::ChatViewModel)
        viewModelOf(::NewChatViewModel)
        viewModelOf(::ProfileViewModel)
        viewModelOf(::EventDetailViewModel)
        viewModelOf(::EventCreateViewModel)
        viewModelOf(::ProfileEditViewModel)
        viewModelOf(::ProfileViewViewModel)
        viewModelOf(::PrivacyViewModel)
    }
