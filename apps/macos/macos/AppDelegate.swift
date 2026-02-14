// AppDelegate.swift - Background launcher for the Eurora unified macOS app.
// Launches the embedded Tauri desktop app (EuroraDesktop.app) and bridges
// Safari extension traffic to the Tauri gRPC backend.

import Cocoa
import SafariServices
import os.log

@main
@available(macOS 15.0, *)
class AppDelegate: NSObject, NSApplicationDelegate, BrowserBridgeClientDelegate, LocalBridgeServerDelegate {
    private let logger = Logger(subsystem: "com.eurora.macos", category: "AppDelegate")
    private let extensionBundleIdentifier = "com.eurora-labs.eurora.macos.extension"
    private let desktopBundleIdentifiers = ["com.eurora-labs.eurora", "com.eurora-labs.eurora.nightly"]
    private var grpcClient: BrowserBridgeClient?
    private var localBridgeServer: LocalBridgeServer?
    private var pendingExtensionRequests: [String: ([String: Any]?) -> Void] = [:]
    private let pendingExtensionRequestsLock = NSLock()
    private var pendingServerRequests: [String: [String: Any]] = [:]
    private let pendingServerRequestsLock = NSLock()

    func applicationDidFinishLaunching(_ notification: Notification) {
        logger.info("Eurora launcher starting")

        // Launch the embedded Tauri desktop app
        launchEuroraDesktop()

        // Observe Tauri app termination so we can shut down with it
        observeDesktopAppTermination()

        // Start the local bridge server for Safari extension communication
        let server = LocalBridgeServer()
        server.delegate = self
        server.start()
        self.localBridgeServer = server

        // Connect gRPC client to the Tauri backend
        let hostPid = UInt32(getpid())
        let browserPid = findSafariPid().map { UInt32($0) } ?? 0
        logger.info("Starting gRPC client: host=\(hostPid), browser=\(browserPid)")
        let client = BrowserBridgeClient(hostPid: hostPid, browserPid: browserPid)
        client.delegate = self
        client.connect()
        self.grpcClient = client
    }

    func applicationWillTerminate(_ notification: Notification) {
        grpcClient?.disconnect(); grpcClient = nil
        localBridgeServer?.stop(); localBridgeServer = nil
    }

    func applicationShouldTerminateAfterLastWindowClosed(_ sender: NSApplication) -> Bool { false }

    // MARK: - Tauri Desktop App Lifecycle

    private func launchEuroraDesktop() {
        guard let resourceURL = Bundle.main.resourceURL else {
            logger.error("Could not locate app Resources directory")
            return
        }
        let desktopAppURL = resourceURL.appendingPathComponent("EuroraDesktop.app")

        guard FileManager.default.fileExists(atPath: desktopAppURL.path) else {
            logger.error("EuroraDesktop.app not found at \(desktopAppURL.path)")
            return
        }

        let config = NSWorkspace.OpenConfiguration()
        config.activates = true

        NSWorkspace.shared.openApplication(at: desktopAppURL, configuration: config) { [weak self] app, error in
            if let error = error {
                self?.logger.error("Failed to launch EuroraDesktop: \(error.localizedDescription)")
            } else {
                self?.logger.info("EuroraDesktop launched successfully (PID: \(app?.processIdentifier ?? 0))")
            }
        }
    }

    private func observeDesktopAppTermination() {
        NSWorkspace.shared.notificationCenter.addObserver(
            self, selector: #selector(appDidTerminate(_:)),
            name: NSWorkspace.didTerminateApplicationNotification, object: nil
        )
    }

    @objc private func appDidTerminate(_ notification: Notification) {
        guard let app = notification.userInfo?[NSWorkspace.applicationUserInfoKey] as? NSRunningApplication,
              let bundleId = app.bundleIdentifier,
              desktopBundleIdentifiers.contains(bundleId)
        else { return }
        logger.info("EuroraDesktop terminated, shutting down launcher")
        NSApplication.shared.terminate(nil)
    }

    // MARK: - Safari PID Detection

    private func findSafariPid() -> pid_t? {
        let safariIds = ["com.apple.Safari", "com.apple.SafariTechnologyPreview"]
        return NSWorkspace.shared.runningApplications.first { safariIds.contains($0.bundleIdentifier ?? "") }?.processIdentifier
    }

    // MARK: - BrowserBridgeClientDelegate

    func browserBridgeClientDidConnect(_ client: BrowserBridgeClient) { logger.info("Connected to gRPC server") }
    func browserBridgeClientDidDisconnect(_ client: BrowserBridgeClient, error: Error?) {
        if let error { logger.warning("Disconnected: \(error.localizedDescription)") }
        else { logger.info("Disconnected from gRPC server") }
    }
    func browserBridgeClient(_ client: BrowserBridgeClient, didReceiveFrame frame: BrowserBridge_Frame) {
        handleFrameFromServer(frame)
    }

    // MARK: - LocalBridgeServerDelegate

    func localBridgeServer(_ server: LocalBridgeServer, didReceiveMessage message: [String: Any],
                           completion: @escaping ([String: Any]?) -> Void) {
        if let kind = message["kind"] as? [String: Any], let resp = kind["Response"] as? [String: Any], let rid = resp["id"] {
            let idStr = "\(rid)"
            pendingServerRequestsLock.lock()
            let had = pendingServerRequests.removeValue(forKey: idStr) != nil
            pendingServerRequestsLock.unlock()
            if had { sendDictToServer(message); completion(["status": "forwarded"]); return }
        }
        forwardExtRequest(message, completion: completion)
    }

    private func forwardExtRequest(_ message: [String: Any], completion: @escaping ([String: Any]?) -> Void) {
        guard let client = grpcClient, client.isConnected else {
            completion(["kind": ["Error": ["message": "gRPC client not connected"]]]); return
        }
        var reqId: String?
        if let kind = message["kind"] as? [String: Any], let req = kind["Request"] as? [String: Any], let id = req["id"] {
            reqId = "\(id)"
        }
        if let reqId { pendingExtensionRequestsLock.lock(); pendingExtensionRequests[reqId] = completion; pendingExtensionRequestsLock.unlock() }
        sendDictToServer(message)
        if reqId == nil { completion(nil) }
    }

    private func sendDictToServer(_ dict: [String: Any]) {
        guard let frame = Self.frameFromDictionary(dict) else { return }
        grpcClient?.send(frame: frame)
    }

    private func handleFrameFromServer(_ frame: BrowserBridge_Frame) {
        guard let fk = frame.kind else { return }
        switch fk {
        case .response(let r): deliverResponse(id: r.id, frame: frame)
        case .error(let e): deliverResponse(id: e.id, frame: frame)
        case .request(let r): forwardServerReq(request: r, frame: frame)
        case .event, .cancel: if let d = Self.dictionaryFromFrame(frame) { localBridgeServer?.broadcast(message: d) }
        case .register: break
        }
    }

    private func deliverResponse(id: UInt32, frame: BrowserBridge_Frame) {
        let idStr = "\(id)"
        pendingExtensionRequestsLock.lock()
        let completion = pendingExtensionRequests.removeValue(forKey: idStr)
        pendingExtensionRequestsLock.unlock()
        guard let completion else { return }
        guard let dict = Self.dictionaryFromFrame(frame) else {
            completion(["kind": ["Error": ["message": "Convert failed"]]]); return
        }
        completion(dict)
    }

    private func forwardServerReq(request: BrowserBridge_RequestFrame, frame: BrowserBridge_Frame) {
        let reqIdStr = "\(request.id)", action = request.action
        pendingServerRequestsLock.lock()
        pendingServerRequests[reqIdStr] = ["id": Int(request.id), "action": action]
        pendingServerRequestsLock.unlock()
        guard let dict = Self.dictionaryFromFrame(frame) else {
            sendErrResp(requestId: reqIdStr, action: action, error: "Frame conversion failed"); return
        }
        do {
            let jsonData = try JSONSerialization.data(withJSONObject: dict, options: [])
            guard let jsonStr = String(data: jsonData, encoding: .utf8) else {
                sendErrResp(requestId: reqIdStr, action: action, error: "JSON encoding failed"); return
            }
            let userInfo: [String: Any] = ["frame": dict, "frameJson": jsonStr, "action": action, "requestId": reqIdStr]
            SFSafariApplication.dispatchMessage(withName: "NativeRequest", toExtensionWithIdentifier: extensionBundleIdentifier, userInfo: userInfo) { [weak self] err in
                if let err { self?.sendErrResp(requestId: reqIdStr, action: action, error: err.localizedDescription) }
            }
        } catch { sendErrResp(requestId: reqIdStr, action: action, error: error.localizedDescription) }
    }

    private func sendErrResp(requestId: String, action: String, error: String) {
        pendingServerRequestsLock.lock(); pendingServerRequests.removeValue(forKey: requestId); pendingServerRequestsLock.unlock()
        let idVal: UInt32 = UInt32(requestId) ?? 0
        var ef = BrowserBridge_ErrorFrame(); ef.id = idVal; ef.message = error
        var f = BrowserBridge_Frame(); f.error = ef
        grpcClient?.send(frame: f)
    }
}

// MARK: - Frame ↔ Dictionary Conversion (moved outside class to reduce type body length)

@available(macOS 15.0, *)
extension AppDelegate {
    /// Convert a JSON dictionary to a protobuf Frame
    static func frameFromDictionary(_ dict: [String: Any]) -> BrowserBridge_Frame? {
        guard let kind = dict["kind"] as? [String: Any] else { return nil }
        var frame = BrowserBridge_Frame()

        if let request = kind["Request"] as? [String: Any] {
            frame.request = makeRequestFrame(from: request)
        } else if let response = kind["Response"] as? [String: Any] {
            frame.response = makeResponseFrame(from: response)
        } else if let event = kind["Event"] as? [String: Any] {
            frame.event = makeEventFrame(from: event)
        } else if let error = kind["Error"] as? [String: Any] {
            frame.error = makeErrorFrame(from: error)
        } else if let cancel = kind["Cancel"] as? [String: Any] {
            frame.cancel = makeCancelFrame(from: cancel)
        } else if let register = kind["Register"] as? [String: Any] {
            frame.register = makeRegisterFrame(from: register)
        } else { return nil }

        return frame
    }

    private static func makeRequestFrame(from dict: [String: Any]) -> BrowserBridge_RequestFrame {
        var reqFrame = BrowserBridge_RequestFrame()
        if let identifier = dict["id"] as? Int { reqFrame.id = UInt32(identifier) }
        if let action = dict["action"] as? String { reqFrame.action = action }
        if let payload = dict["payload"] as? String { reqFrame.payload = payload }
        return reqFrame
    }

    private static func makeResponseFrame(from dict: [String: Any]) -> BrowserBridge_ResponseFrame {
        var respFrame = BrowserBridge_ResponseFrame()
        if let identifier = dict["id"] as? Int { respFrame.id = UInt32(identifier) }
        if let action = dict["action"] as? String { respFrame.action = action }
        if let payload = dict["payload"] as? String { respFrame.payload = payload }
        return respFrame
    }

    private static func makeEventFrame(from dict: [String: Any]) -> BrowserBridge_EventFrame {
        var evtFrame = BrowserBridge_EventFrame()
        if let action = dict["action"] as? String { evtFrame.action = action }
        if let payload = dict["payload"] as? String { evtFrame.payload = payload }
        return evtFrame
    }

    private static func makeErrorFrame(from dict: [String: Any]) -> BrowserBridge_ErrorFrame {
        var errFrame = BrowserBridge_ErrorFrame()
        if let identifier = dict["id"] as? Int { errFrame.id = UInt32(identifier) }
        if let code = dict["code"] as? Int { errFrame.code = UInt32(code) }
        if let message = dict["message"] as? String { errFrame.message = message }
        if let details = dict["details"] as? String { errFrame.details = details }
        return errFrame
    }

    private static func makeCancelFrame(from dict: [String: Any]) -> BrowserBridge_CancelFrame {
        var cancelFrame = BrowserBridge_CancelFrame()
        if let identifier = dict["id"] as? Int { cancelFrame.id = UInt32(identifier) }
        return cancelFrame
    }

    private static func makeRegisterFrame(from dict: [String: Any]) -> BrowserBridge_RegisterFrame {
        var regFrame = BrowserBridge_RegisterFrame()
        if let hostPid = dict["host_pid"] as? Int { regFrame.hostPid = UInt32(hostPid) }
        if let browserPid = dict["browser_pid"] as? Int { regFrame.browserPid = UInt32(browserPid) }
        return regFrame
    }

    /// Convert a protobuf Frame to a JSON dictionary
    static func dictionaryFromFrame(_ frame: BrowserBridge_Frame) -> [String: Any]? {
        guard let frameKind = frame.kind else { return nil }
        guard let kind = kindDictFromFrameKind(frameKind) else { return nil }
        return ["kind": kind]
    }

    private static func kindDictFromFrameKind(_ frameKind: BrowserBridge_Frame.OneOf_Kind) -> [String: Any]? {
        switch frameKind {
        case .request(let req): return ["Request": requestDict(from: req)]
        case .response(let resp): return ["Response": responseDict(from: resp)]
        case .event(let evt): return ["Event": eventDict(from: evt)]
        case .error(let err): return ["Error": errorDict(from: err)]
        case .cancel(let cnl): return ["Cancel": ["id": Int(cnl.id)]]
        case .register: return nil
        }
    }

    private static func requestDict(from req: BrowserBridge_RequestFrame) -> [String: Any] {
        var dict: [String: Any] = ["id": Int(req.id), "action": req.action]
        if req.hasPayload { dict["payload"] = req.payload }
        return dict
    }

    private static func responseDict(from resp: BrowserBridge_ResponseFrame) -> [String: Any] {
        var dict: [String: Any] = ["id": Int(resp.id), "action": resp.action]
        if resp.hasPayload { dict["payload"] = resp.payload }
        return dict
    }

    private static func eventDict(from evt: BrowserBridge_EventFrame) -> [String: Any] {
        var dict: [String: Any] = ["action": evt.action]
        if evt.hasPayload { dict["payload"] = evt.payload }
        return dict
    }

    private static func errorDict(from err: BrowserBridge_ErrorFrame) -> [String: Any] {
        var dict: [String: Any] = ["id": Int(err.id), "code": Int(err.code), "message": err.message]
        if err.hasDetails { dict["details"] = err.details }
        return dict
    }
}
