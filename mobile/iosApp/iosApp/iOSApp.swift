import SwiftUI
import ComposeApp
import FirebaseCore

@main
struct iOSApp: App {
    init() {
        FirebaseApp.configure()
        // Chat uses a native URLSessionWebSocketTask instead of Ktor's Darwin
        // engine, whose WebSocket message-size cap drops the conversations frame.
        IosWebSocketBridgeKt.registerIosWebSocket(factory: IosWebSocketFactory())
        let versionCode = (Bundle.main.infoDictionary?["CFBundleVersion"] as? String).flatMap(Int32.init) ?? 0
        KoinKt.doInitKoin(versionCode: versionCode)
    }

    var body: some Scene {
        WindowGroup {
            ContentView()
        }
    }
}

/// Native WebSocket backing the shared chat client on iOS. `URLSessionWebSocketTask`
/// defaults to a 1 MB message limit, so it carries the conversations/history frames
/// that Ktor's Darwin engine resets with "Message too long".
final class IosWebSocketImpl: NSObject, NativeWebSocket {
    private let listener: NativeWebSocketListener
    private var task: URLSessionWebSocketTask?

    init(url: String, origin: String, listener: NativeWebSocketListener) {
        self.listener = listener
        super.init()
        guard let parsed = URL(string: url) else {
            listener.onError(message: "invalid ws url: \(url)")
            return
        }
        var request = URLRequest(url: parsed)
        request.setValue(origin, forHTTPHeaderField: "Origin")
        let task = URLSession.shared.webSocketTask(with: request)
        self.task = task
        task.resume()
        receiveLoop()
    }

    private func receiveLoop() {
        task?.receive { [weak self] result in
            guard let self else { return }
            switch result {
            case .failure(let error):
                self.listener.onError(message: error.localizedDescription)
            case .success(let message):
                switch message {
                case .string(let text):
                    self.listener.onText(text: text)
                case .data(let data):
                    if let text = String(data: data, encoding: .utf8) {
                        self.listener.onText(text: text)
                    }
                @unknown default:
                    break
                }
                self.receiveLoop()
            }
        }
    }

    func send(text: String) {
        task?.send(.string(text)) { [weak self] error in
            if let error {
                self?.listener.onError(message: error.localizedDescription)
            }
        }
    }

    func close() {
        task?.cancel(with: .goingAway, reason: nil)
        task = nil
    }
}

final class IosWebSocketFactory: NativeWebSocketFactory {
    func create(url: String, origin: String, listener: NativeWebSocketListener) -> NativeWebSocket {
        IosWebSocketImpl(url: url, origin: origin, listener: listener)
    }
}
