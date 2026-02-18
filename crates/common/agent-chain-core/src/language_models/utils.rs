//! Utility functions for language models.
//!
//! This module contains helper functions for working with language models,
//! including message normalization and content block utilities.
//! Mirrors `langchain_core.language_models._utils`.

use crate::messages::BaseMessage;
use crate::messages::content::{ContentPart, MessageContent};
use std::collections::HashMap;

use regex::Regex;
use serde::{Deserialize, Serialize};

/// Filter type for OpenAI data blocks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataBlockFilter {
    /// Only match image blocks.
    Image,
    /// Only match audio blocks.
    Audio,
    /// Only match file blocks.
    File,
}

/// Check whether a block contains multimodal data in OpenAI Chat Completions format.
///
/// Supports both data and ID-style blocks (e.g. `'file_data'` and `'file_id'`)
///
/// # Arguments
///
/// * `block` - The content block to check.
/// * `filter` - If provided, only return true for blocks matching this specific type.
///
/// # Returns
///
/// `true` if the block is a valid OpenAI data block and matches the filter (if provided).
pub fn is_openai_data_block(block: &serde_json::Value, filter: Option<DataBlockFilter>) -> bool {
    let block_type = block.get("type").and_then(|t| t.as_str());

    match block_type {
        Some("image_url") => {
            if let Some(f) = filter
                && f != DataBlockFilter::Image
            {
                return false;
            }

            if let Some(obj) = block.as_object()
                && !obj
                    .keys()
                    .all(|k| k == "type" || k == "image_url" || k == "detail")
            {
                return false;
            }

            if let Some(image_url) = block.get("image_url")
                && let Some(obj) = image_url.as_object()
            {
                return obj.get("url").and_then(|u| u.as_str()).is_some();
            }
            false
        }
        Some("input_audio") => {
            if let Some(f) = filter
                && f != DataBlockFilter::Audio
            {
                return false;
            }

            if let Some(audio) = block.get("input_audio")
                && let Some(obj) = audio.as_object()
            {
                let has_data = obj.get("data").and_then(|d| d.as_str()).is_some();
                let has_format = obj.get("format").and_then(|f| f.as_str()).is_some();
                return has_data && has_format;
            }
            false
        }
        Some("file") => {
            if let Some(f) = filter
                && f != DataBlockFilter::File
            {
                return false;
            }

            if let Some(file) = block.get("file")
                && let Some(obj) = file.as_object()
            {
                let has_file_data = obj.get("file_data").and_then(|d| d.as_str()).is_some();
                let has_file_id = obj.get("file_id").and_then(|d| d.as_str()).is_some();
                return has_file_data || has_file_id;
            }
            false
        }
        _ => false,
    }
}

/// Parsed data URI components.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedDataUri {
    /// Source type (always "base64" for data URIs).
    pub source_type: String,
    /// The base64-encoded data.
    pub data: String,
    /// The MIME type of the data.
    pub mime_type: String,
}

/// Parse a data URI into its components.
///
/// # Arguments
///
/// * `uri` - The data URI to parse (e.g., "data:image/jpeg;base64,/9j/4AAQ...")
///
/// # Returns
///
/// `Some(ParsedDataUri)` if parsing succeeds, `None` otherwise.
pub fn parse_data_uri(uri: &str) -> Option<ParsedDataUri> {
    let re = Regex::new(r"^data:(?P<mime_type>[^;]+);base64,(?P<data>.+)$").ok()?;
    let captures = re.captures(uri)?;

    let mime_type = captures.name("mime_type")?.as_str();
    let data = captures.name("data")?.as_str();

    if mime_type.is_empty() || data.is_empty() {
        return None;
    }

    Some(ParsedDataUri {
        source_type: "base64".to_string(),
        data: data.to_string(),
        mime_type: mime_type.to_string(),
    })
}

/// Get a default tokenizer estimate for token counting.
///
/// This provides a rough estimate based on whitespace splitting.
/// For accurate counts, use a proper tokenizer for the specific model.
///
/// # Arguments
///
/// * `text` - The text to tokenize.
///
/// # Returns
///
/// Estimated token IDs (just indices in this simple implementation).
pub fn get_token_ids_default(text: &str) -> Vec<u32> {
    text.split_whitespace()
        .enumerate()
        .map(|(i, _)| i as u32)
        .collect()
}

/// Estimate the number of tokens in a text.
///
/// This is a rough estimate. For accurate counts, use model-specific tokenizers.
///
/// # Arguments
///
/// * `text` - The text to count tokens for.
///
/// # Returns
///
/// Estimated token count.
pub fn estimate_token_count(text: &str) -> usize {
    let char_count = text.chars().count();
    char_count.div_ceil(4)
}

/// Convert a v0 content block format to v1 format.
///
/// LangChain v0 content blocks had different structure than v1.
/// This function converts the older format to the newer standard.
pub fn convert_legacy_v0_content_block_to_v1(
    block: &HashMap<String, serde_json::Value>,
) -> HashMap<String, serde_json::Value> {
    let mut result = HashMap::new();

    let block_type = block.get("type").and_then(|t| t.as_str()).unwrap_or("text");
    result.insert(
        "type".to_string(),
        serde_json::Value::String(block_type.to_string()),
    );

    let source_type = block.get("source_type").and_then(|t| t.as_str());

    match source_type {
        Some("base64") => {
            if let Some(data) = block.get("data") {
                result.insert("base64".to_string(), data.clone());
            }
            if let Some(mime_type) = block.get("mime_type") {
                result.insert("mime_type".to_string(), mime_type.clone());
            }
        }
        Some("url") => {
            if let Some(url) = block.get("url") {
                result.insert("url".to_string(), url.clone());
            }
            if let Some(mime_type) = block.get("mime_type") {
                result.insert("mime_type".to_string(), mime_type.clone());
            }
        }
        Some("id") => {
            if let Some(id) = block.get("id") {
                result.insert("file_id".to_string(), id.clone());
            }
        }
        Some("text") => {
            if let Some(text) = block.get("text") {
                result.insert("text".to_string(), text.clone());
            }
        }
        _ => {
            for (key, value) in block {
                if key != "source_type" {
                    result.insert(key.clone(), value.clone());
                }
            }
        }
    }

    result
}

/// Convert an OpenAI format content block to a standard data block.
pub fn convert_openai_format_to_data_block(
    block: &serde_json::Value,
) -> HashMap<String, serde_json::Value> {
    let mut result = HashMap::new();

    let block_type = block.get("type").and_then(|t| t.as_str()).unwrap_or("");

    match block_type {
        "image_url" => {
            result.insert(
                "type".to_string(),
                serde_json::Value::String("image".to_string()),
            );

            if let Some(image_url) = block.get("image_url").and_then(|i| i.as_object()) {
                if let Some(url) = image_url.get("url").and_then(|u| u.as_str()) {
                    if let Some(parsed) = parse_data_uri(url) {
                        result.insert("base64".to_string(), serde_json::Value::String(parsed.data));
                        result.insert(
                            "mime_type".to_string(),
                            serde_json::Value::String(parsed.mime_type),
                        );
                    } else {
                        result.insert(
                            "url".to_string(),
                            serde_json::Value::String(url.to_string()),
                        );
                    }
                }
                if let Some(detail) = image_url.get("detail") {
                    result.insert("detail".to_string(), detail.clone());
                }
            }
        }
        "input_audio" => {
            result.insert(
                "type".to_string(),
                serde_json::Value::String("audio".to_string()),
            );

            if let Some(audio) = block.get("input_audio").and_then(|a| a.as_object()) {
                if let Some(data) = audio.get("data").and_then(|d| d.as_str()) {
                    result.insert(
                        "base64".to_string(),
                        serde_json::Value::String(data.to_string()),
                    );
                }
                if let Some(format) = audio.get("format").and_then(|f| f.as_str()) {
                    let mime_type = match format {
                        "wav" => "audio/wav",
                        "mp3" => "audio/mpeg",
                        _ => format,
                    };
                    result.insert(
                        "mime_type".to_string(),
                        serde_json::Value::String(mime_type.to_string()),
                    );
                }
            }
        }
        "file" => {
            result.insert(
                "type".to_string(),
                serde_json::Value::String("file".to_string()),
            );

            if let Some(file) = block.get("file").and_then(|f| f.as_object()) {
                if let Some(file_data) = file.get("file_data").and_then(|d| d.as_str()) {
                    result.insert(
                        "base64".to_string(),
                        serde_json::Value::String(file_data.to_string()),
                    );
                }
                if let Some(file_id) = file.get("file_id").and_then(|d| d.as_str()) {
                    result.insert(
                        "file_id".to_string(),
                        serde_json::Value::String(file_id.to_string()),
                    );
                }
                if let Some(filename) = file.get("filename").and_then(|f| f.as_str()) {
                    result.insert(
                        "filename".to_string(),
                        serde_json::Value::String(filename.to_string()),
                    );
                }
            }
        }
        _ => {
            if let Some(obj) = block.as_object() {
                for (key, value) in obj {
                    result.insert(key.clone(), value.clone());
                }
            }
        }
    }

    result
}

/// Update message content to use content blocks format.
///
/// Creates a copy of the AIMessage where `content` is replaced by the
/// serialized `content_blocks()` result, and `response_metadata["output_version"]`
/// is set to the given version string.
///
/// This mirrors Python's `_update_message_content_to_blocks`.
pub fn update_message_content_to_blocks(
    message: &crate::messages::AIMessage,
    output_version: &str,
) -> crate::messages::AIMessage {
    let content_blocks = message.content_blocks();

    let block_values: Vec<serde_json::Value> = content_blocks
        .iter()
        .filter_map(|block| serde_json::to_value(block).ok())
        .collect();

    let new_content: MessageContent = if block_values.is_empty() {
        message.content.clone()
    } else {
        let values: Vec<serde_json::Value> = block_values;
        values.into()
    };

    let mut new_metadata = message.response_metadata.clone();
    new_metadata.insert(
        "output_version".to_string(),
        serde_json::Value::String(output_version.to_string()),
    );

    crate::messages::AIMessage::builder()
        .content(new_content)
        .response_metadata(new_metadata)
        .tool_calls(message.tool_calls.clone())
        .invalid_tool_calls(message.invalid_tool_calls.clone())
        .maybe_id(message.id.clone())
        .maybe_name(message.name.clone())
        .maybe_usage_metadata(message.usage_metadata.clone())
        .build()
}

/// Normalize message content blocks to LangChain v1 standard format.
///
/// Converts OpenAI Chat Completions multimodal blocks and LangChain v0
/// blocks to v1 standard format. Messages with plain string content or
/// already-v1 blocks pass through unchanged.
///
/// This mirrors Python's `_normalize_messages` from
/// `langchain_core.language_models._utils`.
pub fn normalize_messages(messages: Vec<BaseMessage>) -> Vec<BaseMessage> {
    messages.into_iter().map(normalize_single_message).collect()
}

fn normalize_single_message(mut message: BaseMessage) -> BaseMessage {
    let parts = match message.content() {
        MessageContent::Parts(parts) => parts.clone(),
        MessageContent::Text(_) => return message,
    };

    let mut modified = false;
    let new_parts: Vec<ContentPart> = parts
        .into_iter()
        .map(|part| match &part {
            ContentPart::Other(value) => {
                let block_type = value.get("type").and_then(|t| t.as_str());

                if matches!(block_type, Some("input_audio") | Some("file"))
                    && is_openai_data_block(value, None)
                {
                    modified = true;
                    let converted = convert_openai_format_to_data_block(value);
                    let value = serde_json::to_value(converted).unwrap_or_else(|_| value.clone());
                    return ContentPart::Other(value);
                }

                let source_type = value.get("source_type").and_then(|s| s.as_str());
                if matches!(block_type, Some("image") | Some("audio") | Some("file"))
                    && matches!(
                        source_type,
                        Some("url") | Some("base64") | Some("id") | Some("text")
                    )
                {
                    modified = true;
                    if let Some(obj) = value.as_object() {
                        let block_map: HashMap<String, serde_json::Value> =
                            obj.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
                        let converted = convert_legacy_v0_content_block_to_v1(&block_map);
                        let value =
                            serde_json::to_value(converted).unwrap_or_else(|_| value.clone());
                        return ContentPart::Other(value);
                    }
                }

                part
            }
            _ => part,
        })
        .collect();

    if modified {
        let new_content = MessageContent::Parts(new_parts);
        match &mut message {
            BaseMessage::Human(m) => m.content = new_content,
            BaseMessage::System(m) => m.content = new_content,
            BaseMessage::AI(m) => m.content = new_content,
            BaseMessage::Tool(m) => m.content = new_content,
            BaseMessage::Chat(m) => m.content = new_content,
            BaseMessage::Function(m) => m.content = new_content,
            BaseMessage::Remove(_) => {}
        }
    }

    message
}

/// Update an AIMessageChunk's content to use content blocks format.
///
/// Creates a new AIMessageChunk where `content` is replaced by the
/// serialized `content_blocks()` result, and
/// `response_metadata["output_version"]` is set to the given version string.
///
/// This mirrors Python's `_update_message_content_to_blocks` for chunks.
pub fn update_chunk_content_to_blocks(
    chunk: &crate::messages::AIMessageChunk,
    output_version: &str,
) -> crate::messages::AIMessageChunk {
    let content_blocks = chunk.content_blocks();

    let block_values: Vec<serde_json::Value> = content_blocks
        .iter()
        .filter_map(|block| serde_json::to_value(block).ok())
        .collect();

    let new_content: MessageContent = if block_values.is_empty() {
        chunk.content.clone()
    } else {
        let values: Vec<serde_json::Value> = block_values;
        values.into()
    };

    let mut new_metadata = chunk.response_metadata.clone();
    new_metadata.insert(
        "output_version".to_string(),
        serde_json::Value::String(output_version.to_string()),
    );

    crate::messages::AIMessageChunk::builder()
        .content(new_content)
        .response_metadata(new_metadata)
        .tool_calls(chunk.tool_calls.clone())
        .invalid_tool_calls(chunk.invalid_tool_calls.clone())
        .tool_call_chunks(chunk.tool_call_chunks.clone())
        .maybe_id(chunk.id.clone())
        .maybe_name(chunk.name.clone())
        .maybe_usage_metadata(chunk.usage_metadata.clone())
        .maybe_chunk_position(chunk.chunk_position.clone())
        .build()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_is_openai_data_block_image() {
        let block = json!({
            "type": "image_url",
            "image_url": {
                "url": "https://example.com/image.png"
            }
        });

        assert!(is_openai_data_block(&block, None));
        assert!(is_openai_data_block(&block, Some(DataBlockFilter::Image)));
        assert!(!is_openai_data_block(&block, Some(DataBlockFilter::Audio)));
    }

    #[test]
    fn test_is_openai_data_block_audio() {
        let block = json!({
            "type": "input_audio",
            "input_audio": {
                "data": "base64data",
                "format": "wav"
            }
        });

        assert!(is_openai_data_block(&block, None));
        assert!(is_openai_data_block(&block, Some(DataBlockFilter::Audio)));
        assert!(!is_openai_data_block(&block, Some(DataBlockFilter::Image)));
    }

    #[test]
    fn test_is_openai_data_block_file() {
        let block = json!({
            "type": "file",
            "file": {
                "file_id": "file-123"
            }
        });

        assert!(is_openai_data_block(&block, None));
        assert!(is_openai_data_block(&block, Some(DataBlockFilter::File)));
        assert!(!is_openai_data_block(&block, Some(DataBlockFilter::Image)));
    }

    #[test]
    fn test_is_openai_data_block_invalid() {
        let block = json!({
            "type": "text",
            "text": "Hello"
        });

        assert!(!is_openai_data_block(&block, None));
    }

    #[test]
    fn test_parse_data_uri() {
        let uri = "data:image/jpeg;base64,/9j/4AAQSkZJRg==";
        let parsed = parse_data_uri(uri).unwrap();

        assert_eq!(parsed.source_type, "base64");
        assert_eq!(parsed.mime_type, "image/jpeg");
        assert_eq!(parsed.data, "/9j/4AAQSkZJRg==");
    }

    #[test]
    fn test_parse_data_uri_invalid() {
        let uri = "https://example.com/image.png";
        assert!(parse_data_uri(uri).is_none());

        let uri = "data:;base64,";
        assert!(parse_data_uri(uri).is_none());
    }

    #[test]
    fn test_estimate_token_count() {
        let text = "Hello, world!";
        let count = estimate_token_count(text);
        assert!(count > 0);
        assert!(count < 10);
    }

    #[test]
    fn test_get_token_ids_default() {
        let text = "Hello world test";
        let ids = get_token_ids_default(text);
        assert_eq!(ids.len(), 3);
        assert_eq!(ids, vec![0, 1, 2]);
    }

    #[test]
    fn test_convert_openai_format_to_data_block_image_url() {
        let block = json!({
            "type": "image_url",
            "image_url": {
                "url": "https://example.com/image.png",
                "detail": "high"
            }
        });

        let result = convert_openai_format_to_data_block(&block);

        assert_eq!(result.get("type").unwrap(), "image");
        assert_eq!(result.get("url").unwrap(), "https://example.com/image.png");
        assert_eq!(result.get("detail").unwrap(), "high");
    }

    #[test]
    fn test_convert_openai_format_to_data_block_data_uri() {
        let block = json!({
            "type": "image_url",
            "image_url": {
                "url": "data:image/png;base64,iVBORw0KGgo="
            }
        });

        let result = convert_openai_format_to_data_block(&block);

        assert_eq!(result.get("type").unwrap(), "image");
        assert_eq!(result.get("base64").unwrap(), "iVBORw0KGgo=");
        assert_eq!(result.get("mime_type").unwrap(), "image/png");
    }

    #[test]
    fn test_convert_legacy_v0_content_block_to_v1_base64() {
        let mut block = HashMap::new();
        block.insert("type".to_string(), json!("image"));
        block.insert("source_type".to_string(), json!("base64"));
        block.insert("data".to_string(), json!("base64data"));
        block.insert("mime_type".to_string(), json!("image/png"));

        let result = convert_legacy_v0_content_block_to_v1(&block);

        assert_eq!(result.get("type").unwrap(), "image");
        assert_eq!(result.get("base64").unwrap(), "base64data");
        assert_eq!(result.get("mime_type").unwrap(), "image/png");
        assert!(!result.contains_key("source_type"));
    }

    #[test]
    fn test_normalize_messages_plain_text_passthrough() {
        use crate::messages::HumanMessage;

        let messages = vec![BaseMessage::Human(
            HumanMessage::builder().content("Hello").build(),
        )];
        let result = normalize_messages(messages.clone());
        assert_eq!(result, messages);
    }

    #[test]
    fn test_normalize_messages_v1_blocks_passthrough() {
        use crate::messages::HumanMessage;
        use crate::messages::content::ContentPart;

        let parts = vec![
            ContentPart::Text {
                text: "Hello".to_string(),
            },
            ContentPart::Other(json!({"type": "image", "url": "https://example.com/img.png"})),
        ];
        let messages = vec![BaseMessage::Human(
            HumanMessage::builder()
                .content(MessageContent::Parts(parts))
                .build(),
        )];
        let result = normalize_messages(messages.clone());
        assert_eq!(result, messages);
    }

    #[test]
    fn test_normalize_messages_openai_audio_converted() {
        use crate::messages::HumanMessage;
        use crate::messages::content::ContentPart;

        let parts = vec![ContentPart::Other(json!({
            "type": "input_audio",
            "input_audio": {
                "data": "base64audiodata",
                "format": "wav"
            }
        }))];
        let messages = vec![BaseMessage::Human(
            HumanMessage::builder()
                .content(MessageContent::Parts(parts))
                .build(),
        )];
        let result = normalize_messages(messages);
        let content = result[0].content();
        if let MessageContent::Parts(parts) = content {
            if let ContentPart::Other(val) = &parts[0] {
                assert_eq!(val.get("type").unwrap(), "audio");
                assert_eq!(val.get("base64").unwrap(), "base64audiodata");
                assert_eq!(val.get("mime_type").unwrap(), "audio/wav");
            } else {
                panic!("Expected Other content part");
            }
        } else {
            panic!("Expected Parts content");
        }
    }

    #[test]
    fn test_normalize_messages_openai_file_converted() {
        use crate::messages::HumanMessage;
        use crate::messages::content::ContentPart;

        let parts = vec![ContentPart::Other(json!({
            "type": "file",
            "file": {
                "file_id": "file-123"
            }
        }))];
        let messages = vec![BaseMessage::Human(
            HumanMessage::builder()
                .content(MessageContent::Parts(parts))
                .build(),
        )];
        let result = normalize_messages(messages);
        let content = result[0].content();
        if let MessageContent::Parts(parts) = content {
            if let ContentPart::Other(val) = &parts[0] {
                assert_eq!(val.get("type").unwrap(), "file");
                assert_eq!(val.get("file_id").unwrap(), "file-123");
            } else {
                panic!("Expected Other content part");
            }
        } else {
            panic!("Expected Parts content");
        }
    }

    #[test]
    fn test_normalize_messages_v0_image_url_converted() {
        use crate::messages::HumanMessage;
        use crate::messages::content::ContentPart;

        let parts = vec![ContentPart::Other(json!({
            "type": "image",
            "source_type": "url",
            "url": "https://example.com/img.png",
            "mime_type": "image/png"
        }))];
        let messages = vec![BaseMessage::Human(
            HumanMessage::builder()
                .content(MessageContent::Parts(parts))
                .build(),
        )];
        let result = normalize_messages(messages);
        let content = result[0].content();
        if let MessageContent::Parts(parts) = content {
            if let ContentPart::Other(val) = &parts[0] {
                assert_eq!(val.get("type").unwrap(), "image");
                assert_eq!(val.get("url").unwrap(), "https://example.com/img.png");
                assert!(val.get("source_type").is_none());
            } else {
                panic!("Expected Other content part");
            }
        } else {
            panic!("Expected Parts content");
        }
    }

    #[test]
    fn test_normalize_messages_v0_image_base64_converted() {
        use crate::messages::HumanMessage;
        use crate::messages::content::ContentPart;

        let parts = vec![ContentPart::Other(json!({
            "type": "image",
            "source_type": "base64",
            "data": "iVBORw0KGgo=",
            "mime_type": "image/png"
        }))];
        let messages = vec![BaseMessage::Human(
            HumanMessage::builder()
                .content(MessageContent::Parts(parts))
                .build(),
        )];
        let result = normalize_messages(messages);
        let content = result[0].content();
        if let MessageContent::Parts(parts) = content {
            if let ContentPart::Other(val) = &parts[0] {
                assert_eq!(val.get("type").unwrap(), "image");
                assert_eq!(val.get("base64").unwrap(), "iVBORw0KGgo=");
                assert_eq!(val.get("mime_type").unwrap(), "image/png");
                assert!(val.get("source_type").is_none());
            } else {
                panic!("Expected Other content part");
            }
        } else {
            panic!("Expected Parts content");
        }
    }

    #[test]
    fn test_normalize_messages_mixed_blocks() {
        use crate::messages::HumanMessage;
        use crate::messages::content::ContentPart;

        let parts = vec![
            ContentPart::Text {
                text: "Hello".to_string(),
            },
            ContentPart::Other(json!({
                "type": "input_audio",
                "input_audio": { "data": "audiodata", "format": "mp3" }
            })),
            ContentPart::Other(json!({"type": "image", "url": "https://example.com/img.png"})),
        ];
        let messages = vec![BaseMessage::Human(
            HumanMessage::builder()
                .content(MessageContent::Parts(parts))
                .build(),
        )];
        let result = normalize_messages(messages);
        let content = result[0].content();
        if let MessageContent::Parts(parts) = content {
            assert_eq!(parts.len(), 3);
            assert!(matches!(&parts[0], ContentPart::Text { text } if text == "Hello"));
            if let ContentPart::Other(val) = &parts[1] {
                assert_eq!(val.get("type").unwrap(), "audio");
            } else {
                panic!("Expected Other content part for audio");
            }
            if let ContentPart::Other(val) = &parts[2] {
                assert_eq!(val.get("type").unwrap(), "image");
                assert_eq!(val.get("url").unwrap(), "https://example.com/img.png");
            } else {
                panic!("Expected Other content part for image");
            }
        } else {
            panic!("Expected Parts content");
        }
    }

    #[test]
    fn test_update_chunk_content_to_blocks_basic() {
        use crate::messages::AIMessageChunk;

        let chunk = AIMessageChunk::builder().content("Hello world").build();
        let updated = update_chunk_content_to_blocks(&chunk, "v1");
        let metadata = updated.response_metadata;
        assert_eq!(
            metadata.get("output_version").unwrap(),
            &serde_json::Value::String("v1".to_string())
        );
    }

    #[test]
    fn test_update_chunk_content_to_blocks_preserves_metadata() {
        use crate::messages::AIMessageChunk;

        let mut existing_metadata = HashMap::new();
        existing_metadata.insert("model".to_string(), json!("gpt-4"));
        let chunk = AIMessageChunk::builder()
            .content("test")
            .response_metadata(existing_metadata)
            .build();
        let updated = update_chunk_content_to_blocks(&chunk, "v1");
        assert_eq!(
            updated.response_metadata.get("model").unwrap(),
            &json!("gpt-4")
        );
        assert_eq!(
            updated.response_metadata.get("output_version").unwrap(),
            &json!("v1")
        );
    }
}
