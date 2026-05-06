package com.poziomki.app.ui.navigation

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.asPaddingValues
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.navigationBars
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.statusBars
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.compositionLocalOf
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.hapticfeedback.HapticFeedbackType
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.platform.LocalHapticFeedback
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.navigation.NavBackStackEntry
import androidx.navigation.NavDestination.Companion.hasRoute
import androidx.navigation.NavGraph.Companion.findStartDestination
import androidx.navigation.NavHostController
import androidx.navigation.compose.NavHost
import androidx.navigation.compose.composable
import androidx.navigation.compose.currentBackStackEntryAsState
import androidx.navigation.compose.navigation
import androidx.navigation.compose.rememberNavController
import androidx.navigation.toRoute
import coil3.compose.AsyncImage
import com.adamglin.PhosphorIcons
import com.adamglin.phosphoricons.Bold
import com.adamglin.phosphoricons.Fill
import com.adamglin.phosphoricons.Regular
import com.adamglin.phosphoricons.bold.GearSix
import com.adamglin.phosphoricons.fill.CalendarDots
import com.adamglin.phosphoricons.fill.ChatCircle
import com.adamglin.phosphoricons.fill.UsersThree
import com.adamglin.phosphoricons.regular.CalendarDots
import com.adamglin.phosphoricons.regular.ChatCircle
import com.adamglin.phosphoricons.regular.UsersThree
import com.poziomki.app.chat.api.ChatClient
import com.poziomki.app.chat.push.NotificationChatTarget
import com.poziomki.app.data.repository.ChatRoomRepository
import com.poziomki.app.ui.designsystem.components.OfflineBanner
import com.poziomki.app.ui.designsystem.components.UserAvatar
import com.poziomki.app.ui.designsystem.theme.Background
import com.poziomki.app.ui.designsystem.theme.Primary
import com.poziomki.app.ui.designsystem.theme.TextMuted
import com.poziomki.app.ui.designsystem.theme.backSlide
import com.poziomki.app.ui.designsystem.theme.backSlideExit
import com.poziomki.app.ui.designsystem.theme.forwardSlide
import com.poziomki.app.ui.designsystem.theme.forwardSlideExit
import com.poziomki.app.ui.designsystem.theme.tabFadeIn
import com.poziomki.app.ui.designsystem.theme.tabFadeOut
import com.poziomki.app.ui.feature.auth.AuthViewModel
import com.poziomki.app.ui.feature.auth.ForgotPasswordScreen
import com.poziomki.app.ui.feature.auth.LoginScreen
import com.poziomki.app.ui.feature.auth.RegisterScreen
import com.poziomki.app.ui.feature.auth.ResetPasswordScreen
import com.poziomki.app.ui.feature.auth.VerifyScreen
import com.poziomki.app.ui.feature.chat.ChatScreen
import com.poziomki.app.ui.feature.chat.NewChatScreen
import com.poziomki.app.ui.feature.event.EventChatScreen
import com.poziomki.app.ui.feature.event.EventCreateScreen
import com.poziomki.app.ui.feature.home.EventsScreen
import com.poziomki.app.ui.feature.home.ExploreScreen
import com.poziomki.app.ui.feature.home.MessagesScreen
import com.poziomki.app.ui.feature.home.ProfileScreen
import com.poziomki.app.ui.feature.home.ProfileViewModel
import com.poziomki.app.ui.feature.home.SavedScreen
import com.poziomki.app.ui.feature.onboarding.BasicInfoScreen
import com.poziomki.app.ui.feature.onboarding.InterestsScreen
import com.poziomki.app.ui.feature.onboarding.ProfileSetupScreen
import com.poziomki.app.ui.feature.profile.PrivacyScreen
import com.poziomki.app.ui.feature.profile.ProfileEditScreen
import com.poziomki.app.ui.feature.profile.ProfileViewScreen
import kotlinx.coroutines.launch
import org.koin.compose.koinInject
import org.koin.compose.viewmodel.koinViewModel

data class BottomNavItem(
    val label: String,
    val icon: ImageVector,
    val selectedIcon: ImageVector,
    val route: Route,
)

val LocalNavBarPadding = compositionLocalOf { 0.dp }

val bottomNavItems =
    listOf(
        BottomNavItem("Poznaj", PhosphorIcons.Regular.UsersThree, PhosphorIcons.Fill.UsersThree, Route.Explore),
        BottomNavItem("Wydarzenia", PhosphorIcons.Regular.CalendarDots, PhosphorIcons.Fill.CalendarDots, Route.Events),
        BottomNavItem(
            "Wiadomości",
            PhosphorIcons.Regular.ChatCircle,
            PhosphorIcons.Fill.ChatCircle,
            Route.Messages,
        ),
    )

@Composable
private fun rememberGraphEntry(
    entry: NavBackStackEntry,
    navController: NavHostController,
    graphRoute: Route,
) = remember(entry) {
    try {
        navController.getBackStackEntry(graphRoute)
    } catch (_: Exception) {
        entry
    }
}

@Composable
fun AppNavigation(
    startDestination: Route,
    isLoggedIn: Boolean,
    navController: NavHostController = rememberNavController(),
) {
    val chatClient = koinInject<ChatClient>()
    val chatRoomRepository = koinInject<ChatRoomRepository>()
    val navigationScope = rememberCoroutineScope()

    // Navigate to auth screen only on actual logout (true → false), not on initial composition.
    var wasLoggedIn by remember { mutableStateOf(isLoggedIn) }
    LaunchedEffect(isLoggedIn) {
        if (wasLoggedIn && !isLoggedIn) {
            runCatching { chatClient.stop() }
            navController.navigate(Route.AuthGraph) {
                popUpTo(0) { inclusive = true }
            }
        }
        wasLoggedIn = isLoggedIn
    }

    val notificationChatTarget by NotificationChatTarget.roomId.collectAsState()
    LaunchedEffect(isLoggedIn, notificationChatTarget) {
        val roomId = notificationChatTarget ?: return@LaunchedEffect
        if (!isLoggedIn || startDestination == Route.OnboardingGraph) return@LaunchedEffect
        navController.navigate(Route.MainGraph) {
            popUpTo(0) { inclusive = true }
        }
        navController.navigate(Route.Chat(roomId))
        NotificationChatTarget.consume(roomId)
    }

    val navigateToChat: (String, String?) -> Unit = navigateToChat@{ chatTargetId, avatarHint ->
        if (chatTargetId.isBlank()) return@navigateToChat
        navigationScope.launch {
            val roomId =
                when {
                    chatTargetId.contains("-") -> {
                        // UUID conversation ID — use directly
                        chatTargetId
                    }

                    else -> {
                        // User ID — resolve DM conversation
                        chatClient.createDM(chatTargetId).getOrElse { error ->
                            println("Failed to create DM with $chatTargetId: ${error.message}")
                            return@launch
                        }
                    }
                }

            navController.navigate(Route.Chat(id = roomId, seedAvatarUrl = avatarHint))
        }
    }

    val navigateToDm: (String, String, String?) -> Unit = navigateToDm@{ userId, displayName, profileId ->
        if (userId.isBlank()) return@navigateToDm
        navigationScope.launch {
            val roomId =
                chatRoomRepository.resolveDirectRoom(userId).getOrElse { error ->
                    println("Failed to resolve DM room for $userId: ${error.message}")
                    return@launch
                }
            runCatching { chatClient.refreshRooms() }
            // Pre-hydrate the room locally before opening ChatScreen.
            // Fresh backend-created DMs can exist server-side for a moment before the local SDK
            // has a joined room + timeline, which otherwise leaves the composer in "connecting".
            val hydratedRoom = runCatching { chatClient.getJoinedRoom(roomId) }.getOrNull()
            if (hydratedRoom == null) {
                println("DM room $roomId is not hydrated locally yet; opening chat with retry path")
            }
            navController.navigate(
                Route.Chat(
                    id = roomId,
                    title = displayName.trim().ifBlank { null },
                    directUserId = userId,
                    directProfileId = profileId,
                ),
            )
        }
    }

    NavHost(
        navController = navController,
        startDestination = startDestination,
        modifier = Modifier.fillMaxSize(),
        enterTransition = { forwardSlide() },
        exitTransition = { forwardSlideExit() },
        popEnterTransition = { backSlide() },
        popExitTransition = { backSlideExit() },
    ) {
        // Auth graph
        navigation<Route.AuthGraph>(startDestination = Route.Login()) {
            composable<Route.Login> { backStackEntry ->
                val login = backStackEntry.toRoute<Route.Login>()
                LoginScreen(
                    onNavigateToRegister = { navController.navigate(Route.Register) },
                    onLoginSuccess = {
                        navController.navigate(Route.MainGraph) {
                            popUpTo(Route.AuthGraph) { inclusive = true }
                        }
                    },
                    onNeedsVerification = { email ->
                        navController.navigate(Route.Verify(email))
                    },
                    onNeedsOnboarding = {
                        navController.navigate(Route.OnboardingGraph) {
                            popUpTo(Route.AuthGraph) { inclusive = true }
                        }
                    },
                    onForgotPassword = {
                        navController.navigate(Route.ForgotPassword())
                    },
                    prefillEmail = login.prefillEmail,
                )
            }
            composable<Route.Register> {
                RegisterScreen(
                    onNavigateToLogin = { navController.popBackStack() },
                    onRegisterSuccess = { email ->
                        navController.navigate(Route.Verify(email))
                    },
                    onUserExists = { email ->
                        navController.navigate(Route.Login(prefillEmail = email)) {
                            popUpTo(Route.Login()) { inclusive = true }
                        }
                    },
                )
            }
            composable<Route.Verify> { backStackEntry ->
                val verify = backStackEntry.toRoute<Route.Verify>()
                VerifyScreen(
                    email = verify.email,
                    onVerifySuccess = {
                        navController.navigate(Route.OnboardingGraph) {
                            popUpTo(Route.AuthGraph) { inclusive = true }
                        }
                    },
                )
            }
            composable<Route.ForgotPassword> { entry ->
                val authEntry = rememberGraphEntry(entry, navController, Route.AuthGraph)
                val route = entry.toRoute<Route.ForgotPassword>()
                ForgotPasswordScreen(
                    onNavigateBack = { navController.popBackStack() },
                    onSuccess = { email ->
                        navController.navigate(Route.ForgotPasswordVerify(email))
                    },
                    prefillEmail = route.prefillEmail,
                    viewModel = koinViewModel(viewModelStoreOwner = authEntry),
                )
            }
            composable<Route.ForgotPasswordVerify> { entry ->
                val authEntry = rememberGraphEntry(entry, navController, Route.AuthGraph)
                val route = entry.toRoute<Route.ForgotPasswordVerify>()
                val viewModel = koinViewModel<AuthViewModel>(viewModelStoreOwner = authEntry)
                VerifyScreen(
                    email = route.email,
                    title = "resetowanie has\u0142a",
                    onSubmit = { email, otp, _ ->
                        viewModel.forgotPasswordVerify(email, otp) { resetToken ->
                            navController.navigate(Route.ResetPassword(email = route.email, resetToken = resetToken))
                        }
                    },
                    onResend = { email -> viewModel.forgotPasswordResend(email) },
                    viewModel = viewModel,
                )
            }
            composable<Route.ResetPassword> { entry ->
                val authEntry = rememberGraphEntry(entry, navController, Route.AuthGraph)
                val route = entry.toRoute<Route.ResetPassword>()
                ResetPasswordScreen(
                    email = route.email,
                    resetToken = route.resetToken,
                    onSuccess = {
                        navController.navigate(Route.MainGraph) {
                            popUpTo(Route.AuthGraph) { inclusive = true }
                        }
                    },
                    onNeedsOnboarding = {
                        navController.navigate(Route.OnboardingGraph) {
                            popUpTo(Route.AuthGraph) { inclusive = true }
                        }
                    },
                    viewModel = koinViewModel(viewModelStoreOwner = authEntry),
                )
            }
        }

        // Onboarding graph — shared ViewModel across all screens
        navigation<Route.OnboardingGraph>(startDestination = Route.BasicInfo) {
            composable<Route.BasicInfo> { entry ->
                val parentEntry = rememberGraphEntry(entry, navController, Route.OnboardingGraph)
                BasicInfoScreen(
                    onNext = { navController.navigate(Route.Interests) },
                    onBack = {
                        navController.navigate(Route.AuthGraph) {
                            popUpTo(Route.OnboardingGraph) { inclusive = true }
                        }
                    },
                    viewModel = koinViewModel(viewModelStoreOwner = parentEntry),
                )
            }
            composable<Route.Interests> { entry ->
                val parentEntry = rememberGraphEntry(entry, navController, Route.OnboardingGraph)
                InterestsScreen(
                    onNext = { navController.navigate(Route.ProfileSetup) },
                    onBack = { navController.popBackStack() },
                    viewModel = koinViewModel(viewModelStoreOwner = parentEntry),
                )
            }
            composable<Route.ProfileSetup> { entry ->
                val parentEntry = rememberGraphEntry(entry, navController, Route.OnboardingGraph)
                ProfileSetupScreen(
                    onComplete = {
                        navController.navigate(Route.MainGraph) {
                            popUpTo(Route.OnboardingGraph) { inclusive = true }
                        }
                    },
                    onBack = { navController.popBackStack() },
                    viewModel = koinViewModel(viewModelStoreOwner = parentEntry),
                )
            }
        }

        // Main graph with bottom navigation
        composable<Route.MainGraph> {
            MainScreen(
                onNavigateToEventDetail = { id -> navController.navigate(Route.EventDetail(id)) },
                onNavigateToEventCreate = { navController.navigate(Route.EventCreate) },
                onNavigateToProfileView = { id -> navController.navigate(Route.ProfileView(id)) },
                onNavigateToProfileEdit = { navController.navigate(Route.ProfileEdit) },
                onNavigateToPrivacy = { navController.navigate(Route.Privacy) },
                onNavigateToSaved = { navController.navigate(Route.Saved) },
                onNavigateToChat = navigateToChat,
                onNavigateToNewChat = { navController.navigate(Route.NewChat) },
                onSignOut = {
                    navController.navigate(Route.AuthGraph) {
                        popUpTo(Route.MainGraph) { inclusive = true }
                    }
                },
            )
        }

        // Detail screens
        composable<Route.EventDetail> {
            EventChatScreen(
                onBack = { navController.popBackStack() },
                onNavigateToProfile = { id -> navController.navigate(Route.ProfileView(id)) },
                onNavigateToEditEvent = { id -> navController.navigate(Route.EventEdit(id)) },
            )
        }
        composable<Route.EventCreate> {
            EventCreateScreen(
                onBack = { navController.popBackStack() },
                onCreated = { navController.popBackStack() },
            )
        }
        composable<Route.EventEdit> { backStackEntry ->
            val route = backStackEntry.toRoute<Route.EventEdit>()
            EventCreateScreen(
                onBack = { navController.popBackStack() },
                onCreated = { navController.popBackStack() },
                eventId = route.id,
            )
        }
        composable<Route.Saved> {
            SavedScreen(
                onBack = { navController.popBackStack() },
                onNavigateToEventDetail = { id -> navController.navigate(Route.EventDetail(id)) },
                onNavigateToProfileView = { id -> navController.navigate(Route.ProfileView(id)) },
            )
        }
        composable<Route.ProfileView> {
            ProfileViewScreen(
                onBack = { navController.popBackStack() },
                onNavigateToChat = navigateToDm,
            )
        }
        composable<Route.ProfileEdit> {
            ProfileEditScreen(
                onBack = { navController.popBackStack() },
            )
        }
        composable<Route.Privacy> {
            PrivacyScreen(
                onBack = { navController.popBackStack() },
                onPasswordChanged = {
                    navController.navigate(Route.AuthGraph) {
                        popUpTo(0) { inclusive = true }
                    }
                },
                onAccountDeleted = {
                    navController.navigate(Route.AuthGraph) {
                        popUpTo(0) { inclusive = true }
                    }
                },
            )
        }
        composable<Route.Chat> { backStackEntry ->
            val chat = backStackEntry.toRoute<Route.Chat>()
            ChatScreen(
                chatId = chat.id,
                initialTitle = chat.title,
                initialDirectUserId = chat.directUserId,
                initialDirectProfileId = chat.directProfileId,
                initialAvatarUrl = chat.seedAvatarUrl,
                onBack = { navController.popBackStack() },
                onNavigateToProfile = { id -> navController.navigate(Route.ProfileView(id)) },
            )
        }
        composable<Route.NewChat> {
            NewChatScreen(
                onBack = { navController.popBackStack() },
                onUserSelected = { userId, displayName, profileId ->
                    navigateToDm(userId, displayName, profileId)
                    navController.popBackStack(Route.NewChat, inclusive = true)
                },
            )
        }
    }
}

@Suppress("LongMethod", "LongParameterList")
@Composable
fun MainScreen(
    onNavigateToEventDetail: (String) -> Unit,
    onNavigateToEventCreate: () -> Unit,
    onNavigateToProfileView: (String) -> Unit,
    onNavigateToProfileEdit: () -> Unit,
    onNavigateToPrivacy: () -> Unit,
    onNavigateToSaved: () -> Unit,
    onNavigateToChat: (String, String?) -> Unit,
    onNavigateToNewChat: () -> Unit,
    onSignOut: () -> Unit,
) {
    val tabNavController = rememberNavController()
    val navBackStackEntry by tabNavController.currentBackStackEntryAsState()
    val currentDestination = navBackStackEntry?.destination
    val navBarHeight = 60.dp
    val bottomInsets = WindowInsets.navigationBars.asPaddingValues().calculateBottomPadding()
    val topInsets = WindowInsets.statusBars.asPaddingValues().calculateTopPadding()
    val safeTop = maxOf(topInsets, 16.dp)

    val profileViewModel: ProfileViewModel = koinViewModel()
    val profileState by profileViewModel.state.collectAsState()
    val profilePicture = profileState.profile?.profilePicture

    val navigateToProfileTab: () -> Unit = {
        tabNavController.navigate(Route.ProfileTab) {
            popUpTo(tabNavController.graph.findStartDestination().id) {
                saveState = true
            }
            launchSingleTop = true
            restoreState = true
        }
    }

    val profileAvatarAction: @Composable () -> Unit = {
        ProfileAvatarButton(
            profilePicture = profilePicture,
            onClick = navigateToProfileTab,
        )
    }

    Scaffold(
        containerColor = Background,
        bottomBar = {},
        contentWindowInsets = WindowInsets(0),
    ) { _ ->
        CompositionLocalProvider(LocalNavBarPadding provides (navBarHeight + bottomInsets)) {
            Column(modifier = Modifier.fillMaxSize().padding(top = safeTop)) {
                OfflineBanner()
                Box(modifier = Modifier.fillMaxSize().weight(1f)) {
                    NavHost(
                        navController = tabNavController,
                        startDestination = Route.Explore,
                        enterTransition = { tabFadeIn() },
                        exitTransition = { tabFadeOut() },
                        popEnterTransition = { tabFadeIn() },
                        popExitTransition = { tabFadeOut() },
                        modifier = Modifier.fillMaxSize(),
                    ) {
                        composable<Route.Explore> {
                            ExploreScreen(
                                onNavigateToProfile = onNavigateToProfileView,
                                onNavigateToEventDetail = onNavigateToEventDetail,
                                profileAvatarAction = profileAvatarAction,
                            )
                        }
                        composable<Route.Events> {
                            EventsScreen(
                                onNavigateToEventDetail = onNavigateToEventDetail,
                                onNavigateToEventCreate = onNavigateToEventCreate,
                                onNavigateToProfile = onNavigateToProfileView,
                                profileAvatarAction = profileAvatarAction,
                            )
                        }
                        composable<Route.Messages> {
                            MessagesScreen(
                                onNavigateToChat = onNavigateToChat,
                                onNavigateToNewChat = onNavigateToNewChat,
                                onNavigateToProfile = onNavigateToProfileView,
                                profileAvatarAction = profileAvatarAction,
                            )
                        }
                        composable<Route.ProfileTab> {
                            ProfileScreen(
                                onNavigateToEdit = onNavigateToProfileEdit,
                                onNavigateToPrivacy = onNavigateToPrivacy,
                                onNavigateToSaved = onNavigateToSaved,
                                onNavigateToProfileView = onNavigateToProfileView,
                                onSignOut = onSignOut,
                            )
                        }
                    }

                    // Bottom navbar
                    Box(
                        modifier =
                            Modifier
                                .align(Alignment.BottomCenter)
                                .fillMaxWidth()
                                .background(MaterialTheme.colorScheme.surface),
                        contentAlignment = Alignment.BottomCenter,
                    ) {
                        Row(
                            modifier =
                                Modifier
                                    .fillMaxWidth()
                                    .padding(
                                        start = 8.dp,
                                        top = 12.dp,
                                        end = 8.dp,
                                        bottom = 4.dp + bottomInsets,
                                    ),
                            horizontalArrangement = Arrangement.SpaceEvenly,
                            verticalAlignment = Alignment.CenterVertically,
                        ) {
                            val haptic = LocalHapticFeedback.current
                            bottomNavItems.forEach { item ->
                                val selected = currentDestination?.hasRoute(item.route::class) == true
                                val tint =
                                    if (selected) {
                                        MaterialTheme.colorScheme.onSurface
                                    } else {
                                        MaterialTheme.colorScheme.onSurfaceVariant
                                    }
                                Column(
                                    modifier =
                                        Modifier
                                            .weight(1f)
                                            .clickable(
                                                interactionSource = remember { MutableInteractionSource() },
                                                indication = null,
                                            ) {
                                                haptic.performHapticFeedback(HapticFeedbackType.TextHandleMove)
                                                tabNavController.navigate(item.route) {
                                                    popUpTo(tabNavController.graph.findStartDestination().id) {
                                                        saveState = true
                                                    }
                                                    launchSingleTop = true
                                                    restoreState = true
                                                }
                                            },
                                    horizontalAlignment = Alignment.CenterHorizontally,
                                ) {
                                    Icon(
                                        if (selected) item.selectedIcon else item.icon,
                                        contentDescription = item.label,
                                        modifier = Modifier.size(26.dp),
                                        tint = tint,
                                    )
                                    Spacer(modifier = Modifier.height(2.dp))
                                    Text(
                                        text = item.label,
                                        fontSize = 10.sp,
                                        fontWeight = if (selected) FontWeight.Bold else FontWeight.Normal,
                                        color = tint,
                                    )
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

@Composable
private fun ProfileAvatarButton(
    profilePicture: String?,
    onClick: () -> Unit,
) {
    val avatarSize = 28.dp
    IconButton(onClick = onClick) {
        if (profilePicture != null) {
            UserAvatar(
                picture = profilePicture,
                displayName = null,
                size = avatarSize,
            )
        } else {
            Icon(
                PhosphorIcons.Bold.GearSix,
                contentDescription = "Profil",
                modifier = Modifier.size(22.dp),
                tint = TextMuted,
            )
        }
    }
}
