import AppKit
import SafariServices
import os.log

@available(macOS 15.0, *)
class SafariWebExtensionHandler: NSObject, NSExtensionRequestHandling {

    private let logger = Logger(subsystem: "com.eurora.macos.extension", category: "SafariWebExtensionHandler")

    func beginRequest(with context: NSExtensionContext) {
        let request = context.inputItems.first as? NSExtensionItem

        let profile: UUID?
        if #available(iOS 17.0, macOS 14.0, *) {
            profile = request?.userInfo?[SFExtensionProfileKey] as? UUID
        } else {
            profile = request?.userInfo?["profile"] as? UUID
        }

        let message: Any?
        if #available(iOS 15.0, macOS 11.0, *) {
            message = request?.userInfo?[SFExtensionMessageKey]
        } else {
            message = request?.userInfo?["message"]
        }

        let profileStr = profile?.uuidString ?? "none"
        let msgDesc = String(describing: message)
        logger.debug("Received native message: \(msgDesc) (profile: \(profileStr))")

        guard let messageDict = message as? [String: Any] else {
            logger.error("Invalid message format — expected dictionary")
            completeWithError(context: context, error: "Invalid message format")
            return
        }

        // Check if this is a response to a pending native request (from gRPC server)
        // rather than a new outbound request from the extension.
        if let kind = messageDict["kind"] as? [String: Any],
           kind["Response"] != nil {
            if NativeMessagingBridge.shared.handleResponseFromExtension(messageDict) {
                logger.debug("Forwarded response to container app")
                completeWithResponse(context: context, response: ["status": "forwarded"])
            } else {
                logger.warning("Received Response frame with no matching pending request")
                completeWithResponse(context: context, response: ["status": "error", "error": "unmatched_response"])
            }
            return
        }

        ensureContainerAppLaunched()

        NativeMessagingBridge.shared.ensureConnected()

        NativeMessagingBridge.shared.sendMessage(messageDict) { [weak self] result in
            switch result {
            case .success(let responseDict):
                self?.completeWithResponse(context: context, response: responseDict)
            case .failure(let error):
                self?.logger.error("Bridge error: \(error.localizedDescription)")
                self?.completeWithError(context: context, error: error.localizedDescription)
            }
        }
    }

    /// Ensure the launcher app is running before talking to the bridge. If
    /// launchd already started it (the common case once the user has
    /// approved the agent in System Settings), the running-process check
    /// short-circuits. If not — first install, manual `launchctl unload`,
    /// or any other reason the agent isn't loaded — fire an explicit
    /// background launch so the bridge has something to connect to.
    /// The bridge buffers the in-flight `sendMessage` call across the
    /// launch, so callers do not need to wait here.
    private func ensureContainerAppLaunched() {
        let containerAppURL = Bundle.main.bundleURL
            .deletingLastPathComponent()  // …/Contents/PlugIns
            .deletingLastPathComponent()  // …/Contents
            .deletingLastPathComponent()  // …/EuroraLauncher.app

        guard let containerBundle = Bundle(url: containerAppURL),
              let containerBundleId = containerBundle.bundleIdentifier else {
            logger.error("Could not resolve container app bundle at \(containerAppURL.path)")
            return
        }

        let alreadyRunning = NSWorkspace.shared.runningApplications.contains { app in
            app.bundleIdentifier == containerBundleId
        }
        if alreadyRunning { return }

        let configuration = NSWorkspace.OpenConfiguration()
        configuration.activates = false
        configuration.addsToRecentItems = false
        configuration.hides = true

        logger.info("Container app not running; launching \(containerBundleId)")
        NSWorkspace.shared.openApplication(
            at: containerAppURL, configuration: configuration
        ) { [weak self] _, error in
            if let error {
                self?.logger.error(
                    "Failed to launch container app: \(error.localizedDescription)")
            } else {
                self?.logger.info("Container app launched on demand")
            }
        }
    }

    private func completeWithResponse(context: NSExtensionContext, response: [String: Any]) {
        let responseItem = NSExtensionItem()

        if #available(iOS 15.0, macOS 11.0, *) {
            responseItem.userInfo = [SFExtensionMessageKey: response]
        } else {
            responseItem.userInfo = ["message": response]
        }

        context.completeRequest(returningItems: [responseItem], completionHandler: nil)
    }

    private func completeWithError(context: NSExtensionContext, error: String) {
        let responseItem = NSExtensionItem()
        let errorResponse: [String: Any] = [
            "kind": [
                "Error": [
                    "message": error
                ]
            ]
        ]

        if #available(iOS 15.0, macOS 11.0, *) {
            responseItem.userInfo = [SFExtensionMessageKey: errorResponse]
        } else {
            responseItem.userInfo = ["message": errorResponse]
        }

        context.completeRequest(returningItems: [responseItem], completionHandler: nil)
    }
}
