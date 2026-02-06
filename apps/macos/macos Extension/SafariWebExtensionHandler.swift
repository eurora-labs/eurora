//
//  SafariWebExtensionHandler.swift
//  macos Extension
//
//  Handles messages from the Safari web extension JavaScript and forwards them
//  to the container app via NativeMessagingBridge.
//
//  This is the extension-side equivalent of reading from/writing to stdin/stdout
//  in the Chrome native messaging model.
//

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

        // Ensure the bridge is connected, then send the message
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
