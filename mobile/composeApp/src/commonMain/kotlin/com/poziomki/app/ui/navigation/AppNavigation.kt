package com.poziomki.app.ui.navigation

import androidx.compose.animation.EnterTransition
import androidx.compose.animation.ExitTransition
import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.animation.core.tween
import androidx.compose.foundation.background
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
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.Chat
import androidx.compose.material.icons.filled.CalendarMonth
import androidx.compose.material.icons.filled.Groups
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
import androidx.compose.ui.draw.drawBehind
import androidx.compose.ui.geometry.Offset
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
import com.poziomki.app.ui.component.OfflineBanner
import com.poziomki.app.ui.screen.auth.LoginScreen
import com.poziomki.app.ui.screen.auth.RegisterScreen
import com.poziomki.app.ui.screen.auth.VerifyScreen
import com.poziomki.app.ui.screen.chat.ChatScreen
import com.poziomki.app.ui.screen.chat.NewChatScreen
import com.poziomki.app.ui.screen.event.EventCreateScreen
import com.poziomki.app.ui.screen.event.EventDetailScreen
import com.poziomki.app.ui.screen.main.EventsScreen
import com.poziomki.app.ui.screen.main.ExploreScreen
import com.poziomki.app.ui.screen.main.MessagesScreen
import com.poziomki.app.ui.screen.main.ProfileScreen
import com.poziomki.app.ui.screen.onboarding.BasicInfoScreen
import com.poziomki.app.ui.screen.onboarding.InterestsScreen
import com.poziomki.app.ui.screen.onboarding.ProfileSetupScreen
import com.poziomki.app.ui.screen.profile.PrivacyScreen
import com.poziomki.app.ui.screen.profile.ProfileEditScreen
import com.poziomki.app.ui.screen.profile.ProfileViewScreen
import com.poziomki.app.ui.theme.Background
import com.poziomki.app.ui.theme.Border
import com.poziomki.app.ui.theme.Primary
import com.poziomki.app.ui.theme.TextMuted
import kotlinx.coroutines.launch
import org.koin.compose.koinInject
import org.koin.compose.viewmodel.koinViewModel
import com.poziomki.app.ui.theme.Surface as SurfaceColor

data class BottomNavItem(
    val label: String,
    val icon: ImageVector,
    val route: Route,
)

val bottomNavItems =
    listOf(
        BottomNavItem("Explore", Icons.Filled.Groups, Route.Explore),
        BottomNavItem("Events", Icons.Filled.CalendarMonth, Route.Events),
        BottomNavItem("Messages", Icons.AutoMirrored.Filled.Chat, Route.Messages),
        BottomNavItem("Profile", Icons.Outlined.Person, Route.ProfileTab),
    )

@Composable
fun AppNavigation(
    startDestination: Route,
    isLoggedIn: Boolean,
    navController: NavHostController = rememberNavController(),
) {
    val matrixClient = koinInject<MatrixClient>()
    val navigationScope = rememberCoroutineScope()

    // Navigate to auth screen only on actual logout (true → false), not on initial composition.
    var wasLoggedIn by remember { mutableStateOf(isLoggedIn) }
    LaunchedEffect(isLoggedIn) {
        if (wasLoggedIn && !isLoggedIn) {
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

    val navigateToDm: (String, String) -> Unit = navigateToDm@{ userId, displayName ->
        if (userId.isBlank()) return@navigateToDm
        navigationScope.launch {
            val roomId = matrixClient.createDM(userId, displayName).getOrNull() ?: return@launch
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
            EventDetailScreen(
                onBack = { navController.popBackStack() },
                onNavigateToChat = navigateToChat,
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
                onChatCreated = { id ->
                    navController.navigate(Route.Chat(id)) {
                        popUpTo(Route.NewChat) { inclusive = true }
                    }
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

    Scaffold(
        containerColor = Background,
        bottomBar = {},
    ) { innerPadding ->
        Column(modifier = Modifier.fillMaxSize().padding(innerPadding)) {
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
                            .padding(bottom = navBarHeight + bottomInsets + 24.dp),
                ) {
                    composable<Route.Explore> {
                        ExploreScreen(
                            onNavigateToProfile = onNavigateToProfileView,
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

                // Floating pill-shaped bottom navbar
                val selectedIndex =
                    bottomNavItems
                        .indexOfFirst { item ->
                            currentDestination?.hasRoute(item.route::class) == true
                        }.coerceAtLeast(0)
                val glowFraction by animateFloatAsState(
                    targetValue = (selectedIndex + 0.5f) / bottomNavItems.size,
                    animationSpec = tween(durationMillis = 350),
                )

                Surface(
                    modifier =
                        Modifier
                            .align(Alignment.BottomCenter)
                            .padding(
                                start = 16.dp,
                                end = 16.dp,
                                bottom = bottomInsets + 8.dp,
                            ),
                    shape = RoundedCornerShape(28.dp),
                    color = Color.Transparent,
                    border = androidx.compose.foundation.BorderStroke(1.dp, Border),
                ) {
                    Row(
                        modifier =
                            Modifier
                                .fillMaxWidth()
                                .background(
                                    Brush.verticalGradient(
                                        colors =
                                            listOf(
                                                Color(0xFF1A2029),
                                                Color(0xFF161B22),
                                            ),
                                    ),
                                ).drawBehind {
                                    // Subtle light shadow that follows the selected tab
                                    val centerX = size.width * glowFraction
                                    val centerY = size.height * 0.45f
                                    drawCircle(
                                        brush =
                                            Brush.radialGradient(
                                                colors =
                                                    listOf(
                                                        Color(0x0CFFFFFF),
                                                        Color.Transparent,
                                                    ),
                                                center = Offset(centerX, centerY),
                                                radius = size.width * 0.18f,
                                            ),
                                        radius = size.width * 0.18f,
                                        center = Offset(centerX, centerY),
                                    )
                                }.padding(horizontal = 8.dp, vertical = 8.dp),
                        horizontalArrangement = Arrangement.SpaceEvenly,
                        verticalAlignment = Alignment.CenterVertically,
                    ) {
                        val haptic = LocalHapticFeedback.current
                        bottomNavItems.forEachIndexed { index, item ->
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
                                    item.icon,
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
