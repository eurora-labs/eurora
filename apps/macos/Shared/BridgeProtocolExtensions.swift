import Foundation

/// Wire-protocol constants shared with the Rust `euro-bridge-protocol` crate.
public enum BridgeProtocol {
    /// Hostname the launcher dials. The OS resolver routes this to the
    /// loopback interface where the desktop binds the listener, so the
    /// channel never leaves the kernel. Mirrors `BRIDGE_HOST` on the
    /// Rust side; kept as a hostname (not `127.0.0.1`) so every
    /// component in the workspace canonicalizes to the same string.
    public static let host = "localhost"

    /// URL scheme. Plaintext WebSocket — the bridge is loopback-only
    /// and rejects non-loopback peers at upgrade time, so `ws://`
    /// carries no confidentiality cost a local attacker doesn't already
    /// have. The `Register` frame is the authentication boundary, not
    /// TLS. Mirrors `BRIDGE_SCHEME` on the Rust side.
    public static let scheme = "ws"

    /// Port the desktop bridge listens on.
    public static let port: UInt16 = 1431

    /// HTTP path that performs the WebSocket upgrade.
    public static let path = "/bridge"

    /// Full WebSocket URL for connecting to the local bridge.
    public static var url: URL {
        guard let url = URL(string: "\(scheme)://\(host):\(port)\(path)") else {
            fatalError("invalid bridge URL — scheme/host/port/path are constants")
        }
        return url
    }

    /// Cap on the size of any single JSON frame on the bridge. Matches the
    /// `max_message_size` configured by the desktop in
    /// `crates/app/euro-bridge/src/server.rs`.
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

public extension Frame {
    /// Mirrors the Rust `From<RequestFrame> for Frame` impl.
    init(_ request: RequestFrame) {
        self.init(kind: .request(request))
    }

    init(_ response: ResponseFrame) {
        self.init(kind: .response(response))
    }

    init(_ event: EventFrame) {
        self.init(kind: .event(event))
    }

    init(_ error: ErrorFrame) {
        self.init(kind: .error(error))
    }

    init(_ cancel: CancelFrame) {
        self.init(kind: .cancel(cancel))
    }

    init(_ register: RegisterFrame) {
        self.init(kind: .register(register))
    }

    init(_ shutdown: ShutdownFrame) {
        self.init(kind: .shutdown(shutdown))
    }
}

public extension ErrorFrame {
    /// Convenience: build an `ErrorFrame` with a default `code = 0` and no
    /// extra `details`. Distinct signature from the synthesized memberwise
    /// initializer (which requires all four fields).
    init(id: UInt32, message: String) {
        self.init(id: id, code: 0, message: message, details: nil)
    }
}

public extension Payload {
    /// Wrap a `Codable` value as an inline bridge `Payload`.
    ///
    /// Round-trips through `JSONSerialization` so the JSON-value enum's
    /// `case` is filled from the encoded shape — i.e. `[String: Any]`
    /// for an object, `[Any]` for an array, scalars for primitives.
    /// Mirrors the Rust [`Payload::from_value`] helper.
    static func encoding(_ value: some Encodable) throws -> Payload {
        let data = try BridgeProtocol.encoder.encode(value)
        return try BridgeProtocol.decoder.decode(Payload.self, from: data)
    }

    /// Decode the payload into a typed Rust-mirror value. Mirrors the
    /// Rust [`Payload::deserialize`] helper.
    func decode<T: Decodable>(as type: T.Type = T.self) throws -> T {
        let data = try BridgeProtocol.encoder.encode(self)
        return try BridgeProtocol.decoder.decode(type, from: data)
    }
}

public extension Frame {
    /// Encode the frame as a UTF-8 JSON `Data` payload using the wire shape
    /// the Rust bridge expects.
    func encodeJSON() throws -> Data {
        try BridgeProtocol.encoder.encode(self)
    }

    /// Encode the frame as a JSON string suitable for `URLSessionWebSocketTask.Message.string`.
    func encodeJSONString() throws -> String {
        let data = try encodeJSON()
        guard let string = String(data: data, encoding: .utf8) else {
            throw BridgeProtocolError.invalidUTF8
        }
        return string
    }

    /// Decode a frame from a UTF-8 JSON `Data` payload.
    static func decode(_ data: Data) throws -> Frame {
        try BridgeProtocol.decoder.decode(Frame.self, from: data)
    }

    /// Decode a frame from a JSON string.
    static func decode(_ string: String) throws -> Frame {
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
            "Bridge frame payload was not valid UTF-8"
        }
    }
}

/// String constants that pin both ends of the bridge to the same wire
/// vocabulary. Mirrors the constants on the Rust side
/// (`euro_bridge::EXTENSION_STATE_EVENT`, `OPEN_BROWSER_EXTENSION_SETTINGS_ACTION`);
/// the build can't enforce that automatically, so these values are
/// duplicated by hand and a regression here is caught at runtime by the
/// frame-handler tests.
public enum BridgeAction {
    /// `EventFrame.action` the launcher publishes to inform the desktop
    /// of a Safari extension state transition. Payload is JSON-encoded
    /// `ExtensionStateEventPayload`.
    public static let extensionStateChanged = "EXTENSION_STATE_CHANGED"

    /// `RequestFrame.action` the desktop sends to deep-link into Safari's
    /// extension settings. The launcher responds with an empty `Response`
    /// on success or an `Error` if `SFSafariApplication` rejected the
    /// call.
    public static let openBrowserExtensionSettings = "OPEN_BROWSER_EXTENSION_SETTINGS"
}

/// Logical client identifier the launcher registers with on the bridge.
/// Mirrors `euro_tauri`'s `SAFARI_BRIDGE_APP_KIND` constant; pinned here
/// because it appears in both `RegisterFrame.appKind` and every
/// `ExtensionStateEventPayload.appKind` we publish.
public let kSafariBridgeAppKind = "safari"

/// Mirrors `euro_bridge::BundledExtensionState` on the Rust side.
/// Encoded as a snake-case string in the `state` field of the
/// [`BridgeAction.extensionStateChanged`] event payload.
public enum BundledExtensionState: String, Codable {
    case enabled
    case disabled
    case notDiscovered = "not_discovered"
    case unknown
}

/// Wire payload of [`BridgeAction.extensionStateChanged`]. Serialized as
/// JSON into `EventFrame.payload`.
public struct ExtensionStateEventPayload: Codable {
    public let appKind: String
    public let state: BundledExtensionState

    private enum CodingKeys: String, CodingKey {
        case appKind = "app_kind"
        case state
    }

    public init(appKind: String, state: BundledExtensionState) {
        self.appKind = appKind
        self.state = state
    }
}

/// Build an `EventFrame` reporting the current Safari extension state.
/// Encoding errors here are theoretically impossible — the payload is a
/// fixed shape — but JSON encoding is fallible by signature, so we fall
/// back to an empty payload rather than crashing the launcher.
public func makeExtensionStateEvent(state: BundledExtensionState) -> Frame {
    let body = ExtensionStateEventPayload(appKind: kSafariBridgeAppKind, state: state)
    let payload = try? Payload.encoding(body)
    return Frame(EventFrame(action: BridgeAction.extensionStateChanged, payload: payload))
}

public extension FrameKind {
    /// Short label for logging. Mirrors `frame_kind_label` on the Rust side.
    var label: String {
        switch self {
        case .request: "Request"
        case .response: "Response"
        case .event: "Event"
        case .error: "Error"
        case .cancel: "Cancel"
        case .register: "Register"
        case .shutdown: "Shutdown"
        }
    }
}

public extension Frame {
    /// Brief one-line summary suitable for trace logging.
    var summary: String {
        switch kind {
        case let .request(request):
            return "Request(id=\(request.id), action=\(request.action))"
        case let .response(response):
            return "Response(id=\(response.id), action=\(response.action))"
        case let .event(event):
            return "Event(action=\(event.action))"
        case let .error(errorFrame):
            return "Error(id=\(errorFrame.id), code=\(errorFrame.code))"
        case let .cancel(cancel):
            return "Cancel(id=\(cancel.id))"
        case let .register(register):
            if let kind = register.appKind {
                return "Register(host=\(register.hostPid), app=\(register.appPid), kind=\(kind))"
            }
            return "Register(host=\(register.hostPid), app=\(register.appPid))"
        case let .shutdown(shutdown):
            if let reason = shutdown.reason {
                return "Shutdown(reason=\(reason))"
            }
            return "Shutdown"
        }
    }
}
