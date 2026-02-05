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
        logger.info(">>> beginRequest called")
        
        // Ensure the bridge is running (the extension manages its own bridge instance)
        ensureBridgeStarted()
        let request = context.inputItems.first as? NSExtensionItem
        
        logger.info(">>> Request inputItems count: \(context.inputItems.count)")

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

        logger.info(">>> Received message from JS: \(String(describing: message)) (profile: \(profile?.uuidString ?? "none"))")

        // Forward message to native messaging bridge
        guard let messageDict = message as? [String: Any] else {
            logger.error("Invalid message format - expected dictionary")
            completeWithError(context: context, error: "Invalid message format")
            return
        }
        
        // Check message type
        if let messageType = messageDict["type"] as? String {
            switch messageType {
            case "nativeResponse":
                // Safari extension is sending a response back to euro-native-messaging
                if let response = messageDict["response"] as? [String: Any] {
                    handleNativeResponse(context: context, response: response)
                } else {
                    completeWithError(context: context, error: "Missing response in nativeResponse message")
                }
                return
                
            case "pollNativeRequests":
                // JS is polling for pending requests from euro-native-messaging
                handlePollRequest(context: context)
                return
                
            default:
                break
            }
        }
        
        // Standard message - forward to bridge and wait for response (include pending requests)
        NativeMessagingBridge.shared.sendMessage(messageDict) { [weak self] (result: Result<[String: Any], Error>) in
            // Also get any pending native requests to piggyback on the response
            let pendingRequests = NativeMessagingBridge.shared.getPendingRequests()
            
            switch result {
            case .success(var responseDict):
                // Attach pending requests to the response
                if !pendingRequests.isEmpty {
                    responseDict["_pendingNativeRequests"] = pendingRequests
                }
                self?.completeWithResponse(context: context, response: responseDict)
            case .failure(let error):
                self?.logger.error("Bridge error: \(error.localizedDescription)")
                // Even on error, return pending requests
                if !pendingRequests.isEmpty {
                    let errorResponse: [String: Any] = [
                        "kind": ["Error": ["message": error.localizedDescription]],
                        "_pendingNativeRequests": pendingRequests
                    ]
                    self?.completeWithResponse(context: context, response: errorResponse)
                } else {
                    self?.completeWithError(context: context, error: error.localizedDescription)
                }
            }
        }
    }
    
    private func handlePollRequest(context: NSExtensionContext) {
        logger.info(">>> handlePollRequest called")
        let pendingRequests = NativeMessagingBridge.shared.getPendingRequests()
        logger.info(">>> Poll request - returning \(pendingRequests.count) pending native requests")
        
        if !pendingRequests.isEmpty {
            logger.info(">>> Pending requests: \(pendingRequests)")
        }
        
        let response: [String: Any] = [
            "type": "pollResponse",
            "pendingRequests": pendingRequests
        ]
        logger.info(">>> Sending poll response: \(response)")
        completeWithResponse(context: context, response: response)
    }
    
    private func handleNativeResponse(context: NSExtensionContext, response: [String: Any]) {
        logger.debug("Forwarding response from Safari extension to euro-native-messaging")
        
        NativeMessagingBridge.shared.sendNativeResponse(response)
        
        // Acknowledge receipt
        completeWithResponse(context: context, response: ["type": "ack", "success": true])
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
