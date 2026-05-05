import SafariServices
import os.log

private let extensionLogger = Logger(
    subsystem: "com.eurora.macos.extension",
    category: "SafariWebExtensionHandler"
)

@available(macOS 13.0, *)
class SafariWebExtensionHandler: NSObject, NSExtensionRequestHandling {

    func beginRequest(with context: NSExtensionContext) {
        let request = context.inputItems.first as? NSExtensionItem
        let raw = Self.extractMessage(from: request)
        let profileStr = Self.extractProfile(from: request).map { $0.uuidString } ?? "none"
        extensionLogger.debug("Received native message (profile: \(profileStr, privacy: .public))")

        guard let raw else {
            extensionLogger.error("Empty native message")
            Self.completeWithError(context: context, message: "Empty native message")
            return
        }

        let frame: Frame
        do {
            frame = try Self.decodeFrame(from: raw)
        } catch {
            extensionLogger.error("Failed to decode incoming frame: \(error.localizedDescription, privacy: .public)")
            Self.completeWithError(context: context, message: "Invalid message format")
            return
        }

        switch frame.kind {
        case .response, .error:
            // Reply to a server-pushed request — forward and acknowledge.
            NativeMessagingBridge.shared.ensureConnected()
            NativeMessagingBridge.shared.forward(frame)
            Self.completeWithStatus(context: context, status: "forwarded")

        case .request(let request):
            NativeMessagingBridge.shared.ensureConnected()
            Task {
                do {
                    let response = try await NativeMessagingBridge.shared.send(request: request)
                    Self.completeWithFrame(context: context, frame: response)
                } catch {
                    extensionLogger.error("Bridge error: \(error.localizedDescription, privacy: .public)")
                    Self.completeWithError(
                        context: context,
                        id: request.id,
                        message: error.localizedDescription
                    )
                }
            }

        case .event, .cancel:
            // Best-effort: forward and acknowledge with no reply expected.
            NativeMessagingBridge.shared.ensureConnected()
            NativeMessagingBridge.shared.forward(frame)
            Self.completeWithStatus(context: context, status: "forwarded")

        case .register:
            extensionLogger.warning("Ignoring unexpected Register frame from extension")
            Self.completeWithError(
                context: context,
                message: "Register frames are not accepted from the extension"
            )
        }
    }

    // MARK: - Decoding

    private static func extractMessage(from request: NSExtensionItem?) -> Any? {
        if #available(iOS 15.0, macOS 11.0, *) {
            return request?.userInfo?[SFExtensionMessageKey]
        }
        return request?.userInfo?["message"]
    }

    private static func extractProfile(from request: NSExtensionItem?) -> UUID? {
        if #available(iOS 17.0, macOS 14.0, *) {
            return request?.userInfo?[SFExtensionProfileKey] as? UUID
        }
        return request?.userInfo?["profile"] as? UUID
    }

    private static func decodeFrame(from raw: Any) throws -> Frame {
        let data: Data
        if let frameData = raw as? Data {
            data = frameData
        } else if let string = raw as? String, let stringData = string.data(using: .utf8) {
            data = stringData
        } else {
            data = try JSONSerialization.data(withJSONObject: raw, options: [])
        }
        return try Frame.decode(data)
    }

    // MARK: - Replying back to the JS extension

    private static func completeWithFrame(context: NSExtensionContext, frame: Frame) {
        do {
            let data = try frame.encodeJSON()
            guard let object = try JSONSerialization.jsonObject(with: data) as? [String: Any] else {
                completeWithError(context: context, message: "Frame did not encode to a JSON object")
                return
            }
            sendMessage(context: context, payload: object)
        } catch {
            completeWithError(context: context, message: error.localizedDescription)
        }
    }

    private static func completeWithStatus(context: NSExtensionContext, status: String) {
        sendMessage(context: context, payload: ["status": status])
    }

    private static func completeWithError(
        context: NSExtensionContext,
        id: UInt32 = 0,
        message: String
    ) {
        let frame = Frame(ErrorFrame(id: id, message: message))
        do {
            let data = try frame.encodeJSON()
            if let object = try JSONSerialization.jsonObject(with: data) as? [String: Any] {
                sendMessage(context: context, payload: object)
                return
            }
        } catch {
            extensionLogger.error("Failed to encode error frame: \(error.localizedDescription, privacy: .public)")
        }
        sendMessage(context: context, payload: ["error": message])
    }

    private static func sendMessage(context: NSExtensionContext, payload: [String: Any]) {
        let item = NSExtensionItem()
        if #available(iOS 15.0, macOS 11.0, *) {
            item.userInfo = [SFExtensionMessageKey: payload]
        } else {
            item.userInfo = ["message": payload]
        }
        context.completeRequest(returningItems: [item], completionHandler: nil)
    }
}
