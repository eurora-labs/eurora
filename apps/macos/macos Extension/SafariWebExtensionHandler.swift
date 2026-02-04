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

        logger.debug("Received message from browser.runtime.sendNativeMessage: \(String(describing: message)) (profile: \(profile?.uuidString ?? "none"))")

        // Forward message to native messaging bridge
        guard let messageDict = message as? [String: Any] else {
            logger.error("Invalid message format - expected dictionary")
            completeWithError(context: context, error: "Invalid message format")
            return
        }
        
        // Extract URL from Event frames (TAB_UPDATED, TAB_ACTIVATED) to store for GET_METADATA
        extractAndStoreMetadata(from: messageDict)
        
        // Send to bridge and wait for response
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
    
    /// Extract URL and icon from Event frames and store it in the bridge for GET_METADATA responses
    private func extractAndStoreMetadata(from message: [String: Any]) {
        guard let kind = message["kind"] as? [String: Any] else { return }
        
        // Check for Event frames (TAB_UPDATED, TAB_ACTIVATED)
        if let event = kind["Event"] as? [String: Any],
           let action = event["action"] as? String,
           (action == "TAB_UPDATED" || action == "TAB_ACTIVATED"),
           let payloadString = event["payload"] as? String,
           let payloadData = payloadString.data(using: .utf8),
           let payload = try? JSONSerialization.jsonObject(with: payloadData) as? [String: Any],
           let data = payload["data"] as? [String: Any] {
            
            let url = data["url"] as? String
            let iconBase64 = data["icon_base64"] as? String
            logger.debug("Extracted from \(action) event - URL: \(url ?? "nil"), hasIcon: \(iconBase64 != nil)")
            NativeMessagingBridge.shared.updateCurrentTab(url: url, title: nil, iconBase64: iconBase64)
        }
        
        // Also check for Request frames that might contain URL info
        if let request = kind["Request"] as? [String: Any],
           let payloadString = request["payload"] as? String,
           let payloadData = payloadString.data(using: .utf8),
           let payload = try? JSONSerialization.jsonObject(with: payloadData) as? [String: Any] {
            
            let url = payload["url"] as? String
            let title = payload["title"] as? String
            let iconBase64 = payload["icon_base64"] as? String
            
            if url != nil || title != nil || iconBase64 != nil {
                logger.debug("Extracted from Request payload - URL: \(url ?? "nil"), Title: \(title ?? "nil"), hasIcon: \(iconBase64 != nil)")
                NativeMessagingBridge.shared.updateCurrentTab(url: url, title: title, iconBase64: iconBase64)
            }
        }
    }
}
