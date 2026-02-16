//! Derivations of standard content blocks from LangChain v0 multimodal content.
//!
//! Mirrors `langchain_core.messages.block_translators.langchain_v0`.

use std::collections::HashMap;

use serde_json::Value;

use crate::messages::content::KNOWN_BLOCK_TYPES;

/// Convert v0 multimodal blocks to v1 format.
///
/// During the `content_blocks` parsing process, blocks not recognized as a v1
/// block are wrapped as a `non_standard` block with the original block stored
/// in the `value` field. This function attempts to unpack those blocks and
/// convert any v0 format blocks to v1 format.
///
/// If conversion fails, the block is left as a `non_standard` block.
pub fn convert_v0_multimodal_input_to_v1(
    content: &[HashMap<String, Value>],
) -> Vec<HashMap<String, Value>> {
    let unpacked: Vec<HashMap<String, Value>> = content
        .iter()
        .map(|block| {
            let block_type = block.get("type").and_then(|t| t.as_str());
            if block_type == Some("non_standard")
                && let Some(Value::Object(inner)) = block.get("value")
            {
                return inner.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
            }
            block.clone()
        })
        .collect();

    unpacked
        .into_iter()
        .map(|block| {
            let block_type = block.get("type").and_then(|t| t.as_str());
            let has_source_type = block.contains_key("source_type");

            if matches!(block_type, Some("image" | "audio" | "file")) && has_source_type {
                convert_legacy_v0_content_block_to_v1(&block)
            } else if block_type.is_some_and(|t| KNOWN_BLOCK_TYPES.contains(&t)) {
                block
            } else {
                let mut result = HashMap::new();
                result.insert(
                    "type".to_string(),
                    Value::String("non_standard".to_string()),
                );
                result.insert(
                    "value".to_string(),
                    serde_json::to_value(&block).unwrap_or_default(),
                );
                result
            }
        })
        .collect()
}

/// Convert a LangChain v0 content block to v1 format.
///
/// Preserves unknown keys as extras to avoid data loss.
/// Returns the original block unchanged if it's not in v0 format.
pub fn convert_legacy_v0_content_block_to_v1(
    block: &HashMap<String, Value>,
) -> HashMap<String, Value> {
    let block_type = match block.get("type").and_then(|t| t.as_str()) {
        Some(t @ ("image" | "audio" | "file")) => t,
        _ => return block.clone(),
    };
    let source_type = match block.get("source_type").and_then(|s| s.as_str()) {
        Some(s) => s,
        None => return block.clone(),
    };

    let mut result = HashMap::new();
    result.insert("type".to_string(), Value::String(block_type.to_string()));

    match (block_type, source_type) {
        ("image" | "audio" | "file", "url") => {
            let known_keys: &[&str] = &["mime_type", "type", "source_type", "url"];
            if let Some(url) = block.get("url") {
                result.insert("url".to_string(), url.clone());
            }
            if let Some(mime) = block.get("mime_type") {
                result.insert("mime_type".to_string(), mime.clone());
            }
            if let Some(id) = block.get("id") {
                result.insert("id".to_string(), id.clone());
            }
            insert_extras(&mut result, block, known_keys);
        }
        ("image" | "audio" | "file", "base64") => {
            let known_keys: &[&str] = &["mime_type", "type", "source_type", "data"];
            if let Some(data) = block.get("data") {
                result.insert("base64".to_string(), data.clone());
            }
            if let Some(mime) = block.get("mime_type") {
                result.insert("mime_type".to_string(), mime.clone());
            }
            if let Some(id) = block.get("id") {
                result.insert("id".to_string(), id.clone());
            }
            insert_extras(&mut result, block, known_keys);
        }
        ("image" | "audio" | "file", "id") => {
            let known_keys: &[&str] = &["type", "source_type", "id"];
            if let Some(id) = block.get("id") {
                result.insert("file_id".to_string(), id.clone());
            }
            insert_extras(&mut result, block, known_keys);
        }
        ("file", "text") => {
            let known_keys: &[&str] = &["mime_type", "type", "source_type", "url"];
            result.insert("type".to_string(), Value::String("text-plain".to_string()));
            result.insert(
                "mime_type".to_string(),
                block
                    .get("mime_type")
                    .cloned()
                    .unwrap_or(Value::String("text/plain".to_string())),
            );
            if let Some(url) = block.get("url") {
                result.insert("text".to_string(), url.clone());
            }
            if let Some(id) = block.get("id") {
                result.insert("id".to_string(), id.clone());
            }
            insert_extras(&mut result, block, known_keys);
        }
        _ => return block.clone(),
    }

    result
}

/// Extract unknown keys from a v0 block and insert them as extras.
fn insert_extras(
    result: &mut HashMap<String, Value>,
    block: &HashMap<String, Value>,
    known_keys: &[&str],
) {
    let extras: HashMap<String, Value> = block
        .iter()
        .filter(|(k, v)| !known_keys.contains(&k.as_str()) && k.as_str() != "id" && !v.is_null())
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();

    if !extras.is_empty() {
        result.insert(
            "extras".to_string(),
            serde_json::to_value(extras).unwrap_or_default(),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_convert_v0_image_url() {
        let mut block = HashMap::new();
        block.insert("type".to_string(), json!("image"));
        block.insert("source_type".to_string(), json!("url"));
        block.insert("url".to_string(), json!("https://example.com/img.png"));
        block.insert("mime_type".to_string(), json!("image/png"));

        let result = convert_legacy_v0_content_block_to_v1(&block);
        assert_eq!(result.get("type").unwrap(), "image");
        assert_eq!(result.get("url").unwrap(), "https://example.com/img.png");
        assert_eq!(result.get("mime_type").unwrap(), "image/png");
        assert!(!result.contains_key("source_type"));
    }

    #[test]
    fn test_convert_v0_image_base64() {
        let mut block = HashMap::new();
        block.insert("type".to_string(), json!("image"));
        block.insert("source_type".to_string(), json!("base64"));
        block.insert("data".to_string(), json!("abc123"));
        block.insert("mime_type".to_string(), json!("image/png"));

        let result = convert_legacy_v0_content_block_to_v1(&block);
        assert_eq!(result.get("type").unwrap(), "image");
        assert_eq!(result.get("base64").unwrap(), "abc123");
        assert_eq!(result.get("mime_type").unwrap(), "image/png");
    }

    #[test]
    fn test_convert_v0_image_id() {
        let mut block = HashMap::new();
        block.insert("type".to_string(), json!("image"));
        block.insert("source_type".to_string(), json!("id"));
        block.insert("id".to_string(), json!("file-123"));

        let result = convert_legacy_v0_content_block_to_v1(&block);
        assert_eq!(result.get("type").unwrap(), "image");
        assert_eq!(result.get("file_id").unwrap(), "file-123");
    }

    #[test]
    fn test_convert_v0_audio_url() {
        let mut block = HashMap::new();
        block.insert("type".to_string(), json!("audio"));
        block.insert("source_type".to_string(), json!("url"));
        block.insert("url".to_string(), json!("https://example.com/audio.mp3"));

        let result = convert_legacy_v0_content_block_to_v1(&block);
        assert_eq!(result.get("type").unwrap(), "audio");
        assert_eq!(result.get("url").unwrap(), "https://example.com/audio.mp3");
    }

    #[test]
    fn test_convert_v0_file_text() {
        let mut block = HashMap::new();
        block.insert("type".to_string(), json!("file"));
        block.insert("source_type".to_string(), json!("text"));
        block.insert("url".to_string(), json!("Some plaintext content"));

        let result = convert_legacy_v0_content_block_to_v1(&block);
        assert_eq!(result.get("type").unwrap(), "text-plain");
        assert_eq!(result.get("mime_type").unwrap(), "text/plain");
        assert_eq!(result.get("text").unwrap(), "Some plaintext content");
    }

    #[test]
    fn test_convert_v0_preserves_extras() {
        let mut block = HashMap::new();
        block.insert("type".to_string(), json!("image"));
        block.insert("source_type".to_string(), json!("url"));
        block.insert("url".to_string(), json!("https://example.com/img.png"));
        block.insert("custom_field".to_string(), json!("custom_value"));

        let result = convert_legacy_v0_content_block_to_v1(&block);
        let extras = result.get("extras").unwrap().as_object().unwrap();
        assert_eq!(extras.get("custom_field").unwrap(), "custom_value");
    }

    #[test]
    fn test_convert_v0_not_v0_format() {
        let mut block = HashMap::new();
        block.insert("type".to_string(), json!("text"));
        block.insert("text".to_string(), json!("hello"));

        let result = convert_legacy_v0_content_block_to_v1(&block);
        assert_eq!(result, block);
    }

    #[test]
    fn test_convert_v0_multimodal_input() {
        let block1: HashMap<String, Value> = [
            ("type".to_string(), json!("image")),
            ("source_type".to_string(), json!("url")),
            ("url".to_string(), json!("https://example.com/img.png")),
        ]
        .into();

        let block2: HashMap<String, Value> = [
            ("type".to_string(), json!("text")),
            ("text".to_string(), json!("hello")),
        ]
        .into();

        let result = convert_v0_multimodal_input_to_v1(&[block1, block2]);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].get("type").unwrap(), "image");
        assert_eq!(result[0].get("url").unwrap(), "https://example.com/img.png");
        assert!(!result[0].contains_key("source_type"));
        assert_eq!(result[1].get("type").unwrap(), "text");
    }
}
