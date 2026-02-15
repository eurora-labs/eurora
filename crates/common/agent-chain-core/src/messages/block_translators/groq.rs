//! Derivations of standard content blocks from Groq content.
//!
//! Mirrors `langchain_core/messages/block_translators/groq.py`.
//!
//! Groq content requires message-level context (additional_kwargs,
//! tool_calls) in addition to content blocks, because reasoning content
//! and executed tools are stored in additional_kwargs.

use std::collections::{HashMap, HashSet};

use regex::Regex;
use serde_json::{Value, json};

use crate::messages::base::extract_reasoning_from_additional_kwargs;
use crate::messages::tool::ToolCall;

/// Populate extras field with unknown fields from the original block.
fn populate_extras(standard_block: &mut Value, block: &Value, known_fields: &HashSet<&str>) {
    if standard_block.get("type").and_then(|v| v.as_str()) == Some("non_standard") {
        return;
    }

    if let Some(block_obj) = block.as_object() {
        for (key, value) in block_obj {
            if !known_fields.contains(key.as_str()) {
                let extras = standard_block
                    .as_object_mut()
                    .expect("standard_block should be an object")
                    .entry("extras")
                    .or_insert_with(|| json!({}));
                extras
                    .as_object_mut()
                    .expect("extras should be an object")
                    .insert(key.clone(), value.clone());
            }
        }
    }
}

/// Extract Python code from Groq built-in tool content.
///
/// Extracts the value of the 'code' field from a string of the form:
/// `{"code": some_arbitrary_text_with_unescaped_quotes}`
///
/// Groq may not escape quotes in the executed tools, e.g.:
/// `{"code": "import math; print(\"hello\"); print(math.sqrt(101))"}`
fn parse_code_json(s: &str) -> Option<Value> {
    let re = Regex::new(r#"(?s)\s*\{\s*"code"\s*:\s*"(.*)"\s*\}\s*"#).ok()?;
    let caps = re.captures(s)?;
    let code = caps.get(1)?.as_str();
    Some(json!({"code": code}))
}

/// Convert Groq message content to v1 format.
///
/// Unlike most other translators, this function needs message-level context
/// because Groq stores reasoning and executed tools in `additional_kwargs`.
pub fn convert_to_standard_blocks_with_message_context(
    content: &[Value],
    _is_chunk: bool,
    additional_kwargs: &HashMap<String, Value>,
    tool_calls: &[ToolCall],
    text_content: Option<&str>,
) -> Vec<Value> {
    let mut content_blocks: Vec<Value> = Vec::new();

    // Extract reasoning from additional_kwargs
    if let Some(reasoning) = extract_reasoning_from_additional_kwargs(additional_kwargs) {
        content_blocks.push(json!({
            "type": "reasoning",
            "reasoning": reasoning.reasoning,
        }));
    }

    // Process executed tools from additional_kwargs
    if let Some(Value::Array(executed_tools)) = additional_kwargs.get("executed_tools") {
        for (idx, executed_tool) in executed_tools.iter().enumerate() {
            let mut args: Option<Value> = None;

            if let Some(arguments) = executed_tool.get("arguments").and_then(|a| a.as_str()) {
                // Try parsing as JSON first
                match serde_json::from_str::<Value>(arguments) {
                    Ok(parsed) if parsed.is_object() => {
                        args = Some(parsed);
                    }
                    _ => {
                        let tool_type = executed_tool
                            .get("type")
                            .and_then(|v| v.as_str())
                            .unwrap_or("");
                        let tool_name = executed_tool
                            .get("name")
                            .and_then(|v| v.as_str())
                            .unwrap_or("");

                        if tool_type == "python" {
                            args = parse_code_json(arguments);
                        } else if tool_type == "function" && tool_name == "python" {
                            // GPT-OSS
                            args = Some(json!({"code": arguments}));
                        }
                        // If none matched, skip this tool
                    }
                }
            }

            if let Some(args_val) = &args {
                if args_val.is_object() {
                    let tool_type = executed_tool
                        .get("type")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    let tool_name = executed_tool
                        .get("name")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");

                    let name = if tool_type == "search" {
                        "web_search"
                    } else if tool_type == "python"
                        || (tool_type == "function" && tool_name == "python")
                    {
                        "code_interpreter"
                    } else {
                        ""
                    };

                    content_blocks.push(json!({
                        "type": "server_tool_call",
                        "name": name,
                        "id": idx.to_string(),
                        "args": args_val,
                    }));
                }
            }

            if let Some(tool_output) = executed_tool.get("output") {
                let mut tool_result = json!({
                    "type": "server_tool_result",
                    "tool_call_id": idx.to_string(),
                    "output": tool_output,
                    "status": "success",
                });

                let known_fields: HashSet<&str> = ["type", "arguments", "index", "output"]
                    .iter()
                    .copied()
                    .collect();
                populate_extras(&mut tool_result, executed_tool, &known_fields);

                content_blocks.push(tool_result);
            }
        }
    }

    // Add text content
    if content.is_empty() {
        // Content is a string, not blocks
        if let Some(text) = text_content {
            if !text.is_empty() {
                content_blocks.push(json!({
                    "type": "text",
                    "text": text,
                }));
            }
        }
    } else {
        // Content is blocks — pass through text blocks
        for block in content {
            if let Some(block_type) = block.get("type").and_then(|v| v.as_str()) {
                if block_type == "text" {
                    content_blocks.push(block.clone());
                }
            } else if let Some(text) = block.as_str() {
                if !text.is_empty() {
                    content_blocks.push(json!({
                        "type": "text",
                        "text": text,
                    }));
                }
            }
        }
    }

    // Add tool calls from message.tool_calls
    for tool_call in tool_calls {
        content_blocks.push(json!({
            "type": "tool_call",
            "name": tool_call.name,
            "args": tool_call.args,
            "id": tool_call.id,
        }));
    }

    content_blocks
}

/// Convert Groq content blocks to standard format.
///
/// This is a simplified version that only processes content blocks without
/// message-level context. For full translation including reasoning and
/// executed tools, use `convert_to_standard_blocks_with_message_context`.
pub fn convert_to_standard_blocks(content: &[Value], is_chunk: bool) -> Vec<Value> {
    convert_to_standard_blocks_with_message_context(content, is_chunk, &HashMap::new(), &[], None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_text_content() {
        let result = convert_to_standard_blocks_with_message_context(
            &[],
            false,
            &HashMap::new(),
            &[],
            Some("Hello from Groq"),
        );
        assert_eq!(result.len(), 1);
        assert_eq!(result[0]["type"], "text");
        assert_eq!(result[0]["text"], "Hello from Groq");
    }

    #[test]
    fn test_reasoning_extraction() {
        let mut additional_kwargs = HashMap::new();
        additional_kwargs.insert(
            "reasoning_content".to_string(),
            Value::String("Let me think about this...".to_string()),
        );

        let result = convert_to_standard_blocks_with_message_context(
            &[],
            false,
            &additional_kwargs,
            &[],
            Some("The answer is 42"),
        );
        assert_eq!(result.len(), 2);
        assert_eq!(result[0]["type"], "reasoning");
        assert_eq!(result[0]["reasoning"], "Let me think about this...");
        assert_eq!(result[1]["type"], "text");
        assert_eq!(result[1]["text"], "The answer is 42");
    }

    #[test]
    fn test_executed_tools_python() {
        let mut additional_kwargs = HashMap::new();
        additional_kwargs.insert(
            "executed_tools".to_string(),
            json!([{
                "type": "python",
                "arguments": "{\"code\": \"print('hello')\"}",
                "output": "hello"
            }]),
        );

        let result = convert_to_standard_blocks_with_message_context(
            &[],
            false,
            &additional_kwargs,
            &[],
            None,
        );
        assert_eq!(result.len(), 2);
        assert_eq!(result[0]["type"], "server_tool_call");
        assert_eq!(result[0]["name"], "code_interpreter");
        assert_eq!(result[0]["id"], "0");
        assert_eq!(result[0]["args"]["code"], "print('hello')");
        assert_eq!(result[1]["type"], "server_tool_result");
        assert_eq!(result[1]["output"], "hello");
        assert_eq!(result[1]["status"], "success");
    }

    #[test]
    fn test_executed_tools_search() {
        let mut additional_kwargs = HashMap::new();
        additional_kwargs.insert(
            "executed_tools".to_string(),
            json!([{
                "type": "search",
                "arguments": "{\"query\": \"rust programming\"}",
                "output": "Rust is a systems programming language."
            }]),
        );

        let result = convert_to_standard_blocks_with_message_context(
            &[],
            false,
            &additional_kwargs,
            &[],
            None,
        );
        assert_eq!(result.len(), 2);
        assert_eq!(result[0]["type"], "server_tool_call");
        assert_eq!(result[0]["name"], "web_search");
        assert_eq!(result[0]["args"]["query"], "rust programming");
        assert_eq!(result[1]["type"], "server_tool_result");
        assert_eq!(result[1]["tool_call_id"], "0");
    }

    #[test]
    fn test_executed_tools_gpt_oss_function_python() {
        let mut additional_kwargs = HashMap::new();
        additional_kwargs.insert(
            "executed_tools".to_string(),
            json!([{
                "type": "function",
                "name": "python",
                "arguments": "print(42)",
                "output": "42"
            }]),
        );

        let result = convert_to_standard_blocks_with_message_context(
            &[],
            false,
            &additional_kwargs,
            &[],
            None,
        );
        assert_eq!(result.len(), 2);
        assert_eq!(result[0]["type"], "server_tool_call");
        assert_eq!(result[0]["name"], "code_interpreter");
        assert_eq!(result[0]["args"]["code"], "print(42)");
    }

    #[test]
    fn test_parse_code_json_with_unescaped_quotes() {
        let input =
            r#"{"code": "import math; print("The square root is: "); print(math.sqrt(101))"}"#;
        let result = parse_code_json(input);
        assert!(result.is_some());
        let result = result.expect("should parse");
        assert!(
            result["code"]
                .as_str()
                .expect("should be string")
                .contains("import math")
        );
    }

    #[test]
    fn test_tool_calls_from_message() {
        let tool_calls = vec![
            ToolCall::builder()
                .name("get_weather")
                .args(json!({"city": "Seattle"}))
                .id("tc_1".to_string())
                .build(),
        ];

        let result = convert_to_standard_blocks_with_message_context(
            &[],
            false,
            &HashMap::new(),
            &tool_calls,
            Some("Here's the weather"),
        );
        assert_eq!(result.len(), 2);
        assert_eq!(result[0]["type"], "text");
        assert_eq!(result[0]["text"], "Here's the weather");
        assert_eq!(result[1]["type"], "tool_call");
        assert_eq!(result[1]["name"], "get_weather");
        assert_eq!(result[1]["args"]["city"], "Seattle");
        assert_eq!(result[1]["id"], "tc_1");
    }

    #[test]
    fn test_executed_tool_extras() {
        let mut additional_kwargs = HashMap::new();
        additional_kwargs.insert(
            "executed_tools".to_string(),
            json!([{
                "type": "python",
                "arguments": "{\"code\": \"1+1\"}",
                "output": "2",
                "custom_field": "custom_value"
            }]),
        );

        let result = convert_to_standard_blocks_with_message_context(
            &[],
            false,
            &additional_kwargs,
            &[],
            None,
        );
        // server_tool_call + server_tool_result
        assert_eq!(result.len(), 2);
        let tool_result = &result[1];
        assert_eq!(tool_result["type"], "server_tool_result");
        assert_eq!(tool_result["extras"]["custom_field"], "custom_value");
        // "name" is an extra field since it's not in known_fields
        assert_eq!(tool_result["extras"].get("type"), None);
    }

    #[test]
    fn test_content_blocks_passthrough() {
        let content = vec![
            json!({"type": "text", "text": "Hello"}),
            json!({"type": "image", "url": "http://example.com/img.png"}),
        ];
        let result = convert_to_standard_blocks(&content, false);
        // Only text blocks are passed through in groq
        assert_eq!(result.len(), 1);
        assert_eq!(result[0]["type"], "text");
        assert_eq!(result[0]["text"], "Hello");
    }

    #[test]
    fn test_empty_content_no_text() {
        let result =
            convert_to_standard_blocks_with_message_context(&[], false, &HashMap::new(), &[], None);
        assert!(result.is_empty());
    }

    #[test]
    fn test_full_groq_message() {
        let mut additional_kwargs = HashMap::new();
        additional_kwargs.insert(
            "reasoning_content".to_string(),
            Value::String("Thinking...".to_string()),
        );
        additional_kwargs.insert(
            "executed_tools".to_string(),
            json!([{
                "type": "python",
                "arguments": "{\"code\": \"2+2\"}",
                "output": "4"
            }]),
        );

        let tool_calls = vec![
            ToolCall::builder()
                .name("calculator")
                .args(json!({"expr": "2+2"}))
                .id("tc_0".to_string())
                .build(),
        ];

        let result = convert_to_standard_blocks_with_message_context(
            &[],
            false,
            &additional_kwargs,
            &tool_calls,
            Some("The answer is 4"),
        );

        // reasoning + server_tool_call + server_tool_result + text + tool_call
        assert_eq!(result.len(), 5);
        assert_eq!(result[0]["type"], "reasoning");
        assert_eq!(result[1]["type"], "server_tool_call");
        assert_eq!(result[2]["type"], "server_tool_result");
        assert_eq!(result[3]["type"], "text");
        assert_eq!(result[4]["type"], "tool_call");
    }
}
