package com.poziomki.app.ui.navigation

import androidx.compose.animation.EnterTransition
import androidx.compose.animation.ExitTransition
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
import androidx.compose.foundation.layout.navigationBars
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.statusBars
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.CalendarMonth
import androidx.compose.material.icons.filled.Chat
import androidx.compose.material.icons.filled.Groups
import androidx.compose.material.icons.filled.Person
import androidx.compose.material.icons.outlined.CalendarMonth
import androidx.compose.material.icons.outlined.Chat
import androidx.compose.material.icons.outlined.Groups
import androidx.compose.material.icons.outlined.Person
import androidx.compose.material3.Icon
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Surface
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.hapticfeedback.HapticFeedbackType
import androidx.compose.ui.platform.LocalHapticFeedback
import androidx.compose.ui.unit.dp
import androidx.navigation.NavDestination.Companion.hasRoute
import androidx.navigation.NavGraph.Companion.findStartDestination
import androidx.navigation.NavHostController
import androidx.navigation.compose.NavHost
import androidx.navigation.compose.composable
import androidx.navigation.compose.currentBackStackEntryAsState
import androidx.navigation.compose.navigation
import androidx.navigation.compose.rememberNavController
import androidx.navigation.toRoute
import com.poziomki.app.chat.matrix.api.MatrixClient
import com.poziomki.app.data.repository.ChatRoomRepository
import com.poziomki.app.ui.designsystem.components.OfflineBanner
import com.poziomki.app.ui.designsystem.theme.Background
import com.poziomki.app.ui.designsystem.theme.Primary
import com.poziomki.app.ui.designsystem.theme.TextMuted
import com.poziomki.app.ui.feature.auth.LoginScreen
import com.poziomki.app.ui.feature.auth.RegisterScreen
import com.poziomki.app.ui.feature.auth.VerifyScreen
import com.poziomki.app.ui.feature.chat.ChatScreen
import com.poziomki.app.ui.feature.chat.NewChatScreen
import com.poziomki.app.ui.feature.event.EventChatScreen
import com.poziomki.app.ui.feature.event.EventCreateScreen
import com.poziomki.app.ui.feature.home.EventsScreen
import com.poziomki.app.ui.feature.home.ExploreScreen
import com.poziomki.app.ui.feature.home.MessagesScreen
import com.poziomki.app.ui.feature.home.ProfileScreen
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

val bottomNavItems =
    listOf(
        BottomNavItem("Explore", Icons.Outlined.Groups, Icons.Filled.Groups, Route.Explore),
        BottomNavItem("Events", Icons.Outlined.CalendarMonth, Icons.Filled.CalendarMonth, Route.Events),
        BottomNavItem("Messages", Icons.Outlined.Chat, Icons.Filled.Chat, Route.Messages),
        BottomNavItem("Profile", Icons.Outlined.Person, Icons.Filled.Person, Route.ProfileTab),
    )

@Composable
fun AppNavigation(
    startDestination: Route,
    isLoggedIn: Boolean,
    navController: NavHostController = rememberNavController(),
) {
    val matrixClient = koinInject<MatrixClient>()
    val chatRoomRepository = koinInject<ChatRoomRepository>()
    val navigationScope = rememberCoroutineScope()

    // Navigate to auth screen only on actual logout (true → false), not on initial composition.
    var wasLoggedIn by remember { mutableStateOf(isLoggedIn) }
    LaunchedEffect(isLoggedIn) {
        if (wasLoggedIn && !isLoggedIn) {
            matrixClient.stop()
            navController.navigate(Route.AuthGraph) {
                popUpTo(0) { inclusive = true }
            }
        }
        wasLoggedIn = isLoggedIn
    }

    val navigateToChat: (String) -> Unit = navigateToChat@{ chatTargetId ->
        if (chatTargetId.isBlank()) return@navigateToChat
        navigationScope.launch {
            val roomId =
                when {
                    chatTargetId.startsWith("!") -> chatTargetId
                    else -> matrixClient.createDM(chatTargetId).getOrNull()
                } ?: return@launch

            navController.navigate(Route.Chat(roomId))
        }
    }

    val navigateToDm: (String, String) -> Unit = navigateToDm@{ userId, _ ->
        if (userId.isBlank()) return@navigateToDm
        navigationScope.launch {
            val roomId = chatRoomRepository.resolveDirectRoom(userId).getOrNull() ?: return@launch
            navController.navigate(Route.Chat(roomId))
        }
    }

    NavHost(
        navController = navController,
        startDestination = startDestination,
        modifier = Modifier.fillMaxSize(),
        enterTransition = { EnterTransition.None },
        exitTransition = { ExitTransition.None },
        popEnterTransition = { EnterTransition.None },
        popExitTransition = { ExitTransition.None },
    ) {
        // Auth graph
        navigation<Route.AuthGraph>(startDestination = Route.Login) {
            composable<Route.Login> {
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
                )
            }
            composable<Route.Register> {
                RegisterScreen(
                    onNavigateToLogin = { navController.popBackStack() },
                    onRegisterSuccess = { email ->
                        navController.navigate(Route.Verify(email))
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
        }

        // Onboarding graph — shared ViewModel across all screens
        navigation<Route.OnboardingGraph>(startDestination = Route.BasicInfo) {
            composable<Route.BasicInfo> { entry ->
                val parentEntry =
                    remember(entry) {
                        try {
                            navController.getBackStackEntry(Route.OnboardingGraph)
                        } catch (_: Exception) {
                            entry
                        }
                    }
                BasicInfoScreen(
                    onNext = { navController.navigate(Route.Interests) },
                    viewModel = koinViewModel(viewModelStoreOwner = parentEntry),
                )
            }
            composable<Route.Interests> { entry ->
                val parentEntry =
                    remember(entry) {
                        try {
                            navController.getBackStackEntry(Route.OnboardingGraph)
                        } catch (_: Exception) {
                            entry
                        }
                    }
                InterestsScreen(
                    onNext = { navController.navigate(Route.ProfileSetup) },
                    onBack = { navController.popBackStack() },
                    viewModel = koinViewModel(viewModelStoreOwner = parentEntry),
                )
            }
            composable<Route.ProfileSetup> { entry ->
                val parentEntry =
                    remember(entry) {
                        try {
                            navController.getBackStackEntry(Route.OnboardingGraph)
                        } catch (_: Exception) {
                            entry
                        }
                    }
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
            )
        }
        composable<Route.EventCreate> {
            EventCreateScreen(
                onBack = { navController.popBackStack() },
                onCreated = { navController.popBackStack() },
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
                onBack = { navController.popBackStack() },
                onNavigateToProfile = { id -> navController.navigate(Route.ProfileView(id)) },
            )
        }
        composable<Route.NewChat> {
            NewChatScreen(
                onBack = { navController.popBackStack() },
                onUserSelected = { userId, displayName ->
                    navigateToDm(userId, displayName)
                    navController.popBackStack(Route.NewChat, inclusive = true)
                },
            )
        }
    }
}

@Composable
fun MainScreen(
    onNavigateToEventDetail: (String) -> Unit,
    onNavigateToEventCreate: () -> Unit,
    onNavigateToProfileView: (String) -> Unit,
    onNavigateToProfileEdit: () -> Unit,
    onNavigateToPrivacy: () -> Unit,
    onNavigateToChat: (String) -> Unit,
    onNavigateToNewChat: () -> Unit,
    onSignOut: () -> Unit,
) {
    val tabNavController = rememberNavController()
    val navBackStackEntry by tabNavController.currentBackStackEntryAsState()
    val currentDestination = navBackStackEntry?.destination
    val navBarHeight = 56.dp
    val bottomInsets = WindowInsets.navigationBars.asPaddingValues().calculateBottomPadding()
    val topInsets = WindowInsets.statusBars.asPaddingValues().calculateTopPadding()
    val safeTop = maxOf(topInsets, 16.dp)

    Scaffold(
        containerColor = Background,
        bottomBar = {},
        contentWindowInsets = WindowInsets(0),
    ) { _ ->
        Column(modifier = Modifier.fillMaxSize().padding(top = safeTop)) {
            OfflineBanner()
            Box(modifier = Modifier.fillMaxSize().weight(1f)) {
                NavHost(
                    navController = tabNavController,
                    startDestination = Route.Explore,
                    enterTransition = { EnterTransition.None },
                    exitTransition = { ExitTransition.None },
                    popEnterTransition = { EnterTransition.None },
                    popExitTransition = { ExitTransition.None },
                    modifier =
                        Modifier
                            .fillMaxSize()
                            .padding(bottom = navBarHeight + bottomInsets + 8.dp),
                ) {
                    composable<Route.Explore> {
                        ExploreScreen(
                            onNavigateToProfile = onNavigateToProfileView,
                            onNavigateToEventDetail = onNavigateToEventDetail,
                        )
                    }
                    composable<Route.Events> {
                        EventsScreen(
                            onNavigateToEventDetail = onNavigateToEventDetail,
                            onNavigateToEventCreate = onNavigateToEventCreate,
                        )
                    }
                    composable<Route.Messages> {
                        MessagesScreen(
                            onNavigateToChat = onNavigateToChat,
                            onNavigateToNewChat = onNavigateToNewChat,
                            onNavigateToProfile = onNavigateToProfileView,
                        )
                    }
                    composable<Route.ProfileTab> {
                        ProfileScreen(
                            onNavigateToEdit = onNavigateToProfileEdit,
                            onNavigateToPrivacy = onNavigateToPrivacy,
                            onNavigateToProfileView = onNavigateToProfileView,
                            onSignOut = onSignOut,
                        )
                    }
                }

                // Floating pill-shaped bottom navbar with liquid glass effect
                val navBarShape = RoundedCornerShape(28.dp)
                Box(
                    modifier =
                        Modifier
                            .align(Alignment.BottomCenter)
                            .padding(
                                start = 16.dp,
                                end = 16.dp,
                                bottom = bottomInsets + 16.dp,
                            ).clip(navBarShape)
                            .background(
                                Brush.linearGradient(
                                    colors =
                                        listOf(
                                            Color(0xFF141A22),
                                            Color(0xFF0E1318),
                                            Color(0xFF101820),
                                        ),
                                    start =
                                        androidx.compose.ui.geometry
                                            .Offset(0f, 0f),
                                    end =
                                        androidx.compose.ui.geometry.Offset(
                                            Float.POSITIVE_INFINITY,
                                            Float.POSITIVE_INFINITY,
                                        ),
                                ),
                            ).background(
                                Brush.verticalGradient(
                                    colors =
                                        listOf(
                                            Color(0x08FFFFFF),
                                            Color.Transparent,
                                            Color(0x04FFFFFF),
                                        ),
                                ),
                            ).border(
                                width = 1.dp,
                                brush =
                                    Brush.linearGradient(
                                        colors =
                                            listOf(
                                                Color(0x18FFFFFF),
                                                Color(0x0AFFFFFF),
                                                Color(0x10FFFFFF),
                                            ),
                                    ),
                                shape = navBarShape,
                            ),
                ) {
                    Row(
                        modifier =
                            Modifier
                                .fillMaxWidth()
                                .padding(horizontal = 8.dp, vertical = 8.dp),
                        horizontalArrangement = Arrangement.SpaceEvenly,
                        verticalAlignment = Alignment.CenterVertically,
                    ) {
                        val haptic = LocalHapticFeedback.current
                        bottomNavItems.forEach { item ->
                            val selected = currentDestination?.hasRoute(item.route::class) == true
                            Box(
                                modifier =
                                    Modifier
                                        .size(48.dp)
                                        .clip(CircleShape)
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
                                contentAlignment = Alignment.Center,
                            ) {
                                Icon(
                                    if (selected) item.selectedIcon else item.icon,
                                    contentDescription = item.label,
                                    modifier = Modifier.size(26.dp),
                                    tint = if (selected) Primary else TextMuted,
                                )
                            }
                        }
                    }
                }
            }
        }
    }
}
