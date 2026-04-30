import UIKit
import UserNotifications
import ComposeApp

/// Wires APNs into the Compose Multiplatform app. The actual chat push
/// dispatch happens server-side; iOS just needs to:
///   1. Ask the user for permission to show notifications.
///   2. Register with APNs to obtain a device token.
///   3. Forward that token (as hex) to Kotlin so the backend can target it.
///
/// Swift can't construct the Koin-managed `IosPushBridge` directly, so we
/// route through a top-level Kotlin entry point: `KoinKt.doRegisterApnsToken`.
final class AppDelegate: NSObject, UIApplicationDelegate, UNUserNotificationCenterDelegate {
    func application(
        _ application: UIApplication,
        didFinishLaunchingWithOptions launchOptions: [UIApplication.LaunchOptionsKey: Any]? = nil
    ) -> Bool {
        UNUserNotificationCenter.current().delegate = self
        UNUserNotificationCenter.current().requestAuthorization(options: [.alert, .badge, .sound]) { granted, error in
            if let error = error {
                NSLog("APNs authorization error: \(error.localizedDescription)")
                return
            }
            guard granted else {
                NSLog("APNs authorization declined")
                return
            }
            DispatchQueue.main.async {
                application.registerForRemoteNotifications()
            }
        }
        return true
    }

    // MARK: APNs token

    func application(
        _ application: UIApplication,
        didRegisterForRemoteNotificationsWithDeviceToken deviceToken: Data
    ) {
        let hex = deviceToken.map { String(format: "%02x", $0) }.joined()
        KoinKt.doRegisterApnsToken(hexToken: hex)
    }

    func application(
        _ application: UIApplication,
        didFailToRegisterForRemoteNotificationsWithError error: Error
    ) {
        NSLog("APNs registration failed: \(error.localizedDescription)")
    }

    // MARK: Notification presentation

    /// Show banners for foreground notifications too — by default iOS
    /// suppresses them, but chat messages should still surface.
    func userNotificationCenter(
        _ center: UNUserNotificationCenter,
        willPresent notification: UNNotification,
        withCompletionHandler completionHandler: @escaping (UNNotificationPresentationOptions) -> Void
    ) {
        completionHandler([.banner, .sound, .badge])
    }

    /// Notification tap → currently a no-op routing path. The payload's
    /// `conversationId` is here for a future deep-link bridge; until then
    /// tapping just opens the app.
    func userNotificationCenter(
        _ center: UNUserNotificationCenter,
        didReceive response: UNNotificationResponse,
        withCompletionHandler completionHandler: @escaping () -> Void
    ) {
        completionHandler()
    }
}
