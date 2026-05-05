import Foundation
import Network
import os.log

private let kBridgeConnectionPort: UInt16 = 14311

/// Errors raised by the local extension <-> launcher bridge.
public enum BridgeError: Error, LocalizedError {
    case bridgeDeallocated
    case notConnected
    case disconnected
    case timeout
    case malformedFrame
    case duplicateRequestId(UInt32)

    public var errorDescription: String? {
        switch self {
        case .bridgeDeallocated:
            return "Native messaging bridge was deallocated"
        case .notConnected:
            return "Launcher app is not running"
        case .disconnected:
            return "Launcher connection was closed"
        case .timeout:
            return "Request timed out"
        case .malformedFrame:
            return "Malformed bridge frame"
        case .duplicateRequestId(let id):
            return "Two in-flight requests share id \(id)"
        }
    }
}

/// Connection between the Safari extension and the launcher app over a
/// length-prefixed JSON socket on `127.0.0.1:14311`. Outbound messages are
/// `Frame` values; replies are correlated by the request `id` of the
/// originating `RequestFrame`.
@available(macOS 13.0, *)
public final class NativeMessagingBridge: @unchecked Sendable {

    public static let shared = NativeMessagingBridge()

    private let logger = Logger(subsystem: "com.eurora.macos", category: "NativeMessagingBridge")
    private let queue = DispatchQueue(label: "com.eurora.native-messaging-bridge", qos: .userInitiated)

    private var connection: NWConnection?
    private var isConnected = false
    private var isConnecting = false

    private let pendingLock = NSLock()
    private var pending: [UInt32: CheckedContinuation<Frame, Error>] = [:]

    private init() {}

    // MARK: - Public API

    /// Open a connection to the launcher if one isn't already in flight.
    public func ensureConnected() {
        queue.async { [weak self] in
            self?.connectIfNeeded()
        }
    }

    /// Send a request frame and await the correlated reply (a `Response` or
    /// `Error` frame whose `id` matches the request). Throws on timeout,
    /// disconnect, or transport failure.
    public func send(
        request: RequestFrame,
        timeout: TimeInterval = 10.0
    ) async throws -> Frame {
        let frame = Frame(request)
        let id = request.id

        if !isConnected {
            ensureConnected()
            try? await Task.sleep(nanoseconds: 500_000_000)
        }

        return try await withTimeout(timeout) {
            try await withCheckedThrowingContinuation { (continuation: CheckedContinuation<Frame, Error>) in
                self.queue.async {
                    guard let connection = self.connection, self.isConnected else {
                        continuation.resume(throwing: BridgeError.notConnected)
                        return
                    }

                    self.pendingLock.lock()
                    if self.pending[id] != nil {
                        self.pendingLock.unlock()
                        continuation.resume(throwing: BridgeError.duplicateRequestId(id))
                        return
                    }
                    self.pending[id] = continuation
                    self.pendingLock.unlock()

                    self.write(frame, on: connection) { error in
                        if let error {
                            self.removePending(id: id)?.resume(throwing: error)
                        }
                    }
                }
            }
        } onTimeout: {
            self.removePending(id: id)?.resume(throwing: BridgeError.timeout)
        }
    }

    /// Forward a server-initiated response (or any unsolicited frame) back
    /// to the launcher, e.g. when the Safari extension answers a request
    /// that was originally pushed from the desktop. No reply is awaited.
    public func forward(_ frame: Frame) {
        queue.async { [weak self] in
            guard let self, let connection = self.connection, self.isConnected else {
                self?.logger.error("Cannot forward — not connected: \(frame.summary, privacy: .public)")
                return
            }
            self.write(frame, on: connection) { error in
                if let error {
                    self.logger.error("Forward error: \(error.localizedDescription, privacy: .public)")
                }
            }
        }
    }

    public func stop() {
        queue.async { [weak self] in
            self?.tearDown()
        }
    }

    // MARK: - Connection

    private func connectIfNeeded() {
        guard !isConnected, !isConnecting else { return }
        isConnecting = true

        connection?.cancel()
        connection = nil

        logger.info("Connecting to local bridge server on port \(kBridgeConnectionPort, privacy: .public)")

        guard let port = NWEndpoint.Port(rawValue: kBridgeConnectionPort) else {
            isConnecting = false
            return
        }
        let endpoint = NWEndpoint.hostPort(host: .ipv4(.loopback), port: port)
        let conn = NWConnection(to: endpoint, using: .tcp)

        conn.stateUpdateHandler = { [weak self] state in
            self?.handleState(state)
        }
        connection = conn
        conn.start(queue: queue)
    }

    private func handleState(_ state: NWConnection.State) {
        switch state {
        case .ready:
            logger.info("Connected to local bridge server")
            isConnected = true
            isConnecting = false
            receiveLength()
        case .failed(let error):
            logger.error("Connection failed: \(error.localizedDescription, privacy: .public)")
            tearDown()
        case .cancelled:
            tearDown()
        case .waiting(let error):
            logger.warning("Connection waiting: \(error.localizedDescription, privacy: .public)")
        default:
            break
        }
    }

    private func tearDown() {
        isConnected = false
        isConnecting = false
        connection?.cancel()
        connection = nil

        let drained = drainPending()
        for continuation in drained {
            continuation.resume(throwing: BridgeError.disconnected)
        }
    }

    // MARK: - Length-prefixed framing

    private func receiveLength() {
        guard let connection, isConnected else { return }
        connection.receive(minimumIncompleteLength: 4, maximumLength: 4) { [weak self] data, _, isComplete, error in
            guard let self else { return }

            if let error {
                self.logger.error("Receive error: \(error.localizedDescription, privacy: .public)")
                self.tearDown()
                return
            }
            if isComplete {
                self.logger.debug("Server closed connection")
                self.tearDown()
                return
            }
            guard let lengthData = data, lengthData.count == 4 else {
                self.receiveLength()
                return
            }
            let length = lengthData.withUnsafeBytes { $0.load(as: UInt32.self).littleEndian }
            guard length > 0, Int(length) <= BridgeProtocol.maxFrameSize else {
                self.logger.error("Invalid message length: \(length, privacy: .public)")
                self.tearDown()
                return
            }
            self.receiveBody(length: Int(length))
        }
    }

    private func receiveBody(length: Int) {
        guard let connection else { return }
        connection.receive(minimumIncompleteLength: length, maximumLength: length) { [weak self] data, _, isComplete, error in
            guard let self else { return }
            if let error {
                self.logger.error("Receive body error: \(error.localizedDescription, privacy: .public)")
                self.tearDown()
                return
            }
            if isComplete && data == nil {
                self.tearDown()
                return
            }
            guard let body = data, body.count == length else {
                self.receiveLength()
                return
            }
            self.handleBody(body)
            self.receiveLength()
        }
    }

    private func handleBody(_ data: Data) {
        let frame: Frame
        do {
            frame = try Frame.decode(data)
        } catch {
            logger.error("Decode error: \(error.localizedDescription, privacy: .public)")
            return
        }

        let id: UInt32?
        switch frame.kind {
        case .response(let r): id = r.id
        case .error(let e): id = e.id
        default: id = nil
        }

        guard let id, let continuation = removePending(id: id) else {
            logger.debug("No pending request for: \(frame.summary, privacy: .public)")
            return
        }
        continuation.resume(returning: frame)
    }

    private func write(
        _ frame: Frame,
        on connection: NWConnection,
        completion: @escaping (Error?) -> Void
    ) {
        let data: Data
        do {
            data = try frame.encodeJSON()
        } catch {
            completion(error)
            return
        }
        let framed = Self.frame(data)
        connection.send(content: framed, completion: .contentProcessed { error in
            completion(error.map { $0 as Error })
        })
    }

    // MARK: - Pending request bookkeeping

    private func removePending(id: UInt32) -> CheckedContinuation<Frame, Error>? {
        pendingLock.lock()
        defer { pendingLock.unlock() }
        return pending.removeValue(forKey: id)
    }

    private func drainPending() -> [CheckedContinuation<Frame, Error>] {
        pendingLock.lock()
        defer { pendingLock.unlock() }
        let drained = Array(pending.values)
        pending.removeAll()
        return drained
    }

    // MARK: - Static helpers

    static func frame(_ data: Data) -> Data {
        var length = UInt32(data.count).littleEndian
        var framed = Data(bytes: &length, count: 4)
        framed.append(data)
        return framed
    }
}

/// Run `operation` with a timeout. If it doesn't finish first, run
/// `onTimeout` (e.g. to fail any pending continuation) and throw
/// `BridgeError.timeout`.
@available(macOS 13.0, *)
private func withTimeout<T: Sendable>(
    _ seconds: TimeInterval,
    operation: @escaping @Sendable () async throws -> T,
    onTimeout: @escaping @Sendable () -> Void
) async throws -> T {
    try await withThrowingTaskGroup(of: T.self) { group in
        group.addTask {
            try await operation()
        }
        group.addTask {
            try await Task.sleep(nanoseconds: UInt64(seconds * 1_000_000_000))
            onTimeout()
            throw BridgeError.timeout
        }
        guard let first = try await group.next() else {
            throw BridgeError.timeout
        }
        group.cancelAll()
        return first
    }
}
