//
//  SafariWebExtensionHandler.swift
//  macos Extension
//
//  Handles messages from the Safari web extension and forwards them
//  to the euro-native-messaging bridge.
//

import SafariServices
import os.log

@available(macOS 11.0, *)
class SafariWebExtensionHandler: NSObject, NSExtensionRequestHandling {

    private let logger = Logger(subsystem: "com.eurora.macos.extension", category: "SafariWebExtensionHandler")
    
    // The bridge is started lazily on first request
    private static var bridgeStarted = false
    private static let bridgeLock = NSLock()
    
    private func ensureBridgeStarted() {
        SafariWebExtensionHandler.bridgeLock.lock()
        defer { SafariWebExtensionHandler.bridgeLock.unlock() }
        
        if !SafariWebExtensionHandler.bridgeStarted {
            logger.info("Starting NativeMessagingBridge from extension handler")
            NativeMessagingBridge.shared.start()
            SafariWebExtensionHandler.bridgeStarted = true
        }
    }
    
    func beginRequest(with context: NSExtensionContext) {
        // Ensure the bridge is running (the extension manages its own bridge instance)
        ensureBridgeStarted()
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

        logger.debug("Received message from browser.runtime.sendNativeMessage: \(String(describing: message)) (profile: \(profile?.uuidString ?? "none"))")

        // Forward message to native messaging bridge
        guard let messageDict = message as? [String: Any] else {
            logger.error("Invalid message format - expected dictionary")
            completeWithError(context: context, error: "Invalid message format")
            return
        }
        
        // Forward to bridge and wait for response
        NativeMessagingBridge.shared.sendMessage(messageDict) { [weak self] (result: Result<[String: Any], Error>) in
            switch result {
            case .success(let responseDict):
                self?.completeWithResponse(context: context, response: responseDict)
            case .failure(let error):
                self?.logger.error("Bridge error: \(error.localizedDescription)")
                self?.completeWithError(context: context, error: error.localizedDescription)
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
