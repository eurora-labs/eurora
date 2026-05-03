import Foundation
import os.log

@available(macOS 13.0, *)
public protocol BridgeWebSocketClientDelegate: AnyObject {
    func bridgeWebSocketClientDidConnect(_ client: BridgeWebSocketClient)
    func bridgeWebSocketClientDidDisconnect(_ client: BridgeWebSocketClient, error: Error?)
    func bridgeWebSocketClient(_ client: BridgeWebSocketClient, didReceive frame: Frame)
}

/// WebSocket client that connects the macOS launcher to the desktop bridge
/// (`ws://127.0.0.1:1431/bridge`). Mirrors the role of the Rust
/// `euro-native-messaging` host: send a `Register` frame on connect, replay
/// any cached `ASSETS`/`SNAPSHOT` events, then pump frames in both directions
/// until the connection drops, at which point we back off and reconnect.
@available(macOS 13.0, *)
public final class BridgeWebSocketClient: @unchecked Sendable {

    public weak var delegate: BridgeWebSocketClientDelegate?

    private let logger = Logger(
        subsystem: "com.eurora.macos", category: "BridgeWebSocketClient"
    )

    private let hostPid: UInt32
    private let url: URL
    private let urlSession: URLSession

    private let stateLock = NSLock()
    private var appPid: UInt32
    private var currentTask: URLSessionWebSocketTask?
    private var connectionTask: Task<Void, Never>?
    private var shouldRun = false

    /// Last-seen `ASSETS` and `SNAPSHOT` event frames. We replay these after
    /// a reconnect so the desktop bridge sees the latest browser state even
    /// if it restarted while Safari was running.
    private var cachedAssets: Frame?
    private var cachedSnapshot: Frame?

    public var isConnected: Bool {
        stateLock.lock()
        defer { stateLock.unlock() }
        return currentTask != nil
    }

    public init(
        hostPid: UInt32,
        appPid: UInt32,
        url: URL = BridgeProtocol.url,
        urlSession: URLSession = .shared
    ) {
        self.hostPid = hostPid
        self.appPid = appPid
        self.url = url
        self.urlSession = urlSession
    }

    deinit {
        // Best-effort: stop any in-flight session. We can't await the task
        // here, but cancelling is safe and idempotent.
        stateLock.lock()
        let task = currentTask
        let connection = connectionTask
        currentTask = nil
        connectionTask = nil
        shouldRun = false
        stateLock.unlock()

        task?.cancel(with: .goingAway, reason: nil)
        connection?.cancel()
    }

    // MARK: - Public API

    public func start() {
        stateLock.lock()
        guard connectionTask == nil else {
            stateLock.unlock()
            return
        }
        shouldRun = true
        let task = Task { [weak self] in
            await self?.runConnectLoop()
        }
        connectionTask = task
        stateLock.unlock()
    }

    public func stop() {
        stateLock.lock()
        shouldRun = false
        let task = currentTask
        let connection = connectionTask
        currentTask = nil
        connectionTask = nil
        stateLock.unlock()

        task?.cancel(with: .goingAway, reason: nil)
        connection?.cancel()
    }

    /// Update the application (browser) PID this client represents. If we're
    /// currently connected, we re-send a `Register` frame so the desktop
    /// bridge replaces the registration; otherwise the new PID is used on
    /// the next connect.
    public func updateAppPid(_ newPid: UInt32) {
        stateLock.lock()
        appPid = newPid
        let task = currentTask
        stateLock.unlock()

        logger.info("App PID updated to \(newPid, privacy: .public)")

        if let task {
            let register = Frame(RegisterFrame(hostPid: hostPid, appPid: newPid))
            send(frame: register, on: task)
        }
    }

    /// Send a frame to the desktop bridge. If we're not currently connected
    /// the frame is dropped; this matches the previous gRPC client behavior.
    public func send(frame: Frame) {
        cacheReplayableEvent(frame)

        stateLock.lock()
        let task = currentTask
        stateLock.unlock()

        guard let task else {
            logger.warning("Dropping frame — not connected: \(frame.summary, privacy: .public)")
            return
        }
        send(frame: frame, on: task)
    }

    // MARK: - Connect loop

    private func runConnectLoop() async {
        while !Task.isCancelled {
            stateLock.lock()
            let keepRunning = shouldRun
            stateLock.unlock()
            guard keepRunning else { break }

            await runOneSession()

            stateLock.lock()
            let stillRunning = shouldRun
            stateLock.unlock()
            guard stillRunning, !Task.isCancelled else { break }

            do {
                try await Task.sleep(nanoseconds: UInt64(BridgeProtocol.reconnectInterval * 1_000_000_000))
            } catch {
                break
            }
        }
        logger.debug("Connect loop ended")
    }

    private func runOneSession() async {
        var request = URLRequest(url: url)
        request.timeoutInterval = 30
        let task = urlSession.webSocketTask(with: request)
        task.maximumMessageSize = BridgeProtocol.maxFrameSize
        task.resume()

        stateLock.lock()
        currentTask = task
        let pid = appPid
        let assets = cachedAssets
        let snapshot = cachedSnapshot
        stateLock.unlock()

        logger.info(
            "Connecting to bridge at \(self.url.absoluteString, privacy: .public): host=\(self.hostPid, privacy: .public), app=\(pid, privacy: .public)"
        )

        let register = Frame(RegisterFrame(hostPid: hostPid, appPid: pid))
        guard await sendAwait(frame: register, on: task) else {
            await teardown(task: task, error: nil)
            return
        }

        if let assets {
            logger.info("Replaying cached ASSETS frame")
            _ = await sendAwait(frame: assets, on: task)
        }
        if let snapshot {
            logger.info("Replaying cached SNAPSHOT frame")
            _ = await sendAwait(frame: snapshot, on: task)
        }

        await notifyConnected()

        let heartbeat = Task { [weak self, weak task] in
            await self?.runHeartbeat(task: task)
        }

        var disconnectError: Error?
        do {
            while !Task.isCancelled {
                let message = try await task.receive()
                handleIncoming(message: message)
            }
        } catch {
            disconnectError = error
            if !Task.isCancelled {
                logger.info(
                    "Bridge stream ended: \(String(describing: error), privacy: .public)"
                )
            }
        }

        heartbeat.cancel()
        await teardown(task: task, error: disconnectError)
    }

    private func runHeartbeat(task: URLSessionWebSocketTask?) async {
        let interval = BridgeProtocol.heartbeatInterval
        while !Task.isCancelled {
            do {
                try await Task.sleep(nanoseconds: UInt64(interval * 1_000_000_000))
            } catch {
                return
            }
            guard let task else { return }
            await withCheckedContinuation { (continuation: CheckedContinuation<Void, Never>) in
                task.sendPing { [weak self] error in
                    if let error {
                        self?.logger.debug(
                            "Heartbeat ping failed: \(error.localizedDescription, privacy: .public)"
                        )
                    }
                    continuation.resume()
                }
            }
        }
    }

    private func handleIncoming(message: URLSessionWebSocketTask.Message) {
        let text: String
        switch message {
        case .string(let s):
            text = s
        case .data(let data):
            guard let s = String(data: data, encoding: .utf8) else {
                logger.warning("Ignoring non-UTF8 binary frame from bridge")
                return
            }
            text = s
        @unknown default:
            logger.warning("Ignoring unknown websocket message variant")
            return
        }

        let frame: Frame
        do {
            frame = try Frame.decode(text)
        } catch {
            logger.warning(
                "Bad JSON frame from bridge: \(error.localizedDescription, privacy: .public)"
            )
            return
        }

        logger.debug("Received from bridge: \(frame.summary, privacy: .public)")
        let delegate = self.delegate
        DispatchQueue.main.async { [weak self] in
            guard let self else { return }
            delegate?.bridgeWebSocketClient(self, didReceive: frame)
        }
    }

    // MARK: - Send helpers

    private func send(frame: Frame, on task: URLSessionWebSocketTask) {
        let json: String
        do {
            json = try frame.encodeJSONString()
        } catch {
            logger.error(
                "Failed to encode outbound frame: \(error.localizedDescription, privacy: .public)"
            )
            return
        }
        task.send(.string(json)) { [weak self] error in
            if let error {
                self?.logger.warning(
                    "Bridge send error: \(error.localizedDescription, privacy: .public)"
                )
            }
        }
    }

    private func sendAwait(frame: Frame, on task: URLSessionWebSocketTask) async -> Bool {
        let json: String
        do {
            json = try frame.encodeJSONString()
        } catch {
            logger.error(
                "Failed to encode outbound frame: \(error.localizedDescription, privacy: .public)"
            )
            return false
        }
        do {
            try await task.send(.string(json))
            return true
        } catch {
            logger.warning(
                "Bridge send error: \(error.localizedDescription, privacy: .public)"
            )
            return false
        }
    }

    private func cacheReplayableEvent(_ frame: Frame) {
        guard case .event(let event) = frame.kind else { return }
        switch event.action {
        case "ASSETS":
            stateLock.lock()
            cachedAssets = frame
            stateLock.unlock()
        case "SNAPSHOT":
            stateLock.lock()
            cachedSnapshot = frame
            stateLock.unlock()
        default:
            break
        }
    }

    private func teardown(task: URLSessionWebSocketTask, error: Error?) async {
        task.cancel(with: .goingAway, reason: nil)

        stateLock.lock()
        if currentTask === task {
            currentTask = nil
        }
        stateLock.unlock()

        let delegate = self.delegate
        await MainActor.run { [weak self] in
            guard let self else { return }
            delegate?.bridgeWebSocketClientDidDisconnect(self, error: error)
        }
    }

    private func notifyConnected() async {
        let delegate = self.delegate
        await MainActor.run { [weak self] in
            guard let self else { return }
            delegate?.bridgeWebSocketClientDidConnect(self)
        }
    }
}
