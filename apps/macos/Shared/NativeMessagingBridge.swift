//
//  NativeMessagingBridge.swift
//  Eurora
//
//  Bridge between Safari extension and the container app.
//  Connects to the local bridge server running in the container app
//  over TCP on localhost:14311, using length-prefixed JSON messages.
//
//  This replaces Chrome's stdin/stdout native messaging channel.
//

import Foundation
import Network
import os.log

/// Port for connecting to the local bridge server in the container app
private let kBridgeConnectionPort: UInt16 = 14311

/// Singleton bridge that manages communication with the container app
@available(macOS 15.0, *)
class NativeMessagingBridge {

    static let shared = NativeMessagingBridge()

    private let logger = Logger(subsystem: "com.eurora.macos", category: "NativeMessagingBridge")
    private let queue = DispatchQueue(label: "com.eurora.native-messaging-bridge", qos: .userInitiated)

    private var connection: NWConnection?
    private var isConnected = false
    private var isConnecting = false

    /// Pending response callbacks, keyed by request ID
    private let responseLock = NSLock()
    private var pendingCallbacks: [String: (Result<Data, Error>) -> Void] = [:]

    private init() {}

    /// Ensure the bridge is connected. Call this before sending messages.
    func ensureConnected() {
        queue.async { [weak self] in
            self?.connectIfNeeded()
        }
    }

    /// Send a message to the container app and wait for a response.
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

            // Ensure we're connected
            if !self.isConnected {
                self.connectIfNeeded()

                // Wait briefly for connection to establish, then retry once
                self.queue.asyncAfter(deadline: .now() + 0.5) { [weak self] in
                    guard let self else {
                        completion(.failure(BridgeError.bridgeDeallocated))
                        return
                    }
                    if self.isConnected {
                        self.doSendMessage(message, timeout: timeout, completion: completion)
                    } else {
                        completion(.failure(BridgeError.processNotRunning))
                    }
                }
                return
            }

            self.doSendMessage(message, timeout: timeout, completion: completion)
        }
    }

    /// Forward a response from the extension back to the container app.
    /// Returns true if the response was handled.
    func handleResponseFromExtension(_ response: [String: Any]) -> Bool {
        guard let kind = response["kind"] as? [String: Any],
              kind["Response"] != nil else {
            return false
        }

        queue.async { [weak self] in
            guard let self, let connection = self.connection, self.isConnected else {
                self?.logger.error("Cannot forward response — not connected")
                return
            }

            do {
                let jsonData = try JSONSerialization.data(withJSONObject: response, options: [])
                let framedData = Self.frameMessage(jsonData)

                connection.send(content: framedData, completion: .contentProcessed { error in
                    if let error {
                        self.logger.error("Failed to forward response: \(error.localizedDescription)")
                    }
                })
            } catch {
                self.logger.error("Failed to serialize response: \(error.localizedDescription)")
            }
        }

        return true
    }

    /// Stop the bridge connection.
    func stop() {
        queue.async { [weak self] in
            self?.disconnectInternal()
        }
    }

    // MARK: - Private: Connection

    private func connectIfNeeded() {
        // Already connected or connecting — nothing to do
        guard !isConnected && !isConnecting else { return }

        isConnecting = true

        // Clean up any stale connection
        connection?.cancel()
        connection = nil

        logger.info("Connecting to local bridge server on port \(kBridgeConnectionPort)")

        guard let port = NWEndpoint.Port(rawValue: kBridgeConnectionPort) else { return }
        let endpoint = NWEndpoint.hostPort(host: .ipv4(.loopback), port: port)
        let conn = NWConnection(to: endpoint, using: .tcp)

        conn.stateUpdateHandler = { [weak self] state in
            self?.handleConnectionState(state)
        }

        self.connection = conn
        conn.start(queue: queue)
    }

    private func disconnectInternal() {
        isConnected = false
        isConnecting = false
        connection?.cancel()
        connection = nil

        // Cancel all pending callbacks
        responseLock.lock()
        let callbacks = pendingCallbacks
        pendingCallbacks.removeAll()
        responseLock.unlock()

        for (_, callback) in callbacks {
            callback(.failure(BridgeError.processStopped))
        }
    }

    private func handleConnectionState(_ state: NWConnection.State) {
        switch state {
        case .ready:
            logger.info("Connected to local bridge server")
            isConnected = true
            isConnecting = false
            startReceiving()

        case .failed(let error):
            logger.error("Connection failed: \(error.localizedDescription)")
            isConnected = false
            isConnecting = false
            // Don't auto-reconnect — let the next sendMessage call trigger reconnection

        case .cancelled:
            logger.debug("Connection cancelled")
            isConnected = false
            isConnecting = false

        case .waiting(let error):
            logger.warning("Connection waiting: \(error.localizedDescription)")

        default:
            break
        }
    }

    // MARK: - Private: Sending

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
            completion(.failure(BridgeError.processNotRunning))
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
            sendFramedMessage(jsonData, via: connection)
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

    private func sendFramedMessage(_ jsonData: Data, via connection: NWConnection) {
        let framedData = Self.frameMessage(jsonData)
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

// MARK: - Receiving (extension to reduce type body length)
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
        if let error { logger.error("Receive error: \(error.localizedDescription)"); isConnected = false; return }
        if isComplete { logger.debug("Connection closed by server"); isConnected = false; return }
        guard let lengthData = data, lengthData.count == 4 else { receiveNextMessage(); return }
        let length = lengthData.withUnsafeBytes { $0.load(as: UInt32.self).littleEndian }
        guard length > 0 && length < 8 * 1024 * 1024 else {
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
            logger.error("Receive body error: \(error.localizedDescription)")
            isConnected = false
            return
        }
        if isComplete && data == nil { isConnected = false; return }
        guard let messageData = data, messageData.count == length else { receiveNextMessage(); return }
        handleReceivedMessage(messageData)
        receiveNextMessage()
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
