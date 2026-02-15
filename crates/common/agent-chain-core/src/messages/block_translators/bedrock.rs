//! Derivations of standard content blocks from Bedrock content.
//!
//! Mirrors `langchain_core/messages/block_translators/bedrock.py`.
//!
//! The Bedrock (non-Converse) API uses the same format as Anthropic for
//! Claude models, so this module delegates to the Anthropic translator.

use serde_json::Value;

use super::anthropic;

/// Convert Bedrock content blocks to standard format.
///
/// For Claude models, Bedrock uses Anthropic's format, so we delegate
/// directly to the Anthropic translator.
pub fn convert_to_standard_blocks(content: &[Value], is_chunk: bool) -> Vec<Value> {
    anthropic::convert_to_standard_blocks(content, is_chunk)
}

/// Convert Bedrock content blocks to standard format with context.
pub fn convert_to_standard_blocks_with_context(
    content: &[Value],
    is_chunk: bool,
    context: Option<&anthropic::ChunkContext>,
) -> Vec<Value> {
    anthropic::convert_to_standard_blocks_with_context(content, is_chunk, context)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_convert_text_block() {
        let content = vec![json!({"type": "text", "text": "Hello from Bedrock"})];
        let result = convert_to_standard_blocks(&content, false);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0]["type"], "text");
        assert_eq!(result[0]["text"], "Hello from Bedrock");
    }

    #[test]
    fn test_convert_tool_use_block() {
        let content = vec![json!({
            "type": "tool_use",
            "id": "tool_1",
            "name": "get_weather",
            "input": {"city": "Seattle"}
        })];
        let result = convert_to_standard_blocks(&content, false);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0]["type"], "tool_call");
        assert_eq!(result[0]["name"], "get_weather");
        assert_eq!(result[0]["id"], "tool_1");
    }

    #[test]
    fn test_convert_thinking_block() {
        let content = vec![json!({
            "type": "thinking",
            "thinking": "Let me think...",
            "signature": "sig_abc"
        })];
        let result = convert_to_standard_blocks(&content, false);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0]["type"], "reasoning");
        assert_eq!(result[0]["reasoning"], "Let me think...");
        assert_eq!(result[0]["extras"]["signature"], "sig_abc");
    }
}
