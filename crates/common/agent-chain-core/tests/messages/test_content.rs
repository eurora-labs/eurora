//! Tests for content module types and functions.
//!
//! Converted from `langchain/libs/core/tests/unit_tests/messages/test_content.py`

use std::collections::{HashMap, HashSet};

use agent_chain_core::messages::{
    Annotation, AudioContentBlock, ContentBlock, DataContentBlock, FileContentBlock,
    ImageContentBlock, InvalidToolCallBlock, KNOWN_BLOCK_TYPES, NonStandardContentBlock,
    PlainTextBlockConfig, PlainTextContentBlock, ReasoningContentBlock, ServerToolCall,
    ServerToolCallChunk, ServerToolResult, ServerToolStatus, TextContentBlock, ToolCallBlock,
    ToolCallChunkBlock, VideoContentBlock, create_audio_block, create_citation, create_file_block,
    create_image_block, create_non_standard_block, create_plaintext_block, create_reasoning_block,
    create_text_block, create_tool_call, create_video_block, get_data_content_block_types,
    is_data_content_block,
};
use serde_json::json;


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


#[test]
fn test_basic_text_block() {
    let block = create_text_block("Hello world", None, None, None, None);
    assert_eq!(block.block_type, "text");
    assert_eq!(block.text, "Hello world");
    assert!(block.id.as_ref().unwrap().starts_with("lc_"));
}

#[test]
fn test_text_block_with_custom_id() {
    let block = create_text_block("Hello", Some("custom-id".to_string()), None, None, None);
    assert_eq!(block.id.as_ref().unwrap(), "custom-id");
}

#[test]
fn test_text_block_with_annotations() {
    let citation = create_citation(
        Some("https://example.com".to_string()),
        None,
        None,
        None,
        None,
        None,
        None,
    );
    let block = create_text_block("Hello", None, Some(vec![citation]), None, None);
    assert!(block.annotations.is_some());
    assert_eq!(block.annotations.as_ref().unwrap().len(), 1);
}

#[test]
fn test_text_block_with_index() {
    let block = create_text_block("Hello", None, None, Some(0.into()), None);
    assert!(block.index.is_some());
}

#[test]
fn test_text_block_with_extras() {
    let mut extras = HashMap::new();
    extras.insert(
        "custom_field".to_string(),
        serde_json::Value::String("custom_value".to_string()),
    );
    let block = create_text_block("Hello", None, None, None, Some(extras));
    assert_eq!(
        block.extras.as_ref().unwrap()["custom_field"],
        json!("custom_value")
    );
}

#[test]
fn test_text_block_empty_text() {
    let block = create_text_block("", None, None, None, None);
    assert_eq!(block.text, "");
}

#[test]
fn test_text_block_with_none_extras() {
    let block = create_text_block("hello", None, None, None, None);
    assert!(block.extras.is_none());
}


#[test]
fn test_create_image_block_with_url() {
    let block = create_image_block(
        Some("https://example.com/image.png".to_string()),
        None,
        None,
        None,
        None,
        None,
        None,
    )
    .unwrap();
    assert_eq!(block.block_type, "image");
    assert_eq!(block.url.as_ref().unwrap(), "https://example.com/image.png");
    assert!(block.id.is_some());
}

#[test]
fn test_create_image_block_with_base64() {
    let block = create_image_block(
        None,
        Some("dGVzdA==".to_string()),
        None,
        Some("image/png".to_string()),
        None,
        None,
        None,
    )
    .unwrap();
    assert_eq!(block.block_type, "image");
    assert_eq!(block.base64.as_ref().unwrap(), "dGVzdA==");
    assert_eq!(block.mime_type.as_ref().unwrap(), "image/png");
}

#[test]
fn test_create_image_block_with_file_id() {
    let block = create_image_block(
        None,
        None,
        Some("file-123".to_string()),
        None,
        None,
        None,
        None,
    )
    .unwrap();
    assert_eq!(block.block_type, "image");
    assert_eq!(block.file_id.as_ref().unwrap(), "file-123");
}

#[test]
fn test_create_image_block_requires_source() {
    let result = create_image_block(None, None, None, None, None, None, None);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Must provide one of"));
}

#[test]
fn test_create_image_block_with_custom_id() {
    let block = create_image_block(
        Some("https://example.com/img.png".to_string()),
        None,
        None,
        None,
        Some("img-123".to_string()),
        None,
        None,
    )
    .unwrap();
    assert_eq!(block.id.as_ref().unwrap(), "img-123");
}

#[test]
fn test_create_image_block_with_index() {
    let block = create_image_block(
        Some("https://example.com/img.png".to_string()),
        None,
        None,
        None,
        None,
        Some(0.into()),
        None,
    )
    .unwrap();
    assert!(block.index.is_some());
}

#[test]
fn test_create_image_block_with_extras() {
    let mut extras = HashMap::new();
    extras.insert(
        "alt".to_string(),
        serde_json::Value::String("Test image".to_string()),
    );
    let block = create_image_block(
        Some("https://example.com/img.png".to_string()),
        None,
        None,
        None,
        None,
        None,
        Some(extras),
    )
    .unwrap();
    assert_eq!(block.extras.as_ref().unwrap()["alt"], json!("Test image"));
}

#[test]
fn test_create_image_block_base64_without_mime_type_does_not_raise() {
    let block = create_image_block(
        None,
        Some("iVBORw0KGgo=".to_string()),
        None,
        None,
        None,
        None,
        None,
    )
    .unwrap();
    assert_eq!(block.block_type, "image");
    assert_eq!(block.base64.as_ref().unwrap(), "iVBORw0KGgo=");
    assert!(block.mime_type.is_none());
}


#[test]
fn test_create_video_block_with_url() {
    let block = create_video_block(
        Some("https://example.com/video.mp4".to_string()),
        None,
        None,
        None,
        None,
        None,
        None,
    )
    .unwrap();
    assert_eq!(block.block_type, "video");
    assert_eq!(block.url.as_ref().unwrap(), "https://example.com/video.mp4");
}

#[test]
fn test_create_video_block_with_base64() {
    let block = create_video_block(
        None,
        Some("dGVzdA==".to_string()),
        None,
        Some("video/mp4".to_string()),
        None,
        None,
        None,
    )
    .unwrap();
    assert_eq!(block.block_type, "video");
    assert_eq!(block.base64.as_ref().unwrap(), "dGVzdA==");
    assert_eq!(block.mime_type.as_ref().unwrap(), "video/mp4");
}

#[test]
fn test_create_video_block_with_file_id() {
    let block = create_video_block(
        None,
        None,
        Some("file-123".to_string()),
        None,
        None,
        None,
        None,
    )
    .unwrap();
    assert_eq!(block.block_type, "video");
    assert_eq!(block.file_id.as_ref().unwrap(), "file-123");
}

#[test]
fn test_create_video_block_requires_source() {
    let result = create_video_block(None, None, None, None, None, None, None);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Must provide one of"));
}

#[test]
fn test_create_video_block_base64_requires_mime_type() {
    let result = create_video_block(
        None,
        Some("dGVzdA==".to_string()),
        None,
        None,
        None,
        None,
        None,
    );
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("mime_type is required"));
}

#[test]
fn test_create_video_block_with_custom_id() {
    let block = create_video_block(
        Some("https://example.com/video.mp4".to_string()),
        None,
        None,
        None,
        Some("vid-123".to_string()),
        None,
        None,
    )
    .unwrap();
    assert_eq!(block.id.as_ref().unwrap(), "vid-123");
}


#[test]
fn test_create_audio_block_with_url() {
    let block = create_audio_block(
        Some("https://example.com/audio.mp3".to_string()),
        None,
        None,
        None,
        None,
        None,
        None,
    )
    .unwrap();
    assert_eq!(block.block_type, "audio");
    assert_eq!(block.url.as_ref().unwrap(), "https://example.com/audio.mp3");
}

#[test]
fn test_create_audio_block_with_base64() {
    let block = create_audio_block(
        None,
        Some("dGVzdA==".to_string()),
        None,
        Some("audio/mp3".to_string()),
        None,
        None,
        None,
    )
    .unwrap();
    assert_eq!(block.block_type, "audio");
    assert_eq!(block.base64.as_ref().unwrap(), "dGVzdA==");
    assert_eq!(block.mime_type.as_ref().unwrap(), "audio/mp3");
}

#[test]
fn test_create_audio_block_with_file_id() {
    let block = create_audio_block(
        None,
        None,
        Some("file-123".to_string()),
        None,
        None,
        None,
        None,
    )
    .unwrap();
    assert_eq!(block.block_type, "audio");
    assert_eq!(block.file_id.as_ref().unwrap(), "file-123");
}

#[test]
fn test_create_audio_block_requires_source() {
    let result = create_audio_block(None, None, None, None, None, None, None);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Must provide one of"));
}

#[test]
fn test_create_audio_block_base64_requires_mime_type() {
    let result = create_audio_block(
        None,
        Some("dGVzdA==".to_string()),
        None,
        None,
        None,
        None,
        None,
    );
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("mime_type is required"));
}

#[test]
fn test_create_audio_block_with_custom_id() {
    let block = create_audio_block(
        Some("https://example.com/audio.mp3".to_string()),
        None,
        None,
        None,
        Some("aud-123".to_string()),
        None,
        None,
    )
    .unwrap();
    assert_eq!(block.id.as_ref().unwrap(), "aud-123");
}


#[test]
fn test_create_file_block_with_url() {
    let block = create_file_block(
        Some("https://example.com/doc.pdf".to_string()),
        None,
        None,
        None,
        None,
        None,
        None,
    )
    .unwrap();
    assert_eq!(block.block_type, "file");
    assert_eq!(block.url.as_ref().unwrap(), "https://example.com/doc.pdf");
}

#[test]
fn test_create_file_block_with_base64() {
    let block = create_file_block(
        None,
        Some("dGVzdA==".to_string()),
        None,
        Some("application/pdf".to_string()),
        None,
        None,
        None,
    )
    .unwrap();
    assert_eq!(block.block_type, "file");
    assert_eq!(block.base64.as_ref().unwrap(), "dGVzdA==");
    assert_eq!(block.mime_type.as_ref().unwrap(), "application/pdf");
}

#[test]
fn test_create_file_block_with_file_id() {
    let block = create_file_block(
        None,
        None,
        Some("file-123".to_string()),
        None,
        None,
        None,
        None,
    )
    .unwrap();
    assert_eq!(block.block_type, "file");
    assert_eq!(block.file_id.as_ref().unwrap(), "file-123");
}

#[test]
fn test_create_file_block_requires_source() {
    let result = create_file_block(None, None, None, None, None, None, None);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Must provide one of"));
}

#[test]
fn test_create_file_block_base64_requires_mime_type() {
    let result = create_file_block(
        None,
        Some("dGVzdA==".to_string()),
        None,
        None,
        None,
        None,
        None,
    );
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("mime_type is required"));
}

#[test]
fn test_create_file_block_with_custom_id() {
    let block = create_file_block(
        Some("https://example.com/doc.pdf".to_string()),
        None,
        None,
        None,
        Some("file-custom".to_string()),
        None,
        None,
    )
    .unwrap();
    assert_eq!(block.id.as_ref().unwrap(), "file-custom");
}

#[test]
fn test_create_file_block_all_fields_populated() {
    let block = create_file_block(
        Some("https://example.com/report.pdf".to_string()),
        None,
        None,
        Some("application/pdf".to_string()),
        Some("file-all".to_string()),
        Some(7.into()),
        None,
    )
    .unwrap();
    assert_eq!(block.block_type, "file");
    assert_eq!(
        block.url.as_ref().unwrap(),
        "https://example.com/report.pdf"
    );
    assert_eq!(block.mime_type.as_ref().unwrap(), "application/pdf");
    assert_eq!(block.id.as_ref().unwrap(), "file-all");
    assert!(block.index.is_some());
}


#[test]
fn test_plaintext_block_with_text() {
    let block = create_plaintext_block(PlainTextBlockConfig {
        text: Some("Hello world".to_string()),
        ..Default::default()
    });
    assert_eq!(block.block_type, "text-plain");
    assert_eq!(block.text.as_ref().unwrap(), "Hello world");
    assert_eq!(block.mime_type, "text/plain");
}

#[test]
fn test_plaintext_block_with_url() {
    let block = create_plaintext_block(PlainTextBlockConfig {
        url: Some("https://example.com/file.txt".to_string()),
        ..Default::default()
    });
    assert_eq!(block.block_type, "text-plain");
    assert_eq!(block.url.as_ref().unwrap(), "https://example.com/file.txt");
}

#[test]
fn test_plaintext_block_with_base64() {
    let block = create_plaintext_block(PlainTextBlockConfig {
        base64: Some("SGVsbG8gd29ybGQ=".to_string()),
        ..Default::default()
    });
    assert_eq!(block.block_type, "text-plain");
    assert_eq!(block.base64.as_ref().unwrap(), "SGVsbG8gd29ybGQ=");
}

#[test]
fn test_plaintext_block_with_file_id() {
    let block = create_plaintext_block(PlainTextBlockConfig {
        file_id: Some("file-123".to_string()),
        ..Default::default()
    });
    assert_eq!(block.block_type, "text-plain");
    assert_eq!(block.file_id.as_ref().unwrap(), "file-123");
}

#[test]
fn test_plaintext_block_with_title() {
    let block = create_plaintext_block(PlainTextBlockConfig {
        text: Some("Hello".to_string()),
        title: Some("My Document".to_string()),
        ..Default::default()
    });
    assert_eq!(block.title.as_ref().unwrap(), "My Document");
}

#[test]
fn test_plaintext_block_with_context() {
    let block = create_plaintext_block(PlainTextBlockConfig {
        text: Some("Hello".to_string()),
        context: Some("Important information".to_string()),
        ..Default::default()
    });
    assert_eq!(block.context.as_ref().unwrap(), "Important information");
}

#[test]
fn test_plaintext_block_with_custom_id() {
    let block = create_plaintext_block(PlainTextBlockConfig {
        text: Some("Hello".to_string()),
        id: Some("txt-123".to_string()),
        ..Default::default()
    });
    assert_eq!(block.id.as_ref().unwrap(), "txt-123");
}

#[test]
fn test_plaintext_block_all_fields_populated() {
    let block = create_plaintext_block(PlainTextBlockConfig {
        text: Some("some text".to_string()),
        url: Some("https://example.com/file.txt".to_string()),
        base64: Some("c29tZSB0ZXh0".to_string()),
        file_id: Some("file-xyz".to_string()),
        title: Some("My Document".to_string()),
        context: Some("Summary context".to_string()),
        id: Some("pt-999".to_string()),
        index: Some(3.into()),
        extras: None,
    });
    assert_eq!(block.block_type, "text-plain");
    assert_eq!(block.mime_type, "text/plain");
    assert_eq!(block.text.as_ref().unwrap(), "some text");
    assert_eq!(block.url.as_ref().unwrap(), "https://example.com/file.txt");
    assert_eq!(block.base64.as_ref().unwrap(), "c29tZSB0ZXh0");
    assert_eq!(block.file_id.as_ref().unwrap(), "file-xyz");
    assert_eq!(block.title.as_ref().unwrap(), "My Document");
    assert_eq!(block.context.as_ref().unwrap(), "Summary context");
    assert_eq!(block.id.as_ref().unwrap(), "pt-999");
    assert!(block.index.is_some());
}

#[test]
fn test_plaintext_block_with_extras() {
    let mut extras = HashMap::new();
    extras.insert("custom_key".to_string(), json!("custom_value"));
    extras.insert("another".to_string(), json!("val"));
    let block = create_plaintext_block(PlainTextBlockConfig {
        text: Some("hello".to_string()),
        extras: Some(extras),
        ..Default::default()
    });
    let block_extras = block.extras.as_ref().unwrap();
    assert_eq!(block_extras["custom_key"], json!("custom_value"));
    assert_eq!(block_extras["another"], json!("val"));
}


#[test]
fn test_basic_tool_call() {
    let mut args = HashMap::new();
    args.insert("param".to_string(), json!("value"));
    let block = create_tool_call("test_tool", args, None, None, None);
    assert_eq!(block.block_type, "tool_call");
    assert_eq!(block.name, "test_tool");
    assert_eq!(block.args["param"], json!("value"));
    assert!(block.id.as_ref().unwrap().starts_with("lc_"));
}

#[test]
fn test_tool_call_with_custom_id() {
    let block = create_tool_call(
        "test_tool",
        HashMap::new(),
        Some("call-123".to_string()),
        None,
        None,
    );
    assert_eq!(block.id.as_ref().unwrap(), "call-123");
}

#[test]
fn test_tool_call_with_index() {
    let block = create_tool_call("test_tool", HashMap::new(), None, Some(0.into()), None);
    assert!(block.index.is_some());
}

#[test]
fn test_tool_call_with_extras() {
    let mut extras = HashMap::new();
    extras.insert("custom".to_string(), json!("value"));
    let block = create_tool_call("test_tool", HashMap::new(), None, None, Some(extras));
    assert_eq!(block.extras.as_ref().unwrap()["custom"], json!("value"));
}

#[test]
fn test_tool_call_empty_args() {
    let block = create_tool_call("test_tool", HashMap::new(), None, None, None);
    assert!(block.args.is_empty());
}

#[test]
fn test_tool_call_complex_args() {
    let mut nested = HashMap::new();
    nested.insert("key".to_string(), json!("value"));

    let mut args = HashMap::new();
    args.insert("string".to_string(), json!("value"));
    args.insert("number".to_string(), json!(42));
    args.insert("nested".to_string(), json!({"key": "value"}));
    args.insert("list".to_string(), json!([1, 2, 3]));

    let block = create_tool_call("test_tool", args.clone(), None, None, None);
    assert_eq!(block.args["string"], json!("value"));
    assert_eq!(block.args["number"], json!(42));
    assert_eq!(block.args["nested"], json!({"key": "value"}));
    assert_eq!(block.args["list"], json!([1, 2, 3]));
}

#[test]
fn test_tool_call_auto_generates_id_when_not_provided() {
    let block = create_tool_call(
        "my_tool",
        HashMap::from([("x".to_string(), json!(1))]),
        None,
        None,
        None,
    );
    let id = block.id.as_ref().unwrap();
    assert!(id.starts_with("lc_"));
}

#[test]
fn test_tool_call_auto_generated_ids_are_unique() {
    let block_a = create_tool_call("t", HashMap::new(), None, None, None);
    let block_b = create_tool_call("t", HashMap::new(), None, None, None);
    assert_ne!(block_a.id, block_b.id);
}


#[test]
fn test_basic_reasoning_block() {
    let block = create_reasoning_block(
        Some("Let me think about this...".to_string()),
        None,
        None,
        None,
    );
    assert_eq!(block.block_type, "reasoning");
    assert_eq!(
        block.reasoning.as_ref().unwrap(),
        "Let me think about this..."
    );
    assert!(block.id.is_some());
}

#[test]
fn test_reasoning_block_with_custom_id() {
    let block = create_reasoning_block(
        Some("Thinking...".to_string()),
        Some("reason-123".to_string()),
        None,
        None,
    );
    assert_eq!(block.id.as_ref().unwrap(), "reason-123");
}

#[test]
fn test_reasoning_block_with_index() {
    let block = create_reasoning_block(Some("Thinking...".to_string()), None, Some(0.into()), None);
    assert!(block.index.is_some());
}

#[test]
fn test_reasoning_block_empty_reasoning() {
    let block = create_reasoning_block(None, None, None, None);
    assert_eq!(block.reasoning.as_ref().unwrap(), "");
}

#[test]
fn test_reasoning_block_with_extras() {
    let mut extras = HashMap::new();
    extras.insert("signature".to_string(), json!("abc123"));
    let block = create_reasoning_block(Some("Thinking...".to_string()), None, None, Some(extras));
    assert_eq!(block.extras.as_ref().unwrap()["signature"], json!("abc123"));
}

#[test]
fn test_reasoning_block_none_reasoning_defaults_to_empty_string() {
    let block = create_reasoning_block(None, None, None, None);
    assert_eq!(block.reasoning.as_ref().unwrap(), "");
    assert_eq!(block.block_type, "reasoning");
    assert!(block.id.is_some());
}


#[test]
fn test_basic_citation() {
    let annotation = create_citation(
        Some("https://example.com/source".to_string()),
        None,
        None,
        None,
        None,
        None,
        None,
    );
    match &annotation {
        Annotation::Citation { url, id, .. } => {
            assert_eq!(url.as_ref().unwrap(), "https://example.com/source");
            assert!(id.as_ref().unwrap().starts_with("lc_"));
        }
        _ => panic!("Expected Citation variant"),
    }
}

#[test]
fn test_citation_with_all_fields() {
    let annotation = create_citation(
        Some("https://example.com/source".to_string()),
        Some("Source Document".to_string()),
        Some(0),
        Some(100),
        Some("This is the cited text.".to_string()),
        Some("cite-123".to_string()),
        None,
    );
    match &annotation {
        Annotation::Citation {
            url,
            title,
            start_index,
            end_index,
            cited_text,
            id,
            ..
        } => {
            assert_eq!(url.as_ref().unwrap(), "https://example.com/source");
            assert_eq!(title.as_ref().unwrap(), "Source Document");
            assert_eq!(*start_index, Some(0));
            assert_eq!(*end_index, Some(100));
            assert_eq!(cited_text.as_ref().unwrap(), "This is the cited text.");
            assert_eq!(id.as_ref().unwrap(), "cite-123");
        }
        _ => panic!("Expected Citation variant"),
    }
}

#[test]
fn test_citation_with_extras() {
    let mut extras = HashMap::new();
    extras.insert("custom_field".to_string(), json!("value"));
    let annotation = create_citation(
        Some("https://example.com".to_string()),
        None,
        None,
        None,
        None,
        None,
        Some(extras),
    );
    match &annotation {
        Annotation::Citation { extras, .. } => {
            assert_eq!(extras.as_ref().unwrap()["custom_field"], json!("value"));
        }
        _ => panic!("Expected Citation variant"),
    }
}

#[test]
fn test_citation_with_no_optional_fields() {
    let annotation = create_citation(None, None, None, None, None, None, None);
    match &annotation {
        Annotation::Citation {
            id,
            url,
            title,
            start_index,
            end_index,
            cited_text,
            extras,
        } => {
            assert!(id.as_ref().unwrap().starts_with("lc_"));
            assert!(url.is_none());
            assert!(title.is_none());
            assert!(start_index.is_none());
            assert!(end_index.is_none());
            assert!(cited_text.is_none());
            assert!(extras.is_none());
        }
        _ => panic!("Expected Citation variant"),
    }
}


#[test]
fn test_basic_non_standard_block() {
    let mut value = HashMap::new();
    value.insert("custom".to_string(), json!("data"));
    let block = create_non_standard_block(value.clone(), None, None);
    assert_eq!(block.block_type, "non_standard");
    assert_eq!(block.value["custom"], json!("data"));
    assert!(block.id.is_some());
}

#[test]
fn test_non_standard_block_with_custom_id() {
    let mut value = HashMap::new();
    value.insert("key".to_string(), json!("value"));
    let block = create_non_standard_block(value, Some("ns-123".to_string()), None);
    assert_eq!(block.id.as_ref().unwrap(), "ns-123");
}

#[test]
fn test_non_standard_block_with_index() {
    let mut value = HashMap::new();
    value.insert("key".to_string(), json!("value"));
    let block = create_non_standard_block(value, None, Some(0.into()));
    assert!(block.index.is_some());
}

#[test]
fn test_non_standard_block_complex_value() {
    let mut value = HashMap::new();
    value.insert("nested".to_string(), json!({"deep": {"data": "value"}}));
    value.insert("list".to_string(), json!([1, 2, 3]));
    value.insert("string".to_string(), json!("text"));
    let block = create_non_standard_block(value, None, None);
    assert_eq!(block.value["nested"], json!({"deep": {"data": "value"}}));
    assert_eq!(block.value["list"], json!([1, 2, 3]));
    assert_eq!(block.value["string"], json!("text"));
}

#[test]
fn test_non_standard_block_empty_dict_value() {
    let block = create_non_standard_block(HashMap::new(), None, None);
    assert_eq!(block.block_type, "non_standard");
    assert!(block.value.is_empty());
    assert!(block.id.as_ref().unwrap().starts_with("lc_"));
}


#[test]
fn test_text_content_block_structure() {
    let block = TextContentBlock::new("Hello");
    assert_eq!(block.block_type, "text");
    assert_eq!(block.text, "Hello");
}

#[test]
fn test_tool_call_block_structure() {
    let mut args = HashMap::new();
    args.insert("param".to_string(), json!("value"));
    let mut block = ToolCallBlock::new("test_tool", args);
    block.id = Some("123".to_string());
    assert_eq!(block.block_type, "tool_call");
    assert_eq!(block.name, "test_tool");
}

#[test]
fn test_tool_call_chunk_block_structure() {
    let mut block = ToolCallChunkBlock::new();
    block.id = Some("123".to_string());
    block.name = Some("test_tool".to_string());
    block.args = Some(r#"{"param": "value"}"#.to_string());
    assert_eq!(block.block_type, "tool_call_chunk");
    assert_eq!(block.args.as_ref().unwrap(), r#"{"param": "value"}"#);
}

#[test]
fn test_invalid_tool_call_block_structure() {
    let mut block = InvalidToolCallBlock::new();
    block.id = Some("123".to_string());
    block.name = Some("test_tool".to_string());
    block.args = Some("invalid json".to_string());
    block.error = Some("JSON parse error".to_string());
    assert_eq!(block.block_type, "invalid_tool_call");
    assert_eq!(block.error.as_ref().unwrap(), "JSON parse error");
}

#[test]
fn test_reasoning_content_block_structure() {
    let block = ReasoningContentBlock::new("Let me think...");
    assert_eq!(block.block_type, "reasoning");
}

#[test]
fn test_image_content_block_structure() {
    let mut block = ImageContentBlock::new();
    block.url = Some("https://example.com/image.png".to_string());
    assert_eq!(block.block_type, "image");
}

#[test]
fn test_audio_content_block_structure() {
    let mut block = AudioContentBlock::new();
    block.url = Some("https://example.com/audio.mp3".to_string());
    assert_eq!(block.block_type, "audio");
}

#[test]
fn test_video_content_block_structure() {
    let mut block = VideoContentBlock::new();
    block.url = Some("https://example.com/video.mp4".to_string());
    assert_eq!(block.block_type, "video");
}

#[test]
fn test_file_content_block_structure() {
    let mut block = FileContentBlock::new();
    block.url = Some("https://example.com/doc.pdf".to_string());
    assert_eq!(block.block_type, "file");
}

#[test]
fn test_plaintext_content_block_structure() {
    let mut block = PlainTextContentBlock::new();
    block.text = Some("Hello".to_string());
    assert_eq!(block.block_type, "text-plain");
    assert_eq!(block.mime_type, "text/plain");
}

#[test]
fn test_citation_structure() {
    let annotation = Annotation::citation();
    match &annotation {
        Annotation::Citation { .. } => {}
        _ => panic!("Expected Citation variant"),
    }
}

#[test]
fn test_non_standard_annotation_structure() {
    let mut value = HashMap::new();
    value.insert("custom".to_string(), json!("data"));
    let annotation = Annotation::non_standard(value);
    match &annotation {
        Annotation::NonStandardAnnotation { value, .. } => {
            assert_eq!(value["custom"], json!("data"));
        }
        _ => panic!("Expected NonStandardAnnotation variant"),
    }
}

#[test]
fn test_server_tool_call_structure() {
    let mut args = HashMap::new();
    args.insert("query".to_string(), json!("test"));
    let block = ServerToolCall::new("stc-123", "web_search", args);
    assert_eq!(block.block_type, "server_tool_call");
}

#[test]
fn test_server_tool_call_chunk_structure() {
    let block = ServerToolCallChunk::new();
    assert_eq!(block.block_type, "server_tool_call_chunk");
}

#[test]
fn test_server_tool_result_structure() {
    let block = ServerToolResult::success("stc-123");
    assert_eq!(block.block_type, "server_tool_result");
    assert_eq!(block.tool_call_id, "stc-123");
    assert_eq!(block.status, ServerToolStatus::Success);
}

#[test]
fn test_non_standard_content_block_structure() {
    let mut value = HashMap::new();
    value.insert("provider".to_string(), json!("custom"));
    value.insert("data".to_string(), json!("value"));
    let block = NonStandardContentBlock::new(value);
    assert_eq!(block.block_type, "non_standard");
}


#[test]
fn test_known_block_types_exact_set() {
    let expected: HashSet<&str> = [
        "text",
        "reasoning",
        "tool_call",
        "invalid_tool_call",
        "tool_call_chunk",
        "image",
        "audio",
        "file",
        "text-plain",
        "video",
        "server_tool_call",
        "server_tool_call_chunk",
        "server_tool_result",
        "non_standard",
    ]
    .into_iter()
    .collect();

    let actual: HashSet<&str> = KNOWN_BLOCK_TYPES.iter().copied().collect();
    assert_eq!(actual, expected);
}

#[test]
fn test_known_block_types_count() {
    assert_eq!(KNOWN_BLOCK_TYPES.len(), 14);
}

#[test]
fn test_citation_not_in_known_block_types() {
    assert!(!KNOWN_BLOCK_TYPES.contains(&"citation"));
}

#[test]
fn test_non_standard_annotation_not_in_known_block_types() {
    assert!(!KNOWN_BLOCK_TYPES.contains(&"non_standard_annotation"));
}


#[test]
fn test_get_data_content_block_types_returns_slice() {
    let result = get_data_content_block_types();
    assert!(!result.is_empty());
}

#[test]
fn test_get_data_content_block_types_exact_type_literals() {
    let result = get_data_content_block_types();
    let result_set: HashSet<&str> = result.iter().copied().collect();
    let expected: HashSet<&str> = ["image", "video", "audio", "text-plain", "file"]
        .into_iter()
        .collect();
    assert_eq!(result_set, expected);
    assert_eq!(result.len(), 5);
}

#[test]
fn test_get_data_content_block_types_each_member_is_string() {
    for item in get_data_content_block_types() {
        assert!(!item.is_empty());
    }
}


#[test]
fn test_plaintext_block_with_source_type_text_and_url() {
    let block = json!({
        "type": "text-plain",
        "source_type": "text",
        "url": "https://example.com/file.txt"
    });
    assert!(is_data_content_block(&block));
}

#[test]
fn test_image_block_with_type_only_no_data_fields() {
    let block = json!({"type": "image"});
    assert!(!is_data_content_block(&block));
}

#[test]
fn test_video_block_with_file_id_is_data_content() {
    let block = json!({"type": "video", "file_id": "vid-file-001"});
    assert!(is_data_content_block(&block));
}

#[test]
fn test_audio_block_with_file_id_is_data_content() {
    let block = json!({"type": "audio", "file_id": "aud-file-002"});
    assert!(is_data_content_block(&block));
}


fn assert_content_block_has_type(block: &ContentBlock) {
    let json = serde_json::to_value(block).unwrap();
    assert!(json.get("type").is_some());
}

#[test]
fn test_content_block_text() {
    let block = ContentBlock::Text(TextContentBlock::new("hi"));
    assert_content_block_has_type(&block);
}

#[test]
fn test_content_block_tool_call() {
    let mut block = ToolCallBlock::new("fn", HashMap::new());
    block.id = Some("tc1".to_string());
    let cb = ContentBlock::ToolCall(block);
    assert_content_block_has_type(&cb);
}

#[test]
fn test_content_block_invalid_tool_call() {
    let mut block = InvalidToolCallBlock::new();
    block.id = Some("itc1".to_string());
    block.name = Some("fn".to_string());
    block.args = Some("bad".to_string());
    block.error = Some("parse error".to_string());
    let cb = ContentBlock::InvalidToolCall(block);
    assert_content_block_has_type(&cb);
}

#[test]
fn test_content_block_tool_call_chunk() {
    let mut block = ToolCallChunkBlock::new();
    block.id = Some("tcc1".to_string());
    block.name = Some("fn".to_string());
    block.args = Some(r#"{"a":1}"#.to_string());
    let cb = ContentBlock::ToolCallChunk(block);
    assert_content_block_has_type(&cb);
}

#[test]
fn test_content_block_reasoning() {
    let block = ReasoningContentBlock::new("thinking");
    let cb = ContentBlock::Reasoning(block);
    assert_content_block_has_type(&cb);
}

#[test]
fn test_content_block_image() {
    let cb = ContentBlock::Image(ImageContentBlock::from_url("https://example.com/img.png"));
    assert_content_block_has_type(&cb);
}

#[test]
fn test_content_block_video() {
    let mut block = VideoContentBlock::new();
    block.url = Some("https://example.com/vid.mp4".to_string());
    let cb = ContentBlock::Video(block);
    assert_content_block_has_type(&cb);
}

#[test]
fn test_content_block_audio() {
    let mut block = AudioContentBlock::new();
    block.url = Some("https://example.com/aud.mp3".to_string());
    let cb = ContentBlock::Audio(block);
    assert_content_block_has_type(&cb);
}

#[test]
fn test_content_block_plaintext() {
    let mut block = PlainTextContentBlock::new();
    block.text = Some("hello".to_string());
    let cb = ContentBlock::PlainText(block);
    assert_content_block_has_type(&cb);
}

#[test]
fn test_content_block_file() {
    let mut block = FileContentBlock::new();
    block.url = Some("https://example.com/doc.pdf".to_string());
    let cb = ContentBlock::File(block);
    assert_content_block_has_type(&cb);
}

#[test]
fn test_content_block_server_tool_call() {
    let block = ServerToolCall::new("stc1", "search", HashMap::new());
    let cb = ContentBlock::ServerToolCall(block);
    assert_content_block_has_type(&cb);
}

#[test]
fn test_content_block_server_tool_call_chunk() {
    let cb = ContentBlock::ServerToolCallChunk(ServerToolCallChunk::new());
    assert_content_block_has_type(&cb);
}

#[test]
fn test_content_block_server_tool_result() {
    let cb = ContentBlock::ServerToolResult(ServerToolResult::success("stc1"));
    assert_content_block_has_type(&cb);
}

#[test]
fn test_content_block_non_standard() {
    let mut value = HashMap::new();
    value.insert("data".to_string(), json!("custom"));
    let cb = ContentBlock::NonStandard(NonStandardContentBlock::new(value));
    assert_content_block_has_type(&cb);
}


fn assert_data_block_has_type(block: &DataContentBlock) {
    let json = serde_json::to_value(block).unwrap();
    assert!(json.get("type").is_some());
}

#[test]
fn test_data_content_block_image() {
    let block = DataContentBlock::Image(ImageContentBlock::from_url("https://example.com/img.png"));
    assert_data_block_has_type(&block);
}

#[test]
fn test_data_content_block_video() {
    let mut block = VideoContentBlock::new();
    block.url = Some("https://example.com/vid.mp4".to_string());
    let dcb = DataContentBlock::Video(block);
    assert_data_block_has_type(&dcb);
}

#[test]
fn test_data_content_block_audio() {
    let mut block = AudioContentBlock::new();
    block.url = Some("https://example.com/aud.mp3".to_string());
    let dcb = DataContentBlock::Audio(block);
    assert_data_block_has_type(&dcb);
}

#[test]
fn test_data_content_block_plaintext() {
    let mut block = PlainTextContentBlock::new();
    block.text = Some("hi".to_string());
    let dcb = DataContentBlock::PlainText(block);
    assert_data_block_has_type(&dcb);
}

#[test]
fn test_data_content_block_file() {
    let mut block = FileContentBlock::new();
    block.url = Some("https://example.com/doc.pdf".to_string());
    let dcb = DataContentBlock::File(block);
    assert_data_block_has_type(&dcb);
}

#[test]
fn test_data_content_block_union_has_exactly_five_members() {
    let variants: Vec<DataContentBlock> = vec![
        DataContentBlock::Image(ImageContentBlock::new()),
        DataContentBlock::Video(VideoContentBlock::new()),
        DataContentBlock::Audio(AudioContentBlock::new()),
        DataContentBlock::PlainText(PlainTextContentBlock::new()),
        DataContentBlock::File(FileContentBlock::new()),
    ];
    assert_eq!(variants.len(), 5);

    for variant in &variants {
        match variant {
            DataContentBlock::Image(_) => {}
            DataContentBlock::Video(_) => {}
            DataContentBlock::Audio(_) => {}
            DataContentBlock::PlainText(_) => {}
            DataContentBlock::File(_) => {}
        }
    }
}
