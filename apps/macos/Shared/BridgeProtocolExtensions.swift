import Foundation

/// Wire-protocol constants shared with the Rust `euro-bridge-protocol` crate.
public enum BridgeProtocol {
    /// Loopback host the desktop bridge listens on.
    public static let host = "127.0.0.1"

    /// Port the desktop bridge listens on.
    public static let port: UInt16 = 1431

    /// HTTP path that performs the WebSocket upgrade.
    public static let path = "/bridge"

    /// Full WebSocket URL for connecting to the local bridge.
    public static var url: URL {
        guard let url = URL(string: "ws://\(host):\(port)\(path)") else {
            fatalError("invalid bridge URL — host/port/path are constants")
        }
        return url
    }

    /// Cap on the size of any single JSON frame on the bridge. Matches the
    /// `max_message_size` configured by the desktop in
    /// `crates/app/euro-browser/src/server.rs`.
    public static let maxFrameSize = 16 * 1024 * 1024

    /// Backoff between reconnect attempts.
    public static let reconnectInterval: TimeInterval = 2.0

    /// Heartbeat interval used by both client and server. Matches
    /// `HEARTBEAT_INTERVAL` on the Rust side.
    public static let heartbeatInterval: TimeInterval = 30.0

    /// Shared encoder. Default settings match the Rust serde shape — the
    /// snake_case key mappings live on the individual `Codable` types.
    public static let encoder = JSONEncoder()

    /// Shared decoder. As with the encoder, defaults match the Rust shape.
    public static let decoder = JSONDecoder()
}

extension Frame {
    /// Mirrors the Rust `From<RequestFrame> for Frame` impl.
    public init(_ request: RequestFrame) { self.init(kind: .request(request)) }
    public init(_ response: ResponseFrame) { self.init(kind: .response(response)) }
    public init(_ event: EventFrame) { self.init(kind: .event(event)) }
    public init(_ error: ErrorFrame) { self.init(kind: .error(error)) }
    public init(_ cancel: CancelFrame) { self.init(kind: .cancel(cancel)) }
    public init(_ register: RegisterFrame) { self.init(kind: .register(register)) }
}

extension ErrorFrame {
    /// Convenience: build an `ErrorFrame` with a default `code = 0` and no
    /// extra `details`. Distinct signature from the synthesized memberwise
    /// initializer (which requires all four fields).
    public init(id: UInt32, message: String) {
        self.init(id: id, code: 0, message: message, details: nil)
    }
}

extension Frame {
    /// Encode the frame as a UTF-8 JSON `Data` payload using the wire shape
    /// the Rust bridge expects.
    public func encodeJSON() throws -> Data {
        try BridgeProtocol.encoder.encode(self)
    }

    /// Encode the frame as a JSON string suitable for `URLSessionWebSocketTask.Message.string`.
    public func encodeJSONString() throws -> String {
        let data = try encodeJSON()
        guard let string = String(data: data, encoding: .utf8) else {
            throw BridgeProtocolError.invalidUTF8
        }
        return string
    }

    /// Decode a frame from a UTF-8 JSON `Data` payload.
    public static func decode(_ data: Data) throws -> Frame {
        try BridgeProtocol.decoder.decode(Frame.self, from: data)
    }

    /// Decode a frame from a JSON string.
    public static func decode(_ string: String) throws -> Frame {
        guard let data = string.data(using: .utf8) else {
            throw BridgeProtocolError.invalidUTF8
        }
        return try decode(data)
    }
}

public enum BridgeProtocolError: Error, LocalizedError {
    case invalidUTF8

    public var errorDescription: String? {
        switch self {
        case .invalidUTF8:
            return "Bridge frame payload was not valid UTF-8"
        }
    }
}

extension FrameKind {
    /// Short label for logging. Mirrors `frame_kind_label` on the Rust side.
    public var label: String {
        switch self {
        case .request: return "Request"
        case .response: return "Response"
        case .event: return "Event"
        case .error: return "Error"
        case .cancel: return "Cancel"
        case .register: return "Register"
        }
    }
}

extension Frame {
    /// Brief one-line summary suitable for trace logging.
    public var summary: String {
        switch kind {
        case .request(let r): return "Request(id=\(r.id), action=\(r.action))"
        case .response(let r): return "Response(id=\(r.id), action=\(r.action))"
        case .event(let e): return "Event(action=\(e.action))"
        case .error(let e): return "Error(id=\(e.id), code=\(e.code))"
        case .cancel(let c): return "Cancel(id=\(c.id))"
        case .register(let r):
            if let kind = r.appKind {
                return "Register(host=\(r.hostPid), app=\(r.appPid), kind=\(kind))"
            }
            return "Register(host=\(r.hostPid), app=\(r.appPid))"
        }
    }
}
