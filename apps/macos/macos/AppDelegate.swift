import Cocoa
import os.log
import SafariServices
import ServiceManagement

@main
@available(macOS 13.0, *)
class AppDelegate: NSObject, NSApplicationDelegate, BridgeWebSocketClientDelegate, LocalBridgeServerDelegate {
    private let logger = Logger(subsystem: "com.eurora.macos", category: "AppDelegate")

    private let extensionBundleIdentifier = "com.eurora-labs.eurora.macos.extension"
    private let desktopBundleIdentifiers = [
        "com.eurora-labs.eurora",
        "com.eurora-labs.eurora.nightly",
    ]
    private let safariBundleIdentifiers = [
        "com.apple.Safari",
        "com.apple.SafariTechnologyPreview",
    ]

    private var bridgeClient: BridgeWebSocketClient?
    private var localBridgeServer: LocalBridgeServer?
    private var requestRouter: BridgeRequestRouter?

    func applicationDidFinishLaunching(_: Notification) {
        logger.info("Eurora launcher starting")

        #if !DEBUG
            registerAsLoginItem()
        #endif

        launchEuroraDesktop()
        observeWorkspaceAppLifecycle()

        let server = LocalBridgeServer()
        server.delegate = self
        server.start()
        localBridgeServer = server

        let hostPid = UInt32(getpid())
        let appPid = findSafariPid().map { UInt32($0) } ?? 0
        let appKind = "safari"
        logger.info(
            """
            Starting bridge client: host=\(hostPid, privacy: .public), \
            app=\(appPid, privacy: .public), kind=\(appKind, privacy: .public)
            """
        )
        let client = BridgeWebSocketClient(hostPid: hostPid, appPid: appPid, appKind: appKind)
        client.delegate = self
        client.start()
        bridgeClient = client

        // Router holds a strong-ref-by-closure to client; that's fine because
        // we own both and tear them down together in applicationWillTerminate.
        requestRouter = BridgeRequestRouter { [weak client] frame in
            client?.send(frame: frame)
        }
    }

    func applicationWillTerminate(_: Notification) {
        bridgeClient?.stop()
        bridgeClient = nil
        localBridgeServer?.stop()
        localBridgeServer = nil
        requestRouter = nil
    }

    func applicationShouldTerminateAfterLastWindowClosed(_: NSApplication) -> Bool {
        false
    }

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
                    logger.error(
                        "Failed to register as login item: \(error.localizedDescription, privacy: .public)"
                    )
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
            logger.info(
                "Safari terminated (was PID \(app.processIdentifier, privacy: .public)), clearing app PID"
            )
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
                self?.logger.info(
                    "Eurora launched successfully (PID: \(app?.processIdentifier ?? 0, privacy: .public))"
                )
            }
        }
    }

    private func findSafariPid() -> pid_t? {
        NSWorkspace.shared.runningApplications.first {
            safariBundleIdentifiers.contains($0.bundleIdentifier ?? "")
        }?.processIdentifier
    }

    // MARK: - BridgeWebSocketClientDelegate

    func bridgeWebSocketClientDidConnect(_: BridgeWebSocketClient) {
        logger.info("Connected to desktop bridge")
    }

    func bridgeWebSocketClientDidDisconnect(_: BridgeWebSocketClient, error: Error?) {
        if let error {
            logger.warning(
                "Disconnected from desktop bridge: \(error.localizedDescription, privacy: .public)"
            )
        } else {
            logger.info("Disconnected from desktop bridge")
        }

        let drained = requestRouter?.failAllExtensionRequests(reason: "Bridge client disconnected") ?? 0
        if drained > 0 {
            logger.info("Drained \(drained, privacy: .public) pending extension request(s) due to disconnect")
        }
    }

    func bridgeWebSocketClient(_: BridgeWebSocketClient, didReceive frame: Frame) {
        switch frame.kind {
        case let .response(response):
            requestRouter?.deliverToExtensionRequest(id: response.id, frame: frame)
        case let .error(errorFrame):
            requestRouter?.deliverToExtensionRequest(id: errorFrame.id, frame: frame)
        case let .request(request):
            requestRouter?.storeServerRequest(id: request.id, frame: frame)
        case .event, .cancel:
            localBridgeServer?.broadcast(frame: frame)
        case .register:
            break
        }
    }

    // MARK: - LocalBridgeServerDelegate

    func localBridgeServer(
        _: LocalBridgeServer,
        didReceive frame: Frame,
        completion: @escaping (Frame?) -> Void
    ) {
        DispatchQueue.main.async { [weak self] in
            self?.requestRouter?.routeFromExtension(frame: frame, completion: completion)
        }
    }
}
