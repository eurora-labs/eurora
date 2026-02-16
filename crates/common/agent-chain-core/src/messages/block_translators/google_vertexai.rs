//! Derivations of standard content blocks from Google (VertexAI) content.
//!
//! Mirrors `langchain_core/messages/block_translators/google_vertexai.py`.
//!
//! VertexAI uses the same format as Google GenAI, so this module delegates
//! to the GenAI translator.

use serde_json::Value;

use super::google_genai;

/// Convert VertexAI content blocks to standard format.
///
/// Delegates to the Google GenAI translator since VertexAI uses the same format.
pub fn convert_to_standard_blocks(content: &[Value], is_chunk: bool) -> Vec<Value> {
    google_genai::convert_to_standard_blocks(content, is_chunk)
}

/// Convert VertexAI input content blocks to standard format.
pub fn convert_input_to_standard_blocks(content: &[Value]) -> Vec<Value> {
    google_genai::convert_input_to_standard_blocks(content)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_delegates_to_genai_text() {
        let content = vec![json!({"type": "text", "text": "Hello from VertexAI"})];
        let result = convert_to_standard_blocks(&content, false);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0]["type"], "text");
        assert_eq!(result[0]["text"], "Hello from VertexAI");
    }

    #[test]
    fn test_delegates_to_genai_function_call() {
        let content = vec![json!({
            "type": "function_call",
            "name": "get_weather",
            "args": {"city": "Seattle"}
        })];
        let result = convert_to_standard_blocks(&content, false);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0]["type"], "tool_call");
        assert_eq!(result[0]["name"], "get_weather");
    }

    #[test]
    fn test_delegates_input_to_genai() {
        // Input format uses type-less Google Parts
        let content = vec![json!({
            "text": "User message"
        })];
        let result = convert_input_to_standard_blocks(&content);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0]["type"], "text");
        assert_eq!(result[0]["text"], "User message");
    }
}
