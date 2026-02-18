//! Utility functions for interacting with the OpenAI API.
//!
//! Matches Python `langchain_openai.chat_models._client_utils`.

use std::collections::HashMap;

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
