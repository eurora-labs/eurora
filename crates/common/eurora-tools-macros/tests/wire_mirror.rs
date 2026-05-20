//! Runtime integration tests for `#[derive(WireMirror)]`.
//!
//! The derive's primary user is `eurora_tools::ToolError` (exercised by
//! that crate's own tests). The cases here pin the *general* behaviour
//! against a focused source/wire pair so future macro tweaks regress
//! against something simpler than the full `ToolError`.

use std::borrow::Cow;

use eurora_tools_macros::WireMirror;

/// User-maintained wire enum, defined in a sibling module to mimic the
/// production split (where `ToolErrorWire` lives in `thread-core` and
/// `ToolError` lives in `eurora-tools`).
pub mod wire {
    #[derive(Debug, Clone, PartialEq)]
    #[non_exhaustive]
    pub enum Wire {
        Unit,
        WithText { message: String },
        WithMulti { tool: String, reason: String },
        WithKeptField { code: u32, message: String },
    }
}

#[derive(Debug, WireMirror)]
#[wire_mirror(
    target = "crate::wire::Wire",
    catch_all = "WithText",
    catch_all_message = "unknown variant: {variant:?}"
)]
#[non_exhaustive]
pub enum Source {
    Unit,
    WithText {
        message: Cow<'static, str>,
    },
    WithMulti {
        tool: Cow<'static, str>,
        reason: Cow<'static, str>,
    },
    WithKeptField {
        code: u32,
        message: Cow<'static, str>,
        #[wire_mirror(skip)]
        cause: Option<String>,
    },
}

#[test]
fn forward_unit_round_trip() {
    let wire: wire::Wire = Source::Unit.into();
    assert_eq!(wire, wire::Wire::Unit);
}

#[test]
fn forward_cow_field_becomes_owned_string() {
    let wire: wire::Wire = Source::WithText {
        message: Cow::Borrowed("borrowed"),
    }
    .into();
    assert_eq!(
        wire,
        wire::Wire::WithText {
            message: "borrowed".to_string(),
        }
    );
}

#[test]
fn forward_multiple_cow_fields() {
    let wire: wire::Wire = Source::WithMulti {
        tool: Cow::Borrowed("t"),
        reason: Cow::Borrowed("r"),
    }
    .into();
    assert_eq!(
        wire,
        wire::Wire::WithMulti {
            tool: "t".into(),
            reason: "r".into(),
        }
    );
}

#[test]
fn forward_skip_drops_field_in_wire() {
    let wire: wire::Wire = Source::WithKeptField {
        code: 7,
        message: Cow::Borrowed("hi"),
        cause: Some("important context".into()),
    }
    .into();
    // Skipped field is gone on the wire side; non-skipped fields survive.
    assert_eq!(
        wire,
        wire::Wire::WithKeptField {
            code: 7,
            message: "hi".into(),
        }
    );
}

#[test]
fn reverse_unit_round_trip() {
    let back: Source = wire::Wire::Unit.into();
    assert!(matches!(back, Source::Unit));
}

#[test]
fn reverse_cow_field_becomes_cow_owned() {
    let back: Source = wire::Wire::WithText {
        message: "hi".into(),
    }
    .into();
    match back {
        Source::WithText { message } => {
            assert_eq!(message.as_ref(), "hi");
            // Newly Owned, not Borrowed — round-trip is lossy for the
            // borrowed lifetime but lossless for the data.
            assert!(matches!(message, Cow::Owned(_)));
        }
        other => panic!("expected WithText, got {other:?}"),
    }
}

#[test]
fn reverse_skip_rebuilds_with_default() {
    let back: Source = wire::Wire::WithKeptField {
        code: 200,
        message: "ok".into(),
    }
    .into();
    match back {
        Source::WithKeptField {
            code,
            message,
            cause,
        } => {
            assert_eq!(code, 200);
            assert_eq!(message.as_ref(), "ok");
            assert!(cause.is_none(), "skipped field is rebuilt via Default");
        }
        other => panic!("expected WithKeptField, got {other:?}"),
    }
}
