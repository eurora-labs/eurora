//
//  AppDelegate.swift
//  macos
//
//  Container app for the Eurora Safari extension.
//  Manages the gRPC client connection to euro-activity and the local bridge server
//  for communication with the Safari extension.
//

import Cocoa
import SafariServices
import os.log
import AppKit


@main
@available(macOS 13.0, *)
class AppDelegate: NSObject, NSApplicationDelegate, BrowserBridgeClientDelegate, LocalBridgeServerDelegate {
    
    private let logger = Logger(subsystem: "com.eurora.macos", category: "AppDelegate")
    
    /// Bundle identifier for the Safari extension
    private let extensionBundleIdentifier = "com.eurora.macos.Extension"
    
    /// gRPC client for connecting to euro-activity browser bridge service
    private var grpcClient: BrowserBridgeClient?
    
    /// Local server for communication with Safari extension
    private var localBridgeServer: LocalBridgeServer?
    
    /// Pending requests from the extension waiting for gRPC responses
    /// Key: request ID (as String), Value: completion handler
    private var pendingExtensionRequests: [String: ([String: Any]?) -> Void] = [:]
    private let pendingRequestsLock = NSLock()
    
    /// Pending requests from the gRPC server waiting for extension responses
    /// Key: request ID (as String), Value: the original request
    private var pendingServerRequests: [String: [String: Any]] = [:]
    private let pendingServerRequestsLock = NSLock()

    func applicationDidFinishLaunching(_ notification: Notification) {
        logger.info("Eurora container app starting")
        
        // Start the local bridge server for extension communication
        startLocalBridgeServer()
        
        // Start the gRPC client to connect to euro-activity
        startGrpcClient()
    }

    func applicationWillTerminate(_ notification: Notification) {
        logger.info("Eurora container app terminating")
        
        // Stop the gRPC client
        grpcClient?.disconnect()
        grpcClient = nil
        
        // Stop the local bridge server
        localBridgeServer?.stop()
        localBridgeServer = nil
    }

    func applicationShouldTerminateAfterLastWindowClosed(_ sender: NSApplication) -> Bool {
        return true
    }
    
    // MARK: - Private Methods
    
    private func startLocalBridgeServer() {
        let server = LocalBridgeServer()
        server.delegate = self
        server.start()
        self.localBridgeServer = server
        logger.info("Local bridge server started")
    }
    
    private func startGrpcClient() {
        // Get PIDs for registration
        let hostPid = UInt32(getpid())
        let browserPid = findSafariPid().map { UInt32($0) } ?? 0
        
        logger.info("Starting gRPC client: host_pid=\(hostPid), browser_pid=\(browserPid)")
        
        let client = BrowserBridgeClient(hostPid: hostPid, browserPid: browserPid)
        client.delegate = self
        client.connect()
        self.grpcClient = client
    }
    
    /// Find Safari's PID by looking for running Safari processes
    private func findSafariPid() -> pid_t? {
        let workspace = NSWorkspace.shared
        let runningApps = workspace.runningApplications
        
        // Look for Safari by bundle identifier
        for app in runningApps {
            if app.bundleIdentifier == "com.apple.Safari" {
                return app.processIdentifier
            }
        }
        
        // Fallback: look for Safari Technology Preview
        for app in runningApps {
            if app.bundleIdentifier == "com.apple.SafariTechnologyPreview" {
                return app.processIdentifier
            }
        }
        
        return nil
    }
    
    // MARK: - LocalBridgeServerDelegate
    
    func localBridgeServer(_ server: LocalBridgeServer, didReceiveMessage message: [String: Any], completion: @escaping ([String: Any]?) -> Void) {
        logger.debug("Received message from extension via local bridge")
        
        // Check if this is a response to a pending server request
        if let kind = message["kind"] as? [String: Any],
           let responseData = kind["Response"] as? [String: Any],
           let responseId = responseData["id"] {
            let responseIdString = "\(responseId)"
            
            pendingServerRequestsLock.lock()
            let pendingRequest = pendingServerRequests.removeValue(forKey: responseIdString)
            pendingServerRequestsLock.unlock()
            
            if pendingRequest != nil {
                // This is a response to a request from the gRPC server
                logger.info("Forwarding extension response to gRPC server: id=\(responseIdString)")
                forwardToGrpcServer(message)
                completion(["status": "forwarded"])
                return
            }
        }
        
        // This is a request from the extension - forward to gRPC server
        forwardExtensionRequestToGrpc(message, completion: completion)
    }
    
    private func forwardExtensionRequestToGrpc(_ message: [String: Any], completion: @escaping ([String: Any]?) -> Void) {
        guard let client = grpcClient else {
            logger.error("gRPC client not available")
            completion(["kind": ["Error": ["message": "gRPC client not connected"]]])
            return
        }
        
        // Extract request ID for tracking
        var requestId: String?
        if let kind = message["kind"] as? [String: Any],
           let request = kind["Request"] as? [String: Any],
           let id = request["id"] {
            requestId = "\(id)"
        }
        
        // If we have a request ID, store the completion handler for later
        if let requestId = requestId {
            pendingRequestsLock.lock()
            pendingExtensionRequests[requestId] = completion
            pendingRequestsLock.unlock()
        }
        
        // Send the message directly as dictionary (BrowserBridgeClient handles conversion)
        client.send(frame: message)
        logger.debug("Forwarded extension request to gRPC server")
        
        // If no request ID, complete immediately (fire-and-forget)
        if requestId == nil {
            completion(nil)
        }
    }
    
    private func forwardToGrpcServer(_ message: [String: Any]) {
        guard let client = grpcClient else {
            logger.error("gRPC client not available for forwarding response")
            return
        }
        
        client.send(frame: message)
    }
    
    // MARK: - BrowserBridgeClientDelegate
    
    func browserBridgeClientDidConnect(_ client: BrowserBridgeClient) {
        logger.info("Connected to euro-activity gRPC server")
    }
    
    func browserBridgeClientDidDisconnect(_ client: BrowserBridgeClient, error: Error?) {
        if let error = error {
            logger.warning("Disconnected from gRPC server: \(error.localizedDescription)")
        } else {
            logger.info("Disconnected from gRPC server")
        }
    }
    
    func browserBridgeClient(_ client: BrowserBridgeClient, didReceiveFrame frame: [String: Any]) {
        handleFrameFromGrpcServer(frame)
    }
    
    private func handleFrameFromGrpcServer(_ frame: [String: Any]) {
        guard let kind = frame["kind"] as? [String: Any] else {
            logger.error("Invalid frame format")
            return
        }
        
        // Check if this is a response to a pending extension request
        if let response = kind["Response"] as? [String: Any],
           let responseId = response["id"] {
            let responseIdString = "\(responseId)"
            
            pendingRequestsLock.lock()
            let completion = pendingExtensionRequests.removeValue(forKey: responseIdString)
            pendingRequestsLock.unlock()
            
            if let completion = completion {
                logger.debug("Delivering gRPC response to extension: id=\(responseIdString)")
                completion(frame)
                return
            }
        }
        
        // Check for error responses too
        if let error = kind["Error"] as? [String: Any],
           let errorId = error["id"] {
            let errorIdString = "\(errorId)"
            
            pendingRequestsLock.lock()
            let completion = pendingExtensionRequests.removeValue(forKey: errorIdString)
            pendingRequestsLock.unlock()
            
            if let completion = completion {
                logger.debug("Delivering gRPC error to extension: id=\(errorIdString)")
                completion(frame)
                return
            }
        }
        
        // This is a request from the server - need to forward to extension
        if let request = kind["Request"] as? [String: Any],
           let requestId = request["id"],
           let action = request["action"] as? String {
            let requestIdString = "\(requestId)"
            
            logger.debug("Received request from gRPC server: action=\(action), id=\(requestIdString)")
            
            // Store pending request
            pendingServerRequestsLock.lock()
            pendingServerRequests[requestIdString] = request
            pendingServerRequestsLock.unlock()
            
            // Forward to extension via dispatch message
            dispatchToExtension(frame: frame, action: action, requestId: requestIdString)
            return
        }
        
        // Events are fire-and-forget, forward to extension
        if kind["Event"] != nil {
            localBridgeServer?.broadcast(message: frame)
        }
    }
    
    /// Dispatch a message to the Safari extension's JavaScript
    private func dispatchToExtension(frame: [String: Any], action: String, requestId: String) {
        do {
            let jsonData = try JSONSerialization.data(withJSONObject: frame, options: [])
            guard let jsonString = String(data: jsonData, encoding: .utf8) else {
                logger.error("Failed to convert frame to JSON string")
                return
            }
            
            let userInfo: [String: Any] = [
                "frame": frame,
                "frameJson": jsonString,
                "action": action,
                "requestId": requestId
            ]
            
            logger.info("Dispatching message to extension: action=\(action), requestId=\(requestId)")
            
            SFSafariApplication.dispatchMessage(
                withName: "NativeRequest",
                toExtensionWithIdentifier: extensionBundleIdentifier,
                userInfo: userInfo
            ) { [weak self] error in
                if let error = error {
                    self?.logger.error("Failed to dispatch message to extension: \(error.localizedDescription)")
                    self?.sendErrorResponseToServer(requestId: requestId, action: action, error: error.localizedDescription)
                } else {
                    self?.logger.debug("Successfully dispatched message to extension")
                }
            }
        } catch {
            logger.error("Failed to serialize frame for dispatch: \(error.localizedDescription)")
            sendErrorResponseToServer(requestId: requestId, action: action, error: error.localizedDescription)
        }
    }
    
    /// Send an error response back to the gRPC server when extension dispatch fails
    private func sendErrorResponseToServer(requestId: String, action: String, error: String) {
        pendingServerRequestsLock.lock()
        pendingServerRequests.removeValue(forKey: requestId)
        pendingServerRequestsLock.unlock()
        
        let idValue: Any = Int(requestId) ?? requestId
        
        let response: [String: Any] = [
            "kind": [
                "Response": [
                    "id": idValue,
                    "action": action,
                    "payload": "{\"kind\":\"Error\",\"data\":\"\(error)\"}"
                ]
            ]
        ]
        
        forwardToGrpcServer(response)
    }
}
