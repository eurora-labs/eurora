import Cocoa
import SafariServices
import ServiceManagement
import os.log

@main
@available(macOS 15.0, *)
class AppDelegate: NSObject, NSApplicationDelegate, BrowserBridgeClientDelegate,
    LocalBridgeServerDelegate
{
    private let logger = Logger(subsystem: "com.eurora.macos", category: "AppDelegate")
    private let requestTimeoutSeconds: TimeInterval = 30
    private let extensionBundleIdentifier = "com.eurora-labs.eurora.macos.extension"
    private let desktopBundleIdentifiers = [
        "com.eurora-labs.eurora", "com.eurora-labs.eurora.nightly",
    ]
    private let safariBundleIdentifiers = [
        "com.apple.Safari", "com.apple.SafariTechnologyPreview",
    ]
    private var grpcClient: BrowserBridgeClient?
    private var localBridgeServer: LocalBridgeServer?
    private var pendingExtensionRequests: [String: ([String: Any]?) -> Void] = [:]
    private let pendingExtensionRequestsLock = NSLock()
    private var pendingServerRequests: [String: [String: Any]] = [:]
    private let pendingServerRequestsLock = NSLock()

    // Extension requests that arrived while the gRPC client was disconnected.
    // They are forwarded as soon as the gRPC stream comes back up. Bounded
    // so a permanently-down Tauri server cannot grow this without limit.
    private struct PendingForwardRequest {
        let message: [String: Any]
        let deadline: Date
        let completion: ([String: Any]?) -> Void
    }
    private var pendingForwardRequests: [PendingForwardRequest] = []
    private let pendingForwardRequestsLock = NSLock()
    private let maxPendingForwardRequests = 256
    private let launchdAgentPlistName = "com.eurora-labs.eurora.launcher.plist"

    func applicationDidFinishLaunching(_ notification: Notification) {
        logger.info("Eurora launcher starting")

        // Start the bridge listener first — it is the resource the Safari
        // extension is waiting on. Everything else (launchd registration,
        // observers, the gRPC client, launching Eurora) can run after.
        let server = LocalBridgeServer()
        server.delegate = self
        server.start()
        self.localBridgeServer = server

        let hostPid = UInt32(getpid())
        let browserPid = findSafariPid().map { UInt32($0) } ?? 0
        logger.info("Starting gRPC client: host=\(hostPid), browser=\(browserPid)")
        let client = BrowserBridgeClient(hostPid: hostPid, browserPid: browserPid)
        client.delegate = self
        client.connect()
        self.grpcClient = client

        // Observe Eurora and Safari lifecycle so we can keep the browser
        // PID current and log Eurora terminations.
        observeWorkspaceAppLifecycle()

        // Register as a launchd user agent. launchd both starts the launcher
        // at login (RunAtLoad) and respawns it on crash (KeepAlive), giving
        // us a single source of truth for the "always running" property the
        // Safari extension relies on. Registration is idempotent.
        #if !DEBUG
            registerLaunchdAgent()
        #endif

        launchEuroraDesktop()
    }

    func applicationWillTerminate(_ notification: Notification) {
        grpcClient?.disconnect()
        grpcClient = nil
        localBridgeServer?.stop()
        localBridgeServer = nil
        drainPendingForwardRequests(reason: "launcher shutting down")
    }

    func applicationShouldTerminateAfterLastWindowClosed(_ sender: NSApplication) -> Bool { false }

    // MARK: - Launchd Agent Registration

    #if !DEBUG
        private func registerLaunchdAgent() {
            let service = SMAppService.agent(plistName: launchdAgentPlistName)
            let initialStatus = self.describe(status: service.status)
            logger.info("Launchd agent initial status: \(initialStatus, privacy: .public)")

            switch service.status {
            case .enabled:
                logger.info(
                    "Launchd agent already enabled — launchd should keep launcher alive at login"
                )
            case .notRegistered, .notFound:
                do {
                    try service.register()
                    let postStatus = self.describe(status: service.status)
                    logger.info(
                        "Launchd agent registered (post-register status: \(postStatus, privacy: .public))"
                    )
                } catch {
                    logger.error(
                        "Failed to register launchd agent: \(error.localizedDescription, privacy: .public)"
                    )
                }
            case .requiresApproval:
                logger.warning(
                    "Launchd agent requires user approval in System Settings → General → Login Items & Extensions → Allow in the Background"
                )
            @unknown default:
                logger.warning(
                    "Unknown launchd agent status: \(initialStatus, privacy: .public)"
                )
            }
        }

        private func describe(status: SMAppService.Status) -> String {
            switch status {
            case .notRegistered: return "notRegistered"
            case .enabled: return "enabled"
            case .requiresApproval: return "requiresApproval"
            case .notFound: return "notFound"
            @unknown default: return "unknown(\(status.rawValue))"
            }
        }
    #endif

    // MARK: - App Lifecycle Observation

    private func observeWorkspaceAppLifecycle() {
        let center = NSWorkspace.shared.notificationCenter
        center.addObserver(
            self, selector: #selector(workspaceAppDidTerminate(_:)),
            name: NSWorkspace.didTerminateApplicationNotification, object: nil
        )
        center.addObserver(
            self, selector: #selector(workspaceAppDidLaunch(_:)),
            name: NSWorkspace.didLaunchApplicationNotification, object: nil
        )
    }

    @objc private func workspaceAppDidTerminate(_ notification: Notification) {
        guard
            let app = notification.userInfo?[NSWorkspace.applicationUserInfoKey]
                as? NSRunningApplication,
            let bundleId = app.bundleIdentifier
        else { return }

        if desktopBundleIdentifiers.contains(bundleId) {
            // The launcher is the always-on bridge between Safari and the
            // desktop app; it must outlive Eurora so the bridge is ready
            // when Eurora is relaunched. The gRPC client's connect loop
            // continues retrying in the background.
            logger.info("Eurora terminated; launcher staying alive to bridge future sessions")
        } else if safariBundleIdentifiers.contains(bundleId) {
            logger.info(
                "Safari terminated (was PID \(app.processIdentifier)), clearing browser PID")
            grpcClient?.updateBrowserPid(0)
        }
    }

    @objc private func workspaceAppDidLaunch(_ notification: Notification) {
        guard
            let app = notification.userInfo?[NSWorkspace.applicationUserInfoKey]
                as? NSRunningApplication,
            let bundleId = app.bundleIdentifier,
            safariBundleIdentifiers.contains(bundleId)
        else { return }

        let pid = UInt32(app.processIdentifier)
        logger.info("Safari launched (PID: \(pid)), updating browser PID")
        grpcClient?.updateBrowserPid(pid)
    }

    // MARK: - Tauri Desktop App Lifecycle

    private func launchEuroraDesktop() {
        // Short-circuit when Eurora is already up (session restore, or the
        // launcher being relaunched while the desktop app is still alive).
        // NSWorkspace.openApplication on a running app is a semantic no-op,
        // but it still does the bundle resolution work synchronously.
        if let running = NSWorkspace.shared.runningApplications.first(where: {
            guard let bundleId = $0.bundleIdentifier else { return false }
            return desktopBundleIdentifiers.contains(bundleId)
        }) {
            logger.info(
                "Eurora already running (PID \(running.processIdentifier)); skipping launch")
            return
        }

        guard let resourceURL = Bundle.main.resourceURL else {
            logger.error("Could not locate app Resources directory")
            return
        }

        // Discover the embedded Tauri app dynamically — the product name
        // differs between release ("Eurora.app") and nightly
        // ("Eurora Nightly.app"), so we scan Resources for any .app
        // whose bundle identifier matches a known desktop build.
        let desktopAppURL: URL? = {
            guard
                let contents = try? FileManager.default.contentsOfDirectory(
                    at: resourceURL, includingPropertiesForKeys: nil)
            else { return nil }
            return contents.first { url in
                guard url.pathExtension == "app" else { return false }
                guard let bundle = Bundle(url: url),
                    let bundleId = bundle.bundleIdentifier
                else { return false }
                return self.desktopBundleIdentifiers.contains(bundleId)
            }
        }()

        guard let desktopAppURL else {
            logger.error(
                "No embedded desktop app found in Resources matching \(self.desktopBundleIdentifiers)"
            )
            return
        }
        logger.info("Found embedded desktop app: \(desktopAppURL.lastPathComponent)")

        let config = NSWorkspace.OpenConfiguration()
        config.activates = true

        NSWorkspace.shared.openApplication(at: desktopAppURL, configuration: config) {
            [weak self] app, error in
            if let error = error {
                self?.logger.error("Failed to launch Eurora: \(error.localizedDescription)")
            } else {
                self?.logger.info(
                    "Eurora launched successfully (PID: \(app?.processIdentifier ?? 0))")
            }
        }
    }

    // MARK: - Safari PID Detection

    private func findSafariPid() -> pid_t? {
        return NSWorkspace.shared.runningApplications.first {
            safariBundleIdentifiers.contains($0.bundleIdentifier ?? "")
        }?.processIdentifier
    }

    // MARK: - BrowserBridgeClientDelegate

    func browserBridgeClientDidConnect(_ client: BrowserBridgeClient) {
        logger.info("Connected to gRPC server")
        flushPendingForwardRequests()
    }
    func browserBridgeClientDidDisconnect(_ client: BrowserBridgeClient, error: Error?) {
        if let error {
            logger.warning("Disconnected: \(error.localizedDescription)")
        } else {
            logger.info("Disconnected from gRPC server")
        }

        // Fail in-flight requests — they were on a now-dead stream and the
        // server has no record of them. Queued (not-yet-forwarded) requests
        // in `pendingForwardRequests` are intentionally left alone so they
        // ride out the reconnect cycle.
        pendingExtensionRequestsLock.lock()
        let pending = pendingExtensionRequests
        pendingExtensionRequests.removeAll()
        pendingExtensionRequestsLock.unlock()

        if !pending.isEmpty {
            logger.info("Draining \(pending.count) pending extension request(s) due to disconnect")
            let errDict: [String: Any] = [
                "kind": ["Error": ["message": "gRPC client disconnected"]]
            ]
            for (_, completion) in pending {
                completion(errDict)
            }
        }
    }
    func browserBridgeClient(
        _ client: BrowserBridgeClient, didReceiveFrame frame: BrowserBridge_Frame
    ) {
        handleFrameFromServer(frame)
    }

    // MARK: - LocalBridgeServerDelegate

    func localBridgeServer(
        _ server: LocalBridgeServer, didReceiveMessage message: [String: Any],
        completion: @escaping ([String: Any]?) -> Void
    ) {
        // The delegate is called on LocalBridgeServer's background queue.
        // Dispatch to main so all grpcClient access is serialized.
        DispatchQueue.main.async { [weak self] in
            guard let self else { return }
            if let kind = message["kind"] as? [String: Any],
                let resp = kind["Response"] as? [String: Any], let rid = resp["id"]
            {
                let idStr = "\(rid)"
                self.pendingServerRequestsLock.lock()
                let had = self.pendingServerRequests.removeValue(forKey: idStr) != nil
                self.pendingServerRequestsLock.unlock()
                if had {
                    self.sendDictToServer(message)
                    completion(["status": "forwarded"])
                    return
                }
            }
            if let kind = message["kind"] as? [String: Any],
                let req = kind["Request"] as? [String: Any],
                let action = req["action"] as? String,
                action == "POLL_REQUESTS"
            {
                self.handlePollRequests(completion: completion)
                return
            }

            self.forwardExtRequest(message, completion: completion)
        }
    }

    private func forwardExtRequest(
        _ message: [String: Any], completion: @escaping ([String: Any]?) -> Void
    ) {
        if let client = grpcClient, client.isConnected {
            sendForwardableMessage(message, completion: completion)
            return
        }

        // gRPC isn't connected. Events are fire-and-forget and stale within
        // seconds, so dropping them is preferable to queuing indefinitely.
        // Request frames are buffered with a deadline; they're forwarded as
        // soon as `browserBridgeClientDidConnect` fires.
        if isEventMessage(message) {
            logger.debug("Dropping event while gRPC client offline")
            completion(["status": "dropped"])
            return
        }

        enqueuePendingForwardRequest(message, completion: completion)
    }

    /// Forward a message on a known-good gRPC stream, registering its
    /// completion in `pendingExtensionRequests` and arming the per-request
    /// timeout. Caller must have verified that `grpcClient.isConnected`.
    private func sendForwardableMessage(
        _ message: [String: Any], completion: @escaping ([String: Any]?) -> Void
    ) {
        let reqId = extractRequestId(from: message)
        if let reqId {
            pendingExtensionRequestsLock.lock()
            pendingExtensionRequests[reqId] = completion
            pendingExtensionRequestsLock.unlock()

            // Schedule a timeout so the Safari extension is not left hanging
            // if the gRPC server never responds.
            let timeout = requestTimeoutSeconds
            DispatchQueue.main.asyncAfter(deadline: .now() + timeout) { [weak self] in
                guard let self else { return }
                self.pendingExtensionRequestsLock.lock()
                let timedOut = self.pendingExtensionRequests.removeValue(forKey: reqId)
                self.pendingExtensionRequestsLock.unlock()
                if let timedOut {
                    self.logger.warning("Request \(reqId) timed out after \(timeout)s")
                    timedOut(["kind": ["Error": ["message": "Request timed out"]]])
                }
            }
        }
        sendDictToServer(message)
        if reqId == nil { completion(["status": "ok"]) }
    }

    private func extractRequestId(from message: [String: Any]) -> String? {
        guard let kind = message["kind"] as? [String: Any],
              let req = kind["Request"] as? [String: Any],
              let id = req["id"] else { return nil }
        return "\(id)"
    }

    private func isEventMessage(_ message: [String: Any]) -> Bool {
        guard let kind = message["kind"] as? [String: Any] else { return false }
        return kind["Event"] != nil
    }

    // MARK: - Pending Forward Queue

    private func enqueuePendingForwardRequest(
        _ message: [String: Any], completion: @escaping ([String: Any]?) -> Void
    ) {
        let entry = PendingForwardRequest(
            message: message,
            deadline: Date().addingTimeInterval(requestTimeoutSeconds),
            completion: completion
        )

        var dropped: PendingForwardRequest?
        pendingForwardRequestsLock.lock()
        if pendingForwardRequests.count >= maxPendingForwardRequests {
            // Bound the queue. Dropping the oldest is the right policy: it
            // has been waiting longest and is closest to its deadline.
            dropped = pendingForwardRequests.removeFirst()
        }
        pendingForwardRequests.append(entry)
        pendingForwardRequestsLock.unlock()

        if let dropped {
            logger.warning("Pending forward queue full; dropped oldest entry")
            dropped.completion(["kind": ["Error": ["message": "Forward queue overflow"]]])
        }

        scheduleForwardQueueDeadlineCheck()
    }

    private func scheduleForwardQueueDeadlineCheck() {
        pendingForwardRequestsLock.lock()
        let nextDeadline = pendingForwardRequests.map { $0.deadline }.min()
        pendingForwardRequestsLock.unlock()
        guard let nextDeadline else { return }

        let delay = max(0, nextDeadline.timeIntervalSinceNow)
        DispatchQueue.main.asyncAfter(deadline: .now() + delay) { [weak self] in
            self?.failExpiredForwardRequests()
        }
    }

    private func failExpiredForwardRequests() {
        let now = Date()
        var expired: [PendingForwardRequest] = []

        pendingForwardRequestsLock.lock()
        var remaining: [PendingForwardRequest] = []
        for entry in pendingForwardRequests {
            if entry.deadline <= now {
                expired.append(entry)
            } else {
                remaining.append(entry)
            }
        }
        pendingForwardRequests = remaining
        pendingForwardRequestsLock.unlock()

        if !expired.isEmpty {
            logger.warning("Failing \(expired.count) queued forward request(s) past deadline")
            let errDict: [String: Any] = [
                "kind": ["Error": ["message": "Request timed out waiting for desktop app"]]
            ]
            for entry in expired {
                entry.completion(errDict)
            }
        }
    }

    private func flushPendingForwardRequests() {
        pendingForwardRequestsLock.lock()
        let entries = pendingForwardRequests
        pendingForwardRequests.removeAll()
        pendingForwardRequestsLock.unlock()

        guard !entries.isEmpty else { return }
        logger.info("Flushing \(entries.count) queued forward request(s) on reconnect")

        let now = Date()
        for entry in entries {
            if entry.deadline <= now {
                entry.completion(["kind": ["Error": ["message": "Request timed out waiting for desktop app"]]])
                continue
            }
            sendForwardableMessage(entry.message, completion: entry.completion)
        }
    }

    private func drainPendingForwardRequests(reason: String) {
        pendingForwardRequestsLock.lock()
        let entries = pendingForwardRequests
        pendingForwardRequests.removeAll()
        pendingForwardRequestsLock.unlock()

        guard !entries.isEmpty else { return }
        logger.info("Draining \(entries.count) queued forward request(s): \(reason)")
        let errDict: [String: Any] = [
            "kind": ["Error": ["message": reason]]
        ]
        for entry in entries {
            entry.completion(errDict)
        }
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
        case .event, .cancel:
            if let d = Self.dictionaryFromFrame(frame) { localBridgeServer?.broadcast(message: d) }
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
            completion(["kind": ["Error": ["message": "Convert failed"]]])
            return
        }
        completion(dict)
    }

    private func forwardServerReq(request: BrowserBridge_RequestFrame, frame: BrowserBridge_Frame) {
        let reqIdStr = "\(request.id)"
        let action = request.action
        guard let dict = Self.dictionaryFromFrame(frame) else {
            sendErrResp(requestId: reqIdStr, action: action, error: "Frame conversion failed")
            return
        }
        pendingServerRequestsLock.lock()
        pendingServerRequests[reqIdStr] = [
            "frame": dict,
            "storedAt": Date().timeIntervalSince1970,
        ]
        pendingServerRequestsLock.unlock()
    }

    private func handlePollRequests(completion: @escaping ([String: Any]?) -> Void) {
        let now = Date().timeIntervalSince1970
        pendingServerRequestsLock.lock()
        pendingServerRequests = pendingServerRequests.filter {
            guard let storedAt = $0.value["storedAt"] as? TimeInterval else { return false }
            return (now - storedAt) < 10.0
        }
        let requests = pendingServerRequests.values.compactMap { $0["frame"] as? [String: Any] }
        pendingServerRequests.removeAll()
        pendingServerRequestsLock.unlock()

        let payload: String
        if let data = try? JSONSerialization.data(withJSONObject: requests, options: []),
            let str = String(data: data, encoding: .utf8)
        {
            payload = str
        } else {
            payload = "[]"
        }
        completion([
            "kind": [
                "Response": [
                    "id": 0,
                    "action": "POLL_REQUESTS",
                    "payload": payload,
                ]
            ]
        ])
    }

    private func sendErrResp(requestId: String, action: String, error: String) {
        pendingServerRequestsLock.lock()
        pendingServerRequests.removeValue(forKey: requestId)
        pendingServerRequestsLock.unlock()
        let idVal: UInt32 = UInt32(requestId) ?? 0
        var ef = BrowserBridge_ErrorFrame()
        ef.id = idVal
        ef.message = error
        var f = BrowserBridge_Frame()
        f.error = ef
        grpcClient?.send(frame: f)
    }
}

// MARK: - Frame / Dictionary Conversion

@available(macOS 15.0, *)
extension AppDelegate {
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
        } else {
            return nil
        }

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

    static func dictionaryFromFrame(_ frame: BrowserBridge_Frame) -> [String: Any]? {
        guard let frameKind = frame.kind else { return nil }
        guard let kind = kindDictFromFrameKind(frameKind) else { return nil }
        return ["kind": kind]
    }

    private static func kindDictFromFrameKind(_ frameKind: BrowserBridge_Frame.OneOf_Kind)
        -> [String: Any]?
    {
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
        var dict: [String: Any] = [
            "id": Int(err.id), "code": Int(err.code), "message": err.message,
        ]
        if err.hasDetails { dict["details"] = err.details }
        return dict
    }
}
