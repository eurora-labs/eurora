import Foundation
import Network
import os.log

private let kBridgeConnectionPort: UInt16 = 14311
private let kDefaultMessageDeadline: TimeInterval = 10.0
private let kMinReconnectDelay: TimeInterval = 0.25
private let kMaxReconnectDelay: TimeInterval = 2.0
private let kMaxPendingMessages = 256

@available(macOS 15.0, *)
class NativeMessagingBridge {

    static let shared = NativeMessagingBridge()

    private let logger = Logger(subsystem: "com.eurora.macos", category: "NativeMessagingBridge")
    private let queue = DispatchQueue(label: "com.eurora.native-messaging-bridge", qos: .userInitiated)

    private var connection: NWConnection?
    private var isConnected = false
    private var isConnecting = false
    private var stopped = false

    private var reconnectDelay: TimeInterval = kMinReconnectDelay
    private var reconnectScheduled = false

    private struct PendingMessage {
        let body: [String: Any]
        let timeout: TimeInterval
        let deadline: Date
        let completion: (Result<[String: Any], Error>) -> Void
    }
    private var pendingMessages: [PendingMessage] = []

    private let responseLock = NSLock()
    private var pendingCallbacks: [String: (Result<Data, Error>) -> Void] = [:]

    private init() {}

    // MARK: - Public API

    func ensureConnected() {
        queue.async { [weak self] in
            guard let self else { return }
            self.stopped = false
            self.connectIfNeeded()
        }
    }

    func sendMessage(
        _ message: [String: Any],
        timeout: TimeInterval = 10.0,
        completion: @escaping (Result<[String: Any], Error>) -> Void
    ) {
        queue.async { [weak self] in
            guard let self else {
                completion(.failure(BridgeError.bridgeDeallocated))
                return
            }

            self.stopped = false

            if self.isConnected {
                self.doSendMessage(message, timeout: timeout, completion: completion)
                return
            }

            // Buffer until the connection becomes ready, then drain.
            // Deadline bounds how long the caller is willing to wait for the
            // launcher to come up — typically ~10s, generous enough to absorb
            // a launcher restart but tight enough that Safari's own extension
            // timeouts don't fire underneath us.
            let deadline = Date().addingTimeInterval(kDefaultMessageDeadline)
            self.enqueue(
                PendingMessage(
                    body: message, timeout: timeout, deadline: deadline, completion: completion
                ))
            self.connectIfNeeded()
            self.scheduleNextDeadlineCheck()
        }
    }

    func handleResponseFromExtension(_ response: [String: Any]) -> Bool {
        guard let kind = response["kind"] as? [String: Any], kind["Response"] != nil else {
            return false
        }

        queue.async { [weak self] in
            guard let self else { return }

            do {
                let jsonData = try JSONSerialization.data(withJSONObject: response, options: [])
                let framedData = Self.frameMessage(jsonData)

                if self.isConnected, let connection = self.connection {
                    self.send(framedData, via: connection)
                } else {
                    self.logger.debug("Cannot forward response — not connected (dropped)")
                }
            } catch {
                self.logger.error("Failed to serialize response: \(error.localizedDescription)")
            }
        }

        return true
    }

    func stop() {
        queue.async { [weak self] in
            guard let self else { return }
            self.stopped = true
            self.failAllPending(BridgeError.processStopped)
            self.tearDownConnection()
        }
    }

    // MARK: - Pending Queue

    private func enqueue(_ message: PendingMessage) {
        if pendingMessages.count >= kMaxPendingMessages {
            // Bound the queue so a permanently-down launcher can't grow it
            // without limit. Drop the oldest entry — it has been waiting
            // longest, and freeing it lets the caller retry on its own
            // schedule.
            let dropped = pendingMessages.removeFirst()
            logger.warning("Pending message queue full; dropping oldest entry")
            dropped.completion(.failure(BridgeError.processNotRunning))
        }
        pendingMessages.append(message)
    }

    private func drainReadyQueue() {
        guard isConnected else { return }
        let messages = pendingMessages
        pendingMessages.removeAll()
        for message in messages {
            doSendMessage(message.body, timeout: message.timeout, completion: message.completion)
        }
    }

    private func failExpiredMessages() {
        let now = Date()
        var remaining: [PendingMessage] = []
        var expired: [PendingMessage] = []
        for message in pendingMessages {
            if message.deadline <= now {
                expired.append(message)
            } else {
                remaining.append(message)
            }
        }
        pendingMessages = remaining
        for message in expired {
            message.completion(.failure(BridgeError.processNotRunning))
        }
    }

    private func scheduleNextDeadlineCheck() {
        guard let nextDeadline = pendingMessages.map({ $0.deadline }).min() else { return }
        let delay = max(0, nextDeadline.timeIntervalSinceNow)
        queue.asyncAfter(deadline: .now() + delay) { [weak self] in
            self?.failExpiredMessages()
        }
    }

    private func failAllPending(_ error: Error) {
        let messages = pendingMessages
        pendingMessages.removeAll()
        for message in messages {
            message.completion(.failure(error))
        }

        responseLock.lock()
        let callbacks = pendingCallbacks
        pendingCallbacks.removeAll()
        responseLock.unlock()
        for (_, callback) in callbacks {
            callback(.failure(error))
        }
    }

    // MARK: - Connection Lifecycle

    private func connectIfNeeded() {
        guard !stopped, !isConnected, !isConnecting else { return }

        isConnecting = true

        connection?.cancel()
        connection = nil

        guard let port = NWEndpoint.Port(rawValue: kBridgeConnectionPort) else {
            isConnecting = false
            return
        }
        let endpoint = NWEndpoint.hostPort(host: .ipv4(.loopback), port: port)
        let conn = NWConnection(to: endpoint, using: .tcp)

        conn.stateUpdateHandler = { [weak self] state in
            self?.handleConnectionState(state)
        }

        connection = conn
        logger.debug("Connecting to local bridge server on port \(kBridgeConnectionPort)")
        conn.start(queue: queue)
    }

    private func tearDownConnection() {
        isConnected = false
        isConnecting = false
        connection?.cancel()
        connection = nil
    }

    private func handleConnectionState(_ state: NWConnection.State) {
        switch state {
        case .ready:
            logger.info("Connected to local bridge server")
            isConnected = true
            isConnecting = false
            reconnectDelay = kMinReconnectDelay
            startReceiving()
            drainReadyQueue()

        case .failed(let error):
            logger.warning("Connection failed: \(error.localizedDescription)")
            tearDownConnection()
            scheduleReconnect()

        case .cancelled:
            logger.debug("Connection cancelled")
            isConnected = false
            isConnecting = false
            // If `stop()` cancelled us we don't reconnect; otherwise treat
            // this like any other disconnect and back off.
            if !stopped {
                scheduleReconnect()
            }

        case .waiting(let error):
            logger.debug("Connection waiting: \(error.localizedDescription)")

        default:
            break
        }
    }

    private func scheduleReconnect() {
        guard !stopped, !reconnectScheduled else { return }
        reconnectScheduled = true

        let delay = reconnectDelay
        // Exponential backoff capped at kMaxReconnectDelay, mirroring the
        // 2s ceiling used by the launcher's own gRPC client.
        reconnectDelay = min(reconnectDelay * 2, kMaxReconnectDelay)

        queue.asyncAfter(deadline: .now() + delay) { [weak self] in
            guard let self else { return }
            self.reconnectScheduled = false
            self.connectIfNeeded()
        }
    }

    // MARK: - Sending

    private func extractRequestId(from message: [String: Any]) -> String? {
        guard let kind = message["kind"] as? [String: Any],
              let request = kind["Request"] as? [String: Any],
              let id = request["id"] else { return nil }
        return "\(id)"
    }

    private func makeCallback(
        completion: @escaping (Result<[String: Any], Error>) -> Void,
        callbackLock: NSLock,
        callbackFired: UnsafeMutablePointer<Bool>
    ) -> (Result<Data, Error>) -> Void {
        return { result in
            callbackLock.lock()
            guard !callbackFired.pointee else { callbackLock.unlock(); return }
            callbackFired.pointee = true
            callbackLock.unlock()

            switch result {
            case .success(let data):
                do {
                    if let dict = try JSONSerialization.jsonObject(with: data) as? [String: Any] {
                        completion(.success(dict))
                    } else { completion(.failure(BridgeError.invalidResponse)) }
                } catch { completion(.failure(error)) }
            case .failure(let error):
                completion(.failure(error))
            }
        }
    }

    private func doSendMessage(
        _ message: [String: Any],
        timeout: TimeInterval,
        completion: @escaping (Result<[String: Any], Error>) -> Void
    ) {
        guard let connection, isConnected else {
            // Lost connection between enqueue and drain; re-buffer.
            let deadline = Date().addingTimeInterval(kDefaultMessageDeadline)
            enqueue(
                PendingMessage(
                    body: message, timeout: timeout, deadline: deadline, completion: completion
                ))
            connectIfNeeded()
            scheduleNextDeadlineCheck()
            return
        }

        do {
            let jsonData = try JSONSerialization.data(withJSONObject: message, options: [])
            let callbackId = extractRequestId(from: message) ?? UUID().uuidString
            let callbackLock = NSLock()
            let callbackFired = UnsafeMutablePointer<Bool>.allocate(capacity: 1)
            callbackFired.initialize(to: false)

            responseLock.lock()
            pendingCallbacks[callbackId] = makeCallback(
                completion: completion, callbackLock: callbackLock, callbackFired: callbackFired
            )
            responseLock.unlock()

            setupTimeout(callbackId: callbackId, timeout: timeout, callbackLock: callbackLock,
                         callbackFired: callbackFired, completion: completion)

            let framedData = Self.frameMessage(jsonData)
            send(framedData, via: connection)
        } catch { completion(.failure(error)) }
    }

    private func setupTimeout(
        callbackId: String, timeout: TimeInterval, callbackLock: NSLock,
        callbackFired: UnsafeMutablePointer<Bool>,
        completion: @escaping (Result<[String: Any], Error>) -> Void
    ) {
        DispatchQueue.main.asyncAfter(deadline: .now() + timeout) { [weak self] in
            callbackLock.lock()
            guard !callbackFired.pointee else { callbackLock.unlock(); return }
            callbackFired.pointee = true
            callbackLock.unlock()

            self?.responseLock.lock()
            self?.pendingCallbacks.removeValue(forKey: callbackId)
            self?.responseLock.unlock()
            completion(.failure(BridgeError.timeout))
        }
    }

    private func send(_ framedData: Data, via connection: NWConnection) {
        connection.send(content: framedData, completion: .contentProcessed { [weak self] error in
            if let error { self?.logger.error("Send error: \(error.localizedDescription)") }
        })
    }

    // MARK: - Static Helpers

    static func frameMessage(_ data: Data) -> Data {
        var length = UInt32(data.count).littleEndian
        var framedData = Data(bytes: &length, count: 4)
        framedData.append(data)
        return framedData
    }
}

// MARK: - Receiving

@available(macOS 15.0, *)
extension NativeMessagingBridge {
    func startReceiving() { receiveNextMessage() }

    func receiveNextMessage() {
        guard let connection, isConnected else { return }
        connection.receive(minimumIncompleteLength: 4, maximumLength: 4) { [weak self] data, _, isComplete, error in
            self?.handleLengthReceive(data: data, isComplete: isComplete, error: error)
        }
    }

    private func handleLengthReceive(data: Data?, isComplete: Bool, error: NWError?) {
        if let error {
            logger.warning("Receive error: \(error.localizedDescription)")
            handleReadFailure()
            return
        }
        if isComplete {
            logger.debug("Connection closed by server")
            handleReadFailure()
            return
        }
        guard let lengthData = data, lengthData.count == 4 else { receiveNextMessage(); return }
        let length = lengthData.withUnsafeBytes { $0.load(as: UInt32.self).littleEndian }
        guard length > 0 && length < 1024 * 1024 * 1024 else {
            logger.error("Invalid message length: \(length)"); receiveNextMessage(); return
        }
        receiveMessageBody(length: Int(length))
    }

    private func receiveMessageBody(length: Int) {
        guard let connection else { return }
        let len = length
        connection.receive(minimumIncompleteLength: len, maximumLength: len) { [weak self] data, _, isComplete, error in
            self?.handleBodyReceive(data: data, length: length, isComplete: isComplete, error: error)
        }
    }

    private func handleBodyReceive(data: Data?, length: Int, isComplete: Bool, error: NWError?) {
        if let error {
            logger.warning("Receive body error: \(error.localizedDescription)")
            handleReadFailure()
            return
        }
        if isComplete && data == nil {
            handleReadFailure()
            return
        }
        guard let messageData = data, messageData.count == length else { receiveNextMessage(); return }
        handleReceivedMessage(messageData)
        receiveNextMessage()
    }

    /// Triggered when the receive side detects the peer has gone away.
    /// We tear the connection down and let the reconnect loop pick it up;
    /// in-flight per-request callbacks are failed so the extension hears
    /// back rather than hanging until their own timeout fires.
    private func handleReadFailure() {
        tearDownConnection()

        responseLock.lock()
        let callbacks = pendingCallbacks
        pendingCallbacks.removeAll()
        responseLock.unlock()
        for (_, callback) in callbacks {
            callback(.failure(BridgeError.processStopped))
        }

        if !stopped {
            scheduleReconnect()
        }
    }

    private func handleReceivedMessage(_ data: Data) {
        do {
            guard let json = try JSONSerialization.jsonObject(with: data) as? [String: Any],
                  let kind = json["kind"] as? [String: Any] else { return }
            let responseId = extractResponseId(from: kind)
            if let responseId {
                responseLock.lock()
                let callback = pendingCallbacks.removeValue(forKey: responseId)
                responseLock.unlock()
                if let callback { callback(.success(data)); return }
            }
        } catch {
            logger.error("Failed to parse received message: \(error.localizedDescription)")
        }
        logger.debug("Received message with no pending callback")
    }

    private func extractResponseId(from kind: [String: Any]) -> String? {
        if let resp = kind["Response"] as? [String: Any], let id = resp["id"] { return "\(id)" }
        if let err = kind["Error"] as? [String: Any], let id = err["id"] { return "\(id)" }
        return nil
    }
}

// MARK: - Error Types

enum BridgeError: Error, LocalizedError {
    case bridgeDeallocated
    case processNotRunning
    case processStopped
    case timeout
    case invalidResponse

    var errorDescription: String? {
        switch self {
        case .bridgeDeallocated:
            return "Native messaging bridge was deallocated"
        case .processNotRunning:
            return "Container app is not running"
        case .processStopped:
            return "Container app connection was stopped"
        case .timeout:
            return "Request timed out"
        case .invalidResponse:
            return "Invalid response from container app"
        }
    }
}
