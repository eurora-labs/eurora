//! Swift bindings for the bridge wire types, emitted into
//! `apps/macos/Shared/BridgeProtocol.swift`.
//!
//! This module is gated behind the `codegen` feature and called from
//! the workspace-level `euro-codegen` orchestrator. There is no
//! standalone binary — the orchestrator is the only entry point so
//! every binding regenerates together.

use anyhow::{Context, Result};
use specta_swift::{NamingConvention, Swift};
use std::fs;

use crate::type_collection;

const SWIFT_OUT: &str = "apps/macos/Shared/BridgeProtocol.swift";

/// Generate the Swift bindings and write them to [`SWIFT_OUT`].
pub fn run() -> Result<()> {
    let types = type_collection();

    let swift = Swift::default()
        .naming(NamingConvention::PascalCase)
        .export(&types, specta_serde::Format)
        .context("exporting Swift bindings")?;

    let processed = substitute_payload(&collapse_double_optional(&swift));
    fs::write(SWIFT_OUT, processed).context("writing Swift bindings")?;
    println!("wrote {SWIFT_OUT}");

    Ok(())
}

/// Collapse `T??` → `T?` in field declarations.
///
/// specta-serde models `Option<T>` plus `#[serde(default)]` as a nested
/// optional (the field can be missing AND its value can be JSON null), and
/// specta-swift 0.0.3 renders both layers verbatim — `public let foo: T??`.
/// Swift's synthesized `Decodable` already treats a missing key on an
/// `Optional` property as `nil`, so the inner `?` is redundant and `T??`
/// is just an unidiomatic spelling of `T?`. specta-typescript handles the
/// same shape correctly (`foo?: T | null`); Swift does not, so we collapse
/// it here. Drop this once specta-swift learns the same trick.
///
/// `??` is the Swift nil-coalescing operator, but the Specta exporter only
/// emits type declarations, never expressions, so `??` in the generated
/// output is unambiguously the double-Optional bug.
fn collapse_double_optional(input: &str) -> String {
    input.replace("??", "?")
}

/// Hand-rolled JSON-value `Codable` enum that replaces the empty
/// `Payload` struct specta emits.
///
/// The Rust [`crate::frame::Payload`] is a
/// `Box<serde_json::value::RawValue>` — "any JSON value". Specta-swift
/// can't render that polymorphic shape (no `Any`-style native and no
/// `serde_json::Value` because of Chrome native-messaging's no-BigInt
/// constraint), so the frame module emits a hidden empty struct named
/// `Payload`. We swap that empty body here for an enum that round-trips
/// every JSON value the wire actually carries.
///
/// Numbers are decoded as `Double` only — the wire never sees BigInts
/// (Chrome native-messaging on stdio refuses to round-trip them), so a
/// single `Double` case is sufficient and matches the constraints of
/// the TypeScript side.
const PAYLOAD_REPLACEMENT: &str =
    "/// Inline JSON payload carried by Request/Response/Event frames.
///
/// Mirrors `euro_bridge_protocol::frame::Payload` on the wire: any JSON
/// value (`null`, `Bool`, `Double`, `String`, array, object) embedded
/// inline in the outer Frame envelope. The Rust producer hands the
/// payload to serde as a `RawValue`, so consumers see the original
/// shape rather than an escaped JSON string layer.
///
/// Numbers are decoded as `Double` only — Chrome's native-messaging
/// bridge cannot round-trip 64-bit integers safely, so the wire never
/// carries one. Use `.number(Double)` for any numeric value.
public enum Payload: Codable {
    case null
    case bool(Bool)
    case number(Double)
    case string(String)
    case array([Payload])
    case object([String: Payload])

    public init(from decoder: Decoder) throws {
        let container = try decoder.singleValueContainer()
        if container.decodeNil() {
            self = .null
            return
        }
        if let value = try? container.decode(Bool.self) {
            self = .bool(value)
            return
        }
        if let value = try? container.decode(Double.self) {
            self = .number(value)
            return
        }
        if let value = try? container.decode(String.self) {
            self = .string(value)
            return
        }
        if let value = try? container.decode([Payload].self) {
            self = .array(value)
            return
        }
        if let value = try? container.decode([String: Payload].self) {
            self = .object(value)
            return
        }
        throw DecodingError.dataCorruptedError(
            in: container,
            debugDescription: \"Payload: value did not match any known JSON shape\"
        )
    }

    public func encode(to encoder: Encoder) throws {
        var container = encoder.singleValueContainer()
        switch self {
        case .null:
            try container.encodeNil()
        case .bool(let value):
            try container.encode(value)
        case .number(let value):
            try container.encode(value)
        case .string(let value):
            try container.encode(value)
        case .array(let value):
            try container.encode(value)
        case .object(let value):
            try container.encode(value)
        }
    }
}";

/// Replace the empty `Payload` struct specta emits — together with its
/// preceding rustdoc-derived `///` block — with the hand-rolled
/// JSON-value `Codable` enum in [`PAYLOAD_REPLACEMENT`].
///
/// The frame module's `Payload` is `specta(skip)`'d under the codegen
/// feature, which makes specta-swift emit:
///
/// ```text
/// /// <some rustdoc>
/// public struct Payload: Codable {
/// Void}
/// ```
///
/// (or a comparable empty form). We rewind from the struct declaration
/// through any `///` doc lines so the substitution can drop the
/// upstream comments — [`PAYLOAD_REPLACEMENT`] carries its own
/// `///`-formatted documentation.
///
/// Panics if the placeholder isn't present so a future specta upgrade
/// that changes the empty-struct shape fails the codegen loudly instead
/// of silently emitting an unusable `Payload` definition.
fn substitute_payload(input: &str) -> String {
    let marker = "public struct Payload: Codable {";
    let struct_start = input
        .find(marker)
        .expect("codegen: expected specta to emit `public struct Payload: Codable { ... }`");
    // Rewind line-by-line over any `///` doc comments immediately above
    // the struct so we don't leave the upstream rustdoc orphaned. Stop
    // at the first non-doc line; that's where our replacement begins.
    let start = doc_block_start_before(input, struct_start);

    // Find the matching closing brace at the same nesting level. The
    // generated empty struct has no nested braces, so the next `}` after
    // the marker closes it.
    let after_marker = struct_start + marker.len();
    let close_offset = input[after_marker..]
        .find('}')
        .expect("codegen: expected closing brace for `Payload` struct");
    let end = after_marker + close_offset + 1;

    let mut out = String::with_capacity(input.len() + PAYLOAD_REPLACEMENT.len());
    out.push_str(&input[..start]);
    out.push_str(PAYLOAD_REPLACEMENT);
    out.push_str(&input[end..]);
    out
}

/// Walk backwards from `pos` skipping over `///`-prefixed lines (after
/// any leading whitespace) and return the byte offset where the doc
/// block begins. If no doc block precedes `pos`, returns `pos`
/// unchanged.
///
/// `pos` is expected to point at the start of a line — the function
/// only inspects whole lines that fully precede `pos`.
fn doc_block_start_before(input: &str, pos: usize) -> usize {
    let mut block_start = pos;
    loop {
        // The line ending immediately before `block_start` (if any).
        // After consuming the `\n`, `line_end` is the exclusive end of
        // the previous line's content.
        let Some(line_end) = block_start
            .checked_sub(1)
            .filter(|i| input.as_bytes().get(*i) == Some(&b'\n'))
        else {
            // No newline immediately above — we're at the top of the
            // file, or the byte preceding `block_start` isn't a `\n`.
            return block_start;
        };
        let line_start = input[..line_end].rfind('\n').map(|i| i + 1).unwrap_or(0);
        let line = &input[line_start..line_end];
        if !line.trim_start().starts_with("///") {
            return block_start;
        }
        block_start = line_start;
    }
}

#[cfg(test)]
mod tests {
    use super::{PAYLOAD_REPLACEMENT, collapse_double_optional, substitute_payload};

    #[test]
    fn collapses_double_optional_in_field_decl() {
        let input = "    public let payload: String??\n";
        assert_eq!(
            collapse_double_optional(input),
            "    public let payload: String?\n"
        );
    }

    #[test]
    fn collapses_double_optional_on_named_types() {
        let input = "    public let asset: ArticleAsset??\n";
        assert_eq!(
            collapse_double_optional(input),
            "    public let asset: ArticleAsset?\n"
        );
    }

    #[test]
    fn leaves_single_optional_alone() {
        let input = "    public let payload: String?\n";
        assert_eq!(collapse_double_optional(input), input);
    }

    #[test]
    fn substitutes_empty_payload_struct() {
        let input = "// preamble\n\
                     public struct Payload: Codable {\n\
                     Void}\n\
                     \n\
                     public struct After: Codable {}\n";
        let out = substitute_payload(input);
        assert!(out.contains("public enum Payload: Codable"));
        assert!(out.contains("public struct After: Codable"));
        // The empty placeholder body is gone.
        assert!(!out.contains("public struct Payload: Codable {\nVoid"));
    }

    #[test]
    fn substitutes_with_alternative_empty_body() {
        // specta may render the skip'd struct without the `Void`
        // placeholder in a future release; the substitution must still
        // find and replace the empty form.
        let input = "public struct Payload: Codable {}\n\npublic struct After: Codable {}\n";
        let out = substitute_payload(input);
        assert!(out.contains("public enum Payload: Codable"));
        assert!(out.contains("public struct After: Codable"));
    }

    #[test]
    fn strips_preceding_doc_comment_block() {
        // Mirrors the real specta-swift output: rustdoc lines render as
        // `///` comments above the struct. The substitution drops them
        // — `PAYLOAD_REPLACEMENT` carries its own bespoke documentation.
        let input = "public struct Before: Codable {}\n\
                     \n\
                     /// upstream rustdoc line 1\n\
                     /// upstream rustdoc line 2\n\
                     public struct Payload: Codable {\n\
                     Void}\n\
                     \n\
                     public struct After: Codable {}\n";
        let out = substitute_payload(input);
        assert!(out.contains("public struct Before: Codable"));
        assert!(out.contains("public enum Payload: Codable"));
        assert!(out.contains("public struct After: Codable"));
        assert!(
            !out.contains("upstream rustdoc"),
            "preceding doc comments should be stripped:\n{out}"
        );
    }

    #[test]
    #[should_panic(expected = "expected specta to emit")]
    fn panics_when_payload_struct_is_absent() {
        substitute_payload("public struct Other: Codable {}\n");
    }

    #[test]
    fn payload_replacement_has_balanced_braces() {
        let opens = PAYLOAD_REPLACEMENT.matches('{').count();
        let closes = PAYLOAD_REPLACEMENT.matches('}').count();
        assert_eq!(opens, closes, "Payload replacement braces unbalanced");
    }
}
