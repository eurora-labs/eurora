//! Tests for content module types and functions.
//!
//! Converted from `langchain/libs/core/tests/unit_tests/messages/test_content.py`

use agent_chain_core::messages::{KNOWN_BLOCK_TYPES, is_data_content_block};
use serde_json::json;

// ============================================================================
// TestKnownBlockTypes - Tests for KNOWN_BLOCK_TYPES constant
// ============================================================================

#[test]
fn test_known_block_types_contains_text() {
    assert!(KNOWN_BLOCK_TYPES.contains(&"text"));
}

#[test]
fn test_known_block_types_contains_reasoning() {
    assert!(KNOWN_BLOCK_TYPES.contains(&"reasoning"));
}

#[test]
fn test_known_block_types_contains_tool_call() {
    assert!(KNOWN_BLOCK_TYPES.contains(&"tool_call"));
}

#[test]
fn test_known_block_types_contains_image() {
    assert!(KNOWN_BLOCK_TYPES.contains(&"image"));
}

#[test]
fn test_known_block_types_contains_audio() {
    assert!(KNOWN_BLOCK_TYPES.contains(&"audio"));
}

#[test]
fn test_known_block_types_contains_file() {
    assert!(KNOWN_BLOCK_TYPES.contains(&"file"));
}

#[test]
fn test_known_block_types_contains_video() {
    assert!(KNOWN_BLOCK_TYPES.contains(&"video"));
}

#[test]
fn test_known_block_types_contains_non_standard() {
    assert!(KNOWN_BLOCK_TYPES.contains(&"non_standard"));
}

// ============================================================================
// TestIsDataContentBlock - Tests for is_data_content_block function
// ============================================================================

#[test]
fn test_image_block_with_url() {
    let block = json!({"type": "image", "url": "https://example.com/image.png"});
    assert!(is_data_content_block(&block));
}

#[test]
fn test_image_block_with_base64() {
    let block = json!({"type": "image", "base64": "dGVzdA==", "mime_type": "image/png"});
    assert!(is_data_content_block(&block));
}

#[test]
fn test_image_block_with_file_id() {
    let block = json!({"type": "image", "file_id": "file-123"});
    assert!(is_data_content_block(&block));
}

#[test]
fn test_audio_block_with_url() {
    let block = json!({"type": "audio", "url": "https://example.com/audio.mp3"});
    assert!(is_data_content_block(&block));
}

#[test]
fn test_audio_block_with_base64() {
    let block = json!({"type": "audio", "base64": "dGVzdA==", "mime_type": "audio/mp3"});
    assert!(is_data_content_block(&block));
}

#[test]
fn test_file_block_with_url() {
    let block = json!({"type": "file", "url": "https://example.com/doc.pdf"});
    assert!(is_data_content_block(&block));
}

#[test]
fn test_file_block_with_base64() {
    let block = json!({"type": "file", "base64": "dGVzdA==", "mime_type": "application/pdf"});
    assert!(is_data_content_block(&block));
}

#[test]
fn test_video_block_with_url() {
    let block = json!({"type": "video", "url": "https://example.com/video.mp4"});
    assert!(is_data_content_block(&block));
}

#[test]
fn test_plaintext_block() {
    let block = json!({"type": "text-plain", "text": "Hello world", "mime_type": "text/plain"});
    assert!(is_data_content_block(&block));
}

#[test]
fn test_text_block_is_not_data_block() {
    let block = json!({"type": "text", "text": "Hello"});
    assert!(!is_data_content_block(&block));
}

#[test]
fn test_tool_call_is_not_data_block() {
    let block = json!({"type": "tool_call", "name": "test", "args": {}, "id": "1"});
    assert!(!is_data_content_block(&block));
}

#[test]
fn test_v0_style_image_block_with_source_type_url() {
    let block =
        json!({"type": "image", "source_type": "url", "url": "https://example.com/img.png"});
    assert!(is_data_content_block(&block));
}

#[test]
fn test_v0_style_image_block_with_source_type_base64() {
    let block = json!({"type": "image", "source_type": "base64", "data": "dGVzdA=="});
    assert!(is_data_content_block(&block));
}

#[test]
fn test_v0_style_block_with_source_type_id() {
    let block = json!({"type": "file", "source_type": "id", "id": "file-123"});
    assert!(is_data_content_block(&block));
}

#[test]
fn test_block_without_type_is_not_data_block() {
    let block = json!({"url": "https://example.com/image.png"});
    assert!(!is_data_content_block(&block));
}

#[test]
fn test_block_with_unknown_type() {
    let block = json!({"type": "custom_type", "url": "https://example.com"});
    assert!(!is_data_content_block(&block));
}

#[test]
fn test_image_block_with_extras() {
    let block = json!({
        "type": "image",
        "base64": "<base64 data>",
        "mime_type": "image/jpeg",
        "extras": "hi"
    });
    assert!(is_data_content_block(&block));
}

#[test]
fn test_image_block_with_cache_control() {
    let block = json!({
        "type": "image",
        "base64": "<base64 data>",
        "mime_type": "image/jpeg",
        "cache_control": {"type": "ephemeral"}
    });
    assert!(is_data_content_block(&block));
}

#[test]
fn test_image_block_with_metadata() {
    let block = json!({
        "type": "image",
        "base64": "<base64 data>",
        "mime_type": "image/jpeg",
        "metadata": {"cache_control": {"type": "ephemeral"}}
    });
    assert!(is_data_content_block(&block));
}

#[test]
fn test_invalid_case_wrong_type() {
    let block = json!({"type": "text", "text": "foo"});
    assert!(!is_data_content_block(&block));
}

#[test]
fn test_invalid_case_image_url_openai_format() {
    let block = json!({"type": "image_url", "image_url": {"url": "https://..."}});
    assert!(!is_data_content_block(&block));
}

#[test]
fn test_invalid_case_tool_call() {
    let block = json!({"type": "tool_call", "name": "func", "args": {}});
    assert!(!is_data_content_block(&block));
}

#[test]
fn test_invalid_case_invalid_type() {
    let block = json!({"type": "invalid", "url": "something"});
    assert!(!is_data_content_block(&block));
}

#[test]
fn test_invalid_case_valid_type_but_no_data_fields() {
    let block = json!({"type": "image"});
    assert!(!is_data_content_block(&block));
}

#[test]
fn test_invalid_case_valid_type_but_only_mime_type() {
    let block = json!({"type": "video", "mime_type": "video/mp4"});
    assert!(!is_data_content_block(&block));
}

#[test]
fn test_invalid_case_valid_type_but_only_extras() {
    let block = json!({"type": "audio", "extras": {"key": "value"}});
    assert!(!is_data_content_block(&block));
}

#[test]
fn test_invalid_case_wrong_data_field_name() {
    let block = json!({"type": "image", "source": "<base64 data>"});
    assert!(!is_data_content_block(&block));
}

#[test]
fn test_invalid_case_video_with_data_field() {
    let block = json!({"type": "video", "data": "video_data"});
    assert!(!is_data_content_block(&block));
}

#[test]
fn test_edge_case_empty_object() {
    let block = json!({});
    assert!(!is_data_content_block(&block));
}

#[test]
fn test_edge_case_missing_type() {
    let block = json!({"url": "https://..."});
    assert!(!is_data_content_block(&block));
}
