import Foundation
import os.log

/// Periodic WebSocket ping that keeps the bridge connection alive and lets
/// us notice a dead peer earlier than the OS would on its own. Owns a
/// background `Task`; `stop()` (or `deinit`) cancels it.
@available(macOS 13.0, *)
final class BridgeHeartbeat: @unchecked Sendable {
    private let interval: TimeInterval
    private let logger: Logger
    private var task: Task<Void, Never>?

    init(interval: TimeInterval, logger: Logger) {
        self.interval = interval
        self.logger = logger
    }

    deinit {
        task?.cancel()
    }

    /// Starts pinging `webSocketTask` every `interval` seconds. Replaces any
    /// previously running heartbeat.
    func start(pinging webSocketTask: URLSessionWebSocketTask) {
        stop()
        let interval = interval
        let logger = logger
        task = Task { [weak webSocketTask] in
            while !Task.isCancelled {
                do {
                    try await Task.sleep(nanoseconds: UInt64(interval * 1_000_000_000))
                } catch {
                    return
                }
                guard let webSocketTask else { return }
                await withCheckedContinuation { (continuation: CheckedContinuation<Void, Never>) in
                    webSocketTask.sendPing { error in
                        if let error {
                            logger.debug(
                                "Heartbeat ping failed: \(error.localizedDescription, privacy: .public)"
                            )
                        }
                        continuation.resume()
                    }
                }
            }
        }
    }

    func stop() {
        task?.cancel()
        task = nil
    }
}
