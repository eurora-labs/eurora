//
//  AppDelegate.swift
//  macos
//
//  Container app for the Eurora Safari extension.
//  Orchestrates the gRPC client (to euro-activity) and the local bridge server
//  (for Safari extension IPC).
//
//  This mirrors the architecture of crates/app/euro-native-messaging:
//  - gRPC bidirectional stream to euro-activity (BrowserBridgeClient)
//  - Message forwarding between extension and server
//  - Registration on connect
//

import Cocoa
import SafariServices
import os.log

@main
@available(macOS 15.0, *)
class AppDelegate: NSObject, NSApplicationDelegate, BrowserBridgeClientDelegate, LocalBridgeServerDelegate {

    private let logger = Logger(subsystem: "com.eurora.macos", category: "AppDelegate")

    /// Bundle identifier for the Safari extension
    private let extensionBundleIdentifier = "com.eurora.macos.Extension"

    /// gRPC client for connecting to euro-activity browser bridge service
    private var grpcClient: BrowserBridgeClient?

    /// Local TCP server for communication with Safari extension
    private var localBridgeServer: LocalBridgeServer?

    /// Pending requests from the extension waiting for gRPC responses.
    /// Key: request ID string, Value: completion handler to send response back to extension
    private var pendingExtensionRequests: [String: ([String: Any]?) -> Void] = [:]
    private let pendingExtensionRequestsLock = NSLock()

    /// Pending requests from the gRPC server waiting for extension responses.
    /// Key: request ID string, Value: the original request details
    private var pendingServerRequests: [String: [String: Any]] = [:]
    private let pendingServerRequestsLock = NSLock()

    // MARK: - App Lifecycle

    func applicationDidFinishLaunching(_ notification: Notification) {
        logger.info("Eurora container app starting")

        // Start the local bridge server first (extension needs this to connect)
        startLocalBridgeServer()

        // Start the gRPC client to connect to euro-activity
        startGrpcClient()
    }

    func applicationWillTerminate(_ notification: Notification) {
        logger.info("Eurora container app terminating")

        grpcClient?.disconnect()
        grpcClient = nil

        localBridgeServer?.stop()
        localBridgeServer = nil
    }

    func applicationShouldTerminateAfterLastWindowClosed(_ sender: NSApplication) -> Bool {
        return true
    }

    // MARK: - Startup

    private func startLocalBridgeServer() {
        let server = LocalBridgeServer()
        server.delegate = self
        server.start()
        self.localBridgeServer = server
    }

    private func startGrpcClient() {
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
        let runningApps = NSWorkspace.shared.runningApplications
        for app in runningApps {
            if app.bundleIdentifier == "com.apple.Safari" {
                return app.processIdentifier
            }
        }
        for app in runningApps {
            if app.bundleIdentifier == "com.apple.SafariTechnologyPreview" {
                return app.processIdentifier
            }
        }
        return nil
    }

    // MARK: - BrowserBridgeClientDelegate

    func browserBridgeClientDidConnect(_ client: BrowserBridgeClient) {
        logger.info("Connected to euro-activity gRPC server")
    }

    func browserBridgeClientDidDisconnect(_ client: BrowserBridgeClient, error: Error?) {
        if let error {
            logger.warning("Disconnected from gRPC server: \(error.localizedDescription)")
        } else {
            logger.info("Disconnected from gRPC server")
        }
    }

    func browserBridgeClient(_ client: BrowserBridgeClient, didReceiveFrame frame: BrowserBridge_Frame) {
        handleFrameFromServer(frame)
    }

    // MARK: - LocalBridgeServerDelegate

    func localBridgeServer(
        _ server: LocalBridgeServer,
        didReceiveMessage message: [String: Any],
        completion: @escaping ([String: Any]?) -> Void
    ) {
        logger.debug("Received message from extension via local bridge")

        // Check if this is a response to a pending server request
        if let kind = message["kind"] as? [String: Any],
           let responseData = kind["Response"] as? [String: Any],
           let responseId = responseData["id"] {

            let responseIdString = "\(responseId)"

            pendingServerRequestsLock.lock()
            let hadPending = pendingServerRequests.removeValue(forKey: responseIdString) != nil
            pendingServerRequestsLock.unlock()

            if hadPending {
                logger.info("Forwarding extension response to gRPC server: id=\(responseIdString)")
                sendDictionaryToServer(message)
                completion(["status": "forwarded"])
                return
            }
        }

        // This is a request from the extension — forward to gRPC server
        forwardExtensionRequestToServer(message, completion: completion)
    }

    // MARK: - Frame Routing: Extension → Server

    private func forwardExtensionRequestToServer(
        _ message: [String: Any],
        completion: @escaping ([String: Any]?) -> Void
    ) {
        guard let client = grpcClient, client.isConnected else {
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

        // Store completion handler so we can respond when the server replies
        if let requestId {
            pendingExtensionRequestsLock.lock()
            pendingExtensionRequests[requestId] = completion
            pendingExtensionRequestsLock.unlock()
        }

        // Convert dictionary to protobuf Frame and send
        sendDictionaryToServer(message)

        // If no request ID (fire-and-forget), complete immediately
        if requestId == nil {
            completion(nil)
        }
    }

    private func sendDictionaryToServer(_ dict: [String: Any]) {
        guard let frame = Self.frameFromDictionary(dict) else {
            logger.error("Failed to convert dictionary to Frame for gRPC")
            return
        }
        grpcClient?.send(frame: frame)
    }

    // MARK: - Frame Routing: Server → Extension

    private func handleFrameFromServer(_ frame: BrowserBridge_Frame) {
        guard let frameKind = frame.kind else {
            logger.warning("Received frame with no kind from server")
            return
        }

        switch frameKind {
        case .response(let response):
            deliverResponseToExtension(id: response.id, frame: frame)

        case .error(let error):
            deliverResponseToExtension(id: error.id, frame: frame)

        case .request(let request):
            forwardServerRequestToExtension(request: request, frame: frame)

        case .event:
            // Events are fire-and-forget — broadcast to all connected extensions
            if let dict = Self.dictionaryFromFrame(frame) {
                localBridgeServer?.broadcast(message: dict)
            }

        case .cancel:
            // Forward cancel to extension as a broadcast
            if let dict = Self.dictionaryFromFrame(frame) {
                localBridgeServer?.broadcast(message: dict)
            }

        case .register:
            // Registration frames are only sent, not received
            break
        }
    }

    /// Deliver a server response/error to the waiting extension request
    private func deliverResponseToExtension(id: UInt32, frame: BrowserBridge_Frame) {
        let idString = "\(id)"

        pendingExtensionRequestsLock.lock()
        let completion = pendingExtensionRequests.removeValue(forKey: idString)
        pendingExtensionRequestsLock.unlock()

        guard let completion else {
            logger.debug("No pending extension request for response id=\(idString)")
            return
        }

        guard let dict = Self.dictionaryFromFrame(frame) else {
            logger.error("Failed to convert server response to dictionary")
            completion(["kind": ["Error": ["message": "Failed to convert response"]]])
            return
        }

        logger.debug("Delivering server response to extension: id=\(idString)")
        completion(dict)
    }

    /// Forward a server-initiated request to the Safari extension
    private func forwardServerRequestToExtension(request: BrowserBridge_RequestFrame, frame: BrowserBridge_Frame) {
        let requestIdString = "\(request.id)"
        let action = request.action

        logger.debug("Received request from gRPC server: action=\(action), id=\(requestIdString)")

        // Store pending request so we can match the extension's response
        pendingServerRequestsLock.lock()
        pendingServerRequests[requestIdString] = [
            "id": Int(request.id),
            "action": action
        ]
        pendingServerRequestsLock.unlock()

        // Try to dispatch to extension via SFSafariApplication
        guard let dict = Self.dictionaryFromFrame(frame) else {
            logger.error("Failed to convert request to dictionary for dispatch")
            sendErrorResponseToServer(requestId: requestIdString, action: action, error: "Frame conversion failed")
            return
        }

        do {
            let jsonData = try JSONSerialization.data(withJSONObject: dict, options: [])
            guard let jsonString = String(data: jsonData, encoding: .utf8) else {
                sendErrorResponseToServer(requestId: requestIdString, action: action, error: "JSON encoding failed")
                return
            }

            let userInfo: [String: Any] = [
                "frame": dict,
                "frameJson": jsonString,
                "action": action,
                "requestId": requestIdString
            ]

            SFSafariApplication.dispatchMessage(
                withName: "NativeRequest",
                toExtensionWithIdentifier: extensionBundleIdentifier,
                userInfo: userInfo
            ) { [weak self] error in
                if let error {
                    self?.logger.error("Failed to dispatch to extension: \(error.localizedDescription)")
                    self?.sendErrorResponseToServer(requestId: requestIdString, action: action, error: error.localizedDescription)
                }
            }
        } catch {
            logger.error("Failed to serialize frame for dispatch: \(error.localizedDescription)")
            sendErrorResponseToServer(requestId: requestIdString, action: action, error: error.localizedDescription)
        }
    }

    /// Send an error response back to the gRPC server when extension dispatch fails
    private func sendErrorResponseToServer(requestId: String, action: String, error: String) {
        pendingServerRequestsLock.lock()
        pendingServerRequests.removeValue(forKey: requestId)
        pendingServerRequestsLock.unlock()

        let idValue: UInt32 = UInt32(requestId) ?? 0

        let payloadDict: [String: Any] = ["kind": "Error", "data": error]
        guard let payloadData = try? JSONSerialization.data(withJSONObject: payloadDict, options: []),
              let payload = String(data: payloadData, encoding: .utf8) else {
            logger.error("Failed to encode error payload")
            return
        }

        var responseFrame = BrowserBridge_ResponseFrame()
        responseFrame.id = idValue
        responseFrame.action = action
        responseFrame.payload = payload

        var frame = BrowserBridge_Frame()
        frame.response = responseFrame

        grpcClient?.send(frame: frame)
    }

    // MARK: - Frame ↔ Dictionary Conversion

    /// Convert a JSON dictionary to a protobuf Frame
    static func frameFromDictionary(_ dict: [String: Any]) -> BrowserBridge_Frame? {
        guard let kind = dict["kind"] as? [String: Any] else {
            return nil
        }

        var frame = BrowserBridge_Frame()

        if let request = kind["Request"] as? [String: Any] {
            var rf = BrowserBridge_RequestFrame()
            if let id = request["id"] as? Int { rf.id = UInt32(id) }
            if let action = request["action"] as? String { rf.action = action }
            if let payload = request["payload"] as? String { rf.payload = payload }
            frame.request = rf

        } else if let response = kind["Response"] as? [String: Any] {
            var rf = BrowserBridge_ResponseFrame()
            if let id = response["id"] as? Int { rf.id = UInt32(id) }
            if let action = response["action"] as? String { rf.action = action }
            if let payload = response["payload"] as? String { rf.payload = payload }
            frame.response = rf

        } else if let event = kind["Event"] as? [String: Any] {
            var ef = BrowserBridge_EventFrame()
            if let action = event["action"] as? String { ef.action = action }
            if let payload = event["payload"] as? String { ef.payload = payload }
            frame.event = ef

        } else if let error = kind["Error"] as? [String: Any] {
            var ef = BrowserBridge_ErrorFrame()
            if let id = error["id"] as? Int { ef.id = UInt32(id) }
            if let code = error["code"] as? Int { ef.code = UInt32(code) }
            if let message = error["message"] as? String { ef.message = message }
            if let details = error["details"] as? String { ef.details = details }
            frame.error = ef

        } else if let cancel = kind["Cancel"] as? [String: Any] {
            var cf = BrowserBridge_CancelFrame()
            if let id = cancel["id"] as? Int { cf.id = UInt32(id) }
            frame.cancel = cf

        } else if let register = kind["Register"] as? [String: Any] {
            var rf = BrowserBridge_RegisterFrame()
            if let hostPid = register["host_pid"] as? Int { rf.hostPid = UInt32(hostPid) }
            if let browserPid = register["browser_pid"] as? Int { rf.browserPid = UInt32(browserPid) }
            frame.register = rf

        } else {
            return nil
        }

        return frame
    }

    /// Convert a protobuf Frame to a JSON dictionary
    static func dictionaryFromFrame(_ frame: BrowserBridge_Frame) -> [String: Any]? {
        guard let frameKind = frame.kind else { return nil }

        var kind: [String: Any] = [:]

        switch frameKind {
        case .request(let r):
            var d: [String: Any] = ["id": Int(r.id), "action": r.action]
            if r.hasPayload { d["payload"] = r.payload }
            kind["Request"] = d

        case .response(let r):
            var d: [String: Any] = ["id": Int(r.id), "action": r.action]
            if r.hasPayload { d["payload"] = r.payload }
            kind["Response"] = d

        case .event(let e):
            var d: [String: Any] = ["action": e.action]
            if e.hasPayload { d["payload"] = e.payload }
            kind["Event"] = d

        case .error(let e):
            var d: [String: Any] = [
                "id": Int(e.id),
                "code": Int(e.code),
                "message": e.message
            ]
            if e.hasDetails { d["details"] = e.details }
            kind["Error"] = d

        case .cancel(let c):
            kind["Cancel"] = ["id": Int(c.id)]

        case .register:
            return nil
        }

        return ["kind": kind]
    }
}
