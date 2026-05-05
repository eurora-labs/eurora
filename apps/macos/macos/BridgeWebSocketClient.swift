import Foundation
import os.log

@available(macOS 13.0, *)
public protocol BridgeWebSocketClientDelegate: AnyObject {
    func bridgeWebSocketClientDidConnect(_ client: BridgeWebSocketClient)
    func bridgeWebSocketClientDidDisconnect(_ client: BridgeWebSocketClient, error: Error?)
    func bridgeWebSocketClient(_ client: BridgeWebSocketClient, didReceive frame: Frame)
}

/// WebSocket client that connects the macOS launcher to the desktop
/// bridge over `ws://localhost:1431/bridge`. Mirrors the role of the
/// Rust `euro-native-messaging` host: send a `Register` frame on
/// connect, replay any cached `ASSETS`/`SNAPSHOT` events, then pump
/// frames in both directions until the connection drops, at which
/// point we back off and reconnect.
///
/// The bridge is plaintext and loopback-only — see `BridgeProtocol`
/// for the transport rationale. The first-run failure mode is "desktop
/// hasn't bound the listener yet" rather than "user hasn't approved
/// the keychain trust prompt"; the reconnect loop handles either by
/// retrying on a fixed interval.
@available(macOS 13.0, *)
public final class BridgeWebSocketClient: @unchecked Sendable {
    public weak var delegate: BridgeWebSocketClientDelegate?

    private let logger = Logger(
        subsystem: "com.eurora.macos", category: "BridgeWebSocketClient"
    )

    private let hostPid: UInt32
    private let appKind: String?
    private let url: URL
    private let urlSession: URLSession

    private let stateLock = NSLock()
    private var appPid: UInt32
    private var currentTask: URLSessionWebSocketTask?
    private var connectionTask: Task<Void, Never>?
    private var shouldRun = false

    private let replayCache = BridgeReplayCache()
    private let heartbeat: BridgeHeartbeat

    public var isConnected: Bool {
        stateLock.lock()
        defer { stateLock.unlock() }
        return currentTask != nil
    }

    public init(
        hostPid: UInt32,
        appPid: UInt32,
        appKind: String? = nil,
        url: URL = BridgeProtocol.url,
        urlSession: URLSession = .shared
    ) {
        self.hostPid = hostPid
        self.appPid = appPid
        self.appKind = appKind
        self.url = url
        self.urlSession = urlSession
        heartbeat = BridgeHeartbeat(
            interval: BridgeProtocol.heartbeatInterval,
            logger: Logger(subsystem: "com.eurora.macos", category: "BridgeHeartbeat")
        )
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
            guard let self else { return }
            await runConnectLoop()
        }
        connectionTask = task
        stateLock.unlock()

        // Hoist to a local — `logger.info`'s interpolation captures via an
        // @escaping @autoclosure, so referencing `url` directly would require
        // `self.url` (which `swiftformat`'s redundantSelf rule would then strip,
        // re-breaking the build). Same pattern is used in the other log sites
        // below.
        let urlString = url.absoluteString
        logger.info("Bridge URL: \(urlString, privacy: .public)")
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
            let register = Frame(RegisterFrame(hostPid: hostPid, appPid: newPid, appKind: appKind))
            send(frame: register, on: task)
        }
    }

    /// Send a frame to the desktop bridge. If we're not currently connected
    /// the frame is dropped; this matches the previous gRPC client behavior.
    public func send(frame: Frame) {
        replayCache.record(frame)

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
        stateLock.unlock()

        let host = hostPid
        logger.info(
            "Connecting to bridge: host=\(host, privacy: .public), app=\(pid, privacy: .public)"
        )

        let register = Frame(RegisterFrame(hostPid: hostPid, appPid: pid, appKind: appKind))
        guard await sendAwait(frame: register, on: task) else {
            await teardown(task: task, error: nil)
            return
        }

        for frame in replayCache.replayables() {
            logger.info("Replaying cached \(frame.summary, privacy: .public)")
            _ = await sendAwait(frame: frame, on: task)
        }

        await notifyConnected()
        heartbeat.start(pinging: task)

        var disconnectError: Error?
        do {
            while !Task.isCancelled {
                let message = try await task.receive()
                handleIncoming(message: message)
            }
        } catch {
            disconnectError = error
            if !Task.isCancelled {
                let description = describeFailure(error)
                logger.info(
                    "Bridge stream ended: \(description, privacy: .public)"
                )
            }
        }

        heartbeat.stop()
        await teardown(task: task, error: disconnectError)
    }

    private func handleIncoming(message: URLSessionWebSocketTask.Message) {
        let text: String
        switch message {
        case let .string(string):
            text = string
        case let .data(data):
            guard let decoded = String(data: data, encoding: .utf8) else {
                logger.warning("Ignoring non-UTF8 binary frame from bridge")
                return
            }
            text = decoded
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
        let delegate = delegate
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
            if let error, let self {
                let description = describeFailure(error)
                logger.warning(
                    "Bridge send error: \(description, privacy: .public)"
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
            let description = describeFailure(error)
            logger.warning(
                "Bridge send error: \(description, privacy: .public)"
            )
            return false
        }
    }

    private func teardown(task: URLSessionWebSocketTask, error: Error?) async {
        task.cancel(with: .goingAway, reason: nil)

        stateLock.lock()
        if currentTask === task {
            currentTask = nil
        }
        stateLock.unlock()

        let delegate = delegate
        await MainActor.run { [weak self] in
            guard let self else { return }
            delegate?.bridgeWebSocketClientDidDisconnect(self, error: error)
        }
    }

    private func notifyConnected() async {
        let delegate = delegate
        await MainActor.run { [weak self] in
            guard let self else { return }
            delegate?.bridgeWebSocketClientDidConnect(self)
        }
    }
}

// MARK: - Diagnostics

@available(macOS 13.0, *)
private extension BridgeWebSocketClient {
    /// Render a transport-layer failure for logging. The dominant
    /// failure mode is "desktop bridge not yet listening" — typically a
    /// startup race where the launcher comes up before the desktop has
    /// bound port 1431. Tagging that case with the dialed URL beats
    /// `URLError`'s generic `localizedDescription` for triage; every
    /// other code falls through.
    func describeFailure(_ error: Error) -> String {
        guard let urlError = error as? URLError else {
            return error.localizedDescription
        }
        switch urlError.code {
        case .cannotConnectToHost, .cannotFindHost, .networkConnectionLost,
             .notConnectedToInternet, .timedOut:
            return "desktop bridge not reachable at \(url.absoluteString) — \(urlError.localizedDescription)"
        default:
            return urlError.localizedDescription
        }
    }
}
