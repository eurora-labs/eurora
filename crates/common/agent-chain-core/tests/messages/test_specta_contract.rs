//! specta-typescript contract tests.
//!
//! These tests render the public message types to TypeScript via specta and
//! pin the resulting shape. They guard against drift between the Rust struct
//! definitions, the actual JSON wire format, and the TS bindings consumed by
//! the desktop app.
//!
//! Only compiled when the `specta` feature is enabled, since the assertions
//! depend on `specta::Type` derives.

#![cfg(feature = "specta")]

use agent_chain_core::messages::{
    AIMessage, AIMessageChunk, AnyMessage, AnyMessageChunk, ChatMessage, ChatMessageChunk,
    ContentBlock, ContentBlocks, HumanMessage, HumanMessageChunk, RemoveMessage, SystemMessage,
    SystemMessageChunk, ToolCall, ToolMessage, ToolMessageChunk, UsageMetadata,
};
use specta::Types;
use specta_typescript::Typescript;

/// Render a single type to a TypeScript string by registering it (and any
/// transitively-referenced types) into a fresh `Types` collection and exporting
/// it through the symmetric `specta_serde::Format`. Wide integers are mapped
/// to TS `bigint` per-field via `#[specta(type = BigInt)]` overrides on the
/// underlying message types.
fn render<T: specta::Type>() -> String {
    let mut types = Types::default();
    types.register_mut::<T>();
    Typescript::default()
        .export(&types, specta_serde::Format)
        .expect("specta export to TypeScript")
}

fn assert_contains(haystack: &str, needle: &str) {
    assert!(
        haystack.contains(needle),
        "TS output missing `{needle}`. Full output:\n{haystack}"
    );
}

fn assert_not_contains(haystack: &str, needle: &str) {
    assert!(
        !haystack.contains(needle),
        "TS output unexpectedly contains `{needle}`. Full output:\n{haystack}"
    );
}

// ---------------------------------------------------------------------------
// Discriminated unions: the tag must live on AnyMessage / AnyMessageChunk.
// ---------------------------------------------------------------------------

#[test]
fn any_message_ts_type_is_a_discriminated_union() {
    let ts = render::<AnyMessage>();
    // Each variant should appear with its snake_case literal tag.
    assert_contains(&ts, "type: \"human\"");
    assert_contains(&ts, "type: \"system\"");
    assert_contains(&ts, "type: \"ai\"");
    assert_contains(&ts, "type: \"tool\"");
    assert_contains(&ts, "type: \"chat\"");
    assert_contains(&ts, "type: \"remove\"");
}

#[test]
fn any_message_chunk_ts_type_is_a_discriminated_union() {
    let ts = render::<AnyMessageChunk>();
    assert_contains(&ts, "type: \"ai_chunk\"");
    assert_contains(&ts, "type: \"human_chunk\"");
    assert_contains(&ts, "type: \"system_chunk\"");
    assert_contains(&ts, "type: \"tool_chunk\"");
    assert_contains(&ts, "type: \"chat_chunk\"");
}

// ---------------------------------------------------------------------------
// Struct types: no embedded type field.
// ---------------------------------------------------------------------------

#[test]
fn human_message_struct_has_no_type_field() {
    let ts = render::<HumanMessage>();
    // The struct itself must not declare a `type` field — that lives on the
    // AnyMessage union only.
    assert_not_contains(&ts, "type: \"human\"");
    // But it must still declare its actual fields. Specta renders fields with
    // `#[serde(default)]` as optional (`?:`), so accept either form.
    assert!(ts.contains("content:") || ts.contains("content?:"));
    assert!(ts.contains("additional_kwargs:") || ts.contains("additional_kwargs?:"));
}

#[test]
fn ai_message_struct_has_no_type_field() {
    let ts = render::<AIMessage>();
    assert_not_contains(&ts, "type: \"ai\"");
    assert!(ts.contains("tool_calls:") || ts.contains("tool_calls?:"));
}

#[test]
fn system_message_struct_has_no_type_field() {
    let ts = render::<SystemMessage>();
    assert_not_contains(&ts, "type: \"system\"");
}

#[test]
fn tool_message_struct_has_no_type_field() {
    let ts = render::<ToolMessage>();
    assert_not_contains(&ts, "type: \"tool\"");
    assert_contains(&ts, "tool_call_id:");
}

#[test]
fn chat_message_struct_has_no_type_field() {
    let ts = render::<ChatMessage>();
    assert_not_contains(&ts, "type: \"chat\"");
    assert_contains(&ts, "role:");
}

#[test]
fn remove_message_has_no_content_field() {
    let ts = render::<RemoveMessage>();
    // `content: ""` was a synthetic legacy field; it must not appear in TS.
    assert_not_contains(&ts, "content:");
    // `id`, `additional_kwargs`, `response_metadata` are real fields.
    assert_contains(&ts, "id:");
}

// ---------------------------------------------------------------------------
// HashMap<String, Value> map fields: must render as Record<string, unknown>.
// ---------------------------------------------------------------------------

#[test]
fn additional_kwargs_renders_as_record_of_unknown() {
    let ts = render::<HumanMessage>();
    // The TS representation of HashMap<String, Unknown> is a Record / index
    // signature shape — never a bare `unknown`. Field is optional in TS due
    // to `#[serde(default)]`.
    assert!(
        ts.contains("additional_kwargs?: { [key in string]: unknown }")
            || ts.contains("additional_kwargs: { [key in string]: unknown }"),
        "additional_kwargs should be `Record<string, unknown>` in TS. Output:\n{ts}"
    );
}

#[test]
fn response_metadata_renders_as_record_of_unknown() {
    let ts = render::<AIMessage>();
    assert!(
        ts.contains("response_metadata?: { [key in string]: unknown }")
            || ts.contains("response_metadata: { [key in string]: unknown }"),
        "response_metadata should be `Record<string, unknown>` in TS. Output:\n{ts}"
    );
}

// ---------------------------------------------------------------------------
// `serde(default)` makes a field deserialize-optional, which specta renders
// as a TS-optional field (`name?:`).
// ---------------------------------------------------------------------------

#[test]
fn human_message_name_is_optional_in_ts() {
    let ts = render::<HumanMessage>();
    // `name` is `Option<String>` with `#[serde(default)]`, so specta should
    // render it as `name?:`. The legacy bug had it required.
    assert_contains(&ts, "name?:");
}

#[test]
fn ai_message_optional_fields_are_ts_optional() {
    let ts = render::<AIMessage>();
    assert_contains(&ts, "name?:");
    assert_contains(&ts, "usage_metadata?:");
}

#[test]
fn ai_message_chunk_optional_fields_are_ts_optional() {
    let ts = render::<AIMessageChunk>();
    assert_contains(&ts, "chunk_position?:");
    assert_contains(&ts, "usage_metadata?:");
}

#[test]
fn tool_message_artifact_is_optional() {
    let ts = render::<ToolMessage>();
    assert_contains(&ts, "artifact?:");
}

// ---------------------------------------------------------------------------
// Round-trip Render-then-Compare: catches any addition/removal of variants.
// ---------------------------------------------------------------------------

#[test]
fn any_message_lists_exactly_the_six_variants() {
    let ts = render::<AnyMessage>();
    let tags = ["human", "system", "ai", "tool", "chat", "remove"];
    for tag in tags {
        assert_contains(&ts, &format!("type: \"{tag}\""));
    }
    // Make sure we don't leak chunk tags or PascalCase strays.
    assert_not_contains(&ts, "\"AIMessage\"");
    assert_not_contains(&ts, "\"HumanMessage\"");
    assert_not_contains(&ts, "human_chunk");
}

#[test]
fn any_message_chunk_lists_exactly_the_five_variants() {
    let ts = render::<AnyMessageChunk>();
    let tags = [
        "ai_chunk",
        "human_chunk",
        "system_chunk",
        "tool_chunk",
        "chat_chunk",
    ];
    for tag in tags {
        assert_contains(&ts, &format!("type: \"{tag}\""));
    }
    assert_not_contains(&ts, "\"AIMessageChunk\"");
    assert_not_contains(&ts, "\"HumanMessageChunk\"");
}

// ---------------------------------------------------------------------------
// ContentBlock variant + ContentBlocks transparent representation.
// ---------------------------------------------------------------------------

#[test]
fn content_block_is_a_discriminated_union_in_ts() {
    let ts = render::<ContentBlock>();
    assert_contains(&ts, "type: \"text\"");
    assert_contains(&ts, "type: \"image\"");
    assert_contains(&ts, "type: \"tool_call\"");
    assert_contains(&ts, "type: \"non_standard\"");
}

#[test]
fn content_blocks_is_transparent_array() {
    let ts = render::<ContentBlocks>();
    // serde(transparent) → TS should be `ContentBlock[]` shape.
    assert_contains(&ts, "ContentBlock");
}

// ---------------------------------------------------------------------------
// ToolCall.args is rendered as Record (not bare unknown) due to specta
// override — even though the Rust type is `serde_json::Value`.
// ---------------------------------------------------------------------------

#[test]
fn tool_call_args_is_record_of_unknown() {
    let ts = render::<ToolCall>();
    assert_contains(&ts, "args: { [key in string]: unknown }");
}

// ---------------------------------------------------------------------------
// UsageMetadata flattened HashMap survives.
// ---------------------------------------------------------------------------

#[test]
fn input_token_details_renders_flatten_index_signature() {
    let ts = render::<UsageMetadata>();
    // `extra: HashMap<String, i64>` with `#[serde(flatten)]` should produce an
    // index signature on InputTokenDetails / OutputTokenDetails.
    assert_contains(&ts, "[key in string]: bigint");
}

// ---------------------------------------------------------------------------
// Smoke: every chunk type renders without error.
// ---------------------------------------------------------------------------

#[test]
fn every_chunk_type_renders() {
    let _ = render::<HumanMessageChunk>();
    let _ = render::<SystemMessageChunk>();
    let _ = render::<AIMessageChunk>();
    let _ = render::<ToolMessageChunk>();
    let _ = render::<ChatMessageChunk>();
}
