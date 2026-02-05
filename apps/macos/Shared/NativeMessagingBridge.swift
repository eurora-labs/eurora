//
//  NativeMessagingBridge.swift
//  Eurora
//
//  Bridge between Safari extension and the container app.
//  Connects to the local bridge server running in the container app,
//  which in turn forwards messages to the euro-activity gRPC server.
//
//  This file is shared between the container app and the Safari extension.
//

import Foundation
import Network
import os.log

/// Port for connecting to the local bridge server in the container app
private let kBridgeConnectionPort: UInt16 = 14310

/// Singleton bridge that manages communication with the container app
@available(macOS 11.0, *)
class NativeMessagingBridge {

    static let shared = NativeMessagingBridge()

    private let logger = Logger(subsystem: "com.eurora.macos", category: "NativeMessagingBridge")
    private let queue = DispatchQueue(label: "com.eurora.native-messaging-bridge", qos: .userInitiated)
    
    private var connection: NWConnection?
    private var isConnected = false
    private var shouldReconnect = true
    
    private let responseLock = NSLock()
    private var pendingCallbacks: [String: (Data) -> Void] = [:]
    private var readBuffer = Data()
    
    private init() {}

    /// Start the connection to the local bridge server
    func start() {
        queue.async { [weak self] in
            self?.connectToServer()
        }
    }

    /// Stop the connection
    func stop() {
        shouldReconnect = false
        queue.async { [weak self] in
            self?.disconnect()
        }
    }

    /// Send a message to the bridge and wait for response with timeout
    func sendMessage(_ message: [String: Any], timeout: TimeInterval = 10.0, completion: @escaping (Result<[String: Any], Error>) -> Void) {
        queue.async { [weak self] in
            guard let self = self else {
                completion(.failure(BridgeError.bridgeDeallocated))
                return
            }

            guard self.isConnected, let connection = self.connection else {
                self.logger.error("Not connected to local bridge server, attempting reconnect...")
                self.connectToServer()

                // Retry after short delay (only once)
                DispatchQueue.main.asyncAfter(deadline: .now() + 0.5) {
                    if self.isConnected {
                        self.sendMessage(message, timeout: timeout, completion: completion)
                    } else {
                        completion(.failure(BridgeError.processNotRunning))
                    }
                }
                return
            }

            do {
                // Serialize to JSON
                let jsonData = try JSONSerialization.data(withJSONObject: message, options: [])
                
                // Generate or extract request ID for tracking response
                var requestId: String?
                if let kind = message["kind"] as? [String: Any],
                   let request = kind["Request"] as? [String: Any],
                   let id = request["id"] {
                    requestId = "\(id)"
                }
                
                // Create a unique callback ID if no request ID
                let callbackId = requestId ?? UUID().uuidString
                var callbackFired = false
                let callbackLock = NSLock()

                // Register callback for response
                self.responseLock.lock()
                self.pendingCallbacks[callbackId] = { responseData in
                    callbackLock.lock()
                    guard !callbackFired else {
                        callbackLock.unlock()
                        return
                    }
                    callbackFired = true
                    callbackLock.unlock()
                    
                    do {
                        if let response = try JSONSerialization.jsonObject(with: responseData, options: []) as? [String: Any] {
                            completion(.success(response))
                        } else {
                            completion(.failure(BridgeError.invalidResponse))
                        }
                    } catch {
                        completion(.failure(error))
                    }
                }
                self.responseLock.unlock()
                
                // Set up timeout
                DispatchQueue.main.asyncAfter(deadline: .now() + timeout) {
                    callbackLock.lock()
                    guard !callbackFired else {
                        callbackLock.unlock()
                        return
                    }
                    callbackFired = true
                    callbackLock.unlock()
                    
                    self.responseLock.lock()
                    self.pendingCallbacks.removeValue(forKey: callbackId)
                    self.responseLock.unlock()
                    
                    self.logger.warning("Request timed out after \(timeout) seconds")
                    completion(.failure(BridgeError.timeout))
                }

                // Frame the message with length prefix
                let framedData = self.frameMessage(jsonData)
                
                // Send to server
                connection.send(content: framedData, completion: .contentProcessed { [weak self] error in
                    if let error = error {
                        self?.logger.error("Failed to send message: \(error.localizedDescription)")
                    } else {
                        self?.logger.debug("Sent message to local bridge server: \(jsonData.count) bytes")
                    }
                })

            } catch {
                self.logger.error("Failed to send message: \(error.localizedDescription)")
                completion(.failure(error))
            }
        }
    }

    /// Send a message synchronously (blocks until response)
    func sendMessageSync(_ message: [String: Any], timeout: TimeInterval = 30.0) -> Result<[String: Any], Error> {
        let semaphore = DispatchSemaphore(value: 0)
        var result: Result<[String: Any], Error> = .failure(BridgeError.timeout)

        sendMessage(message, timeout: timeout) { response in
            result = response
            semaphore.signal()
        }

        let waitResult = semaphore.wait(timeout: .now() + timeout)
        if waitResult == .timedOut {
            return .failure(BridgeError.timeout)
        }

        return result
    }

    // MARK: - Private Methods

    private func connectToServer() {
        guard connection == nil || !isConnected else {
            logger.debug("Already connected")
            return
        }
        
        logger.info("Connecting to local bridge server on port \(kBridgeConnectionPort)")
        
        // Create TCP connection to localhost
        let endpoint = NWEndpoint.hostPort(host: .ipv4(.loopback), port: NWEndpoint.Port(rawValue: kBridgeConnectionPort)!)
        let connection = NWConnection(to: endpoint, using: .tcp)
        
        connection.stateUpdateHandler = { [weak self] state in
            self?.handleConnectionStateChange(state)
        }
        
        self.connection = connection
        connection.start(queue: queue)
    }

    private func disconnect() {
        isConnected = false
        connection?.cancel()
        connection = nil
        readBuffer.removeAll()

        // Cancel all pending requests
        responseLock.lock()
        pendingCallbacks.removeAll()
        responseLock.unlock()

        logger.info("Disconnected from local bridge server")
    }
    
    private func handleConnectionStateChange(_ state: NWConnection.State) {
        switch state {
        case .ready:
            logger.info("Connected to local bridge server")
            isConnected = true
            startReceiving()
            
        case .failed(let error):
            logger.error("Connection failed: \(error.localizedDescription)")
            isConnected = false
            scheduleReconnect()
            
        case .cancelled:
            logger.debug("Connection cancelled")
            isConnected = false
            
        case .waiting(let error):
            logger.warning("Connection waiting: \(error.localizedDescription)")
            
        default:
            break
        }
    }
    
    private func scheduleReconnect() {
        guard shouldReconnect else { return }
        
        logger.info("Scheduling reconnect in 2 seconds")
        queue.asyncAfter(deadline: .now() + 2.0) { [weak self] in
            self?.disconnect()
            self?.connectToServer()
        }
    }
    
    private func startReceiving() {
        receiveNextMessage()
    }
    
    private func receiveNextMessage() {
        guard let connection = connection, isConnected else { return }
        
        // First read the 4-byte length prefix
        connection.receive(minimumIncompleteLength: 4, maximumLength: 4) { [weak self] data, _, isComplete, error in
            guard let self = self else { return }
            
            if let error = error {
                self.logger.error("Receive error: \(error.localizedDescription)")
                return
            }
            
            if isComplete {
                self.logger.debug("Connection closed by server")
                self.isConnected = false
                self.scheduleReconnect()
                return
            }
            
            guard let lengthData = data, lengthData.count == 4 else {
                self.logger.error("Invalid length prefix")
                self.receiveNextMessage()
                return
            }
            
            // Parse length (little-endian)
            let length = lengthData.withUnsafeBytes { $0.load(as: UInt32.self).littleEndian }
            
            guard length > 0 && length < 8 * 1024 * 1024 else {
                self.logger.error("Invalid message length: \(length)")
                self.receiveNextMessage()
                return
            }
            
            // Read the message body
            self.receiveMessageBody(length: Int(length))
        }
    }
    
    private func receiveMessageBody(length: Int) {
        guard let connection = connection else { return }
        
        connection.receive(minimumIncompleteLength: length, maximumLength: length) { [weak self] data, _, isComplete, error in
            guard let self = self else { return }
            
            if let error = error {
                self.logger.error("Receive body error: \(error.localizedDescription)")
                return
            }
            
            if isComplete && data == nil {
                self.logger.debug("Connection closed")
                self.isConnected = false
                self.scheduleReconnect()
                return
            }
            
            guard let messageData = data, messageData.count == length else {
                self.logger.error("Incomplete message body")
                self.receiveNextMessage()
                return
            }
            
            // Handle the received message
            self.handleReceivedMessage(messageData)
            
            // Continue receiving
            self.receiveNextMessage()
        }
    }
    
    private func handleReceivedMessage(_ data: Data) {
        logger.debug("Received message: \(data.count) bytes")
        
        // Try to extract response ID
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
                
                if let responseId = responseId {
                    responseLock.lock()
                    let callback = pendingCallbacks.removeValue(forKey: responseId)
                    responseLock.unlock()
                    
                    if let callback = callback {
                        DispatchQueue.main.async {
                            callback(data)
                        }
                        return
                    }
                }
            }
        } catch {
            logger.error("Failed to parse received message: \(error.localizedDescription)")
        }
        
        // If no matching callback, just log it
        logger.debug("Received message with no pending callback")
    }
    
    private func frameMessage(_ data: Data) -> Data {
        var length = UInt32(data.count).littleEndian
        var framedData = Data(bytes: &length, count: 4)
        framedData.append(data)
        return framedData
    }
    
    /// Called by SafariWebExtensionHandler when JavaScript sends a response to a server request
    /// Returns true if this was handled as a response to a pending server request
    func handleResponseFromExtension(_ response: [String: Any]) -> Bool {
        // Forward the response to the local bridge server
        // which will forward it to the container app's gRPC client
        
        guard let kind = response["kind"] as? [String: Any],
              kind["Response"] != nil else {
            return false
        }
        
        // Send it to the server (fire-and-forget, no completion needed)
        queue.async { [weak self] in
            guard let self = self, let connection = self.connection, self.isConnected else {
                self?.logger.error("Cannot forward response - not connected")
                return
            }
            
            do {
                let jsonData = try JSONSerialization.data(withJSONObject: response, options: [])
                let framedData = self.frameMessage(jsonData)
                
                connection.send(content: framedData, completion: .contentProcessed { [weak self] error in
                    if let error = error {
                        self?.logger.error("Failed to forward response: \(error.localizedDescription)")
                    } else {
                        self?.logger.debug("Forwarded extension response to local bridge server")
                    }
                })
            } catch {
                self.logger.error("Failed to serialize response: \(error.localizedDescription)")
            }
        }
        
        return true
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
