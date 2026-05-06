//! Wire-format contract tests.
//!
//! These tests pin down the JSON shape that crosses the wire so that no future
//! change can silently move a discriminant, rename a field, or drop a key.
//!
//! Two complementary checks per type:
//! 1. **Bare-message JSON**: the struct's own `serde_json::to_value` shape.
//!    Crucially, no `"type"` field — the discriminant lives only on the
//!    `AnyMessage` / `AnyMessageChunk` enums.
//! 2. **Wrapped JSON**: the same data inside `AnyMessage::*` / `AnyMessageChunk::*`,
//!    which adds the snake_case discriminant tag.
//!
//! If you intentionally change the wire format, update the assertions here in
//! the same commit so the contract is documented.

use agent_chain_core::messages::{
    AIMessage, AIMessageChunk, AnyMessage, AnyMessageChunk, ChatMessage, ChatMessageChunk,
    HumanMessage, HumanMessageChunk, RemoveMessage, SystemMessage, SystemMessageChunk, ToolMessage,
    ToolMessageChunk,
};
use std::collections::BTreeSet;

fn keys(value: &serde_json::Value) -> BTreeSet<String> {
    value
        .as_object()
        .expect("expected JSON object")
        .keys()
        .cloned()
        .collect()
}

fn expect_keys(value: &serde_json::Value, expected: &[&str]) {
    let actual = keys(value);
    let expected: BTreeSet<String> = expected.iter().map(|s| s.to_string()).collect();
    assert_eq!(
        actual, expected,
        "JSON shape drift detected. Got {actual:?}, expected {expected:?}"
    );
}

// ---------------------------------------------------------------------------
// Bare-message shapes — none should carry "type".
// ---------------------------------------------------------------------------

#[test]
fn bare_human_message_has_no_type_field() {
    let msg = HumanMessage::builder()
        .content("hi")
        .name("alice".to_string())
        .build();
    let v = serde_json::to_value(&msg).unwrap();
    expect_keys(
        &v,
        &[
            "content",
            "id",
            "name",
            "additional_kwargs",
            "response_metadata",
        ],
    );
    assert!(v.get("type").is_none());
}

#[test]
fn bare_human_message_omits_name_when_none() {
    let msg = HumanMessage::builder().content("hi").build();
    let v = serde_json::to_value(&msg).unwrap();
    expect_keys(
        &v,
        &["content", "id", "additional_kwargs", "response_metadata"],
    );
}

#[test]
fn bare_system_message_has_no_type_field() {
    let msg = SystemMessage::builder().content("be helpful").build();
    let v = serde_json::to_value(&msg).unwrap();
    assert!(v.get("type").is_none());
    expect_keys(
        &v,
        &["content", "id", "additional_kwargs", "response_metadata"],
    );
}

#[test]
fn bare_ai_message_has_no_type_field() {
    let msg = AIMessage::builder().content("answer").build();
    let v = serde_json::to_value(&msg).unwrap();
    assert!(v.get("type").is_none());
    expect_keys(
        &v,
        &[
            "content",
            "id",
            "tool_calls",
            "invalid_tool_calls",
            "additional_kwargs",
            "response_metadata",
        ],
    );
}

#[test]
fn bare_tool_message_has_no_type_field() {
    let msg = ToolMessage::builder()
        .content("ok")
        .tool_call_id("call-1")
        .build();
    let v = serde_json::to_value(&msg).unwrap();
    assert!(v.get("type").is_none());
    expect_keys(
        &v,
        &[
            "content",
            "tool_call_id",
            "id",
            "status",
            "additional_kwargs",
            "response_metadata",
        ],
    );
}

#[test]
fn bare_chat_message_has_no_type_field() {
    let msg = ChatMessage::builder().content("hi").role("user").build();
    let v = serde_json::to_value(&msg).unwrap();
    assert!(v.get("type").is_none());
    expect_keys(
        &v,
        &[
            "content",
            "role",
            "id",
            "additional_kwargs",
            "response_metadata",
        ],
    );
}

#[test]
fn bare_remove_message_has_no_type_or_content_field() {
    let msg = RemoveMessage::builder().id("msg-1").build();
    let v = serde_json::to_value(&msg).unwrap();
    assert!(v.get("type").is_none());
    // RemoveMessage has no content in its struct; the legacy synthetic `""`
    // emitted by the old custom Serialize is gone.
    assert!(v.get("content").is_none());
    expect_keys(&v, &["id", "additional_kwargs", "response_metadata"]);
}

// ---------------------------------------------------------------------------
// AnyMessage union — discriminant lives here.
// ---------------------------------------------------------------------------

#[test]
fn any_message_human_tag_is_snake_case() {
    let any = AnyMessage::HumanMessage(HumanMessage::builder().content("hi").build());
    let v = serde_json::to_value(&any).unwrap();
    assert_eq!(v["type"], "human");
}

#[test]
fn any_message_system_tag_is_snake_case() {
    let any = AnyMessage::SystemMessage(SystemMessage::builder().content("hi").build());
    let v = serde_json::to_value(&any).unwrap();
    assert_eq!(v["type"], "system");
}

#[test]
fn any_message_ai_tag_is_snake_case() {
    let any = AnyMessage::AIMessage(AIMessage::builder().content("hi").build());
    let v = serde_json::to_value(&any).unwrap();
    assert_eq!(v["type"], "ai");
}

#[test]
fn any_message_tool_tag_is_snake_case() {
    let any = AnyMessage::ToolMessage(
        ToolMessage::builder()
            .content("ok")
            .tool_call_id("call-1")
            .build(),
    );
    let v = serde_json::to_value(&any).unwrap();
    assert_eq!(v["type"], "tool");
}

#[test]
fn any_message_chat_tag_is_snake_case() {
    let any = AnyMessage::ChatMessage(ChatMessage::builder().content("hi").role("user").build());
    let v = serde_json::to_value(&any).unwrap();
    assert_eq!(v["type"], "chat");
}

#[test]
fn any_message_remove_tag_is_snake_case() {
    let any = AnyMessage::RemoveMessage(RemoveMessage::builder().id("msg-1").build());
    let v = serde_json::to_value(&any).unwrap();
    assert_eq!(v["type"], "remove");
}

#[test]
fn any_message_roundtrips_for_every_variant() {
    let cases = [
        AnyMessage::HumanMessage(HumanMessage::builder().content("h").build()),
        AnyMessage::SystemMessage(SystemMessage::builder().content("s").build()),
        AnyMessage::AIMessage(AIMessage::builder().content("a").build()),
        AnyMessage::ToolMessage(
            ToolMessage::builder()
                .content("t")
                .tool_call_id("tc-1")
                .build(),
        ),
        AnyMessage::ChatMessage(ChatMessage::builder().content("c").role("user").build()),
        AnyMessage::RemoveMessage(RemoveMessage::builder().id("m-1").build()),
    ];

    for case in &cases {
        let v = serde_json::to_value(case).unwrap();
        let back: AnyMessage = serde_json::from_value(v.clone()).unwrap();
        assert_eq!(&back, case, "roundtrip mismatch for {:?}", case);
    }
}

// ---------------------------------------------------------------------------
// Chunk wire format — symmetric snake_case scheme.
// ---------------------------------------------------------------------------

#[test]
fn any_message_chunk_human_tag_is_snake_case() {
    let chunk =
        AnyMessageChunk::HumanMessageChunk(HumanMessageChunk::builder().content("hi").build());
    let v = serde_json::to_value(&chunk).unwrap();
    assert_eq!(v["type"], "human_chunk");
}

#[test]
fn any_message_chunk_system_tag_is_snake_case() {
    let chunk =
        AnyMessageChunk::SystemMessageChunk(SystemMessageChunk::builder().content("hi").build());
    let v = serde_json::to_value(&chunk).unwrap();
    assert_eq!(v["type"], "system_chunk");
}

#[test]
fn any_message_chunk_ai_tag_is_snake_case() {
    let chunk = AnyMessageChunk::AIMessageChunk(AIMessageChunk::builder().content("hi").build());
    let v = serde_json::to_value(&chunk).unwrap();
    assert_eq!(v["type"], "ai_chunk");
}

#[test]
fn any_message_chunk_tool_tag_is_snake_case() {
    let chunk = AnyMessageChunk::ToolMessageChunk(
        ToolMessageChunk::builder()
            .content("ok")
            .tool_call_id("call-1")
            .build(),
    );
    let v = serde_json::to_value(&chunk).unwrap();
    assert_eq!(v["type"], "tool_chunk");
}

#[test]
fn any_message_chunk_chat_tag_is_snake_case() {
    let chunk = AnyMessageChunk::ChatMessageChunk(
        ChatMessageChunk::builder()
            .content("hi")
            .role("user")
            .build(),
    );
    let v = serde_json::to_value(&chunk).unwrap();
    assert_eq!(v["type"], "chat_chunk");
}

#[test]
fn any_message_chunk_roundtrips_for_every_variant() {
    let cases = [
        AnyMessageChunk::AIMessageChunk(AIMessageChunk::builder().content("a").build()),
        AnyMessageChunk::HumanMessageChunk(HumanMessageChunk::builder().content("h").build()),
        AnyMessageChunk::SystemMessageChunk(SystemMessageChunk::builder().content("s").build()),
        AnyMessageChunk::ToolMessageChunk(
            ToolMessageChunk::builder()
                .content("t")
                .tool_call_id("tc-1")
                .build(),
        ),
        AnyMessageChunk::ChatMessageChunk(
            ChatMessageChunk::builder()
                .content("c")
                .role("user")
                .build(),
        ),
    ];
    for case in &cases {
        let v = serde_json::to_value(case).unwrap();
        let back: AnyMessageChunk = serde_json::from_value(v.clone()).unwrap();
        assert_eq!(&back, case, "chunk roundtrip mismatch for {:?}", case);
    }
}

// ---------------------------------------------------------------------------
// Bare struct deserialize accepts JSON without the discriminant.
// ---------------------------------------------------------------------------

#[test]
fn bare_human_message_deserializes_without_type_field() {
    let json = serde_json::json!({
        "content": "hi",
        "id": null,
        "additional_kwargs": {},
        "response_metadata": {}
    });
    let msg: HumanMessage = serde_json::from_value(json).unwrap();
    assert_eq!(msg.content.as_text(), "hi");
}

#[test]
fn bare_human_message_deserializes_with_type_field_ignored() {
    // Old wire format compatibility: if a "type" field is present, it's
    // silently ignored on the bare-struct deserialize path. This lets old
    // dumps load even though we no longer emit the field.
    let json = serde_json::json!({
        "type": "human",
        "content": "hi",
        "id": null,
        "additional_kwargs": {},
        "response_metadata": {}
    });
    let msg: HumanMessage = serde_json::from_value(json).unwrap();
    assert_eq!(msg.content.as_text(), "hi");
}
