import SwiftUI
import ComposeApp
import FirebaseCore

@main
struct iOSApp: App {
    init() {
        FirebaseApp.configure()
        let versionCode = (Bundle.main.infoDictionary?["CFBundleVersion"] as? String).flatMap(Int32.init) ?? 0
        KoinKt.doInitKoin(versionCode: versionCode)
    }

    var body: some Scene {
        WindowGroup {
            ContentView()
        }
    }
}
