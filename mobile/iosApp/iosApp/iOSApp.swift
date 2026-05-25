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
    }

    var body: some Scene {
        WindowGroup {
            ContentView()
        }
    }
}
