use serde::{Deserialize, Serialize};
use specta::Type;

/// Top-level envelope sent in both directions on the bridge. Every
/// message is a `Frame`; the [`FrameKind`] discriminator identifies the
/// payload variant.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct Frame {
    pub kind: FrameKind,
}

/// Frame payload. Serialized as an externally-tagged JSON object —
/// `{ "Request": { ... } }` — to match the shape the browser extension
/// already consumes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub enum FrameKind {
    Request(RequestFrame),
    Response(ResponseFrame),
    Event(EventFrame),
    Error(ErrorFrame),
    Cancel(CancelFrame),
    Register(RegisterFrame),
}

/// Desktop-initiated request to a connected client.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct RequestFrame {
    pub id: u32,
    pub action: String,
    #[serde(default)]
    pub payload: Option<String>,
}

/// Client reply to a [`RequestFrame`], correlated by `id`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct ResponseFrame {
    pub id: u32,
    pub action: String,
    #[serde(default)]
    pub payload: Option<String>,
}

/// Unsolicited notification pushed by a client (e.g. browser tab
/// activation, Word selection change).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct EventFrame {
    pub action: String,
    #[serde(default)]
    pub payload: Option<String>,
}

/// Failure response correlated with a [`RequestFrame`] by `id`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct ErrorFrame {
    pub id: u32,
    pub code: u32,
    pub message: String,
    #[serde(default)]
    pub details: Option<String>,
}

/// Either side may send this to abort an in-flight request identified
/// by `id`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct CancelFrame {
    pub id: u32,
}

/// Mandatory first frame on every connection. Identifies the host
/// process (the bridge) and the application process being represented.
///
/// `app_pid` is the OS PID for clients that have one (browsers via the
/// native-messaging host, the macOS launcher representing Safari).
/// Sandboxed clients without access to a real PID — Office.js add-ins
/// in particular — synthesize a stable per-session identifier and set
/// `app_kind` to a logical name (e.g. `"microsoft-word"`) so the
/// desktop can locate the client without relying on OS process lookup.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct RegisterFrame {
    pub host_pid: u32,
    pub app_pid: u32,
    /// Logical client identifier for non-PID-based integrations.
    /// `None` for clients whose `app_pid` corresponds to a real OS
    /// process discoverable via process-name lookup.
    #[serde(default)]
    pub app_kind: Option<String>,
}

impl From<RequestFrame> for Frame {
    fn from(value: RequestFrame) -> Self {
        Self {
            kind: FrameKind::Request(value),
        }
    }
}

impl From<ResponseFrame> for Frame {
    fn from(value: ResponseFrame) -> Self {
        Self {
            kind: FrameKind::Response(value),
        }
    }
}

impl From<EventFrame> for Frame {
    fn from(value: EventFrame) -> Self {
        Self {
            kind: FrameKind::Event(value),
        }
    }
}

impl From<ErrorFrame> for Frame {
    fn from(value: ErrorFrame) -> Self {
        Self {
            kind: FrameKind::Error(value),
        }
    }
}

impl From<CancelFrame> for Frame {
    fn from(value: CancelFrame) -> Self {
        Self {
            kind: FrameKind::Cancel(value),
        }
    }
}

impl From<RegisterFrame> for Frame {
    fn from(value: RegisterFrame) -> Self {
        Self {
            kind: FrameKind::Register(value),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Pin the JSON wire format. The browser extension's
    /// `native-messenger.ts` keys on `'Request' in kind`,
    /// `'Response' in kind`, etc. — so the externally-tagged form must
    /// not regress.
    #[test]
    fn request_frame_round_trips_through_externally_tagged_json() {
        let frame = Frame::from(RequestFrame {
            id: 42,
            action: "GET_METADATA".into(),
            payload: None,
        });

        let json = serde_json::to_value(&frame).expect("serialize");
        assert_eq!(
            json,
            serde_json::json!({
                "kind": {
                    "Request": {
                        "id": 42,
                        "action": "GET_METADATA",
                        "payload": null,
                    }
                }
            }),
        );

        let round_tripped: Frame = serde_json::from_value(json).expect("deserialize");
        assert_eq!(round_tripped, frame);
    }

    #[test]
    fn register_frame_serializes_with_all_fields() {
        let frame = Frame::from(RegisterFrame {
            host_pid: 1,
            app_pid: 2,
            app_kind: None,
        });

        let json = serde_json::to_value(&frame).expect("serialize");
        assert_eq!(
            json,
            serde_json::json!({
                "kind": {
                    "Register": {
                        "host_pid": 1,
                        "app_pid": 2,
                        "app_kind": null,
                    }
                }
            }),
        );
    }

    #[test]
    fn register_frame_round_trips_with_app_kind() {
        let frame = Frame::from(RegisterFrame {
            host_pid: 0,
            app_pid: 4242,
            app_kind: Some("microsoft-word".into()),
        });

        let json = serde_json::to_value(&frame).expect("serialize");
        assert_eq!(
            json,
            serde_json::json!({
                "kind": {
                    "Register": {
                        "host_pid": 0,
                        "app_pid": 4242,
                        "app_kind": "microsoft-word",
                    }
                }
            }),
        );

        let round_tripped: Frame = serde_json::from_value(json).expect("deserialize");
        assert_eq!(round_tripped, frame);
    }

    /// Older client builds emit Register frames without the `app_kind`
    /// field; the desktop must continue to accept them and treat the
    /// kind as `None`.
    #[test]
    fn register_frame_accepts_missing_app_kind() {
        let json = serde_json::json!({
            "kind": {
                "Register": {
                    "host_pid": 1,
                    "app_pid": 2,
                }
            }
        });

        let frame: Frame = serde_json::from_value(json).expect("deserialize");
        let FrameKind::Register(register) = frame.kind else {
            panic!("expected register frame");
        };
        assert_eq!(register.host_pid, 1);
        assert_eq!(register.app_pid, 2);
        assert!(register.app_kind.is_none());
    }

    #[test]
    fn missing_optional_payload_deserializes_to_none() {
        let json = serde_json::json!({
            "kind": {
                "Event": {
                    "action": "TAB_ACTIVATED",
                }
            }
        });

        let frame: Frame = serde_json::from_value(json).expect("deserialize");
        let FrameKind::Event(event) = frame.kind else {
            panic!("expected event frame");
        };
        assert_eq!(event.action, "TAB_ACTIVATED");
        assert!(event.payload.is_none());
    }
}
