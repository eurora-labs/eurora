//
//  BrowserBridgeClient.swift
//  Eurora
//
//  gRPC client for connecting to the euro-activity browser bridge service.
//  Implements bidirectional streaming for Frame messages using grpc-swift 2.x.
//
//  This file belongs to the container app, NOT the extension.
//

import Foundation
import GRPCCore
import GRPCNIOTransportHTTP2
import os.log

/// Port for the browser bridge gRPC server (running in euro-activity)
private let kBrowserBridgePort: Int = 1431

/// Retry interval for connecting to the server (in seconds)
private let kRetryIntervalSecs: TimeInterval = 2.0

/// Protocol for receiving frames from the server
protocol BrowserBridgeClientDelegate: AnyObject {
    /// Called when a frame is received from the server
    func browserBridgeClient(_ client: BrowserBridgeClient, didReceiveFrame frame: [String: Any])
    
    /// Called when the connection is established
    func browserBridgeClientDidConnect(_ client: BrowserBridgeClient)
    
    /// Called when the connection is lost
    func browserBridgeClientDidDisconnect(_ client: BrowserBridgeClient, error: Error?)
}

/// gRPC client for the BrowserBridge service using grpc-swift 2.x
@available(macOS 15.0, *)
class BrowserBridgeClient {
    
    // MARK: - Properties
    
    weak var delegate: BrowserBridgeClientDelegate?
    
    private let logger = Logger(subsystem: "com.eurora.macos", category: "BrowserBridgeClient")
    
    private var transport: HTTP2ClientTransport.Posix?
    private var grpcClient: GRPCClient<HTTP2ClientTransport.Posix>?
    private var bridgeClient: BrowserBridge_BrowserBridge.Client<HTTP2ClientTransport.Posix>?
    
    private var connectionTask: Task<Void, Never>?
    private var streamTask: Task<Void, Error>?
    
    private var isConnected = false
    private var shouldReconnect = true
    
    private let hostPid: UInt32
    private let browserPid: UInt32
    
    /// Continuation for sending frames to the server
    private var outboundContinuation: AsyncStream<BrowserBridge_Frame>.Continuation?
    private let continuationLock = NSLock()
    
    // MARK: - Initialization
    
    init(hostPid: UInt32, browserPid: UInt32) {
        self.hostPid = hostPid
        self.browserPid = browserPid
    }
    
    deinit {
        disconnect()
    }
    
    // MARK: - Connection Management
    
    /// Start the client and connect to the server
    func connect() {
        connectionTask = Task {
            await connectWithRetry()
        }
    }
    
    /// Disconnect from the server
    func disconnect() {
        shouldReconnect = false
        
        continuationLock.lock()
        outboundContinuation?.finish()
        outboundContinuation = nil
        continuationLock.unlock()
        
        streamTask?.cancel()
        streamTask = nil
        connectionTask?.cancel()
        connectionTask = nil
        
        grpcClient?.beginGracefulShutdown()
        grpcClient = nil
        bridgeClient = nil
        transport = nil
        
        isConnected = false
        logger.info("Disconnected from browser bridge server")
    }
    
    /// Send a frame to the server
    func send(frame: [String: Any]) {
        guard let grpcFrame = Self.frameFromDictionary(frame) else {
            logger.error("Failed to convert dictionary to Frame")
            return
        }
        
        continuationLock.lock()
        let continuation = outboundContinuation
        continuationLock.unlock()
        
        guard let continuation = continuation else {
            logger.warning("Cannot send frame - not connected")
            return
        }
        
        continuation.yield(grpcFrame)
        logger.debug("Frame queued for sending")
    }
    
    // MARK: - Private Methods
    
    private func connectWithRetry() async {
        while shouldReconnect && !Task.isCancelled {
            logger.info("Attempting to connect to browser bridge server at [::1]:\(kBrowserBridgePort)")
            
            do {
                try await initializeClient()
                try await runStream()
            } catch {
                logger.error("Connection failed: \(error.localizedDescription)")
            }
            
            // Clean up after disconnect
            cleanup()
            
            if shouldReconnect && !Task.isCancelled {
                logger.info("Reconnecting in \(kRetryIntervalSecs) seconds...")
                try? await Task.sleep(for: .seconds(kRetryIntervalSecs))
            }
        }
    }
    
    private func initializeClient() async throws {
        // Create transport
        let security: HTTP2ClientTransport.Posix.TransportSecurity = .plaintext
        let transport = try HTTP2ClientTransport.Posix(
            target: .ipv6(host: "::1", port: kBrowserBridgePort),
            transportSecurity: security
        )
        self.transport = transport
        
        // Create gRPC client
        let grpcClient = GRPCClient(transport: transport)
        self.grpcClient = grpcClient
        
        // Create service client
        self.bridgeClient = BrowserBridge_BrowserBridge.Client(wrapping: grpcClient)
        
        logger.info("gRPC client initialized, starting run loop")
    }
    
    private func runStream() async throws {
        guard let transport = self.transport else {
            throw BridgeClientError.notConnected
        }
        
        // Create async stream for outbound messages
        let (outboundStream, continuation) = AsyncStream<BrowserBridge_Frame>.makeStream()
        
        continuationLock.lock()
        self.outboundContinuation = continuation
        continuationLock.unlock()
        
        // Send registration frame first
        logger.info("Sending registration frame: host_pid=\(self.hostPid), browser_pid=\(self.browserPid)")
        
        var registerFrame = BrowserBridge_RegisterFrame()
        registerFrame.hostPid = self.hostPid
        registerFrame.browserPid = self.browserPid
        
        var frame = BrowserBridge_Frame()
        frame.register = registerFrame
        
        continuation.yield(frame)
        
        // In grpc-swift 2.x, withGRPCClient manages the client lifecycle
        // It runs the client and ensures proper cleanup
        try await withGRPCClient(transport: transport) { grpcClient in
            // Store the client reference
            self.grpcClient = grpcClient
            
            // Create service client
            let bridgeClient = BrowserBridge_BrowserBridge.Client(wrapping: grpcClient)
            self.bridgeClient = bridgeClient
            
            // Mark as connected once we start the stream
            self.isConnected = true
            
            await MainActor.run { [weak self] in
                guard let self = self else { return }
                self.delegate?.browserBridgeClientDidConnect(self)
            }
            
            self.logger.info("Connected to browser bridge server")
            
            // Make the bidirectional streaming call
            try await bridgeClient.open(
                metadata: [:],
                options: .defaults,
                requestProducer: { writer in
                    // Forward messages from our async stream to the gRPC stream
                    for await message in outboundStream {
                        try await writer.write(message)
                    }
                },
                onResponse: { [self] (response: GRPCCore.StreamingClientResponse<BrowserBridge_Frame>) -> Void in
                    // Process the response stream
                    do {
                        for try await receivedFrame in response.messages {
                            self.handleReceivedFrame(receivedFrame)
                        }
                    } catch {
                        self.logger.error("Error receiving messages: \(error)")
                    }
                }
            )
        }
    }
    
    private func cleanup() {
        continuationLock.lock()
        outboundContinuation?.finish()
        outboundContinuation = nil
        continuationLock.unlock()
        
        isConnected = false
        
        grpcClient?.beginGracefulShutdown()
        grpcClient = nil
        bridgeClient = nil
        transport = nil
        
        Task { @MainActor [weak self] in
            guard let self = self else { return }
            self.delegate?.browserBridgeClientDidDisconnect(self, error: nil)
        }
    }
    
    private func handleReceivedFrame(_ frame: BrowserBridge_Frame) {
        guard let dict = Self.dictionaryFromFrame(frame) else {
            logger.warning("Failed to convert received frame to dictionary")
            return
        }
        
        logger.debug("Received frame from server")
        
        Task { @MainActor [weak self] in
            guard let self = self else { return }
            self.delegate?.browserBridgeClient(self, didReceiveFrame: dict)
        }
    }
}

// MARK: - Error Types

enum BridgeClientError: Error {
    case notConnected
}

// MARK: - Frame Conversion Helpers

@available(macOS 15.0, *)
extension BrowserBridgeClient {
    
    /// Convert a dictionary message to a Frame for sending to the server
    static func frameFromDictionary(_ dict: [String: Any]) -> BrowserBridge_Frame? {
        guard let kind = dict["kind"] as? [String: Any] else {
            return nil
        }
        
        var frame = BrowserBridge_Frame()
        
        if let request = kind["Request"] as? [String: Any] {
            var requestFrame = BrowserBridge_RequestFrame()
            if let id = request["id"] as? Int {
                requestFrame.id = UInt32(id)
            }
            if let action = request["action"] as? String {
                requestFrame.action = action
            }
            if let payload = request["payload"] as? String {
                requestFrame.payload = payload
            }
            frame.request = requestFrame
        } else if let response = kind["Response"] as? [String: Any] {
            var responseFrame = BrowserBridge_ResponseFrame()
            if let id = response["id"] as? Int {
                responseFrame.id = UInt32(id)
            }
            if let action = response["action"] as? String {
                responseFrame.action = action
            }
            if let payload = response["payload"] as? String {
                responseFrame.payload = payload
            }
            frame.response = responseFrame
        } else if let event = kind["Event"] as? [String: Any] {
            var eventFrame = BrowserBridge_EventFrame()
            if let action = event["action"] as? String {
                eventFrame.action = action
            }
            if let payload = event["payload"] as? String {
                eventFrame.payload = payload
            }
            frame.event = eventFrame
        } else if let error = kind["Error"] as? [String: Any] {
            var errorFrame = BrowserBridge_ErrorFrame()
            if let id = error["id"] as? Int {
                errorFrame.id = UInt32(id)
            }
            if let code = error["code"] as? Int {
                errorFrame.code = UInt32(code)
            }
            if let message = error["message"] as? String {
                errorFrame.message = message
            }
            if let details = error["details"] as? String {
                errorFrame.details = details
            }
            frame.error = errorFrame
        } else if let cancel = kind["Cancel"] as? [String: Any] {
            var cancelFrame = BrowserBridge_CancelFrame()
            if let id = cancel["id"] as? Int {
                cancelFrame.id = UInt32(id)
            }
            frame.cancel = cancelFrame
        } else if let register = kind["Register"] as? [String: Any] {
            var registerFrame = BrowserBridge_RegisterFrame()
            if let hostPid = register["host_pid"] as? Int {
                registerFrame.hostPid = UInt32(hostPid)
            }
            if let browserPid = register["browser_pid"] as? Int {
                registerFrame.browserPid = UInt32(browserPid)
            }
            frame.register = registerFrame
        } else {
            return nil
        }
        
        return frame
    }
    
    /// Convert a Frame from the server to a dictionary
    static func dictionaryFromFrame(_ frame: BrowserBridge_Frame) -> [String: Any]? {
        var kind: [String: Any] = [:]
        
        guard let frameKind = frame.kind else { return nil }
        
        switch frameKind {
        case .request(let request):
            var requestDict: [String: Any] = [
                "id": Int(request.id),
                "action": request.action
            ]
            if request.hasPayload {
                requestDict["payload"] = request.payload
            }
            kind["Request"] = requestDict
            
        case .response(let response):
            var responseDict: [String: Any] = [
                "id": Int(response.id),
                "action": response.action
            ]
            if response.hasPayload {
                responseDict["payload"] = response.payload
            }
            kind["Response"] = responseDict
            
        case .event(let event):
            var eventDict: [String: Any] = [
                "action": event.action
            ]
            if event.hasPayload {
                eventDict["payload"] = event.payload
            }
            kind["Event"] = eventDict
            
        case .error(let error):
            var errorDict: [String: Any] = [
                "id": Int(error.id),
                "code": Int(error.code),
                "message": error.message
            ]
            if error.hasDetails {
                errorDict["details"] = error.details
            }
            kind["Error"] = errorDict
            
        case .cancel(let cancel):
            kind["Cancel"] = ["id": Int(cancel.id)]
            
        case .register:
            // Registration frames are only sent, not received
            return nil
        }
        
        return ["kind": kind]
    }
}
