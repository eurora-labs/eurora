import Foundation
import os.log

/// Routes bridge frames between the local Safari extension and the desktop
/// bridge. Holds two independent correlation tables:
///
/// - `extensionRequests` — requests the Safari extension issued that are
///   awaiting a `Response`/`Error` from the desktop.
/// - `serverRequests` — requests the desktop pushed for the extension to
///   pick up the next time it calls `POLL_REQUESTS`.
///
/// All methods are thread-safe. The router is transport-agnostic: callers
/// pass a `forwardToBridge` closure that wraps whatever bridge connection
/// is in use, so the routing logic can be unit-tested without spinning up
/// a real `BridgeWebSocketClient`.
@available(macOS 13.0, *)
final class BridgeRequestRouter: @unchecked Sendable {
    private let logger = Logger(subsystem: "com.eurora.macos", category: "BridgeRequestRouter")
    private let requestTimeoutSeconds: TimeInterval
    private let pendingServerRequestTTL: TimeInterval
    private let forwardToBridge: (Frame) -> Void

    private let extensionRequestsLock = NSLock()
    private var extensionRequests: [UInt32: (Frame) -> Void] = [:]

    private let serverRequestsLock = NSLock()
    private var serverRequests: [UInt32: PendingServerRequest] = [:]

    private struct PendingServerRequest {
        let frame: Frame
        let storedAt: Date
    }

    init(
        requestTimeoutSeconds: TimeInterval = 30,
        pendingServerRequestTTL: TimeInterval = 10,
        forwardToBridge: @escaping (Frame) -> Void
    ) {
        self.requestTimeoutSeconds = requestTimeoutSeconds
        self.pendingServerRequestTTL = pendingServerRequestTTL
        self.forwardToBridge = forwardToBridge
    }

    // MARK: - Inbound from extension (LocalBridgeServer)

    /// Dispatch a frame received from the Safari extension. `completion` is
    /// called with the synchronous reply to send back to the extension, or
    /// `nil` if the reply will arrive later (via the desktop bridge response
    /// path) and be delivered through `broadcast`.
    func routeFromExtension(
        frame: Frame,
        completion: @escaping (Frame?) -> Void
    ) {
        switch frame.kind {
        case let .response(response):
            removeServerRequest(id: response.id)
            forwardToBridge(frame)
            completion(
                Frame(ResponseFrame(
                    id: response.id,
                    action: response.action,
                    payload: .object(["status": .string("forwarded")])
                ))
            )

        case let .error(errorFrame):
            removeServerRequest(id: errorFrame.id)
            forwardToBridge(frame)
            completion(nil)

        case let .request(request) where request.action == "POLL_REQUESTS":
            handlePollRequests(requestId: request.id, completion: completion)

        case let .request(request):
            forwardExtensionRequest(request: request, completion: completion)

        case .event, .cancel:
            forwardToBridge(frame)
            completion(nil)

        case .register:
            logger.warning("Ignoring Register from extension")
            completion(Frame(ErrorFrame(
                id: 0,
                message: "Register frames are not accepted from the extension"
            )))

        case .shutdown:
            logger.warning("Ignoring Shutdown from extension")
            completion(Frame(ErrorFrame(
                id: 0,
                message: "Shutdown frames are not accepted from the extension"
            )))
        }
    }

    // MARK: - Inbound from bridge (BridgeWebSocketClient)

    /// Deliver a desktop bridge `Response`/`Error` to the matching pending
    /// extension request, if any. No-op if the request was already drained
    /// (e.g. due to timeout or disconnect).
    func deliverToExtensionRequest(id: UInt32, frame: Frame) {
        extensionRequestsLock.lock()
        let completion = extensionRequests.removeValue(forKey: id)
        extensionRequestsLock.unlock()
        completion?(frame)
    }

    /// Buffer a desktop-pushed `Request` until the Safari extension polls.
    /// Buffered entries older than `pendingServerRequestTTL` are dropped on
    /// the next `POLL_REQUESTS`.
    func storeServerRequest(id: UInt32, frame: Frame) {
        serverRequestsLock.lock()
        serverRequests[id] = PendingServerRequest(frame: frame, storedAt: Date())
        serverRequestsLock.unlock()
    }

    /// Fulfil all pending extension requests with an `ErrorFrame` carrying
    /// the request's original id and the supplied `reason`. Used on bridge
    /// disconnect, where the desktop can no longer respond.
    @discardableResult
    func failAllExtensionRequests(reason: String) -> Int {
        extensionRequestsLock.lock()
        let pending = extensionRequests
        extensionRequests.removeAll()
        extensionRequestsLock.unlock()

        for (id, completion) in pending {
            completion(Frame(ErrorFrame(id: id, message: reason)))
        }
        return pending.count
    }

    // MARK: - Internals

    private func forwardExtensionRequest(
        request: RequestFrame,
        completion: @escaping (Frame?) -> Void
    ) {
        let id = request.id
        extensionRequestsLock.lock()
        extensionRequests[id] = completion
        extensionRequestsLock.unlock()

        let timeout = requestTimeoutSeconds
        DispatchQueue.main.asyncAfter(deadline: .now() + timeout) { [weak self] in
            guard let self else { return }
            extensionRequestsLock.lock()
            let timedOut = extensionRequests.removeValue(forKey: id)
            extensionRequestsLock.unlock()
            if let timedOut {
                logger.warning(
                    "Extension request \(id, privacy: .public) timed out after \(timeout, privacy: .public)s"
                )
                timedOut(Frame(ErrorFrame(id: id, message: "Request timed out")))
            }
        }

        forwardToBridge(Frame(request))
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

        // Inline the polled requests directly into the bridge payload —
        // the wire shape is now an inline JSON value (a Frame array), not
        // an escaped JSON string. On encoding failure we fall back to an
        // empty array so the poll RPC still completes cleanly.
        let payload: Payload
        do {
            payload = try Payload.encoding(frames)
        } catch {
            logger.error(
                "Failed to encode polled requests: \(error.localizedDescription, privacy: .public)"
            )
            payload = .array([])
        }

        completion(Frame(ResponseFrame(id: requestId, action: "POLL_REQUESTS", payload: payload)))
    }
}
