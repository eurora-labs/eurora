import Cocoa
import SafariServices
import ServiceManagement
import os.log

@main
@available(macOS 13.0, *)
class AppDelegate: NSObject, NSApplicationDelegate, BridgeWebSocketClientDelegate,
    LocalBridgeServerDelegate
{
    private let logger = Logger(subsystem: "com.eurora.macos", category: "AppDelegate")
    private let requestTimeoutSeconds: TimeInterval = 30
    private let pendingServerRequestTTL: TimeInterval = 10

    private let extensionBundleIdentifier = "com.eurora-labs.eurora.macos.extension"
    private let desktopBundleIdentifiers = [
        "com.eurora-labs.eurora", "com.eurora-labs.eurora.nightly",
    ]
    private let safariBundleIdentifiers = [
        "com.apple.Safari", "com.apple.SafariTechnologyPreview",
    ]

    private var bridgeClient: BridgeWebSocketClient?
    private var localBridgeServer: LocalBridgeServer?

    /// Requests we've sent to the desktop on behalf of the Safari extension,
    /// awaiting a `Response`/`Error` reply. Keyed by request id.
    private let extensionRequestsLock = NSLock()
    private var extensionRequests: [UInt32: (Frame) -> Void] = [:]

    /// Requests pushed by the desktop that haven't yet been picked up by the
    /// Safari extension via `POLL_REQUESTS`. Keyed by request id.
    private let serverRequestsLock = NSLock()
    private var serverRequests: [UInt32: PendingServerRequest] = [:]

    private struct PendingServerRequest {
        let frame: Frame
        let storedAt: Date
    }

    func applicationDidFinishLaunching(_ notification: Notification) {
        logger.info("Eurora launcher starting")

        #if !DEBUG
            registerAsLoginItem()
        #endif

        launchEuroraDesktop()
        observeWorkspaceAppLifecycle()

        let server = LocalBridgeServer()
        server.delegate = self
        server.start()
        self.localBridgeServer = server

        let hostPid = UInt32(getpid())
        let appPid = findSafariPid().map { UInt32($0) } ?? 0
        logger.info("Starting bridge WebSocket client: host=\(hostPid, privacy: .public), app=\(appPid, privacy: .public)")
        let client = BridgeWebSocketClient(hostPid: hostPid, appPid: appPid)
        client.delegate = self
        client.start()
        self.bridgeClient = client
    }

    func applicationWillTerminate(_ notification: Notification) {
        bridgeClient?.stop()
        bridgeClient = nil
        localBridgeServer?.stop()
        localBridgeServer = nil
    }

    func applicationShouldTerminateAfterLastWindowClosed(_ sender: NSApplication) -> Bool { false }

    // MARK: - Login item

    #if !DEBUG
    private func registerAsLoginItem() {
        let service = SMAppService.mainApp
        switch service.status {
        case .enabled:
            logger.debug("Already registered as login item")
        case .notRegistered, .notFound:
            do {
                try service.register()
                logger.info("Registered as login item")
            } catch {
                logger.error("Failed to register as login item: \(error.localizedDescription, privacy: .public)")
            }
        case .requiresApproval:
            logger.info("Login item requires user approval in System Settings")
        @unknown default:
            logger.warning("Unknown login item status")
        }
    }
    #endif

    // MARK: - App lifecycle observation

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
            let app = notification.userInfo?[NSWorkspace.applicationUserInfoKey] as? NSRunningApplication,
            let bundleId = app.bundleIdentifier
        else { return }

        if desktopBundleIdentifiers.contains(bundleId) {
            logger.info("Eurora terminated, shutting down launcher")
            NSApplication.shared.terminate(nil)
        } else if safariBundleIdentifiers.contains(bundleId) {
            logger.info("Safari terminated (was PID \(app.processIdentifier, privacy: .public)), clearing app PID")
            bridgeClient?.updateAppPid(0)
        }
    }

    @objc private func workspaceAppDidLaunch(_ notification: Notification) {
        guard
            let app = notification.userInfo?[NSWorkspace.applicationUserInfoKey] as? NSRunningApplication,
            let bundleId = app.bundleIdentifier,
            safariBundleIdentifiers.contains(bundleId)
        else { return }

        let pid = UInt32(app.processIdentifier)
        logger.info("Safari launched (PID: \(pid, privacy: .public)), updating app PID")
        bridgeClient?.updateAppPid(pid)
    }

    // MARK: - Tauri desktop launch

    private func launchEuroraDesktop() {
        guard let resourceURL = Bundle.main.resourceURL else {
            logger.error("Could not locate app Resources directory")
            return
        }

        let desktopAppURL: URL? = {
            guard let contents = try? FileManager.default.contentsOfDirectory(
                at: resourceURL, includingPropertiesForKeys: nil
            ) else { return nil }
            return contents.first { url in
                guard url.pathExtension == "app",
                      let bundle = Bundle(url: url),
                      let bundleId = bundle.bundleIdentifier
                else { return false }
                return self.desktopBundleIdentifiers.contains(bundleId)
            }
        }()

        guard let desktopAppURL else {
            logger.error("No embedded desktop app found in Resources")
            return
        }
        logger.info("Found embedded desktop app: \(desktopAppURL.lastPathComponent, privacy: .public)")

        let config = NSWorkspace.OpenConfiguration()
        config.activates = true
        NSWorkspace.shared.openApplication(at: desktopAppURL, configuration: config) { [weak self] app, error in
            if let error {
                self?.logger.error("Failed to launch Eurora: \(error.localizedDescription, privacy: .public)")
            } else {
                self?.logger.info("Eurora launched successfully (PID: \(app?.processIdentifier ?? 0, privacy: .public))")
            }
        }
    }

    private func findSafariPid() -> pid_t? {
        NSWorkspace.shared.runningApplications.first {
            safariBundleIdentifiers.contains($0.bundleIdentifier ?? "")
        }?.processIdentifier
    }

    // MARK: - BridgeWebSocketClientDelegate

    func bridgeWebSocketClientDidConnect(_ client: BridgeWebSocketClient) {
        logger.info("Connected to desktop bridge")
    }

    func bridgeWebSocketClientDidDisconnect(_ client: BridgeWebSocketClient, error: Error?) {
        if let error {
            logger.warning("Disconnected from desktop bridge: \(error.localizedDescription, privacy: .public)")
        } else {
            logger.info("Disconnected from desktop bridge")
        }

        // Drain pending extension requests — the server can no longer respond.
        let pending = drainExtensionRequests()
        if !pending.isEmpty {
            logger.info("Draining \(pending.count, privacy: .public) pending extension request(s) due to disconnect")
            let errFrame = Frame(ErrorFrame(id: 0, message: "Bridge client disconnected"))
            for (_, completion) in pending {
                completion(errFrame)
            }
        }
    }

    func bridgeWebSocketClient(_ client: BridgeWebSocketClient, didReceive frame: Frame) {
        switch frame.kind {
        case .response(let r): deliverToExtensionRequest(id: r.id, frame: frame)
        case .error(let e): deliverToExtensionRequest(id: e.id, frame: frame)
        case .request(let r): storeServerRequest(id: r.id, frame: frame)
        case .event, .cancel: localBridgeServer?.broadcast(frame: frame)
        case .register: break
        }
    }

    // MARK: - LocalBridgeServerDelegate

    func localBridgeServer(
        _ server: LocalBridgeServer,
        didReceive frame: Frame,
        completion: @escaping (Frame?) -> Void
    ) {
        DispatchQueue.main.async { [weak self] in
            guard let self else { return }
            self.routeFromExtension(frame: frame, completion: completion)
        }
    }

    private func routeFromExtension(frame: Frame, completion: @escaping (Frame?) -> Void) {
        switch frame.kind {
        case .response(let r):
            // Extension is replying to a server-pushed request. Drop the
            // entry from our pending-server map (if it's still there) and
            // forward to the desktop.
            removeServerRequest(id: r.id)
            sendToBridge(frame)
            completion(Frame(ResponseFrame(id: r.id, action: r.action, payload: "{\"status\":\"forwarded\"}")))

        case .error(let e):
            removeServerRequest(id: e.id)
            sendToBridge(frame)
            completion(nil)

        case .request(let r) where r.action == "POLL_REQUESTS":
            handlePollRequests(requestId: r.id, completion: completion)

        case .request(let r):
            forwardExtensionRequest(request: r, completion: completion)

        case .event, .cancel:
            sendToBridge(frame)
            completion(nil)

        case .register:
            logger.warning("Ignoring Register from extension")
            completion(Frame(ErrorFrame(id: 0, message: "Register frames are not accepted from the extension")))
        }
    }

    // MARK: - Extension → desktop request forwarding

    private func forwardExtensionRequest(
        request: RequestFrame,
        completion: @escaping (Frame?) -> Void
    ) {
        guard let bridgeClient, bridgeClient.isConnected else {
            completion(Frame(ErrorFrame(id: request.id, message: "Bridge client not connected")))
            return
        }

        let id = request.id
        extensionRequestsLock.lock()
        extensionRequests[id] = completion
        extensionRequestsLock.unlock()

        let timeout = requestTimeoutSeconds
        DispatchQueue.main.asyncAfter(deadline: .now() + timeout) { [weak self] in
            guard let self else { return }
            self.extensionRequestsLock.lock()
            let timedOut = self.extensionRequests.removeValue(forKey: id)
            self.extensionRequestsLock.unlock()
            if let timedOut {
                self.logger.warning("Extension request \(id, privacy: .public) timed out after \(timeout, privacy: .public)s")
                timedOut(Frame(ErrorFrame(id: id, message: "Request timed out")))
            }
        }

        sendToBridge(Frame(request))
    }

    private func deliverToExtensionRequest(id: UInt32, frame: Frame) {
        extensionRequestsLock.lock()
        let completion = extensionRequests.removeValue(forKey: id)
        extensionRequestsLock.unlock()
        completion?(frame)
    }

    private func drainExtensionRequests() -> [(UInt32, (Frame) -> Void)] {
        extensionRequestsLock.lock()
        let snapshot = extensionRequests.map { ($0.key, $0.value) }
        extensionRequests.removeAll()
        extensionRequestsLock.unlock()
        return snapshot
    }

    // MARK: - Desktop → extension request queueing

    private func storeServerRequest(id: UInt32, frame: Frame) {
        serverRequestsLock.lock()
        serverRequests[id] = PendingServerRequest(frame: frame, storedAt: Date())
        serverRequestsLock.unlock()
    }

    private func removeServerRequest(id: UInt32) {
        serverRequestsLock.lock()
        serverRequests.removeValue(forKey: id)
        serverRequestsLock.unlock()
    }

    private func handlePollRequests(requestId: UInt32, completion: @escaping (Frame?) -> Void) {
        let cutoff = Date().addingTimeInterval(-pendingServerRequestTTL)

        serverRequestsLock.lock()
        serverRequests = serverRequests.filter { $0.value.storedAt > cutoff }
        let frames = serverRequests.values.map(\.frame)
        serverRequests.removeAll()
        serverRequestsLock.unlock()

        let payload: String
        do {
            let data = try JSONEncoder().encode(frames)
            payload = String(data: data, encoding: .utf8) ?? "[]"
        } catch {
            logger.error("Failed to encode polled requests: \(error.localizedDescription, privacy: .public)")
            payload = "[]"
        }

        completion(Frame(ResponseFrame(id: requestId, action: "POLL_REQUESTS", payload: payload)))
    }

    private func sendToBridge(_ frame: Frame) {
        bridgeClient?.send(frame: frame)
    }
}
