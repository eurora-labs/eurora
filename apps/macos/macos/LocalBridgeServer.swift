//
//  LocalBridgeServer.swift
//  Eurora
//
//  Local Unix domain socket server for IPC between Safari extension and container app.
//  The extension connects here, and messages are forwarded to the gRPC client.
//
//  This file belongs to the container app, NOT the extension.
//

import Foundation
import Network
import os.log

/// Port for local bridge server
private let kLocalBridgeServerPort: UInt16 = 14310

/// Protocol for handling messages from the extension
protocol LocalBridgeServerDelegate: AnyObject {
    /// Called when a message is received from the extension
    func localBridgeServer(_ server: LocalBridgeServer, didReceiveMessage message: [String: Any], completion: @escaping ([String: Any]?) -> Void)
}

/// Local TCP server for communication with the Safari extension
@available(macOS 13.0, *)
class LocalBridgeServer {
    
    // MARK: - Properties
    
    weak var delegate: LocalBridgeServerDelegate?
    
    private let logger = Logger(subsystem: "com.eurora.macos", category: "LocalBridgeServer")
    private let queue = DispatchQueue(label: "com.eurora.local-bridge-server", qos: .userInitiated)
    
    private var listener: NWListener?
    private var connections: [NWConnection] = []
    
    // MARK: - Initialization
    
    init() {}
    
    deinit {
        stop()
    }
    
    // MARK: - Server Control
    
    /// Start the local server
    func start() {
        queue.async { [weak self] in
            self?.startInternal()
        }
    }
    
    /// Stop the local server
    func stop() {
        queue.async { [weak self] in
            self?.stopInternal()
        }
    }
    
    /// Send a message to all connected extensions (for server-initiated messages)
    func broadcast(message: [String: Any]) {
        queue.async { [weak self] in
            guard let self = self else { return }
            
            guard let data = try? JSONSerialization.data(withJSONObject: message, options: []) else {
                self.logger.error("Failed to serialize broadcast message")
                return
            }
            
            let framedData = self.frameMessage(data)
            
            for connection in self.connections {
                connection.send(content: framedData, completion: .contentProcessed { error in
                    if let error = error {
                        self.logger.error("Failed to broadcast: \(error.localizedDescription)")
                    }
                })
            }
        }
    }
    
    // MARK: - Private Methods
    
    private func startInternal() {
        guard listener == nil else {
            logger.debug("Server already running")
            return
        }
        
        do {
            // Create TCP listener on localhost only
            let parameters = NWParameters.tcp
            parameters.allowLocalEndpointReuse = true
            
            // Only listen on localhost for security
            let listener = try NWListener(using: parameters, on: NWEndpoint.Port(rawValue: kLocalBridgeServerPort)!)
            
            listener.stateUpdateHandler = { [weak self] state in
                self?.handleListenerStateChange(state)
            }
            
            listener.newConnectionHandler = { [weak self] connection in
                self?.handleNewConnection(connection)
            }
            
            listener.start(queue: queue)
            self.listener = listener
            
            logger.info("Local bridge server starting on port \(kLocalBridgeServerPort)")
            
        } catch {
            logger.error("Failed to create listener: \(error.localizedDescription)")
        }
    }
    
    private func stopInternal() {
        listener?.cancel()
        listener = nil
        
        for connection in connections {
            connection.cancel()
        }
        connections.removeAll()
        
        logger.info("Local bridge server stopped")
    }
    
    private func handleListenerStateChange(_ state: NWListener.State) {
        switch state {
        case .ready:
            logger.info("Local bridge server ready on port \(kLocalBridgeServerPort)")
        case .failed(let error):
            logger.error("Local bridge server failed: \(error.localizedDescription)")
            // Try to restart
            stopInternal()
            DispatchQueue.main.asyncAfter(deadline: .now() + 2.0) { [weak self] in
                self?.startInternal()
            }
        case .cancelled:
            logger.info("Local bridge server cancelled")
        default:
            break
        }
    }
    
    private func handleNewConnection(_ connection: NWConnection) {
        logger.info("New connection from extension")
        
        connection.stateUpdateHandler = { [weak self] state in
            self?.handleConnectionStateChange(connection, state: state)
        }
        
        connections.append(connection)
        connection.start(queue: queue)
        
        // Start receiving messages
        receiveMessage(from: connection)
    }
    
    private func handleConnectionStateChange(_ connection: NWConnection, state: NWConnection.State) {
        switch state {
        case .ready:
            logger.debug("Connection ready")
        case .failed(let error):
            logger.error("Connection failed: \(error.localizedDescription)")
            removeConnection(connection)
        case .cancelled:
            logger.debug("Connection cancelled")
            removeConnection(connection)
        default:
            break
        }
    }
    
    private func removeConnection(_ connection: NWConnection) {
        connections.removeAll { $0 === connection }
    }
    
    private func receiveMessage(from connection: NWConnection) {
        // First read the 4-byte length prefix
        connection.receive(minimumIncompleteLength: 4, maximumLength: 4) { [weak self] data, _, isComplete, error in
            guard let self = self else { return }
            
            if let error = error {
                self.logger.error("Receive error: \(error.localizedDescription)")
                return
            }
            
            if isComplete {
                self.logger.debug("Connection closed by extension")
                self.removeConnection(connection)
                return
            }
            
            guard let lengthData = data, lengthData.count == 4 else {
                self.logger.error("Invalid length prefix")
                self.receiveMessage(from: connection)
                return
            }
            
            // Parse length (little-endian)
            let length = lengthData.withUnsafeBytes { $0.load(as: UInt32.self).littleEndian }
            
            guard length > 0 && length < 8 * 1024 * 1024 else {
                self.logger.error("Invalid message length: \(length)")
                self.receiveMessage(from: connection)
                return
            }
            
            // Read the message body
            self.receiveMessageBody(from: connection, length: Int(length))
        }
    }
    
    private func receiveMessageBody(from connection: NWConnection, length: Int) {
        connection.receive(minimumIncompleteLength: length, maximumLength: length) { [weak self] data, _, isComplete, error in
            guard let self = self else { return }
            
            if let error = error {
                self.logger.error("Receive body error: \(error.localizedDescription)")
                return
            }
            
            if isComplete && data == nil {
                self.logger.debug("Connection closed")
                self.removeConnection(connection)
                return
            }
            
            guard let messageData = data, messageData.count == length else {
                self.logger.error("Incomplete message body")
                self.receiveMessage(from: connection)
                return
            }
            
            // Parse JSON message
            do {
                if let message = try JSONSerialization.jsonObject(with: messageData, options: []) as? [String: Any] {
                    self.handleMessage(message, from: connection)
                } else {
                    self.logger.error("Invalid message format")
                }
            } catch {
                self.logger.error("Failed to parse message: \(error.localizedDescription)")
            }
            
            // Continue receiving
            self.receiveMessage(from: connection)
        }
    }
    
    private func handleMessage(_ message: [String: Any], from connection: NWConnection) {
        logger.debug("Received message from extension")
        
        delegate?.localBridgeServer(self, didReceiveMessage: message) { [weak self] response in
            guard let self = self, let response = response else { return }
            
            self.queue.async {
                self.sendResponse(response, to: connection)
            }
        }
    }
    
    private func sendResponse(_ response: [String: Any], to connection: NWConnection) {
        do {
            let data = try JSONSerialization.data(withJSONObject: response, options: [])
            let framedData = frameMessage(data)
            
            connection.send(content: framedData, completion: .contentProcessed { [weak self] error in
                if let error = error {
                    self?.logger.error("Failed to send response: \(error.localizedDescription)")
                }
            })
        } catch {
            logger.error("Failed to serialize response: \(error.localizedDescription)")
        }
    }
    
    private func frameMessage(_ data: Data) -> Data {
        var length = UInt32(data.count).littleEndian
        var framedData = Data(bytes: &length, count: 4)
        framedData.append(data)
        return framedData
    }
}
