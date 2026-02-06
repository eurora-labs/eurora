//
//  LocalBridgeServer.swift
//  Eurora
//
//  Local TCP server for IPC between the Safari extension and the container app.
//  The Safari extension (via NativeMessagingBridge) connects here over localhost.
//  Messages are length-prefixed JSON (4 bytes LE length + JSON body).
//
//  This replaces stdin/stdout from the Chrome native messaging model.
//

import Foundation
import Network
import os.log

/// Port for the local bridge server (extension connects here)
private let kLocalBridgeServerPort: UInt16 = 14311

/// Protocol for handling messages received from the Safari extension
@available(macOS 15.0, *)
protocol LocalBridgeServerDelegate: AnyObject {
    /// Called when a JSON message is received from an extension connection.
    /// Call `completion` with a response dictionary to send back, or nil for no response.
    func localBridgeServer(
        _ server: LocalBridgeServer,
        didReceiveMessage message: [String: Any],
        completion: @escaping ([String: Any]?) -> Void
    )
}

/// Local TCP server for communication with the Safari extension.
///
/// The extension process connects to this server on localhost:14311.
/// Messages use the same length-prefixed JSON format as Chrome native messaging.
@available(macOS 15.0, *)
class LocalBridgeServer {

    // MARK: - Properties

    weak var delegate: LocalBridgeServerDelegate?

    private let logger = Logger(subsystem: "com.eurora.macos", category: "LocalBridgeServer")
    private let queue = DispatchQueue(label: "com.eurora.local-bridge-server", qos: .userInitiated)

    private var listener: NWListener?
    private var connections: [ObjectIdentifier: NWConnection] = [:]
    private var isRunning = false

    // MARK: - Initialization

    init() {}

    deinit {
        stopSync()
    }

    // MARK: - Server Control

    /// Start the TCP listener on localhost.
    func start() {
        queue.async { [weak self] in
            self?.startInternal()
        }
    }

    /// Stop the server and close all connections.
    func stop() {
        queue.async { [weak self] in
            self?.stopInternal()
        }
    }

    /// Synchronous stop (for deinit).
    private func stopSync() {
        listener?.cancel()
        listener = nil
        for (_, conn) in connections {
            conn.cancel()
        }
        connections.removeAll()
        isRunning = false
    }

    /// Broadcast a message to all connected extension clients.
    /// Used for server-initiated push messages (events, requests from gRPC server).
    func broadcast(message: [String: Any]) {
        guard let data = try? JSONSerialization.data(withJSONObject: message, options: []) else {
            logger.error("Failed to serialize broadcast message")
            return
        }

        let framedData = Self.frameMessage(data)

        queue.async { [weak self] in
            guard let self else { return }
            for (_, connection) in self.connections {
                connection.send(content: framedData, completion: .contentProcessed { error in
                    if let error {
                        self.logger.error("Broadcast send error: \(error.localizedDescription)")
                    }
                })
            }
        }
    }

    // MARK: - Private: Lifecycle

    private func startInternal() {
        guard !isRunning else {
            logger.debug("Server already running")
            return
        }

        do {
            let parameters = NWParameters.tcp
            parameters.allowLocalEndpointReuse = true
            // Restrict to loopback interface for security
            parameters.requiredInterfaceType = .loopback

            let listener = try NWListener(
                using: parameters,
                on: NWEndpoint.Port(rawValue: kLocalBridgeServerPort)!
            )

            listener.stateUpdateHandler = { [weak self] state in
                self?.handleListenerState(state)
            }

            listener.newConnectionHandler = { [weak self] connection in
                self?.handleNewConnection(connection)
            }

            listener.start(queue: queue)
            self.listener = listener
            isRunning = true

            logger.info("Local bridge server starting on port \(kLocalBridgeServerPort)")
        } catch {
            logger.error("Failed to create listener: \(error.localizedDescription)")
        }
    }

    private func stopInternal() {
        guard isRunning else { return }

        isRunning = false

        listener?.cancel()
        listener = nil

        for (_, connection) in connections {
            connection.cancel()
        }
        connections.removeAll()

        logger.info("Local bridge server stopped")
    }

    private func handleListenerState(_ state: NWListener.State) {
        switch state {
        case .ready:
            logger.info("Local bridge server ready on port \(kLocalBridgeServerPort)")

        case .failed(let error):
            logger.error("Local bridge server failed: \(error.localizedDescription)")
            // IMPORTANT: Do NOT auto-restart here.
            // The old code restarted on failure, which caused an infinite loop:
            // port still in use → restart → "Address already in use" → restart → ...
            // The AppDelegate should handle restart if needed, with proper delay.
            stopInternal()

        case .cancelled:
            logger.info("Local bridge server cancelled")
            isRunning = false

        default:
            break
        }
    }

    // MARK: - Private: Connection Handling

    private func handleNewConnection(_ connection: NWConnection) {
        let connId = ObjectIdentifier(connection)
        logger.info("New extension connection")

        connection.stateUpdateHandler = { [weak self] state in
            self?.handleConnectionState(connId, state: state)
        }

        connections[connId] = connection
        connection.start(queue: queue)

        // Begin receiving messages
        receiveMessage(from: connection, connId: connId)
    }

    private func handleConnectionState(_ connId: ObjectIdentifier, state: NWConnection.State) {
        switch state {
        case .ready:
            logger.debug("Extension connection ready")
        case .failed(let error):
            logger.error("Extension connection failed: \(error.localizedDescription)")
            connections.removeValue(forKey: connId)
        case .cancelled:
            logger.debug("Extension connection cancelled")
            connections.removeValue(forKey: connId)
        default:
            break
        }
    }

    // MARK: - Private: Message Framing (length-prefixed JSON)

    private func receiveMessage(from connection: NWConnection, connId: ObjectIdentifier) {
        // Read the 4-byte little-endian length prefix
        connection.receive(minimumIncompleteLength: 4, maximumLength: 4) { [weak self] data, _, isComplete, error in
            guard let self else { return }

            if let error {
                self.logger.error("Receive error: \(error.localizedDescription)")
                self.connections.removeValue(forKey: connId)
                return
            }

            if isComplete {
                self.logger.debug("Extension connection closed")
                self.connections.removeValue(forKey: connId)
                return
            }

            guard let lengthData = data, lengthData.count == 4 else {
                self.logger.error("Invalid length prefix")
                self.receiveMessage(from: connection, connId: connId)
                return
            }

            let length = lengthData.withUnsafeBytes { $0.load(as: UInt32.self).littleEndian }

            guard length > 0 && length < 8 * 1024 * 1024 else {
                self.logger.error("Invalid message length: \(length)")
                self.receiveMessage(from: connection, connId: connId)
                return
            }

            self.receiveMessageBody(from: connection, connId: connId, length: Int(length))
        }
    }

    private func receiveMessageBody(from connection: NWConnection, connId: ObjectIdentifier, length: Int) {
        let len = length
        connection.receive(minimumIncompleteLength: len, maximumLength: len) { [weak self] data, _, isComplete, error in
            guard let self else { return }

            if let error {
                self.logger.error("Receive body error: \(error.localizedDescription)")
                self.connections.removeValue(forKey: connId)
                return
            }

            if isComplete && data == nil {
                self.logger.debug("Connection closed during body read")
                self.connections.removeValue(forKey: connId)
                return
            }

            guard let messageData = data, messageData.count == length else {
                self.logger.error("Incomplete message body")
                self.receiveMessage(from: connection, connId: connId)
                return
            }

            // Parse JSON
            do {
                if let message = try JSONSerialization.jsonObject(with: messageData, options: []) as? [String: Any] {
                    self.handleReceivedMessage(message, from: connection)
                } else {
                    self.logger.error("Invalid JSON message format")
                }
            } catch {
                self.logger.error("JSON parse error: \(error.localizedDescription)")
            }

            // Continue receiving next message
            self.receiveMessage(from: connection, connId: connId)
        }
    }

    private func handleReceivedMessage(_ message: [String: Any], from connection: NWConnection) {
        logger.debug("Received message from extension")

        delegate?.localBridgeServer(self, didReceiveMessage: message) { [weak self] response in
            guard let self, let response else { return }
            self.sendResponse(response, to: connection)
        }
    }

    private func sendResponse(_ response: [String: Any], to connection: NWConnection) {
        queue.async { [weak self] in
            guard let self else { return }

            do {
                let data = try JSONSerialization.data(withJSONObject: response, options: [])
                let framedData = Self.frameMessage(data)

                connection.send(content: framedData, completion: .contentProcessed { error in
                    if let error {
                        self.logger.error("Failed to send response: \(error.localizedDescription)")
                    }
                })
            } catch {
                self.logger.error("Failed to serialize response: \(error.localizedDescription)")
            }
        }
    }

    // MARK: - Static Helpers

    /// Frame a message with a 4-byte little-endian length prefix
    static func frameMessage(_ data: Data) -> Data {
        var length = UInt32(data.count).littleEndian
        var framedData = Data(bytes: &length, count: 4)
        framedData.append(data)
        return framedData
    }
}
