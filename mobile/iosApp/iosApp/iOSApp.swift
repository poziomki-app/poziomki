import SwiftUI
import ComposeApp
import FirebaseCore

@main
struct iOSApp: App {
    init() {
        FirebaseApp.configure()
        let versionCode = (Bundle.main.infoDictionary?["CFBundleVersion"] as? String).flatMap(Int32.init) ?? 0
        let apiBaseUrl = Bundle.main.infoDictionary?["API_BASE_URL"] as? String ?? "https://api.poziomki.app"
        KoinKt.doInitKoin(versionCode: versionCode, apiBaseUrl: apiBaseUrl)

        let env = ProcessInfo.processInfo.environment
        if let token = env["POZIOMKI_REVIEW_TOKEN"],
           let userId = env["POZIOMKI_REVIEW_USER_ID"],
           let email = env["POZIOMKI_REVIEW_EMAIL"],
           let name = env["POZIOMKI_REVIEW_NAME"],
           let profileId = env["POZIOMKI_REVIEW_PROFILE_ID"] {
            KoinKt.injectReviewSession(
                token: token, userId: userId, email: email, name: name, profileId: profileId
            )
        }
    }

    var body: some Scene {
        WindowGroup {
            ContentView()
        }
    }
}
