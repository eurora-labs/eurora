use std::fmt;

use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;
use specta::Type;

/// Top-level envelope sent in both directions on the bridge. Every
/// message is a `Frame`; the [`FrameKind`] discriminator identifies the
/// payload variant.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
pub struct Frame {
    pub kind: FrameKind,
}

/// Frame payload. Serialized as an externally-tagged JSON object —
/// `{ "Request": { ... } }` — to match the shape the browser extension
/// already consumes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
pub enum FrameKind {
    Request(RequestFrame),
    Response(ResponseFrame),
    Event(EventFrame),
    Error(ErrorFrame),
    Cancel(CancelFrame),
    Register(RegisterFrame),
    Shutdown(ShutdownFrame),
}

impl FrameKind {
    /// Stable string label for the variant. Used in log lines and error
    /// messages where Debug-formatting the entire payload would be
    /// noisy. The labels match the externally-tagged JSON discriminator,
    /// so they're identical to the wire form.
    pub fn variant_name(&self) -> &'static str {
        match self {
            FrameKind::Request(_) => "Request",
            FrameKind::Response(_) => "Response",
            FrameKind::Event(_) => "Event",
            FrameKind::Error(_) => "Error",
            FrameKind::Cancel(_) => "Cancel",
            FrameKind::Register(_) => "Register",
            FrameKind::Shutdown(_) => "Shutdown",
        }
    }
}

/// Inline JSON payload carried by Request/Response/Event frames.
///
/// Stored as a [`Box<RawValue>`] so the payload's JSON serializes
/// **inline** into the frame envelope rather than as a JSON-encoded
/// string. Compared to the historical `Option<String>` shape, this:
///
/// - halves the wire size for large payloads (e.g. base64-encoded
///   PNGs) because the outer envelope no longer double-escapes the
///   inner JSON;
/// - drops one parse + one escape per direction (Rust producers hand
///   raw JSON straight to serde; consumers read it without first
///   decoding a string layer);
/// - typed in Rust as JSON rather than "string of unknown structure",
///   which means the encode/decode helpers can be centralised here
///   and call sites stop manually `serde_json::to_string`-ing.
///
/// Construct via [`Payload::from_value`] for typed Rust data, or
/// [`Payload::from_raw_json`] when handing through a literal JSON
/// fragment.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(transparent)]
// `Payload` is "any JSON value" — specta-swift can't render
// `Box<RawValue>` and specta-typescript would over-eagerly inline it,
// so the type is emitted as a named struct with no fields and
// language-specific post-processors swap that placeholder in for the
// real definition:
//
// - The TypeScript post-processor in [`codegen::rewrite_typescript_payload`]
//   rewrites the empty `Payload` alias to `type Payload = unknown`.
// - The Swift post-processor in [`codegen::run`] replaces the empty
//   `struct Payload: Codable` with the hand-rolled JSON-value
//   `Codable` enum downstream Swift consumers actually need.
#[specta(transparent = false)]
pub struct Payload(#[specta(skip)] Box<RawValue>);

impl Payload {
    /// Wrap a Rust value that implements [`Serialize`].
    ///
    /// Returns an error if `value` cannot be serialized — typically
    /// because of a `serde` custom-serializer error, since `RawValue`
    /// itself imposes no shape constraints.
    pub fn from_value<T: Serialize + ?Sized>(value: &T) -> Result<Self, serde_json::Error> {
        Ok(Self(serde_json::value::to_raw_value(value)?))
    }

    /// Wrap a pre-rendered JSON fragment.
    pub fn from_raw_json(raw: Box<RawValue>) -> Self {
        Self(raw)
    }

    /// Decode the payload into a typed Rust value.
    pub fn deserialize<T: serde::de::DeserializeOwned>(&self) -> Result<T, serde_json::Error> {
        serde_json::from_str(self.0.get())
    }

    /// Borrow the underlying JSON fragment as a `&str`. The slice
    /// outlives the borrow but not the [`Payload`] — callers that need
    /// ownership should clone or call [`Payload::into_raw`].
    pub fn as_str(&self) -> &str {
        self.0.get()
    }

    /// Consume `self` and return the boxed `RawValue` for callers that
    /// need to hand the fragment through to another serde structure
    /// without re-encoding.
    pub fn into_raw(self) -> Box<RawValue> {
        self.0
    }
}

impl PartialEq for Payload {
    /// JSON-value equality, not byte equality: two payloads compare
    /// equal iff their decoded `serde_json::Value` trees match. This
    /// matches the intent of the wire — `{"a":1}` and `{ "a": 1 }`
    /// represent the same data — and means [`Frame`]'s derived
    /// `PartialEq` continues to behave as a structural comparator.
    fn eq(&self, other: &Self) -> bool {
        match (
            serde_json::from_str::<serde_json::Value>(self.0.get()),
            serde_json::from_str::<serde_json::Value>(other.0.get()),
        ) {
            (Ok(a), Ok(b)) => a == b,
            _ => self.0.get() == other.0.get(),
        }
    }
}

impl fmt::Display for Payload {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0.get())
    }
}

impl<T: Serialize> From<&T> for Payload {
    /// Convenience for the common "serialize a typed value into a
    /// payload" path. Panics on serialization failure — only safe for
    /// types that can never fail to serialize (the usual case for
    /// `Serialize`-derived plain data).
    fn from(value: &T) -> Self {
        Payload::from_value(value).expect("Payload::from value serializes")
    }
}

/// Desktop-initiated request to a connected client.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
pub struct RequestFrame {
    pub id: u32,
    pub action: String,
    #[serde(default)]
    pub payload: Option<Payload>,
}

/// Client reply to a [`RequestFrame`], correlated by `id`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
pub struct ResponseFrame {
    pub id: u32,
    pub action: String,
    #[serde(default)]
    pub payload: Option<Payload>,
}

/// Unsolicited notification pushed by a client (e.g. browser tab
/// activation, Word selection change).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
pub struct EventFrame {
    pub action: String,
    #[serde(default)]
    pub payload: Option<Payload>,
}

/// Failure response correlated with a [`RequestFrame`] by `id`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
pub struct ErrorFrame {
    pub id: u32,
    pub code: u32,
    pub message: String,
    #[serde(default)]
    pub details: Option<Payload>,
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

/// Desktop-initiated request that the receiving client (currently:
/// browser native-messaging hosts) terminate cleanly. Sent when the
/// desktop has just installed an updated messenger binary on disk and
/// wants stale connections to drop so the browser respawns from the new
/// binary. The `reason` is informational only (logged by the recipient).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct ShutdownFrame {
    #[serde(default)]
    pub reason: Option<String>,
}

impl From<ShutdownFrame> for Frame {
    fn from(value: ShutdownFrame) -> Self {
        Self {
            kind: FrameKind::Shutdown(value),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Pin the JSON wire format. Payload-bearing frames now embed
    /// the inner JSON inline rather than as an escaped string, so the
    /// browser extension and other clients see the payload as a
    /// regular JSON value instead of having to call `JSON.parse(...)`
    /// on it first.
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

    /// A payload-bearing frame must serialize the payload **inline**
    /// (not as an escaped JSON string). This pins the wire shape the
    /// browser extension and Office add-in expect after the
    /// `Option<String>` → `Option<Payload>` change.
    #[test]
    fn request_frame_inlines_payload_on_the_wire() {
        let frame = Frame::from(RequestFrame {
            id: 7,
            action: "YOUTUBE_GET_CURRENT_TIMESTAMP".into(),
            payload: Some(Payload::from_value(&serde_json::json!({"tab_id": 19})).unwrap()),
        });

        let json = serde_json::to_value(&frame).expect("serialize");
        assert_eq!(
            json,
            serde_json::json!({
                "kind": {
                    "Request": {
                        "id": 7,
                        "action": "YOUTUBE_GET_CURRENT_TIMESTAMP",
                        "payload": {"tab_id": 19},
                    }
                }
            })
        );
    }

    /// Round-trip through a typed payload struct: encode → outer JSON →
    /// decode → the same typed struct, exercising the `Payload::from_value`
    /// + `Payload::deserialize` ergonomics.
    #[test]
    fn payload_round_trips_a_typed_struct() {
        #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
        struct Args {
            tab_id: i64,
            label: String,
        }

        let args = Args {
            tab_id: 19,
            label: "primary".into(),
        };
        let frame = Frame::from(RequestFrame {
            id: 1,
            action: "X".into(),
            payload: Some(Payload::from_value(&args).unwrap()),
        });

        let json = serde_json::to_string(&frame).expect("serialize");
        let parsed: Frame = serde_json::from_str(&json).expect("deserialize");

        let FrameKind::Request(req) = parsed.kind else {
            panic!("expected Request");
        };
        let decoded: Args = req.payload.expect("payload present").deserialize().unwrap();
        assert_eq!(decoded, args);
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
    fn shutdown_frame_round_trips_through_externally_tagged_json() {
        let frame = Frame::from(ShutdownFrame {
            reason: Some("messenger binary was just replaced".into()),
        });

        let json = serde_json::to_value(&frame).expect("serialize");
        assert_eq!(
            json,
            serde_json::json!({
                "kind": {
                    "Shutdown": {
                        "reason": "messenger binary was just replaced",
                    }
                }
            }),
        );

        let round_tripped: Frame = serde_json::from_value(json).expect("deserialize");
        assert_eq!(round_tripped, frame);
    }

    #[test]
    fn shutdown_frame_accepts_missing_reason() {
        let json = serde_json::json!({
            "kind": {
                "Shutdown": {}
            }
        });

        let frame: Frame = serde_json::from_value(json).expect("deserialize");
        let FrameKind::Shutdown(shutdown) = frame.kind else {
            panic!("expected shutdown frame");
        };
        assert!(shutdown.reason.is_none());
    }

    #[test]
    fn variant_name_returns_stable_label_for_each_kind() {
        let cases: [(FrameKind, &str); 7] = [
            (
                FrameKind::Request(RequestFrame {
                    id: 1,
                    action: "X".into(),
                    payload: None,
                }),
                "Request",
            ),
            (
                FrameKind::Response(ResponseFrame {
                    id: 1,
                    action: "X".into(),
                    payload: None,
                }),
                "Response",
            ),
            (
                FrameKind::Event(EventFrame {
                    action: "X".into(),
                    payload: None,
                }),
                "Event",
            ),
            (
                FrameKind::Error(ErrorFrame {
                    id: 1,
                    code: 0,
                    message: "x".into(),
                    details: None,
                }),
                "Error",
            ),
            (FrameKind::Cancel(CancelFrame { id: 1 }), "Cancel"),
            (
                FrameKind::Register(RegisterFrame {
                    host_pid: 0,
                    app_pid: 0,
                    app_kind: None,
                }),
                "Register",
            ),
            (
                FrameKind::Shutdown(ShutdownFrame { reason: None }),
                "Shutdown",
            ),
        ];
        for (kind, expected) in cases {
            assert_eq!(kind.variant_name(), expected);
        }
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

    /// Equality on `Payload` is JSON-structural, not byte-wise. Two
    /// payloads that decode to the same `serde_json::Value` compare
    /// equal even if their byte representations differ (whitespace,
    /// key order on objects, …).
    #[test]
    fn payload_equality_is_structural() {
        let a = Payload::from_raw_json(RawValue::from_string(r#"{"a":1,"b":2}"#.into()).unwrap());
        let b =
            Payload::from_raw_json(RawValue::from_string(r#"{ "b": 2, "a": 1 }"#.into()).unwrap());
        assert_eq!(a, b);
    }
}
