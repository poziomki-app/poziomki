package com.poziomki.app.ui.navigation

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
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
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Surface
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
import androidx.compose.ui.unit.dp
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
import com.adamglin.phosphoricons.bold.GearSix
import com.adamglin.phosphoricons.fill.PaperPlaneTilt
import com.poziomki.app.chat.api.ChatClient
import com.poziomki.app.chat.push.NotificationChatTarget
import com.poziomki.app.chat.push.NotificationDeepLinkTarget
import com.poziomki.app.chat.push.NotificationEventTarget
import com.poziomki.app.data.repository.ChatRoomRepository
import com.poziomki.app.ui.designsystem.Text
import com.poziomki.app.ui.designsystem.components.OfflineBanner
import com.poziomki.app.ui.designsystem.components.UserAvatar
import com.poziomki.app.ui.designsystem.theme.Background
import com.poziomki.app.ui.designsystem.theme.Primary
import com.poziomki.app.ui.designsystem.theme.backSlide
import com.poziomki.app.ui.designsystem.theme.backSlideExit
import com.poziomki.app.ui.designsystem.theme.forwardSlide
import com.poziomki.app.ui.designsystem.theme.forwardSlideExit
import com.poziomki.app.ui.designsystem.theme.tabFadeIn
import com.poziomki.app.ui.designsystem.theme.tabFadeOut
import com.poziomki.app.ui.feature.auth.AuthLandingScreen
import com.poziomki.app.ui.feature.auth.AuthViewModel
import com.poziomki.app.ui.feature.auth.ForgotPasswordScreen
import com.poziomki.app.ui.feature.auth.LoginScreen
import com.poziomki.app.ui.feature.auth.RegisterScreen
import com.poziomki.app.ui.feature.auth.ResetPasswordScreen
import com.poziomki.app.ui.feature.auth.VerifyScreen
import com.poziomki.app.ui.feature.chat.ChatScreen
import com.poziomki.app.ui.feature.event.EventChatScreen
import com.poziomki.app.ui.feature.event.EventCreateScreen
import com.poziomki.app.ui.feature.feedback.FeedbackBanner
import com.poziomki.app.ui.feature.feedback.FeedbackDialog
import com.poziomki.app.ui.feature.feedback.FeedbackViewModel
import com.poziomki.app.ui.feature.feedback.WelcomeDialog
import com.poziomki.app.ui.feature.home.EventsScreen
import com.poziomki.app.ui.feature.home.ExploreScreen
import com.poziomki.app.ui.feature.home.MessagesScreen
import com.poziomki.app.ui.feature.home.ProfileScreen
import com.poziomki.app.ui.feature.home.ProfileViewModel
import com.poziomki.app.ui.feature.home.SavedScreen
import com.poziomki.app.ui.feature.onboarding.BasicInfoScreen
import com.poziomki.app.ui.feature.onboarding.InterestsScreen
import com.poziomki.app.ui.feature.onboarding.ProfileSetupScreen
import com.poziomki.app.ui.feature.profile.PowiadomieniaScreen
import com.poziomki.app.ui.feature.profile.PrivacyScreen
import com.poziomki.app.ui.feature.profile.ProfileEditScreen
import com.poziomki.app.ui.feature.profile.ProfileViewScreen
import com.poziomki.app.ui.icons.MingcuteNavIcons
import com.poziomki.app.ui.perf.ScreenTraceHandle
import com.poziomki.app.ui.perf.startScreenTrace
import kotlinx.coroutines.launch
import org.koin.compose.koinInject
import org.koin.compose.viewmodel.koinViewModel

data class BottomNavItem(
    val label: String,
    val icon: ImageVector,
    val selectedIcon: ImageVector,
    val route: Route,
)

@Composable
private fun NavBarIcon(
    icon: ImageVector,
    contentDescription: String,
    tint: Color,
    showTopDot: Boolean,
    showBottomDot: Boolean,
) {
    Box(contentAlignment = Alignment.Center) {
        Icon(
            icon,
            contentDescription = contentDescription,
            modifier = Modifier.size(26.dp),
            tint = tint,
        )
        if (showTopDot) {
            Box(
                modifier =
                    Modifier
                        .align(Alignment.TopCenter)
                        .size(5.dp)
                        .clip(CircleShape)
                        .background(Primary),
            )
        }
        if (showBottomDot) {
            Box(
                modifier =
                    Modifier
                        .align(Alignment.BottomCenter)
                        .size(5.dp)
                        .clip(CircleShape)
                        .background(MaterialTheme.colorScheme.onSurfaceVariant.copy(alpha = 0.6f)),
            )
        }
    }
}

val LocalNavBarPadding = compositionLocalOf { 0.dp }

/**
 * Lets a screen request the bottom navbar (and the matching content
 * padding) to hide for an immersive view — the nearby events map needs
 * the whole height. The MutableState is reset on screen exit via
 * DisposableEffect by the caller.
 */
val LocalImmersive = compositionLocalOf { androidx.compose.runtime.mutableStateOf(false) }

val bottomNavItems =
    listOf(
        BottomNavItem("Poznaj", MingcuteNavIcons.UsersFill, MingcuteNavIcons.UsersFill, Route.Explore),
        BottomNavItem("Wydarzenia", MingcuteNavIcons.CalendarFill, MingcuteNavIcons.CalendarFill, Route.Events),
        BottomNavItem(
            "Wiadomości",
            PhosphorIcons.Fill.PaperPlaneTilt,
            PhosphorIcons.Fill.PaperPlaneTilt,
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

@Suppress("CyclomaticComplexMethod", "LongMethod")
@Composable
fun AppNavigation(
    startDestination: Route,
    isLoggedIn: Boolean,
    navController: NavHostController = rememberNavController(),
) {
    val chatClient = koinInject<ChatClient>()
    val chatRoomRepository = koinInject<ChatRoomRepository>()
    val navigationScope = rememberCoroutineScope()

    LaunchedEffect(navController) {
        var current: ScreenTraceHandle? = null
        navController.currentBackStackEntryFlow.collect { entry ->
            current?.stop()
            val route = entry.destination.route ?: "unknown"
            val name = route.substringAfterLast('.').substringBefore('/')
            current = startScreenTrace(name)
        }
    }

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

    val deepLink by NotificationDeepLinkTarget.link.collectAsState()
    LaunchedEffect(isLoggedIn, deepLink) {
        handleBroadcastDeepLink(deepLink, isLoggedIn, startDestination)
    }

    val notificationEventTarget by NotificationEventTarget.eventId.collectAsState()
    LaunchedEffect(isLoggedIn, notificationEventTarget) {
        val eventId = notificationEventTarget ?: return@LaunchedEffect
        if (!isLoggedIn || startDestination == Route.OnboardingGraph) return@LaunchedEffect
        navController.navigate(Route.MainGraph) {
            popUpTo(0) { inclusive = true }
        }
        navController.navigate(Route.EventDetail(eventId))
        NotificationEventTarget.consume(eventId)
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
        navigation<Route.AuthGraph>(startDestination = Route.AuthLanding) {
            composable<Route.AuthLanding> {
                AuthLandingScreen(
                    onSignUpWithEmail = { navController.navigate(Route.Register) },
                    onSignInWithEmail = { navController.navigate(Route.Login()) },
                )
            }
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
                onNavigateToPowiadomienia = { navController.navigate(Route.Powiadomienia) },
                onNavigateToSaved = { navController.navigate(Route.Saved) },
                onNavigateToChat = navigateToChat,
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
                onPasswordChanged = {},
                onAccountDeleted = {
                    navController.navigate(Route.AuthGraph) {
                        popUpTo(0) { inclusive = true }
                    }
                },
            )
        }
        composable<Route.Powiadomienia> {
            PowiadomieniaScreen(
                onBack = { navController.popBackStack() },
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
    }
}

// Broadcast deep links. Recognised schemes:
//   poziomki://chat/<roomId>   → forwarded into NotificationChatTarget
//   poziomki://event/<eventId> → forwarded into NotificationEventTarget
// Unknown schemes fall through — the app just opens at its current
// destination and the link is dropped so taps still open the app from
// a notification.
private fun handleBroadcastDeepLink(
    deepLink: String?,
    isLoggedIn: Boolean,
    startDestination: Route,
) {
    val link = deepLink ?: return
    if (!isLoggedIn || startDestination == Route.OnboardingGraph) {
        NotificationDeepLinkTarget.consume(link)
        return
    }
    val chatPrefix = "poziomki://chat/"
    val eventPrefix = "poziomki://event/"
    if (link.startsWith(chatPrefix)) {
        link.removePrefix(chatPrefix).takeIf { it.isNotBlank() }?.let(NotificationChatTarget::open)
    } else if (link.startsWith(eventPrefix)) {
        link.removePrefix(eventPrefix).takeIf { it.isNotBlank() }?.let(NotificationEventTarget::open)
    }
    NotificationDeepLinkTarget.consume(link)
}

@Suppress("LongMethod", "LongParameterList", "CyclomaticComplexMethod")
@Composable
fun MainScreen(
    onNavigateToEventDetail: (String) -> Unit,
    onNavigateToEventCreate: () -> Unit,
    onNavigateToProfileView: (String) -> Unit,
    onNavigateToProfileEdit: () -> Unit,
    onNavigateToPrivacy: () -> Unit,
    onNavigateToPowiadomienia: () -> Unit,
    onNavigateToSaved: () -> Unit,
    onNavigateToChat: (String, String?) -> Unit,
    onSignOut: () -> Unit,
) {
    val tabNavController = rememberNavController()
    val navBackStackEntry by tabNavController.currentBackStackEntryAsState()
    val currentDestination = navBackStackEntry?.destination
    // Bottom navbar Row: 16 top + 32 icon + 16 bottom = 64dp; +8dp breathing
    // room so last list items don't visually crowd the navbar.
    val navBarHeight = 72.dp
    val bottomInsets = WindowInsets.navigationBars.asPaddingValues().calculateBottomPadding()
    val topInsets = WindowInsets.statusBars.asPaddingValues().calculateTopPadding()
    val safeTop = maxOf(topInsets, 16.dp)

    val profileViewModel: ProfileViewModel = koinViewModel()
    val profileState by profileViewModel.state.collectAsState()
    val profilePicture = profileState.profile?.profilePicture

    val chatClient = koinInject<ChatClient>()
    val chatRooms by chatClient.rooms.collectAsState()
    val hasUnreadFriendMessages = chatRooms.any { it.isDirect && it.unreadCount > 0 }
    val hasUnreadEventMessages = chatRooms.any { !it.isDirect && it.unreadCount > 0 }

    val feedbackViewModel: FeedbackViewModel = koinViewModel()
    val feedbackState by feedbackViewModel.state.collectAsState()

    val navigateToProfileTab: () -> Unit = {
        tabNavController.navigate(Route.ProfileTab) {
            popUpTo(tabNavController.graph.findStartDestination().id) {
                saveState = true
            }
            launchSingleTop = true
            restoreState = true
        }
    }

    val profileAvatarAction: @Composable () -> Unit = {}

    val immersive = remember { mutableStateOf(false) }
    val navBarPadding = if (immersive.value) 0.dp else navBarHeight + bottomInsets

    Scaffold(
        containerColor = Background,
        bottomBar = {},
        contentWindowInsets = WindowInsets(0),
    ) { _ ->
        CompositionLocalProvider(
            LocalNavBarPadding provides navBarPadding,
            LocalImmersive provides immersive,
        ) {
            Column(modifier = Modifier.fillMaxSize().padding(top = safeTop)) {
                OfflineBanner()
                if (feedbackState.bannerVisible && !immersive.value) {
                    FeedbackBanner(
                        onClick = { feedbackViewModel.openDialog() },
                        onDismiss = { feedbackViewModel.dismissBanner() },
                    )
                }
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
                                onNavigateToProfile = onNavigateToProfileView,
                                profileAvatarAction = profileAvatarAction,
                            )
                        }
                        composable<Route.ProfileTab> {
                            ProfileScreen(
                                onNavigateToEdit = onNavigateToProfileEdit,
                                onNavigateToPrivacy = onNavigateToPrivacy,
                                onNavigateToPowiadomienia = onNavigateToPowiadomienia,
                                onNavigateToSaved = onNavigateToSaved,
                                onNavigateToProfileView = onNavigateToProfileView,
                                onOpenFeedback = { feedbackViewModel.openDialog() },
                                onSignOut = onSignOut,
                            )
                        }
                    }

                    // Bottom navbar — hidden when a screen requests immersive mode.
                    if (!immersive.value) {
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
                                            top = 16.dp,
                                            end = 8.dp,
                                            bottom = 16.dp + bottomInsets,
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
                                            MaterialTheme.colorScheme.onSurfaceVariant.copy(alpha = 0.55f)
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
                                        val isMessagesTab = item.route is Route.Messages
                                        NavBarIcon(
                                            icon = if (selected) item.selectedIcon else item.icon,
                                            contentDescription = item.label,
                                            tint = tint,
                                            showTopDot = isMessagesTab && hasUnreadFriendMessages,
                                            showBottomDot = isMessagesTab && hasUnreadEventMessages,
                                        )
                                    }
                                }
                                val profileSelected =
                                    currentDestination?.hasRoute(Route.ProfileTab::class) == true
                                Column(
                                    modifier =
                                        Modifier
                                            .weight(1f)
                                            .clickable(
                                                interactionSource = remember { MutableInteractionSource() },
                                                indication = null,
                                            ) {
                                                haptic.performHapticFeedback(HapticFeedbackType.TextHandleMove)
                                                navigateToProfileTab()
                                            },
                                    horizontalAlignment = Alignment.CenterHorizontally,
                                ) {
                                    if (profilePicture != null) {
                                        Box(
                                            modifier =
                                                Modifier
                                                    .size(28.dp)
                                                    .then(
                                                        if (profileSelected) {
                                                            Modifier
                                                                .border(
                                                                    width = 1.5.dp,
                                                                    color = MaterialTheme.colorScheme.onSurface,
                                                                    shape = CircleShape,
                                                                ).padding(3.dp)
                                                        } else {
                                                            Modifier
                                                        },
                                                    ),
                                            contentAlignment = Alignment.Center,
                                        ) {
                                            UserAvatar(
                                                picture = profilePicture,
                                                displayName = null,
                                                size = if (profileSelected) 22.dp else 26.dp,
                                            )
                                        }
                                    } else {
                                        Icon(
                                            PhosphorIcons.Bold.GearSix,
                                            contentDescription = "Profil",
                                            modifier = Modifier.size(26.dp),
                                            tint =
                                                if (profileSelected) {
                                                    MaterialTheme.colorScheme.onSurface
                                                } else {
                                                    MaterialTheme.colorScheme.onSurfaceVariant.copy(alpha = 0.55f)
                                                },
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

    if (feedbackState.showWelcome) {
        WelcomeDialog(onDismiss = { feedbackViewModel.dismissWelcome() })
    }
    if (feedbackState.dialogOpen) {
        FeedbackDialog(
            rating = feedbackState.rating,
            message = feedbackState.message,
            isSubmitting = feedbackState.isSubmitting,
            error = feedbackState.error,
            onRatingChange = { feedbackViewModel.setRating(it) },
            onMessageChange = { feedbackViewModel.setMessage(it) },
            onSubmit = { feedbackViewModel.submit(appVersion = null) },
            onDismiss = { feedbackViewModel.closeDialog() },
        )
    }
}
