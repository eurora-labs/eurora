//! Tests for language_models utils module.
//!
//! Ported from `langchain/libs/core/tests/unit_tests/language_models/test_utils.py`
//!
//! The Python file also tests `_ensure_message_copy`, `_update_content_block`,
//! and `_update_message_content_to_blocks` — these are private Python helpers
//! that rely on Python-specific patterns (reference identity, Pydantic model_copy,
//! mutable content lists) with no direct Rust equivalent.

use agent_chain_core::language_models::{
    OpenAiDataBlockFilter, is_openai_data_block, parse_data_uri,
};
use serde_json::json;

/// Ported from `test_image_url_block_valid`.
#[test]
fn test_image_url_block_valid() {
    let block = json!({
        "type": "image_url",
        "image_url": {"url": "https://example.com/image.png"}
    });
    assert!(is_openai_data_block(&block, None));
}

/// Ported from `test_image_url_block_with_detail`.
#[test]
fn test_image_url_block_with_detail() {
    let block = json!({
        "type": "image_url",
        "image_url": {"url": "https://example.com/image.png", "detail": "high"},
        "detail": "high"
    });
    assert!(is_openai_data_block(&block, None));
}

/// Ported from `test_image_url_block_with_filter_image`.
#[test]
fn test_image_url_block_with_filter_image() {
    let block = json!({
        "type": "image_url",
        "image_url": {"url": "https://example.com/image.png"}
    });
    assert!(is_openai_data_block(
        &block,
        Some(OpenAiDataBlockFilter::Image)
    ));
}

/// Ported from `test_image_url_block_with_filter_audio`.
#[test]
fn test_image_url_block_with_filter_audio() {
    let block = json!({
        "type": "image_url",
        "image_url": {"url": "https://example.com/image.png"}
    });
    assert!(!is_openai_data_block(
        &block,
        Some(OpenAiDataBlockFilter::Audio)
    ));
}

/// Ported from `test_image_url_block_with_filter_file`.
#[test]
fn test_image_url_block_with_filter_file() {
    let block = json!({
        "type": "image_url",
        "image_url": {"url": "https://example.com/image.png"}
    });
    assert!(!is_openai_data_block(
        &block,
        Some(OpenAiDataBlockFilter::File)
    ));
}

/// Ported from `test_image_url_block_missing_url`.
#[test]
fn test_image_url_block_missing_url() {
    let block = json!({
        "type": "image_url",
        "image_url": {}
    });
    assert!(!is_openai_data_block(&block, None));
}

/// Ported from `test_image_url_block_url_not_string`.
#[test]
fn test_image_url_block_url_not_string() {
    let block = json!({
        "type": "image_url",
        "image_url": {"url": 123}
    });
    assert!(!is_openai_data_block(&block, None));
}

/// Ported from `test_image_url_block_image_url_not_dict`.
#[test]
fn test_image_url_block_image_url_not_dict() {
    let block = json!({
        "type": "image_url",
        "image_url": "https://example.com/image.png"
    });
    assert!(!is_openai_data_block(&block, None));
}

/// Ported from `test_image_url_block_extra_keys`.
#[test]
fn test_image_url_block_extra_keys() {
    let block = json!({
        "type": "image_url",
        "image_url": {"url": "https://example.com/image.png"},
        "extra_key": "value"
    });
    assert!(!is_openai_data_block(&block, None));
}

/// Ported from `test_input_audio_block_valid`.
#[test]
fn test_input_audio_block_valid() {
    let block = json!({
        "type": "input_audio",
        "input_audio": {"data": "base64data", "format": "wav"}
    });
    assert!(is_openai_data_block(&block, None));
}

/// Ported from `test_input_audio_block_with_filter_audio`.
#[test]
fn test_input_audio_block_with_filter_audio() {
    let block = json!({
        "type": "input_audio",
        "input_audio": {"data": "base64data", "format": "mp3"}
    });
    assert!(is_openai_data_block(
        &block,
        Some(OpenAiDataBlockFilter::Audio)
    ));
}

/// Ported from `test_input_audio_block_with_filter_image`.
#[test]
fn test_input_audio_block_with_filter_image() {
    let block = json!({
        "type": "input_audio",
        "input_audio": {"data": "base64data", "format": "wav"}
    });
    assert!(!is_openai_data_block(
        &block,
        Some(OpenAiDataBlockFilter::Image)
    ));
}

/// Ported from `test_input_audio_block_missing_data`.
#[test]
fn test_input_audio_block_missing_data() {
    let block = json!({
        "type": "input_audio",
        "input_audio": {"format": "wav"}
    });
    assert!(!is_openai_data_block(&block, None));
}

/// Ported from `test_input_audio_block_missing_format`.
#[test]
fn test_input_audio_block_missing_format() {
    let block = json!({
        "type": "input_audio",
        "input_audio": {"data": "base64data"}
    });
    assert!(!is_openai_data_block(&block, None));
}

/// Ported from `test_input_audio_block_data_not_string`.
#[test]
fn test_input_audio_block_data_not_string() {
    let block = json!({
        "type": "input_audio",
        "input_audio": {"data": 123, "format": "wav"}
    });
    assert!(!is_openai_data_block(&block, None));
}

/// Ported from `test_input_audio_block_format_not_string`.
#[test]
fn test_input_audio_block_format_not_string() {
    let block = json!({
        "type": "input_audio",
        "input_audio": {"data": "base64data", "format": 123}
    });
    assert!(!is_openai_data_block(&block, None));
}

/// Ported from `test_input_audio_block_input_audio_not_dict`.
#[test]
fn test_input_audio_block_input_audio_not_dict() {
    let block = json!({
        "type": "input_audio",
        "input_audio": "base64data"
    });
    assert!(!is_openai_data_block(&block, None));
}

/// Ported from `test_file_block_with_file_data`.
#[test]
fn test_file_block_with_file_data() {
    let block = json!({
        "type": "file",
        "file": {"file_data": "base64data"}
    });
    assert!(is_openai_data_block(&block, None));
}

/// Ported from `test_file_block_with_file_id`.
#[test]
fn test_file_block_with_file_id() {
    let block = json!({
        "type": "file",
        "file": {"file_id": "file-123"}
    });
    assert!(is_openai_data_block(&block, None));
}

/// Ported from `test_file_block_with_filter_file`.
#[test]
fn test_file_block_with_filter_file() {
    let block = json!({
        "type": "file",
        "file": {"file_data": "base64data"}
    });
    assert!(is_openai_data_block(
        &block,
        Some(OpenAiDataBlockFilter::File)
    ));
}

/// Ported from `test_file_block_with_filter_image`.
#[test]
fn test_file_block_with_filter_image() {
    let block = json!({
        "type": "file",
        "file": {"file_data": "base64data"}
    });
    assert!(!is_openai_data_block(
        &block,
        Some(OpenAiDataBlockFilter::Image)
    ));
}

/// Ported from `test_file_block_missing_file_data_and_file_id`.
#[test]
fn test_file_block_missing_file_data_and_file_id() {
    let block = json!({
        "type": "file",
        "file": {"filename": "test.pdf"}
    });
    assert!(!is_openai_data_block(&block, None));
}

/// Ported from `test_file_block_file_data_not_string`.
#[test]
fn test_file_block_file_data_not_string() {
    let block = json!({
        "type": "file",
        "file": {"file_data": 123}
    });
    assert!(!is_openai_data_block(&block, None));
}

/// Ported from `test_file_block_file_id_not_string`.
#[test]
fn test_file_block_file_id_not_string() {
    let block = json!({
        "type": "file",
        "file": {"file_id": 123}
    });
    assert!(!is_openai_data_block(&block, None));
}

/// Ported from `test_file_block_file_not_dict`.
#[test]
fn test_file_block_file_not_dict() {
    let block = json!({
        "type": "file",
        "file": "base64data"
    });
    assert!(!is_openai_data_block(&block, None));
}

/// Ported from `test_unknown_type`.
#[test]
fn test_unknown_type() {
    let block = json!({
        "type": "unknown",
        "data": "something"
    });
    assert!(!is_openai_data_block(&block, None));
}

/// Ported from `test_text_type`.
#[test]
fn test_text_type() {
    let block = json!({
        "type": "text",
        "text": "Hello world"
    });
    assert!(!is_openai_data_block(&block, None));
}

/// Ported from `test_missing_type`.
#[test]
fn test_missing_type() {
    let block = json!({
        "image_url": {"url": "https://example.com/image.png"}
    });
    assert!(!is_openai_data_block(&block, None));
}

/// Ported from `test_empty_block`.
#[test]
fn test_empty_block() {
    let block = json!({});
    assert!(!is_openai_data_block(&block, None));
}

/// Ported from `test_valid_data_uri_image_jpeg`.
#[test]
fn test_valid_data_uri_image_jpeg() {
    let uri = "data:image/jpeg;base64,/9j/4AAQSkZJRg...";
    let result = parse_data_uri(uri).unwrap();
    assert_eq!(result.source_type, "base64");
    assert_eq!(result.mime_type, "image/jpeg");
    assert_eq!(result.data, "/9j/4AAQSkZJRg...");
}

/// Ported from `test_valid_data_uri_image_png`.
#[test]
fn test_valid_data_uri_image_png() {
    let uri = "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAUA";
    let result = parse_data_uri(uri).unwrap();
    assert_eq!(result.source_type, "base64");
    assert_eq!(result.mime_type, "image/png");
    assert_eq!(result.data, "iVBORw0KGgoAAAANSUhEUgAAAAUA");
}

/// Ported from `test_valid_data_uri_application_pdf`.
#[test]
fn test_valid_data_uri_application_pdf() {
    let uri = "data:application/pdf;base64,JVBERi0xLjQKJeLjz9MKMSAwIG9iago=";
    let result = parse_data_uri(uri).unwrap();
    assert_eq!(result.source_type, "base64");
    assert_eq!(result.mime_type, "application/pdf");
    assert_eq!(result.data, "JVBERi0xLjQKJeLjz9MKMSAwIG9iago=");
}

/// Ported from `test_valid_data_uri_audio_wav`.
#[test]
fn test_valid_data_uri_audio_wav() {
    let uri = "data:audio/wav;base64,UklGRiQAAABXQVZFZm10IBAAAAABAAEA";
    let result = parse_data_uri(uri).unwrap();
    assert_eq!(result.source_type, "base64");
    assert_eq!(result.mime_type, "audio/wav");
    assert_eq!(result.data, "UklGRiQAAABXQVZFZm10IBAAAAABAAEA");
}

/// Ported from `test_invalid_data_uri_no_data_prefix`.
#[test]
fn test_invalid_data_uri_no_data_prefix() {
    let uri = "https://example.com/image.png";
    assert!(parse_data_uri(uri).is_none());
}

/// Ported from `test_invalid_data_uri_no_base64`.
#[test]
fn test_invalid_data_uri_no_base64() {
    let uri = "data:image/png,rawdata";
    assert!(parse_data_uri(uri).is_none());
}

/// Ported from `test_invalid_data_uri_empty_mime_type`.
#[test]
fn test_invalid_data_uri_empty_mime_type() {
    let uri = "data:;base64,somedata";
    assert!(parse_data_uri(uri).is_none());
}

/// Ported from `test_invalid_data_uri_empty_data`.
#[test]
fn test_invalid_data_uri_empty_data() {
    let uri = "data:image/png;base64,";
    assert!(parse_data_uri(uri).is_none());
}

/// Ported from `test_invalid_data_uri_malformed`.
#[test]
fn test_invalid_data_uri_malformed() {
    let uri = "data:image/png";
    assert!(parse_data_uri(uri).is_none());
}

/// Ported from `test_empty_string`.
#[test]
fn test_empty_string() {
    let uri = "";
    assert!(parse_data_uri(uri).is_none());
}

use agent_chain_core::language_models::update_message_content_to_blocks;
use agent_chain_core::messages::AIMessage;
use std::collections::HashMap;

/// Ported from `test_updates_content_to_content_blocks`.
#[test]
fn test_updates_content_to_content_blocks() {
    let message = AIMessage::builder().content("Hello world").build();
    let result = update_message_content_to_blocks(&message, "v1");

    assert_eq!(
        result
            .response_metadata
            .get("output_version")
            .and_then(|v| v.as_str()),
        Some("v1")
    );
}

/// Ported from `test_preserves_original_message`.
#[test]
fn test_preserves_original_message() {
    let message = AIMessage::builder().content("Hello world").build();
    let original_content = message.content.clone();

    let result = update_message_content_to_blocks(&message, "v1");

    assert_eq!(message.content, original_content);
    assert!(result.response_metadata.contains_key("output_version"));
    assert!(!message.response_metadata.contains_key("output_version"));
}

/// Ported from `test_with_complex_content`.
#[test]
fn test_with_complex_content() {
    let content_list = serde_json::json!([
        {"type": "text", "text": "Hello"},
        {"type": "tool_use", "id": "123", "name": "test", "input": {}}
    ]);
    let message = AIMessage::builder()
        .content(serde_json::to_string(&content_list).unwrap())
        .build();

    let result = update_message_content_to_blocks(&message, "v1");

    assert_eq!(
        result
            .response_metadata
            .get("output_version")
            .and_then(|v| v.as_str()),
        Some("v1")
    );
}

/// Ported from `test_with_different_output_version`.
#[test]
fn test_with_different_output_version() {
    let message = AIMessage::builder().content("Test").build();
    let result = update_message_content_to_blocks(&message, "v2");

    assert_eq!(
        result
            .response_metadata
            .get("output_version")
            .and_then(|v| v.as_str()),
        Some("v2")
    );
}

/// Ported from `test_preserves_existing_response_metadata`.
#[test]
fn test_preserves_existing_response_metadata() {
    let mut response_metadata = HashMap::new();
    response_metadata.insert("model".to_string(), serde_json::json!("test-model"));
    response_metadata.insert("usage".to_string(), serde_json::json!({"tokens": 10}));

    let message = AIMessage::builder()
        .content("Hello")
        .response_metadata(response_metadata)
        .build();

    let result = update_message_content_to_blocks(&message, "v1");

    assert_eq!(
        result
            .response_metadata
            .get("model")
            .and_then(|v| v.as_str()),
        Some("test-model")
    );
    assert_eq!(
        result.response_metadata.get("usage"),
        Some(&serde_json::json!({"tokens": 10}))
    );
    assert_eq!(
        result
            .response_metadata
            .get("output_version")
            .and_then(|v| v.as_str()),
        Some("v1")
    );
}
