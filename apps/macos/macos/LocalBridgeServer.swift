import Foundation
import Network
import os.log

private let kLocalBridgeServerPort: UInt16 = 14311

@available(macOS 13.0, *)
protocol LocalBridgeServerDelegate: AnyObject {
    /// Called when a frame arrives on a local extension connection.
    /// `completion` is called with the reply to send back, or `nil` if no
    /// reply is needed (e.g. the frame was forwarded to the bridge and the
    /// reply will arrive asynchronously through `broadcast(frame:)`).
    func localBridgeServer(
        _ server: LocalBridgeServer,
        didReceive frame: Frame,
        completion: @escaping (Frame?) -> Void
    )
}

/// TCP server that the Safari extension uses to talk to the launcher.
///
/// The wire format is length-prefixed JSON (4-byte little-endian length,
/// then a UTF-8 JSON-encoded `Frame`). This is an internal channel between
/// the extension's `SafariWebExtensionHandler` and the launcher process —
/// the externally-visible bridge protocol on port 1431 is what carries the
/// frames the rest of the way to the desktop app.
@available(macOS 13.0, *)
class LocalBridgeServer {

    weak var delegate: LocalBridgeServerDelegate?

    private let logger = Logger(subsystem: "com.eurora.macos", category: "LocalBridgeServer")
    private let queue = DispatchQueue(label: "com.eurora.local-bridge-server", qos: .userInitiated)

    private var listener: NWListener?
    private var connections: [ObjectIdentifier: NWConnection] = [:]
    private var isRunning = false

    init() {}

    deinit {
        listener?.cancel()
        for (_, conn) in connections {
            conn.cancel()
        }
    }

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

    /// Send a frame to every connected extension. Used to forward
    /// server-pushed events (`Event`) and async deliveries (`Cancel`).
    func broadcast(frame: Frame) {
        let data: Data
        do {
            data = try frame.encodeJSON()
        } catch {
            logger.error("Failed to encode broadcast frame: \(error.localizedDescription, privacy: .public)")
            return
        }
        let framed = LocalBridgeServer.frame(data)

        queue.async { [weak self] in
            guard let self else { return }
            for (_, connection) in self.connections {
                connection.send(content: framed, completion: .contentProcessed { error in
                    if let error {
                        self.logger.error(
                            "Broadcast send error: \(error.localizedDescription, privacy: .public)"
                        )
                    }
                })
            }
        }
    }

    // MARK: - Lifecycle

    private func startInternal() {
        guard !isRunning else {
            logger.debug("Server already running")
            return
        }

        let parameters = NWParameters.tcp
        parameters.allowLocalEndpointReuse = true
        parameters.requiredInterfaceType = .loopback

        guard let port = NWEndpoint.Port(rawValue: kLocalBridgeServerPort) else {
            logger.error("Invalid port: \(kLocalBridgeServerPort, privacy: .public)")
            return
        }

        let listener: NWListener
        do {
            listener = try NWListener(using: parameters, on: port)
        } catch {
            logger.error("Failed to create listener: \(error.localizedDescription, privacy: .public)")
            return
        }

        listener.stateUpdateHandler = { [weak self] state in
            self?.handleListenerState(state)
        }
        listener.newConnectionHandler = { [weak self] connection in
            self?.handleNewConnection(connection)
        }
        listener.start(queue: queue)

        self.listener = listener
        self.isRunning = true
        logger.info("Local bridge server starting on port \(kLocalBridgeServerPort, privacy: .public)")
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
            logger.info("Local bridge server ready on port \(kLocalBridgeServerPort, privacy: .public)")
        case .failed(let error):
            logger.error("Local bridge server failed: \(error.localizedDescription, privacy: .public)")
            // Don't auto-restart — old code looped on "address in use".
            stopInternal()
        case .cancelled:
            isRunning = false
        default:
            break
        }
    }

    // MARK: - Per-connection handling

    private func handleNewConnection(_ connection: NWConnection) {
        let connId = ObjectIdentifier(connection)
        logger.info("New extension connection")

        connection.stateUpdateHandler = { [weak self] state in
            self?.handleConnectionState(connId, state: state)
        }

        connections[connId] = connection
        connection.start(queue: queue)

        receiveLength(from: connection, connId: connId)
    }

    private func handleConnectionState(_ connId: ObjectIdentifier, state: NWConnection.State) {
        switch state {
        case .ready:
            logger.debug("Extension connection ready")
        case .failed(let error):
            logger.error("Extension connection failed: \(error.localizedDescription, privacy: .public)")
            connections.removeValue(forKey: connId)
        case .cancelled:
            connections.removeValue(forKey: connId)
        default:
            break
        }
    }

    // MARK: - Length-prefixed framing

    private func receiveLength(from connection: NWConnection, connId: ObjectIdentifier) {
        connection.receive(minimumIncompleteLength: 4, maximumLength: 4) { [weak self] data, _, isComplete, error in
            guard let self else { return }

            if let error {
                self.logger.error("Receive error: \(error.localizedDescription, privacy: .public)")
                self.connections.removeValue(forKey: connId)
                return
            }

            if isComplete {
                self.connections.removeValue(forKey: connId)
                return
            }

            guard let lengthData = data, lengthData.count == 4 else {
                self.logger.error("Invalid length prefix")
                self.receiveLength(from: connection, connId: connId)
                return
            }

            let length = lengthData.withUnsafeBytes { $0.load(as: UInt32.self).littleEndian }
            guard length > 0, Int(length) <= BridgeProtocol.maxFrameSize else {
                self.logger.error("Invalid message length: \(length, privacy: .public)")
                self.connections.removeValue(forKey: connId)
                return
            }

            self.receiveBody(from: connection, connId: connId, length: Int(length))
        }
    }

    private func receiveBody(from connection: NWConnection, connId: ObjectIdentifier, length: Int) {
        connection.receive(minimumIncompleteLength: length, maximumLength: length) { [weak self] data, _, isComplete, error in
            guard let self else { return }

            if let error {
                self.logger.error("Receive body error: \(error.localizedDescription, privacy: .public)")
                self.connections.removeValue(forKey: connId)
                return
            }
            if isComplete && data == nil {
                self.connections.removeValue(forKey: connId)
                return
            }

            guard let body = data, body.count == length else {
                self.logger.error("Incomplete message body")
                self.connections.removeValue(forKey: connId)
                return
            }

            self.handleBody(body, from: connection)
            self.receiveLength(from: connection, connId: connId)
        }
    }

    private func handleBody(_ data: Data, from connection: NWConnection) {
        let frame: Frame
        do {
            frame = try Frame.decode(data)
        } catch {
            logger.error("JSON decode error: \(error.localizedDescription, privacy: .public)")
            return
        }
        logger.debug("Received from extension: \(frame.summary, privacy: .public)")

        delegate?.localBridgeServer(self, didReceive: frame) { [weak self] reply in
            guard let self, let reply else { return }
            self.send(reply, to: connection)
        }
    }

    private func send(_ frame: Frame, to connection: NWConnection) {
        let data: Data
        do {
            data = try frame.encodeJSON()
        } catch {
            logger.error("Failed to encode reply: \(error.localizedDescription, privacy: .public)")
            return
        }
        let framed = LocalBridgeServer.frame(data)

        queue.async { [weak self] in
            connection.send(content: framed, completion: .contentProcessed { error in
                if let error {
                    self?.logger.error(
                        "Failed to send reply: \(error.localizedDescription, privacy: .public)"
                    )
                }
            })
        }
    }

    // MARK: - Helpers

    static func frame(_ data: Data) -> Data {
        var length = UInt32(data.count).littleEndian
        var framed = Data(bytes: &length, count: 4)
        framed.append(data)
        return framed
    }
}
