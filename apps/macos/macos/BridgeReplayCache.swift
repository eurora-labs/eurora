import Foundation

/// Thread-safe store for the most-recent `ASSETS`/`SNAPSHOT` event frames
/// observed on the bridge. The desktop side may restart while Safari is
/// still running, so we replay the latest of each on reconnect to repopulate
/// its in-memory view of browser state.
@available(macOS 13.0, *)
final class BridgeReplayCache: @unchecked Sendable {
    private let lock = NSLock()
    private var assets: Frame?
    private var snapshot: Frame?

    /// Records `frame` if it carries a replayable event; ignored otherwise.
    func record(_ frame: Frame) {
        guard case let .event(event) = frame.kind else { return }
        lock.lock()
        defer { lock.unlock() }
        switch event.action {
        case "ASSETS":
            assets = frame
        case "SNAPSHOT":
            snapshot = frame
        default:
            break
        }
    }

    /// Cached frames in the order they should be replayed after a reconnect.
    func replayables() -> [Frame] {
        lock.lock()
        defer { lock.unlock() }
        return [assets, snapshot].compactMap { $0 }
    }
}
