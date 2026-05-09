import Cocoa
import os.log
import SafariServices
import ServiceManagement

@main
@available(macOS 13.0, *)
class AppDelegate: NSObject, NSApplicationDelegate {
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
    private var extensionMonitor: SafariExtensionMonitor?

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

        let monitor = SafariExtensionMonitor(
            extensionBundleIdentifier: extensionBundleIdentifier,
            safariBundleIdentifiers: safariBundleIdentifiers
        ) { [weak client] state in
            guard let client else { return }
            client.send(frame: makeExtensionStateEvent(state: state))
        }
        extensionMonitor = monitor
    }

    func applicationWillTerminate(_: Notification) {
        extensionMonitor?.stop()
        extensionMonitor = nil
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
}

// MARK: - BridgeWebSocketClientDelegate

@available(macOS 13.0, *)
extension AppDelegate: BridgeWebSocketClientDelegate {
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
            // Some desktop-originated requests target the launcher itself
            // (open Safari settings, etc.) rather than the extension. We
            // intercept those before the router would buffer them for the
            // extension to poll.
            if handleLauncherRequest(request) {
                return
            }
            requestRouter?.storeServerRequest(id: request.id, frame: frame)
        case .event, .cancel:
            localBridgeServer?.broadcast(frame: frame)
        case .register:
            break
        case let .shutdown(shutdown):
            // The Rust bridge sends Shutdown to native-messaging hosts when
            // it has just replaced the messenger binary on disk and wants
            // stale connections to drop. The macOS launcher isn't a
            // native-messaging host (Safari talks to it via the extension),
            // so the frame doesn't apply to us — log it for traceability.
            if let reason = shutdown.reason {
                logger.info("Received Shutdown frame from desktop (reason=\(reason, privacy: .public)); ignoring")
            } else {
                logger.info("Received Shutdown frame from desktop; ignoring")
            }
        }
    }

    /// Handle requests addressed to the launcher itself rather than the
    /// Safari extension. Returns `true` if the request was recognized and
    /// answered (or rejected) here; `false` otherwise so the caller can
    /// fall through to the extension-poll buffer.
    private func handleLauncherRequest(_ request: RequestFrame) -> Bool {
        switch request.action {
        case BridgeAction.openBrowserExtensionSettings:
            openSafariExtensionSettings(requestId: request.id)
            return true
        default:
            return false
        }
    }

    private func openSafariExtensionSettings(requestId: UInt32) {
        SFSafariApplication.showPreferencesForExtension(
            withIdentifier: extensionBundleIdentifier
        ) { [weak self] error in
            guard let self else { return }
            if let error {
                // `showPreferencesForExtension` is documented to deep-link
                // into Safari Settings → Extensions, but in practice it
                // returns SFErrorDomain.noExtensionFound (code 1) for some
                // legitimately-installed Web Extensions depending on macOS
                // version and how Safari indexed the .appex. Fall back to
                // just bringing Safari to the front — the desktop UI already
                // tells the user the next step is "Settings → Extensions".
                logger.warning(
                    """
                    showPreferencesForExtension failed \
                    (\(error.localizedDescription, privacy: .public)); \
                    falling back to launching Safari
                    """
                )
                launchSafari(requestId: requestId, originalError: error)
            } else {
                replyExtensionSettingsSuccess(requestId: requestId)
            }
        }
    }

    private func launchSafari(requestId: UInt32, originalError: Error) {
        let primarySafariBundleId = safariBundleIdentifiers.first ?? "com.apple.Safari"
        guard let safariURL = NSWorkspace.shared.urlForApplication(
            withBundleIdentifier: primarySafariBundleId
        ) else {
            logger.error(
                "Could not locate Safari bundle (\(primarySafariBundleId, privacy: .public))"
            )
            bridgeClient?.send(frame: Frame(ErrorFrame(
                id: requestId, message: originalError.localizedDescription
            )))
            return
        }

        let config = NSWorkspace.OpenConfiguration()
        config.activates = true
        NSWorkspace.shared.openApplication(at: safariURL, configuration: config) { [weak self] _, openError in
            guard let self else { return }
            if let openError {
                logger.error(
                    "Failed to launch Safari: \(openError.localizedDescription, privacy: .public)"
                )
                // Surface the *original* SafariServices error to the desktop
                // — that's the actionable detail; the launch failure is just
                // "we couldn't even fall back."
                bridgeClient?.send(frame: Frame(ErrorFrame(
                    id: requestId, message: originalError.localizedDescription
                )))
            } else {
                replyExtensionSettingsSuccess(requestId: requestId)
            }
        }
    }

    private func replyExtensionSettingsSuccess(requestId: UInt32) {
        bridgeClient?.send(frame: Frame(ResponseFrame(
            id: requestId,
            action: BridgeAction.openBrowserExtensionSettings,
            payload: nil
        )))
    }
}

// MARK: - LocalBridgeServerDelegate

@available(macOS 13.0, *)
extension AppDelegate: LocalBridgeServerDelegate {
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
