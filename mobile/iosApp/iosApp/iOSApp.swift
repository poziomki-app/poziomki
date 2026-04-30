import SwiftUI
import ComposeApp

@main
struct iOSApp: App {
    @UIApplicationDelegateAdaptor(AppDelegate.self) var appDelegate

    init() {
        let versionCode = (Bundle.main.infoDictionary?["CFBundleVersion"] as? String).flatMap(Int32.init) ?? 0
        KoinKt.doInitKoin(versionCode: versionCode)
    }

    var body: some Scene {
        WindowGroup {
            ContentView()
        }
    }
}
