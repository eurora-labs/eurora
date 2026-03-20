use crate::messages::content::{ContentBlock, ContentBlocks};
use crate::messages::{AnyMessage, BaseMessage};
use std::collections::HashMap;

use regex::Regex;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataBlockFilter {
    Image,
    Audio,
    File,
}

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedDataUri {
    pub source_type: String,
    pub data: String,
    pub mime_type: String,
}

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

pub fn get_token_ids_default(text: &str) -> Vec<u32> {
    text.split_whitespace()
        .enumerate()
        .map(|(i, _)| i as u32)
        .collect()
}

pub fn estimate_token_count(text: &str) -> usize {
    let char_count = text.chars().count();
    char_count.div_ceil(4)
}

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

pub fn update_message_content_to_blocks(
    message: &crate::messages::AIMessage,
    output_version: &str,
) -> crate::messages::AIMessage {
    let content_blocks = message.content_blocks();

    let block_values: Vec<serde_json::Value> = content_blocks
        .iter()
        .filter_map(|block| serde_json::to_value(block).ok())
        .collect();

    let new_content: ContentBlocks = if block_values.is_empty() {
        message.content.clone()
    } else {
        block_values
            .into_iter()
            .filter_map(|v| serde_json::from_value::<ContentBlock>(v).ok())
            .collect()
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

pub fn normalize_messages(messages: Vec<AnyMessage>) -> Vec<AnyMessage> {
    messages.into_iter().map(normalize_single_message).collect()
}

fn normalize_single_message(mut message: AnyMessage) -> AnyMessage {
    let blocks: Vec<serde_json::Value> = message
        .content()
        .iter()
        .filter_map(|block| serde_json::to_value(block).ok())
        .collect();

    let mut modified = false;
    let new_blocks: Vec<serde_json::Value> = blocks
        .into_iter()
        .map(|value| {
            // For NonStandard blocks, the real content is in the "value" sub-object.
            // Unwrap it so downstream checks see the original type.
            let (value, block_type) = if value.get("type").and_then(|t| t.as_str())
                == Some("non_standard")
            {
                if let Some(inner) = value.get("value") {
                    let inner_type = inner.get("type").and_then(|t| t.as_str()).map(String::from);
                    (inner.clone(), inner_type)
                } else {
                    (value, Some("non_standard".to_string()))
                }
            } else {
                let bt = value.get("type").and_then(|t| t.as_str()).map(String::from);
                (value, bt)
            };
            let block_type = block_type.as_deref();

            if matches!(block_type, Some("input_audio") | Some("file"))
                && is_openai_data_block(&value, None)
            {
                modified = true;
                let converted = convert_openai_format_to_data_block(&value);
                return serde_json::to_value(converted).unwrap_or(value);
            }

            let source_type = value.get("source_type").and_then(|s| s.as_str());
            if matches!(block_type, Some("image") | Some("audio") | Some("file"))
                && matches!(
                    source_type,
                    Some("url") | Some("base64") | Some("id") | Some("text")
                )
                && let Some(obj) = value.as_object()
            {
                modified = true;
                let block_map: HashMap<String, serde_json::Value> =
                    obj.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
                let converted = convert_legacy_v0_content_block_to_v1(&block_map);
                return serde_json::to_value(converted).unwrap_or(value);
            }

            value
        })
        .collect();

    if modified {
        let new_content: ContentBlocks = new_blocks
            .into_iter()
            .filter_map(|v| serde_json::from_value::<ContentBlock>(v).ok())
            .collect();
        match &mut message {
            AnyMessage::HumanMessage(m) => m.content = new_content,
            AnyMessage::SystemMessage(m) => m.content = new_content,
            AnyMessage::AIMessage(m) => m.content = new_content,
            AnyMessage::ToolMessage(m) => m.content = new_content,
            AnyMessage::ChatMessage(m) => m.content = new_content,
            AnyMessage::RemoveMessage(_) => {}
        }
    }

    message
}

pub fn update_chunk_content_to_blocks(
    chunk: &crate::messages::AIMessageChunk,
    output_version: &str,
) -> crate::messages::AIMessageChunk {
    let content_blocks = chunk.content_blocks();

    let block_values: Vec<serde_json::Value> = content_blocks
        .iter()
        .filter_map(|block| serde_json::to_value(block).ok())
        .collect();

    let new_content: ContentBlocks = if block_values.is_empty() {
        chunk.content.clone()
    } else {
        block_values
            .into_iter()
            .filter_map(|v| serde_json::from_value::<ContentBlock>(v).ok())
            .collect()
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

        let messages = vec![AnyMessage::HumanMessage(
            HumanMessage::builder().content("Hello").build(),
        )];
        let result = normalize_messages(messages.clone());
        assert_eq!(result, messages);
    }

    #[test]
    fn test_normalize_messages_v1_blocks_passthrough() {
        use crate::messages::HumanMessage;
        use crate::messages::content::{ContentBlock, ContentBlocks, TextContentBlock};

        let blocks: ContentBlocks = vec![
            ContentBlock::Text(TextContentBlock::new("Hello")),
            serde_json::from_value::<ContentBlock>(
                json!({"type": "image", "url": "https://example.com/img.png"}),
            )
            .unwrap(),
        ]
        .into_iter()
        .collect();
        let messages = vec![AnyMessage::HumanMessage(
            HumanMessage::builder().content(blocks).build(),
        )];
        let result = normalize_messages(messages.clone());
        assert_eq!(result, messages);
    }

    #[test]
    fn test_normalize_messages_openai_audio_converted() {
        use crate::messages::HumanMessage;
        use crate::messages::content::{ContentBlock, ContentBlocks, NonStandardContentBlock};

        let mut value_map = HashMap::new();
        value_map.insert("type".to_string(), json!("input_audio"));
        value_map.insert(
            "input_audio".to_string(),
            json!({"data": "base64audiodata", "format": "wav"}),
        );
        let blocks: ContentBlocks = vec![ContentBlock::NonStandard(NonStandardContentBlock::new(
            value_map,
        ))]
        .into_iter()
        .collect();
        let messages = vec![AnyMessage::HumanMessage(
            HumanMessage::builder().content(blocks).build(),
        )];
        let result = normalize_messages(messages);
        let content = result[0].content();
        let values: Vec<serde_json::Value> = content
            .iter()
            .filter_map(|b| serde_json::to_value(b).ok())
            .collect();
        assert_eq!(values[0].get("type").unwrap(), "audio");
        assert_eq!(values[0].get("base64").unwrap(), "base64audiodata");
        assert_eq!(values[0].get("mime_type").unwrap(), "audio/wav");
    }

    #[test]
    fn test_normalize_messages_openai_file_converted() {
        use crate::messages::HumanMessage;
        use crate::messages::content::{ContentBlock, ContentBlocks, NonStandardContentBlock};

        let mut value_map = HashMap::new();
        value_map.insert("type".to_string(), json!("file"));
        value_map.insert("file".to_string(), json!({"file_id": "file-123"}));
        let blocks: ContentBlocks = vec![ContentBlock::NonStandard(NonStandardContentBlock::new(
            value_map,
        ))]
        .into_iter()
        .collect();
        let messages = vec![AnyMessage::HumanMessage(
            HumanMessage::builder().content(blocks).build(),
        )];
        let result = normalize_messages(messages);
        let content = result[0].content();
        let values: Vec<serde_json::Value> = content
            .iter()
            .filter_map(|b| serde_json::to_value(b).ok())
            .collect();
        assert_eq!(values[0].get("type").unwrap(), "file");
        assert_eq!(values[0].get("file_id").unwrap(), "file-123");
    }

    #[test]
    fn test_normalize_messages_v0_image_url_converted() {
        use crate::messages::HumanMessage;
        use crate::messages::content::{ContentBlock, ContentBlocks, NonStandardContentBlock};

        let mut value_map = HashMap::new();
        value_map.insert("type".to_string(), json!("image"));
        value_map.insert("source_type".to_string(), json!("url"));
        value_map.insert("url".to_string(), json!("https://example.com/img.png"));
        value_map.insert("mime_type".to_string(), json!("image/png"));
        let blocks: ContentBlocks = vec![ContentBlock::NonStandard(NonStandardContentBlock::new(
            value_map,
        ))]
        .into_iter()
        .collect();
        let messages = vec![AnyMessage::HumanMessage(
            HumanMessage::builder().content(blocks).build(),
        )];
        let result = normalize_messages(messages);
        let content = result[0].content();
        let values: Vec<serde_json::Value> = content
            .iter()
            .filter_map(|b| serde_json::to_value(b).ok())
            .collect();
        assert_eq!(values[0].get("type").unwrap(), "image");
        assert_eq!(values[0].get("url").unwrap(), "https://example.com/img.png");
        assert!(values[0].get("source_type").is_none());
    }

    #[test]
    fn test_normalize_messages_v0_image_base64_converted() {
        use crate::messages::HumanMessage;
        use crate::messages::content::{ContentBlock, ContentBlocks, NonStandardContentBlock};

        let mut value_map = HashMap::new();
        value_map.insert("type".to_string(), json!("image"));
        value_map.insert("source_type".to_string(), json!("base64"));
        value_map.insert("data".to_string(), json!("iVBORw0KGgo="));
        value_map.insert("mime_type".to_string(), json!("image/png"));
        let blocks: ContentBlocks = vec![ContentBlock::NonStandard(NonStandardContentBlock::new(
            value_map,
        ))]
        .into_iter()
        .collect();
        let messages = vec![AnyMessage::HumanMessage(
            HumanMessage::builder().content(blocks).build(),
        )];
        let result = normalize_messages(messages);
        let content = result[0].content();
        let values: Vec<serde_json::Value> = content
            .iter()
            .filter_map(|b| serde_json::to_value(b).ok())
            .collect();
        assert_eq!(values[0].get("type").unwrap(), "image");
        assert_eq!(values[0].get("base64").unwrap(), "iVBORw0KGgo=");
        assert_eq!(values[0].get("mime_type").unwrap(), "image/png");
        assert!(values[0].get("source_type").is_none());
    }

    #[test]
    fn test_normalize_messages_mixed_blocks() {
        use crate::messages::HumanMessage;
        use crate::messages::content::{
            ContentBlock, ContentBlocks, NonStandardContentBlock, TextContentBlock,
        };

        let mut audio_map = HashMap::new();
        audio_map.insert("type".to_string(), json!("input_audio"));
        audio_map.insert(
            "input_audio".to_string(),
            json!({"data": "audiodata", "format": "mp3"}),
        );

        let blocks: ContentBlocks = vec![
            ContentBlock::Text(TextContentBlock::new("Hello")),
            ContentBlock::NonStandard(NonStandardContentBlock::new(audio_map)),
            serde_json::from_value::<ContentBlock>(
                json!({"type": "image", "url": "https://example.com/img.png"}),
            )
            .unwrap(),
        ]
        .into_iter()
        .collect();
        let messages = vec![AnyMessage::HumanMessage(
            HumanMessage::builder().content(blocks).build(),
        )];
        let result = normalize_messages(messages);
        let content = result[0].content();
        let values: Vec<serde_json::Value> = content
            .iter()
            .filter_map(|b| serde_json::to_value(b).ok())
            .collect();
        assert_eq!(values.len(), 3);
        assert_eq!(values[0].get("type").unwrap(), "text");
        assert_eq!(values[0].get("text").unwrap(), "Hello");
        assert_eq!(values[1].get("type").unwrap(), "audio");
        assert_eq!(values[2].get("type").unwrap(), "image");
        assert_eq!(values[2].get("url").unwrap(), "https://example.com/img.png");
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
