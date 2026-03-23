use std::collections::{HashMap, HashSet};

use agent_chain_core::messages::{
    Annotation, AudioContentBlock, ContentBlock, DataContentBlock, FileContentBlock,
    ImageContentBlock, InvalidToolCallBlock, KNOWN_BLOCK_TYPES, NonStandardContentBlock,
    PlainTextContentBlock, ReasoningContentBlock, ServerToolCall, ServerToolCallChunk,
    ServerToolResult, ServerToolStatus, TextContentBlock, ToolCallBlock, ToolCallChunkBlock,
    VideoContentBlock, get_data_content_block_types, is_data_content_block,
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
    let block = TextContentBlock::builder().text("Hello world").build();
    assert_eq!(block.text, "Hello world");
    assert!(block.id.as_ref().unwrap().starts_with("lc_"));
}

#[test]
fn test_text_block_with_custom_id() {
    let block = TextContentBlock::builder()
        .text("Hello")
        .id("custom-id".to_string())
        .build();
    assert_eq!(block.id.as_ref().unwrap(), "custom-id");
}

#[test]
fn test_text_block_with_annotations() {
    let citation = Annotation::citation()
        .url("https://example.com".to_string())
        .call();
    let block = TextContentBlock::builder()
        .text("Hello")
        .annotations(vec![citation])
        .build();
    assert!(block.annotations.is_some());
    assert_eq!(block.annotations.as_ref().unwrap().len(), 1);
}

#[test]
fn test_text_block_with_index() {
    let block = TextContentBlock::builder()
        .text("Hello")
        .index(0.into())
        .build();
    assert!(block.index.is_some());
}

#[test]
fn test_text_block_with_extras() {
    let mut extras = HashMap::new();
    extras.insert(
        "custom_field".to_string(),
        serde_json::Value::String("custom_value".to_string()),
    );
    let block = TextContentBlock::builder()
        .text("Hello")
        .extras(extras)
        .build();
    assert_eq!(
        block.extras.as_ref().unwrap()["custom_field"],
        json!("custom_value")
    );
}

#[test]
fn test_text_block_empty_text() {
    let block = TextContentBlock::builder().text("").build();
    assert_eq!(block.text, "");
}

#[test]
fn test_text_block_with_none_extras() {
    let block = TextContentBlock::builder().text("hello").build();
    assert!(block.extras.is_none());
}

#[test]
fn test_create_image_block_with_url() {
    let block = ImageContentBlock::builder()
        .url("https://example.com/image.png".to_string())
        .build()
        .unwrap();
    assert_eq!(block.url.as_ref().unwrap(), "https://example.com/image.png");
    assert!(block.id.is_some());
}

#[test]
fn test_create_image_block_with_base64() {
    let block = ImageContentBlock::builder()
        .base64("dGVzdA==".to_string())
        .mime_type("image/png".to_string())
        .build()
        .unwrap();
    assert_eq!(block.base64.as_ref().unwrap(), "dGVzdA==");
    assert_eq!(block.mime_type.as_ref().unwrap(), "image/png");
}

#[test]
fn test_create_image_block_with_file_id() {
    let block = ImageContentBlock::builder()
        .file_id("file-123".to_string())
        .build()
        .unwrap();
    assert_eq!(block.file_id.as_ref().unwrap(), "file-123");
}

#[test]
fn test_create_image_block_requires_source() {
    let result = ImageContentBlock::builder().build();
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Must provide one of"));
}

#[test]
fn test_create_image_block_with_custom_id() {
    let block = ImageContentBlock::builder()
        .url("https://example.com/img.png".to_string())
        .id("img-123".to_string())
        .build()
        .unwrap();
    assert_eq!(block.id.as_ref().unwrap(), "img-123");
}

#[test]
fn test_create_image_block_with_index() {
    let block = ImageContentBlock::builder()
        .url("https://example.com/img.png".to_string())
        .index(0.into())
        .build()
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
    let block = ImageContentBlock::builder()
        .url("https://example.com/img.png".to_string())
        .extras(extras)
        .build()
        .unwrap();
    assert_eq!(block.extras.as_ref().unwrap()["alt"], json!("Test image"));
}

#[test]
fn test_create_image_block_base64_without_mime_type_does_not_raise() {
    let block = ImageContentBlock::builder()
        .base64("iVBORw0KGgo=".to_string())
        .build()
        .unwrap();
    assert_eq!(block.base64.as_ref().unwrap(), "iVBORw0KGgo=");
    assert!(block.mime_type.is_none());
}

#[test]
fn test_create_video_block_with_url() {
    let block = VideoContentBlock::builder()
        .url("https://example.com/video.mp4".to_string())
        .build()
        .unwrap();
    assert_eq!(block.url.as_ref().unwrap(), "https://example.com/video.mp4");
}

#[test]
fn test_create_video_block_with_base64() {
    let block = VideoContentBlock::builder()
        .base64("dGVzdA==".to_string())
        .mime_type("video/mp4".to_string())
        .build()
        .unwrap();
    assert_eq!(block.base64.as_ref().unwrap(), "dGVzdA==");
    assert_eq!(block.mime_type.as_ref().unwrap(), "video/mp4");
}

#[test]
fn test_create_video_block_with_file_id() {
    let block = VideoContentBlock::builder()
        .file_id("file-123".to_string())
        .build()
        .unwrap();
    assert_eq!(block.file_id.as_ref().unwrap(), "file-123");
}

#[test]
fn test_create_video_block_requires_source() {
    let result = VideoContentBlock::builder().build();
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Must provide one of"));
}

#[test]
fn test_create_video_block_base64_requires_mime_type() {
    let result = VideoContentBlock::builder()
        .base64("dGVzdA==".to_string())
        .build();
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("mime_type is required"));
}

#[test]
fn test_create_video_block_with_custom_id() {
    let block = VideoContentBlock::builder()
        .url("https://example.com/video.mp4".to_string())
        .id("vid-123".to_string())
        .build()
        .unwrap();
    assert_eq!(block.id.as_ref().unwrap(), "vid-123");
}

#[test]
fn test_create_audio_block_with_url() {
    let block = AudioContentBlock::builder()
        .url("https://example.com/audio.mp3".to_string())
        .build()
        .unwrap();
    assert_eq!(block.url.as_ref().unwrap(), "https://example.com/audio.mp3");
}

#[test]
fn test_create_audio_block_with_base64() {
    let block = AudioContentBlock::builder()
        .base64("dGVzdA==".to_string())
        .mime_type("audio/mp3".to_string())
        .build()
        .unwrap();
    assert_eq!(block.base64.as_ref().unwrap(), "dGVzdA==");
    assert_eq!(block.mime_type.as_ref().unwrap(), "audio/mp3");
}

#[test]
fn test_create_audio_block_with_file_id() {
    let block = AudioContentBlock::builder()
        .file_id("file-123".to_string())
        .build()
        .unwrap();
    assert_eq!(block.file_id.as_ref().unwrap(), "file-123");
}

#[test]
fn test_create_audio_block_requires_source() {
    let result = AudioContentBlock::builder().build();
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Must provide one of"));
}

#[test]
fn test_create_audio_block_base64_requires_mime_type() {
    let result = AudioContentBlock::builder()
        .base64("dGVzdA==".to_string())
        .build();
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("mime_type is required"));
}

#[test]
fn test_create_audio_block_with_custom_id() {
    let block = AudioContentBlock::builder()
        .url("https://example.com/audio.mp3".to_string())
        .id("aud-123".to_string())
        .build()
        .unwrap();
    assert_eq!(block.id.as_ref().unwrap(), "aud-123");
}

#[test]
fn test_create_file_block_with_url() {
    let block = FileContentBlock::builder()
        .url("https://example.com/doc.pdf".to_string())
        .build()
        .unwrap();
    assert_eq!(block.url.as_ref().unwrap(), "https://example.com/doc.pdf");
}

#[test]
fn test_create_file_block_with_base64() {
    let block = FileContentBlock::builder()
        .base64("dGVzdA==".to_string())
        .mime_type("application/pdf".to_string())
        .build()
        .unwrap();
    assert_eq!(block.base64.as_ref().unwrap(), "dGVzdA==");
    assert_eq!(block.mime_type.as_ref().unwrap(), "application/pdf");
}

#[test]
fn test_create_file_block_with_file_id() {
    let block = FileContentBlock::builder()
        .file_id("file-123".to_string())
        .build()
        .unwrap();
    assert_eq!(block.file_id.as_ref().unwrap(), "file-123");
}

#[test]
fn test_create_file_block_requires_source() {
    let result = FileContentBlock::builder().build();
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Must provide one of"));
}

#[test]
fn test_create_file_block_base64_requires_mime_type() {
    let result = FileContentBlock::builder()
        .base64("dGVzdA==".to_string())
        .build();
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("mime_type is required"));
}

#[test]
fn test_create_file_block_with_custom_id() {
    let block = FileContentBlock::builder()
        .url("https://example.com/doc.pdf".to_string())
        .id("file-custom".to_string())
        .build()
        .unwrap();
    assert_eq!(block.id.as_ref().unwrap(), "file-custom");
}

#[test]
fn test_create_file_block_all_fields_populated() {
    let block = FileContentBlock::builder()
        .url("https://example.com/report.pdf".to_string())
        .mime_type("application/pdf".to_string())
        .id("file-all".to_string())
        .index(7.into())
        .build()
        .unwrap();
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
    let block = PlainTextContentBlock::builder()
        .text("Hello world".to_string())
        .build();
    assert_eq!(block.text.as_ref().unwrap(), "Hello world");
    assert_eq!(block.mime_type, "text/plain");
}

#[test]
fn test_plaintext_block_with_url() {
    let block = PlainTextContentBlock::builder()
        .url("https://example.com/file.txt".to_string())
        .build();
    assert_eq!(block.url.as_ref().unwrap(), "https://example.com/file.txt");
}

#[test]
fn test_plaintext_block_with_base64() {
    let block = PlainTextContentBlock::builder()
        .base64("SGVsbG8gd29ybGQ=".to_string())
        .build();
    assert_eq!(block.base64.as_ref().unwrap(), "SGVsbG8gd29ybGQ=");
}

#[test]
fn test_plaintext_block_with_file_id() {
    let block = PlainTextContentBlock::builder()
        .file_id("file-123".to_string())
        .build();
    assert_eq!(block.file_id.as_ref().unwrap(), "file-123");
}

#[test]
fn test_plaintext_block_with_title() {
    let block = PlainTextContentBlock::builder()
        .text("Hello".to_string())
        .title("My Document".to_string())
        .build();
    assert_eq!(block.title.as_ref().unwrap(), "My Document");
}

#[test]
fn test_plaintext_block_with_context() {
    let block = PlainTextContentBlock::builder()
        .text("Hello".to_string())
        .context("Important information".to_string())
        .build();
    assert_eq!(block.context.as_ref().unwrap(), "Important information");
}

#[test]
fn test_plaintext_block_with_custom_id() {
    let block = PlainTextContentBlock::builder()
        .text("Hello".to_string())
        .id("txt-123".to_string())
        .build();
    assert_eq!(block.id.as_ref().unwrap(), "txt-123");
}

#[test]
fn test_plaintext_block_all_fields_populated() {
    let block = PlainTextContentBlock::builder()
        .text("some text".to_string())
        .url("https://example.com/file.txt".to_string())
        .base64("c29tZSB0ZXh0".to_string())
        .file_id("file-xyz".to_string())
        .title("My Document".to_string())
        .context("Summary context".to_string())
        .id("pt-999".to_string())
        .index(3.into())
        .build();
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
    let block = PlainTextContentBlock::builder()
        .text("hello".to_string())
        .extras(extras)
        .build();
    let block_extras = block.extras.as_ref().unwrap();
    assert_eq!(block_extras["custom_key"], json!("custom_value"));
    assert_eq!(block_extras["another"], json!("val"));
}

#[test]
fn test_basic_tool_call() {
    let mut args = HashMap::new();
    args.insert("param".to_string(), json!("value"));
    let block = ToolCallBlock::builder()
        .name("test_tool")
        .args(args)
        .build();
    assert_eq!(block.name, "test_tool");
    assert_eq!(block.args["param"], json!("value"));
    assert!(block.id.as_ref().unwrap().starts_with("lc_"));
}

#[test]
fn test_tool_call_with_custom_id() {
    let block = ToolCallBlock::builder()
        .name("test_tool")
        .args(HashMap::new())
        .id("call-123".to_string())
        .build();
    assert_eq!(block.id.as_ref().unwrap(), "call-123");
}

#[test]
fn test_tool_call_with_index() {
    let block = ToolCallBlock::builder()
        .name("test_tool")
        .args(HashMap::new())
        .index(0.into())
        .build();
    assert!(block.index.is_some());
}

#[test]
fn test_tool_call_with_extras() {
    let mut extras = HashMap::new();
    extras.insert("custom".to_string(), json!("value"));
    let block = ToolCallBlock::builder()
        .name("test_tool")
        .args(HashMap::new())
        .extras(extras)
        .build();
    assert_eq!(block.extras.as_ref().unwrap()["custom"], json!("value"));
}

#[test]
fn test_tool_call_empty_args() {
    let block = ToolCallBlock::builder()
        .name("test_tool")
        .args(HashMap::new())
        .build();
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

    let block = ToolCallBlock::builder()
        .name("test_tool")
        .args(args.clone())
        .build();
    assert_eq!(block.args["string"], json!("value"));
    assert_eq!(block.args["number"], json!(42));
    assert_eq!(block.args["nested"], json!({"key": "value"}));
    assert_eq!(block.args["list"], json!([1, 2, 3]));
}

#[test]
fn test_tool_call_auto_generates_id_when_not_provided() {
    let block = ToolCallBlock::builder()
        .name("my_tool")
        .args(HashMap::from([("x".to_string(), json!(1))]))
        .build();
    let id = block.id.as_ref().unwrap();
    assert!(id.starts_with("lc_"));
}

#[test]
fn test_tool_call_auto_generated_ids_are_unique() {
    let block_a = ToolCallBlock::builder()
        .name("t")
        .args(HashMap::new())
        .build();
    let block_b = ToolCallBlock::builder()
        .name("t")
        .args(HashMap::new())
        .build();
    assert_ne!(block_a.id, block_b.id);
}

#[test]
fn test_basic_reasoning_block() {
    let block = ReasoningContentBlock::builder()
        .reasoning("Let me think about this...")
        .build();
    assert_eq!(
        block.reasoning.as_ref().unwrap(),
        "Let me think about this..."
    );
    assert!(block.id.is_some());
}

#[test]
fn test_reasoning_block_with_custom_id() {
    let block = ReasoningContentBlock::builder()
        .reasoning("Thinking...")
        .id("reason-123".to_string())
        .build();
    assert_eq!(block.id.as_ref().unwrap(), "reason-123");
}

#[test]
fn test_reasoning_block_with_index() {
    let block = ReasoningContentBlock::builder()
        .reasoning("Thinking...")
        .index(0.into())
        .build();
    assert!(block.index.is_some());
}

#[test]
fn test_reasoning_block_empty_reasoning() {
    let block = ReasoningContentBlock::builder().reasoning("").build();
    assert_eq!(block.reasoning.as_ref().unwrap(), "");
}

#[test]
fn test_reasoning_block_with_extras() {
    let mut extras = HashMap::new();
    extras.insert("signature".to_string(), json!("abc123"));
    let block = ReasoningContentBlock::builder()
        .reasoning("Thinking...")
        .extras(extras)
        .build();
    assert_eq!(block.extras.as_ref().unwrap()["signature"], json!("abc123"));
}

#[test]
fn test_reasoning_block_none_reasoning_defaults_to_empty_string() {
    let block = ReasoningContentBlock::builder().reasoning("").build();
    assert_eq!(block.reasoning.as_ref().unwrap(), "");
    assert!(block.id.is_some());
}

#[test]
fn test_basic_citation() {
    let annotation = Annotation::citation()
        .url("https://example.com/source".to_string())
        .call();
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
    let annotation = Annotation::citation()
        .url("https://example.com/source".to_string())
        .title("Source Document".to_string())
        .start_index(0)
        .end_index(100)
        .cited_text("This is the cited text.".to_string())
        .id("cite-123".to_string())
        .call();
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
            assert_eq!(start_index, &Some(0));
            assert_eq!(end_index, &Some(100));
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
    let annotation = Annotation::citation()
        .url("https://example.com".to_string())
        .extras(extras)
        .call();
    match &annotation {
        Annotation::Citation { extras, .. } => {
            assert_eq!(extras.as_ref().unwrap()["custom_field"], json!("value"));
        }
        _ => panic!("Expected Citation variant"),
    }
}

#[test]
fn test_citation_with_no_optional_fields() {
    let annotation = Annotation::citation().call();
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
    let block = NonStandardContentBlock::builder()
        .value(value.clone())
        .build();
    assert_eq!(block.value["custom"], json!("data"));
    assert!(block.id.is_some());
}

#[test]
fn test_non_standard_block_with_custom_id() {
    let mut value = HashMap::new();
    value.insert("key".to_string(), json!("value"));
    let block = NonStandardContentBlock::builder()
        .value(value)
        .id("ns-123".to_string())
        .build();
    assert_eq!(block.id.as_ref().unwrap(), "ns-123");
}

#[test]
fn test_non_standard_block_with_index() {
    let mut value = HashMap::new();
    value.insert("key".to_string(), json!("value"));
    let block = NonStandardContentBlock::builder()
        .value(value)
        .index(0.into())
        .build();
    assert!(block.index.is_some());
}

#[test]
fn test_non_standard_block_complex_value() {
    let mut value = HashMap::new();
    value.insert("nested".to_string(), json!({"deep": {"data": "value"}}));
    value.insert("list".to_string(), json!([1, 2, 3]));
    value.insert("string".to_string(), json!("text"));
    let block = NonStandardContentBlock::builder().value(value).build();
    assert_eq!(block.value["nested"], json!({"deep": {"data": "value"}}));
    assert_eq!(block.value["list"], json!([1, 2, 3]));
    assert_eq!(block.value["string"], json!("text"));
}

#[test]
fn test_non_standard_block_empty_dict_value() {
    let block = NonStandardContentBlock::builder()
        .value(HashMap::new())
        .build();
    assert!(block.value.is_empty());
    assert!(block.id.as_ref().unwrap().starts_with("lc_"));
}

#[test]
fn test_text_content_block_structure() {
    let block = TextContentBlock::builder().text("Hello").build();
    assert_eq!(block.text, "Hello");
}

#[test]
fn test_tool_call_block_structure() {
    let mut args = HashMap::new();
    args.insert("param".to_string(), json!("value"));
    let mut block = ToolCallBlock::builder()
        .name("test_tool")
        .args(args)
        .build();
    block.id = Some("123".to_string());
    assert_eq!(block.name, "test_tool");
}

#[test]
fn test_tool_call_chunk_block_structure() {
    let mut block = ToolCallChunkBlock::builder().build();
    block.id = Some("123".to_string());
    block.name = Some("test_tool".to_string());
    block.args = Some(r#"{"param": "value"}"#.to_string());
    assert_eq!(block.args.as_ref().unwrap(), r#"{"param": "value"}"#);
}

#[test]
fn test_invalid_tool_call_block_structure() {
    let mut block = InvalidToolCallBlock::builder().build();
    block.id = Some("123".to_string());
    block.name = Some("test_tool".to_string());
    block.args = Some("invalid json".to_string());
    block.error = Some("JSON parse error".to_string());
    assert_eq!(block.error.as_ref().unwrap(), "JSON parse error");
}

#[test]
fn test_reasoning_content_block_structure() {
    let block = ReasoningContentBlock::builder()
        .reasoning("Let me think...")
        .build();
    assert!(block.reasoning.is_some());
}

#[test]
fn test_image_content_block_structure() {
    let block = ImageContentBlock::builder()
        .url("https://example.com/image.png".to_string())
        .build()
        .unwrap();
    assert_eq!(block.url.as_ref().unwrap(), "https://example.com/image.png");
}

#[test]
fn test_audio_content_block_structure() {
    let block = AudioContentBlock::builder()
        .url("https://example.com/audio.mp3".to_string())
        .build()
        .unwrap();
    assert_eq!(block.url.as_ref().unwrap(), "https://example.com/audio.mp3");
}

#[test]
fn test_video_content_block_structure() {
    let block = VideoContentBlock::builder()
        .url("https://example.com/video.mp4".to_string())
        .build()
        .unwrap();
    assert_eq!(block.url.as_ref().unwrap(), "https://example.com/video.mp4");
}

#[test]
fn test_file_content_block_structure() {
    let block = FileContentBlock::builder()
        .url("https://example.com/doc.pdf".to_string())
        .build()
        .unwrap();
    assert_eq!(block.url.as_ref().unwrap(), "https://example.com/doc.pdf");
}

#[test]
fn test_plaintext_content_block_structure() {
    let mut block = PlainTextContentBlock::builder().build();
    block.text = Some("Hello".to_string());
    assert_eq!(block.mime_type, "text/plain");
}

#[test]
fn test_citation_structure() {
    let annotation = Annotation::citation().call();
    match &annotation {
        Annotation::Citation { .. } => {}
        _ => panic!("Expected Citation variant"),
    }
}

#[test]
fn test_non_standard_annotation_structure() {
    let mut value = HashMap::new();
    value.insert("custom".to_string(), json!("data"));
    let annotation = Annotation::non_standard().value(value).call();
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
    let block = ServerToolCall::builder()
        .id("stc-123")
        .name("web_search")
        .args(args)
        .build();
    assert_eq!(block.name, "web_search");
}

#[test]
fn test_server_tool_call_chunk_structure() {
    let block = ServerToolCallChunk::builder().build();
    assert!(block.id.is_none());
}

#[test]
fn test_server_tool_result_structure() {
    let block = ServerToolResult::success().tool_call_id("stc-123").call();
    assert_eq!(block.tool_call_id, "stc-123");
    assert_eq!(block.status, ServerToolStatus::Success);
}

#[test]
fn test_non_standard_content_block_structure() {
    let mut value = HashMap::new();
    value.insert("provider".to_string(), json!("custom"));
    value.insert("data".to_string(), json!("value"));
    let block = NonStandardContentBlock::builder().value(value).build();
    assert_eq!(block.value["provider"], json!("custom"));
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
    let block = ContentBlock::Text(TextContentBlock::builder().text("hi").build());
    assert_content_block_has_type(&block);
}

#[test]
fn test_content_block_tool_call() {
    let mut block = ToolCallBlock::builder()
        .name("fn")
        .args(HashMap::new())
        .build();
    block.id = Some("tc1".to_string());
    let cb = ContentBlock::ToolCall(block);
    assert_content_block_has_type(&cb);
}

#[test]
fn test_content_block_invalid_tool_call() {
    let mut block = InvalidToolCallBlock::builder().build();
    block.id = Some("itc1".to_string());
    block.name = Some("fn".to_string());
    block.args = Some("bad".to_string());
    block.error = Some("parse error".to_string());
    let cb = ContentBlock::InvalidToolCall(block);
    assert_content_block_has_type(&cb);
}

#[test]
fn test_content_block_tool_call_chunk() {
    let mut block = ToolCallChunkBlock::builder().build();
    block.id = Some("tcc1".to_string());
    block.name = Some("fn".to_string());
    block.args = Some(r#"{"a":1}"#.to_string());
    let cb = ContentBlock::ToolCallChunk(block);
    assert_content_block_has_type(&cb);
}

#[test]
fn test_content_block_reasoning() {
    let block = ReasoningContentBlock::builder()
        .reasoning("thinking")
        .build();
    let cb = ContentBlock::Reasoning(block);
    assert_content_block_has_type(&cb);
}

#[test]
fn test_content_block_image() {
    let cb = ContentBlock::Image(
        ImageContentBlock::builder()
            .url("https://example.com/img.png".to_string())
            .build()
            .unwrap(),
    );
    assert_content_block_has_type(&cb);
}

#[test]
fn test_content_block_video() {
    let block = VideoContentBlock::builder()
        .url("https://example.com/vid.mp4".to_string())
        .build()
        .unwrap();
    let cb = ContentBlock::Video(block);
    assert_content_block_has_type(&cb);
}

#[test]
fn test_content_block_audio() {
    let block = AudioContentBlock::builder()
        .url("https://example.com/aud.mp3".to_string())
        .build()
        .unwrap();
    let cb = ContentBlock::Audio(block);
    assert_content_block_has_type(&cb);
}

#[test]
fn test_content_block_plaintext() {
    let mut block = PlainTextContentBlock::builder().build();
    block.text = Some("hello".to_string());
    let cb = ContentBlock::PlainText(block);
    assert_content_block_has_type(&cb);
}

#[test]
fn test_content_block_file() {
    let block = FileContentBlock::builder()
        .url("https://example.com/doc.pdf".to_string())
        .build()
        .unwrap();
    let cb = ContentBlock::File(block);
    assert_content_block_has_type(&cb);
}

#[test]
fn test_content_block_server_tool_call() {
    let block = ServerToolCall::builder()
        .id("stc1")
        .name("search")
        .args(HashMap::new())
        .build();
    let cb = ContentBlock::ServerToolCall(block);
    assert_content_block_has_type(&cb);
}

#[test]
fn test_content_block_server_tool_call_chunk() {
    let cb = ContentBlock::ServerToolCallChunk(ServerToolCallChunk::builder().build());
    assert_content_block_has_type(&cb);
}

#[test]
fn test_content_block_server_tool_result() {
    let cb =
        ContentBlock::ServerToolResult(ServerToolResult::success().tool_call_id("stc1").call());
    assert_content_block_has_type(&cb);
}

#[test]
fn test_content_block_non_standard() {
    let mut value = HashMap::new();
    value.insert("data".to_string(), json!("custom"));
    let cb = ContentBlock::NonStandard(NonStandardContentBlock::builder().value(value).build());
    assert_content_block_has_type(&cb);
}

fn assert_data_block_has_type(block: &DataContentBlock) {
    let json = serde_json::to_value(block).unwrap();
    assert!(json.get("type").is_some());
}

#[test]
fn test_data_content_block_image() {
    let block = DataContentBlock::Image(
        ImageContentBlock::builder()
            .url("https://example.com/img.png".to_string())
            .build()
            .unwrap(),
    );
    assert_data_block_has_type(&block);
}

#[test]
fn test_data_content_block_video() {
    let block = VideoContentBlock::builder()
        .url("https://example.com/vid.mp4".to_string())
        .build()
        .unwrap();
    let dcb = DataContentBlock::Video(block);
    assert_data_block_has_type(&dcb);
}

#[test]
fn test_data_content_block_audio() {
    let block = AudioContentBlock::builder()
        .url("https://example.com/aud.mp3".to_string())
        .build()
        .unwrap();
    let dcb = DataContentBlock::Audio(block);
    assert_data_block_has_type(&dcb);
}

#[test]
fn test_data_content_block_plaintext() {
    let mut block = PlainTextContentBlock::builder().build();
    block.text = Some("hi".to_string());
    let dcb = DataContentBlock::PlainText(block);
    assert_data_block_has_type(&dcb);
}

#[test]
fn test_data_content_block_file() {
    let block = FileContentBlock::builder()
        .url("https://example.com/doc.pdf".to_string())
        .build()
        .unwrap();
    let dcb = DataContentBlock::File(block);
    assert_data_block_has_type(&dcb);
}

#[test]
fn test_data_content_block_union_has_exactly_five_members() {
    let variants: Vec<DataContentBlock> = vec![
        DataContentBlock::Image(
            ImageContentBlock::builder()
                .url("dummy".to_string())
                .build()
                .unwrap(),
        ),
        DataContentBlock::Video(
            VideoContentBlock::builder()
                .url("dummy".to_string())
                .build()
                .unwrap(),
        ),
        DataContentBlock::Audio(
            AudioContentBlock::builder()
                .url("dummy".to_string())
                .build()
                .unwrap(),
        ),
        DataContentBlock::PlainText(PlainTextContentBlock::builder().build()),
        DataContentBlock::File(
            FileContentBlock::builder()
                .url("dummy".to_string())
                .build()
                .unwrap(),
        ),
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
