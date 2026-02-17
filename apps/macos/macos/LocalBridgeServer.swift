import Foundation
import Network
import os.log

private let kLocalBridgeServerPort: UInt16 = 14311

@available(macOS 15.0, *)
protocol LocalBridgeServerDelegate: AnyObject {
    func localBridgeServer(
        _ server: LocalBridgeServer,
        didReceiveMessage message: [String: Any],
        completion: @escaping ([String: Any]?) -> Void
    )
}

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

    func start() {
        queue.async { [weak self] in
            self?.startInternal()
        }
    }

    func stop() {
        queue.async { [weak self] in
            self?.stopInternal()
        }
    }

    private func stopSync() {
        listener?.cancel()
        listener = nil
        for (_, conn) in connections {
            conn.cancel()
        }
        connections.removeAll()
        isRunning = false
    }

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
            // Do NOT auto-restart here — the old code restarted on failure, which caused
            // an infinite loop: port still in use -> restart -> "Address already in use" -> ...
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

    // MARK: - Private: Message Framing

    private func receiveMessage(from connection: NWConnection, connId: ObjectIdentifier) {
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

            do {
                if let message = try JSONSerialization.jsonObject(with: messageData, options: []) as? [String: Any] {
                    self.handleReceivedMessage(message, from: connection)
                } else {
                    self.logger.error("Invalid JSON message format")
                }
            } catch {
                self.logger.error("JSON parse error: \(error.localizedDescription)")
            }

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

    static func frameMessage(_ data: Data) -> Data {
        var length = UInt32(data.count).littleEndian
        var framedData = Data(bytes: &length, count: 4)
        framedData.append(data)
        return framedData
    }
}
