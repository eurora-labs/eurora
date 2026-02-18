//! Utility functions for interacting with the OpenAI API.
//!
//! Matches Python `langchain_openai.chat_models._client_utils`.

use std::collections::HashMap;

/// Fix message content blocks that contain bytes image URLs.
///
/// In Rust this handles the edge case where an `image_url` content block
/// might contain a raw bytes URL that needs to be interpreted as UTF-8.
/// Since Rust strings are always UTF-8, this is largely a no-op validation
/// pass, but it mirrors the Python `create_chat_model_messages` function
/// for structural parity.
///
/// Matches Python `create_chat_model_messages`.
pub fn create_chat_model_messages(messages: Vec<serde_json::Value>) -> Vec<serde_json::Value> {
    messages
        .into_iter()
        .map(|mut message| {
            if let Some(content) = message.get_mut("content")
                && let Some(blocks) = content.as_array_mut()
            {
                for block in blocks.iter_mut() {
                    if let Some(block_type) = block.get("type")
                        && block_type == "image_url"
                        && let Some(image_url) = block.get_mut("image_url")
                        && let Some(url_obj) = image_url.as_object_mut()
                    {
                        if let Some(url_val) = url_obj.get("url")
                            && url_val.is_string()
                        {
                        }
                    }
                }
            }
            message
        })
        .collect()
}

/// Build the default parameter map for an OpenAI API request.
///
/// Filters out `None` values so only explicitly-set parameters are included.
///
/// Matches Python `_default_params`.
pub fn default_params(
    model: &str,
    stream: bool,
    kwargs: HashMap<String, serde_json::Value>,
) -> HashMap<String, serde_json::Value> {
    let mut params = HashMap::new();
    params.insert("model".to_string(), serde_json::json!(model));
    params.insert("stream".to_string(), serde_json::json!(stream));

    for (key, value) in kwargs {
        if !value.is_null() {
            params.insert(key, value);
        }
    }

    params
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_chat_model_messages_passthrough() {
        let messages = vec![
            serde_json::json!({"role": "user", "content": "hello"}),
            serde_json::json!({"role": "assistant", "content": "hi"}),
        ];
        let result = create_chat_model_messages(messages.clone());
        assert_eq!(result, messages);
    }

    #[test]
    fn test_create_chat_model_messages_with_image_url() {
        let messages = vec![serde_json::json!({
            "role": "user",
            "content": [
                {"type": "text", "text": "What is this?"},
                {"type": "image_url", "image_url": {"url": "https://example.com/img.png"}}
            ]
        })];
        let result = create_chat_model_messages(messages.clone());
        assert_eq!(result, messages);
    }

    #[test]
    fn test_default_params_basic() {
        let params = default_params("gpt-4o", false, HashMap::new());
        assert_eq!(params.get("model"), Some(&serde_json::json!("gpt-4o")));
        assert_eq!(params.get("stream"), Some(&serde_json::json!(false)));
        assert_eq!(params.len(), 2);
    }

    #[test]
    fn test_default_params_filters_null() {
        let mut kwargs = HashMap::new();
        kwargs.insert("temperature".to_string(), serde_json::json!(0.7));
        kwargs.insert("top_p".to_string(), serde_json::Value::Null);
        kwargs.insert("seed".to_string(), serde_json::json!(42));

        let params = default_params("gpt-4o", true, kwargs);
        assert_eq!(params.get("temperature"), Some(&serde_json::json!(0.7)));
        assert_eq!(params.get("seed"), Some(&serde_json::json!(42)));
        assert!(!params.contains_key("top_p"));
        assert_eq!(params.len(), 4); // model, stream, temperature, seed
    }
}
