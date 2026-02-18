//! Compatibility module for converting v1 content blocks to Ollama SDK format.
//!
//! Matches Python `langchain_ollama._compat`.

/// Convert v1 content blocks to Ollama format.
///
/// Takes a list of content block values and converts them to the format
/// expected by the Ollama API. Drops reasoning blocks (Ollama doesn't accept
/// reasoning back in). Preserves text and image blocks.
///
/// Matches Python `_convert_from_v1_to_ollama()`.
pub fn convert_from_v1_to_ollama(content: &[serde_json::Value]) -> Vec<serde_json::Value> {
    let mut new_content = Vec::new();

    for block in content {
        let Some(block_obj) = block.as_object() else {
            continue;
        };
        let Some(block_type) = block_obj.get("type").and_then(|t| t.as_str()) else {
            continue;
        };

        match block_type {
            "text" => {
                if let Some(text) = block_obj.get("text") {
                    new_content.push(serde_json::json!({
                        "type": "text",
                        "text": text,
                    }));
                }
            }
            "reasoning" => {}
            "image" => {
                new_content.push(block.clone());
            }
            "non_standard" => {
                let value = block_obj
                    .get("value")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                new_content.push(serde_json::json!({
                    "type": "text",
                    "text": value,
                }));
            }
            _ => {}
        }
    }

    new_content
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_convert_text_block() {
        let content = vec![serde_json::json!({
            "type": "text",
            "text": "hello world",
        })];
        let result = convert_from_v1_to_ollama(&content);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0]["type"], "text");
        assert_eq!(result[0]["text"], "hello world");
    }

    #[test]
    fn test_convert_reasoning_block_dropped() {
        let content = vec![serde_json::json!({
            "type": "reasoning",
            "reasoning": "thinking...",
        })];
        let result = convert_from_v1_to_ollama(&content);
        assert!(result.is_empty());
    }

    #[test]
    fn test_convert_image_block_passthrough() {
        let content = vec![serde_json::json!({
            "type": "image",
            "source_type": "base64",
            "data": "abc123",
        })];
        let result = convert_from_v1_to_ollama(&content);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0]["type"], "image");
        assert_eq!(result[0]["data"], "abc123");
    }

    #[test]
    fn test_convert_non_standard_block() {
        let content = vec![serde_json::json!({
            "type": "non_standard",
            "value": "some value",
        })];
        let result = convert_from_v1_to_ollama(&content);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0]["type"], "text");
        assert_eq!(result[0]["text"], "some value");
    }

    #[test]
    fn test_convert_skips_non_dict_entries() {
        let content = vec![serde_json::json!("just a string"), serde_json::json!(42)];
        let result = convert_from_v1_to_ollama(&content);
        assert!(result.is_empty());
    }

    #[test]
    fn test_convert_skips_entries_without_type() {
        let content = vec![serde_json::json!({
            "text": "no type field",
        })];
        let result = convert_from_v1_to_ollama(&content);
        assert!(result.is_empty());
    }

    #[test]
    fn test_convert_mixed_content() {
        let content = vec![
            serde_json::json!({"type": "text", "text": "hello"}),
            serde_json::json!({"type": "reasoning", "reasoning": "thinking"}),
            serde_json::json!({"type": "image", "base64": "img_data"}),
            serde_json::json!({"type": "non_standard", "value": "extra"}),
        ];
        let result = convert_from_v1_to_ollama(&content);
        assert_eq!(result.len(), 3);
        assert_eq!(result[0]["text"], "hello");
        assert_eq!(result[1]["type"], "image");
        assert_eq!(result[2]["text"], "extra");
    }
}
