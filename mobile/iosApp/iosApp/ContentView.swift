import UIKit
import SwiftUI
import ComposeApp

struct ComposeView: UIViewControllerRepresentable {
    func makeUIViewController(context: Context) -> UIViewController {
        MainViewControllerKt.MainViewController()
    }

    func updateUIViewController(_ uiViewController: UIViewController, context: Context) {}
}

struct ContentView: View {
    var body: some View {
        // Black underlay prevents the iOS default white window from peeking
        // through during Compose navigation transitions (slide animations
        // briefly expose whatever is behind the moving screens).
        ZStack {
            Color.black.ignoresSafeArea()
            ComposeView().ignoresSafeArea()
        }
    }
}
