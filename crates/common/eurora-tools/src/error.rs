//! In-process tool error and conversions to/from the wire shape.
//!
//! The framework's [`ToolError`] is the source of truth for tool-call
//! failures while a call is in flight on a single side of the chat
//! WebSocket. It uses `Cow<'static, str>` for messages and metadata
//! fields so both macro-generated literals (`Cow::Borrowed`) and
//! wire-side `String`s (`Cow::Owned`) can construct the variant without
//! leaks. The `Decode`/`Encode`/`Adapter` variants additionally carry an
//! optional `#[source]` cause: locally-constructed errors preserve the
//! original `serde_json::Error` or boxed adapter error for diagnostics;
//! errors reconstructed from the wire shape carry the rendered message
//! but no source (the original cause can't be reconstructed from a
//! string).
//!
//! Conversion to [`thread_core::ToolErrorWire`] is lossy *only* for the
//! source-cause field — variants and messages survive both directions.

use std::borrow::Cow;

use thread_core::ToolErrorWire;

/// Error type for an in-flight tool call.
///
/// Variants:
/// - `ContextUnavailable` — the required context was active at turn start
///   but is no longer reachable (browser tab closed, focused app exited).
/// - `OriginMismatch` — the runtime [`crate::Origin`] variant doesn't match
///   what the adapter method declared. A defense-in-depth check; in v1's
///   single-source world this is effectively unreachable.
/// - `Timeout` — the descriptor's `timeout_ms` budget elapsed.
/// - `Cancelled` — the turn-level cancellation token fired.
/// - `Transport` — the transport layer (chat WS, bridge, ACP) returned an
///   error before the tool itself ran.
/// - `Remote` — the tool ran on the remote side and returned a structured
///   error with a numeric code, message, and optional details blob.
/// - `Decode` / `Encode` — the framework failed to (de)serialize the
///   tool's arguments or return value. Construct via [`ToolError::decode`]
///   / [`ToolError::encode`] to preserve the underlying `serde_json::Error`
///   in the `source()` chain.
/// - `Adapter` — the user-written adapter implementation surfaced an
///   error. Construct via [`ToolError::adapter`] to box the cause.
///
/// `#[non_exhaustive]` so we can extend without breaking downstream
/// matchers.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ToolError {
    #[error("context unavailable for tool `{tool}`: {reason}")]
    ContextUnavailable {
        tool: Cow<'static, str>,
        reason: Cow<'static, str>,
    },
    #[error("origin mismatch for tool `{tool}`: expected {expected}, got {got}")]
    OriginMismatch {
        tool: Cow<'static, str>,
        expected: Cow<'static, str>,
        got: Cow<'static, str>,
    },
    #[error("tool call timed out")]
    Timeout,
    #[error("tool call cancelled")]
    Cancelled,
    #[error("transport error: {0}")]
    Transport(Cow<'static, str>),
    #[error("remote error {code}: {message}")]
    Remote {
        code: u32,
        message: String,
        details: Option<serde_json::Value>,
    },
    #[error("failed to decode tool payload: {message}")]
    Decode {
        message: Cow<'static, str>,
        #[source]
        source: Option<serde_json::Error>,
    },
    #[error("failed to encode tool payload: {message}")]
    Encode {
        message: Cow<'static, str>,
        #[source]
        source: Option<serde_json::Error>,
    },
    #[error("adapter error: {message}")]
    Adapter {
        message: Cow<'static, str>,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },
}

impl ToolError {
    /// Construct a `Decode` error from a `serde_json::Error`, preserving
    /// the original cause in the `source()` chain.
    pub fn decode(err: serde_json::Error) -> Self {
        Self::Decode {
            message: err.to_string().into(),
            source: Some(err),
        }
    }

    /// Construct an `Encode` error from a `serde_json::Error`, preserving
    /// the original cause in the `source()` chain.
    pub fn encode(err: serde_json::Error) -> Self {
        Self::Encode {
            message: err.to_string().into(),
            source: Some(err),
        }
    }

    /// Construct an `Adapter` error, boxing the cause for the `source()`
    /// chain.
    pub fn adapter<E>(err: E) -> Self
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        Self::Adapter {
            message: err.to_string().into(),
            source: Some(Box::new(err)),
        }
    }
}

impl From<ToolError> for ToolErrorWire {
    fn from(err: ToolError) -> Self {
        match err {
            ToolError::ContextUnavailable { tool, reason } => Self::ContextUnavailable {
                tool: tool.into_owned(),
                reason: reason.into_owned(),
            },
            ToolError::OriginMismatch {
                tool,
                expected,
                got,
            } => Self::OriginMismatch {
                tool: tool.into_owned(),
                expected: expected.into_owned(),
                got: got.into_owned(),
            },
            ToolError::Timeout => Self::Timeout,
            ToolError::Cancelled => Self::Cancelled,
            ToolError::Transport(msg) => Self::Transport {
                message: msg.into_owned(),
            },
            ToolError::Remote {
                code,
                message,
                details,
            } => Self::Remote {
                code,
                message,
                details,
            },
            ToolError::Decode { message, .. } => Self::Decode {
                message: message.into_owned(),
            },
            ToolError::Encode { message, .. } => Self::Encode {
                message: message.into_owned(),
            },
            ToolError::Adapter { message, .. } => Self::Adapter {
                message: message.into_owned(),
            },
        }
    }
}

impl From<ToolErrorWire> for ToolError {
    fn from(err: ToolErrorWire) -> Self {
        match err {
            ToolErrorWire::ContextUnavailable { tool, reason } => Self::ContextUnavailable {
                tool: Cow::Owned(tool),
                reason: Cow::Owned(reason),
            },
            ToolErrorWire::OriginMismatch {
                tool,
                expected,
                got,
            } => Self::OriginMismatch {
                tool: Cow::Owned(tool),
                expected: Cow::Owned(expected),
                got: Cow::Owned(got),
            },
            ToolErrorWire::Timeout => Self::Timeout,
            ToolErrorWire::Cancelled => Self::Cancelled,
            ToolErrorWire::Transport { message } => Self::Transport(Cow::Owned(message)),
            ToolErrorWire::Remote {
                code,
                message,
                details,
            } => Self::Remote {
                code,
                message,
                details,
            },
            ToolErrorWire::Decode { message } => Self::Decode {
                message: Cow::Owned(message),
                source: None,
            },
            ToolErrorWire::Encode { message } => Self::Encode {
                message: Cow::Owned(message),
                source: None,
            },
            ToolErrorWire::Adapter { message } => Self::Adapter {
                message: Cow::Owned(message),
                source: None,
            },
            // ToolErrorWire is #[non_exhaustive]; folding unknown variants
            // into Adapter keeps the message intact for diagnostics and
            // signals (clearly) that eurora-tools needs an update.
            other => Self::Adapter {
                message: Cow::Owned(format!(
                    "unsupported tool error variant: {other:?}; eurora-tools needs an update"
                )),
                source: None,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn decode_err() -> serde_json::Error {
        serde_json::from_str::<u32>("not a number").unwrap_err()
    }

    #[test]
    fn forward_context_unavailable() {
        let err = ToolError::ContextUnavailable {
            tool: Cow::Borrowed("browser::youtube::get_transcript"),
            reason: Cow::Borrowed("no active youtube tab"),
        };
        let wire: ToolErrorWire = err.into();
        match wire {
            ToolErrorWire::ContextUnavailable { tool, reason } => {
                assert_eq!(tool, "browser::youtube::get_transcript");
                assert_eq!(reason, "no active youtube tab");
            }
            other => panic!("expected ContextUnavailable, got {other:?}"),
        }
    }

    #[test]
    fn forward_origin_mismatch() {
        let err = ToolError::OriginMismatch {
            tool: Cow::Borrowed("browser::youtube::get_transcript"),
            expected: Cow::Borrowed("Browser"),
            got: Cow::Borrowed("Focused"),
        };
        let wire: ToolErrorWire = err.into();
        match wire {
            ToolErrorWire::OriginMismatch {
                tool,
                expected,
                got,
            } => {
                assert_eq!(tool, "browser::youtube::get_transcript");
                assert_eq!(expected, "Browser");
                assert_eq!(got, "Focused");
            }
            other => panic!("expected OriginMismatch, got {other:?}"),
        }
    }

    #[test]
    fn forward_unit_variants() {
        assert!(matches!(
            ToolErrorWire::from(ToolError::Timeout),
            ToolErrorWire::Timeout
        ));
        assert!(matches!(
            ToolErrorWire::from(ToolError::Cancelled),
            ToolErrorWire::Cancelled
        ));
    }

    #[test]
    fn forward_transport_preserves_message() {
        let wire: ToolErrorWire = ToolError::Transport(Cow::Borrowed("ws closed")).into();
        match wire {
            ToolErrorWire::Transport { message } => assert_eq!(message, "ws closed"),
            other => panic!("expected Transport, got {other:?}"),
        }
    }

    #[test]
    fn forward_remote_preserves_payload() {
        let wire: ToolErrorWire = ToolError::Remote {
            code: 410,
            message: "tab gone".into(),
            details: Some(json!({"hint": "user closed tab"})),
        }
        .into();
        match wire {
            ToolErrorWire::Remote {
                code,
                message,
                details,
            } => {
                assert_eq!(code, 410);
                assert_eq!(message, "tab gone");
                assert_eq!(details, Some(json!({"hint": "user closed tab"})));
            }
            other => panic!("expected Remote, got {other:?}"),
        }
    }

    #[test]
    fn forward_decode_renders_serde_message() {
        let wire: ToolErrorWire = ToolError::decode(decode_err()).into();
        match wire {
            ToolErrorWire::Decode { message } => {
                assert!(!message.is_empty(), "decode message should not be empty");
            }
            other => panic!("expected Decode, got {other:?}"),
        }
    }

    #[test]
    fn forward_encode_renders_serde_message() {
        let wire: ToolErrorWire = ToolError::encode(decode_err()).into();
        match wire {
            ToolErrorWire::Encode { message } => {
                assert!(!message.is_empty());
            }
            other => panic!("expected Encode, got {other:?}"),
        }
    }

    #[test]
    fn forward_adapter_renders_display() {
        #[derive(Debug)]
        struct AdapterCause;
        impl std::fmt::Display for AdapterCause {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("youtube API rate-limited")
            }
        }
        impl std::error::Error for AdapterCause {}

        let wire: ToolErrorWire = ToolError::adapter(AdapterCause).into();
        match wire {
            ToolErrorWire::Adapter { message } => {
                assert_eq!(message, "youtube API rate-limited");
            }
            other => panic!("expected Adapter, got {other:?}"),
        }
    }

    /// Locally-constructed `Decode` errors expose the original
    /// `serde_json::Error` via `source()`.
    #[test]
    fn decode_preserves_source_locally() {
        let err = ToolError::decode(decode_err());
        let source = std::error::Error::source(&err).expect("decode must keep its serde source");
        assert!(source.downcast_ref::<serde_json::Error>().is_some());
    }

    /// Same guarantee for `Encode`.
    #[test]
    fn encode_preserves_source_locally() {
        let err = ToolError::encode(decode_err());
        let source = std::error::Error::source(&err).expect("encode must keep its serde source");
        assert!(source.downcast_ref::<serde_json::Error>().is_some());
    }

    /// And for `Adapter`.
    #[test]
    fn adapter_preserves_source_locally() {
        #[derive(Debug, thiserror::Error)]
        #[error("rate-limited")]
        struct AdapterCause;

        let err = ToolError::adapter(AdapterCause);
        let source = std::error::Error::source(&err).expect("adapter must keep its boxed source");
        assert!(source.downcast_ref::<AdapterCause>().is_some());
    }

    #[test]
    fn reverse_context_unavailable_round_trips() {
        let original = ToolError::ContextUnavailable {
            tool: Cow::Borrowed("browser::youtube::get_transcript"),
            reason: Cow::Borrowed("no active youtube tab"),
        };
        let wire: ToolErrorWire = original.into();
        let back: ToolError = wire.into();
        match back {
            ToolError::ContextUnavailable { tool, reason } => {
                assert_eq!(tool, "browser::youtube::get_transcript");
                assert_eq!(reason, "no active youtube tab");
            }
            other => panic!("expected ContextUnavailable, got {other:?}"),
        }
    }

    #[test]
    fn reverse_origin_mismatch_round_trips() {
        let original = ToolError::OriginMismatch {
            tool: Cow::Borrowed("browser::youtube::get_transcript"),
            expected: Cow::Borrowed("Browser"),
            got: Cow::Borrowed("Focused"),
        };
        let wire: ToolErrorWire = original.into();
        let back: ToolError = wire.into();
        match back {
            ToolError::OriginMismatch {
                tool,
                expected,
                got,
            } => {
                assert_eq!(tool, "browser::youtube::get_transcript");
                assert_eq!(expected, "Browser");
                assert_eq!(got, "Focused");
            }
            other => panic!("expected OriginMismatch, got {other:?}"),
        }
    }

    #[test]
    fn reverse_unit_variants() {
        assert!(matches!(
            ToolError::from(ToolErrorWire::Timeout),
            ToolError::Timeout
        ));
        assert!(matches!(
            ToolError::from(ToolErrorWire::Cancelled),
            ToolError::Cancelled
        ));
    }

    #[test]
    fn reverse_transport_round_trips() {
        let wire = ToolErrorWire::Transport {
            message: "ws closed".into(),
        };
        let back: ToolError = wire.into();
        match back {
            ToolError::Transport(msg) => assert_eq!(msg.as_ref(), "ws closed"),
            other => panic!("expected Transport, got {other:?}"),
        }
    }

    #[test]
    fn reverse_remote_round_trips() {
        let wire = ToolErrorWire::Remote {
            code: 503,
            message: "upstream unavailable".into(),
            details: Some(json!({"retry_after": 30})),
        };
        let back: ToolError = wire.into();
        match back {
            ToolError::Remote {
                code,
                message,
                details,
            } => {
                assert_eq!(code, 503);
                assert_eq!(message, "upstream unavailable");
                assert_eq!(details, Some(json!({"retry_after": 30})));
            }
            other => panic!("expected Remote, got {other:?}"),
        }
    }

    /// Decode/Encode preserve their kind across a wire round-trip; only
    /// the `source()` chain is dropped (since `serde_json::Error` can't
    /// be reconstructed from a string). The rendered `Display` keeps the
    /// single "failed to decode tool payload: …" prefix — no doubled
    /// "adapter error: decode error: …" framing.
    #[test]
    fn reverse_decode_preserves_kind() {
        let wire = ToolErrorWire::Decode {
            message: "expected number".into(),
        };
        let back: ToolError = wire.into();
        match back {
            ToolError::Decode {
                ref message,
                ref source,
            } => {
                assert_eq!(message, "expected number");
                assert!(source.is_none(), "wire round-trip drops the serde source");
            }
            ref other => panic!("expected Decode (kind preserved), got {other:?}"),
        }
        assert_eq!(back.to_string(), "failed to decode tool payload: expected number");
    }

    #[test]
    fn reverse_encode_preserves_kind() {
        let wire = ToolErrorWire::Encode {
            message: "non-utf8".into(),
        };
        let back: ToolError = wire.into();
        match back {
            ToolError::Encode { message, source } => {
                assert_eq!(message, "non-utf8");
                assert!(source.is_none());
            }
            other => panic!("expected Encode (kind preserved), got {other:?}"),
        }
    }

    #[test]
    fn reverse_adapter_preserves_message_only() {
        let wire = ToolErrorWire::Adapter {
            message: "youtube API rate-limited".into(),
        };
        let back: ToolError = wire.into();
        match back {
            ToolError::Adapter { message, source } => {
                assert_eq!(message, "youtube API rate-limited");
                // Cause chain is gone — only the message survives.
                assert!(source.is_none());
            }
            other => panic!("expected Adapter, got {other:?}"),
        }
    }

    #[test]
    fn display_messages_match_format_strings() {
        let err = ToolError::OriginMismatch {
            tool: Cow::Borrowed("t"),
            expected: Cow::Borrowed("Browser"),
            got: Cow::Borrowed("Focused"),
        };
        assert_eq!(
            err.to_string(),
            "origin mismatch for tool `t`: expected Browser, got Focused"
        );

        let err = ToolError::Remote {
            code: 1,
            message: "x".into(),
            details: None,
        };
        assert_eq!(err.to_string(), "remote error 1: x");
    }

    /// Compile-time check: `ToolError` is `Send + Sync` so it can be
    /// returned across `.await` and stored in shared state.
    #[test]
    fn tool_error_is_send_and_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<ToolError>();
    }
}
