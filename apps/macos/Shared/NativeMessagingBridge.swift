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

        let endpoint = NWEndpoint.hostPort(host: .ipv4(.loopback), port: NWEndpoint.Port(rawValue: kBridgeConnectionPort)!)
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

            // Extract request ID for matching response
            var requestId: String?
            if let kind = message["kind"] as? [String: Any],
               let request = kind["Request"] as? [String: Any],
               let id = request["id"] {
                requestId = "\(id)"
            }

            let callbackId = requestId ?? UUID().uuidString
            var callbackFired = false
            let callbackLock = NSLock()

            // Register callback
            responseLock.lock()
            pendingCallbacks[callbackId] = { result in
                callbackLock.lock()
                guard !callbackFired else {
                    callbackLock.unlock()
                    return
                }
                callbackFired = true
                callbackLock.unlock()

                switch result {
                case .success(let data):
                    do {
                        if let dict = try JSONSerialization.jsonObject(with: data, options: []) as? [String: Any] {
                            completion(.success(dict))
                        } else {
                            completion(.failure(BridgeError.invalidResponse))
                        }
                    } catch {
                        completion(.failure(error))
                    }
                case .failure(let error):
                    completion(.failure(error))
                }
            }
            responseLock.unlock()

            // Set up timeout
            DispatchQueue.main.asyncAfter(deadline: .now() + timeout) { [weak self] in
                callbackLock.lock()
                guard !callbackFired else {
                    callbackLock.unlock()
                    return
                }
                callbackFired = true
                callbackLock.unlock()

                self?.responseLock.lock()
                self?.pendingCallbacks.removeValue(forKey: callbackId)
                self?.responseLock.unlock()

                completion(.failure(BridgeError.timeout))
            }

            // Send framed message
            let framedData = Self.frameMessage(jsonData)

            connection.send(content: framedData, completion: .contentProcessed { [weak self] error in
                if let error {
                    self?.logger.error("Send error: \(error.localizedDescription)")
                }
            })
        } catch {
            completion(.failure(error))
        }
    }

    // MARK: - Private: Receiving

    private func startReceiving() {
        receiveNextMessage()
    }

    private func receiveNextMessage() {
        guard let connection, isConnected else { return }

        // Read 4-byte length prefix
        connection.receive(minimumIncompleteLength: 4, maximumLength: 4) { [weak self] data, _, isComplete, error in
            guard let self else { return }

            if let error {
                self.logger.error("Receive error: \(error.localizedDescription)")
                self.isConnected = false
                return
            }

            if isComplete {
                self.logger.debug("Connection closed by server")
                self.isConnected = false
                return
            }

            guard let lengthData = data, lengthData.count == 4 else {
                self.receiveNextMessage()
                return
            }

            let length = lengthData.withUnsafeBytes { $0.load(as: UInt32.self).littleEndian }

            guard length > 0 && length < 8 * 1024 * 1024 else {
                self.logger.error("Invalid message length: \(length)")
                self.receiveNextMessage()
                return
            }

            self.receiveMessageBody(length: Int(length))
        }
    }

    private func receiveMessageBody(length: Int) {
        guard let connection else { return }

        connection.receive(minimumIncompleteLength: length, maximumLength: length) { [weak self] data, _, isComplete, error in
            guard let self else { return }

            if let error {
                self.logger.error("Receive body error: \(error.localizedDescription)")
                self.isConnected = false
                return
            }

            if isComplete && data == nil {
                self.isConnected = false
                return
            }

            guard let messageData = data, messageData.count == length else {
                self.receiveNextMessage()
                return
            }

            self.handleReceivedMessage(messageData)
            self.receiveNextMessage()
        }
    }

    private func handleReceivedMessage(_ data: Data) {
        // Try to match response to a pending callback
        do {
            if let json = try JSONSerialization.jsonObject(with: data, options: []) as? [String: Any],
               let kind = json["kind"] as? [String: Any] {

                var responseId: String?

                if let response = kind["Response"] as? [String: Any],
                   let id = response["id"] {
                    responseId = "\(id)"
                } else if let error = kind["Error"] as? [String: Any],
                          let id = error["id"] {
                    responseId = "\(id)"
                }

                if let responseId {
                    responseLock.lock()
                    let callback = pendingCallbacks.removeValue(forKey: responseId)
                    responseLock.unlock()

                    if let callback {
                        callback(.success(data))
                        return
                    }
                }
            }
        } catch {
            logger.error("Failed to parse received message: \(error.localizedDescription)")
        }

        logger.debug("Received message with no pending callback")
    }

    // MARK: - Static Helpers

    static func frameMessage(_ data: Data) -> Data {
        var length = UInt32(data.count).littleEndian
        var framedData = Data(bytes: &length, count: 4)
        framedData.append(data)
        return framedData
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
