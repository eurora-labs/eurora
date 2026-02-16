//! Derivations of standard content blocks from Amazon (Bedrock Converse) content.
//!
//! Mirrors `langchain_core/messages/block_translators/bedrock_converse.py`.
//!
//! The Converse API uses a different structure than the legacy Bedrock API:
//! - Content blocks use specific typed keys rather than a "type" field
//! - Images use `{"image": {"format": "png", "source": {"bytes": ...}}}`
//! - Tool use/results use `toolUse`/`toolResult` with camelCase keys

use std::collections::HashSet;

use base64::Engine;
use serde_json::{Value, json};

use crate::messages::content::KNOWN_BLOCK_TYPES;

/// Populate extras field with unknown fields from the original block.
fn populate_extras(standard_block: &mut Value, block: &Value, known_fields: &HashSet<&str>) {
    if standard_block.get("type").and_then(|v| v.as_str()) == Some("non_standard") {
        return;
    }

    if let Some(block_obj) = block.as_object() {
        for (key, value) in block_obj {
            if !known_fields.contains(key.as_str())
                && let Some(obj) = standard_block.as_object_mut()
            {
                let extras = obj.entry("extras").or_insert_with(|| json!({}));
                if let Some(extras_obj) = extras.as_object_mut() {
                    extras_obj.insert(key.clone(), value.clone());
                }
            }
        }
    }
}

/// Convert bytes (as a JSON value) to a base64 string.
fn bytes_to_b64_str(bytes_value: &Value) -> String {
    if let Some(s) = bytes_value.as_str() {
        // Already a string (could be pre-encoded)
        s.to_string()
    } else if let Some(arr) = bytes_value.as_array() {
        // Array of byte values
        let bytes: Vec<u8> = arr
            .iter()
            .filter_map(|v| v.as_u64().map(|n| n as u8))
            .collect();
        base64::engine::general_purpose::STANDARD.encode(&bytes)
    } else {
        String::new()
    }
}

/// Convert Bedrock Converse format input blocks to v1 format.
///
/// During the `content_blocks` parsing process, blocks not recognized as v1
/// are wrapped as `non_standard`. This function unpacks those and converts
/// Converse-format blocks to v1 ContentBlocks.
pub fn convert_input_to_standard_blocks(content: &[Value]) -> Vec<Value> {
    let mut result = Vec::new();

    // Unpack non_standard blocks
    let blocks: Vec<Value> = content
        .iter()
        .map(|block| {
            if block.get("type").and_then(|v| v.as_str()) == Some("non_standard") {
                block.get("value").cloned().unwrap_or_else(|| block.clone())
            } else {
                block.clone()
            }
        })
        .collect();

    for block in &blocks {
        let obj = match block.as_object() {
            Some(o) => o,
            None => continue,
        };

        let num_keys = obj.len();

        // {"text": "..."} -> TextContentBlock
        if num_keys == 1
            && let Some(text) = obj.get("text").and_then(|v| v.as_str())
        {
            result.push(json!({"type": "text", "text": text}));
            continue;
        }

        // {"document": {"format": "pdf", "source": {"bytes": ...}}} -> FileContentBlock
        if num_keys == 1
            && let Some(document) = obj.get("document").and_then(|v| v.as_object())
            && let Some(format) = document.get("format").and_then(|v| v.as_str())
        {
            match format {
                "pdf" => {
                    if let Some(bytes_val) = document
                        .get("source")
                        .and_then(|s| s.as_object())
                        .and_then(|s| s.get("bytes"))
                    {
                        let mut file_block = json!({
                            "type": "file",
                            "base64": bytes_to_b64_str(bytes_val),
                            "mime_type": "application/pdf",
                        });
                        let known: HashSet<&str> = ["format", "source"].into();
                        populate_extras(&mut file_block, &json!(document), &known);
                        result.push(file_block);
                    } else {
                        result.push(json!({"type": "non_standard", "value": block}));
                    }
                }
                "txt" => {
                    if let Some(text) = document
                        .get("source")
                        .and_then(|s| s.as_object())
                        .and_then(|s| s.get("text"))
                        .and_then(|t| t.as_str())
                    {
                        let mut plain_text = json!({
                            "type": "text-plain",
                            "text": text,
                            "mime_type": "text/plain",
                        });
                        let known: HashSet<&str> = ["format", "source"].into();
                        populate_extras(&mut plain_text, &json!(document), &known);
                        result.push(plain_text);
                    } else {
                        result.push(json!({"type": "non_standard", "value": block}));
                    }
                }
                _ => {
                    result.push(json!({"type": "non_standard", "value": block}));
                }
            }
            continue;
        }

        // {"image": {"format": "png", "source": {"bytes": ...}}} -> ImageContentBlock
        if num_keys == 1
            && let Some(image) = obj.get("image").and_then(|v| v.as_object())
            && let Some(format) = image.get("format").and_then(|v| v.as_str())
        {
            if let Some(bytes_val) = image
                .get("source")
                .and_then(|s| s.as_object())
                .and_then(|s| s.get("bytes"))
            {
                let mut image_block = json!({
                    "type": "image",
                    "base64": bytes_to_b64_str(bytes_val),
                    "mime_type": format!("image/{}", format),
                });
                let known: HashSet<&str> = ["format", "source"].into();
                populate_extras(&mut image_block, &json!(image), &known);
                result.push(image_block);
            } else {
                result.push(json!({"type": "non_standard", "value": block}));
            }
            continue;
        }

        // Known v1 block type — pass through
        if let Some(block_type) = obj.get("type").and_then(|v| v.as_str())
            && KNOWN_BLOCK_TYPES.contains(&block_type)
        {
            result.push(block.clone());
            continue;
        }

        // Unknown — wrap as non_standard
        result.push(json!({"type": "non_standard", "value": block}));
    }

    result
}

/// Convert a Converse citation to standard v1 format.
fn convert_citation_to_v1(citation: &Value) -> Value {
    let mut standard_citation = json!({"type": "citation"});

    if let Some(title) = citation.get("title").and_then(|v| v.as_str()) {
        standard_citation["title"] = json!(title);
    }

    // source_content is a list of dicts with "text" keys
    if let Some(source_content) = citation.get("source_content").and_then(|v| v.as_array()) {
        let cited_text: String = source_content
            .iter()
            .filter_map(|item| item.get("text").and_then(|v| v.as_str()))
            .collect::<Vec<_>>()
            .join("");
        if !cited_text.is_empty() {
            standard_citation["cited_text"] = json!(cited_text);
        }
    }

    let known_fields: HashSet<&str> = ["type", "source_content", "title", "index", "extras"].into();
    populate_extras(&mut standard_citation, citation, &known_fields);

    standard_citation
}

/// Convert Bedrock Converse content blocks to standard format.
///
/// This is the main entry point for converting Converse API response content
/// to the standardized v1 format.
pub fn convert_to_standard_blocks(content: &[Value], is_chunk: bool) -> Vec<Value> {
    convert_to_standard_blocks_with_context(content, is_chunk, None)
}

/// Context for chunk translation in Bedrock Converse.
#[derive(Default)]
pub struct ConverseChunkContext {
    /// Tool call chunks from the message.
    pub tool_call_chunks: Vec<Value>,
}

/// Convert Bedrock Converse content blocks to standard format with context.
pub fn convert_to_standard_blocks_with_context(
    content: &[Value],
    is_chunk: bool,
    context: Option<&ConverseChunkContext>,
) -> Vec<Value> {
    let mut result = Vec::new();

    for block in content {
        let obj = match block.as_object() {
            Some(o) => o,
            None => {
                if let Some(s) = block.as_str() {
                    result.push(json!({"type": "text", "text": s}));
                }
                continue;
            }
        };

        let block_type = obj.get("type").and_then(|v| v.as_str()).unwrap_or("");

        match block_type {
            "text" => {
                let mut text_block = json!({
                    "type": "text",
                    "text": block.get("text").and_then(|v| v.as_str()).unwrap_or(""),
                });

                if let Some(citations) = block.get("citations").and_then(|v| v.as_array()) {
                    let annotations: Vec<Value> =
                        citations.iter().map(convert_citation_to_v1).collect();
                    text_block["annotations"] = json!(annotations);
                }

                if let Some(index) = block.get("index") {
                    text_block["index"] = index.clone();
                }

                result.push(text_block);
            }

            "reasoning_content" => {
                let mut reasoning_block = json!({"type": "reasoning"});

                if let Some(reasoning_content) =
                    block.get("reasoning_content").and_then(|v| v.as_object())
                {
                    if let Some(text) = reasoning_content.get("text").and_then(|v| v.as_str()) {
                        reasoning_block["reasoning"] = json!(text);
                    }
                    if let Some(signature) =
                        reasoning_content.get("signature").and_then(|v| v.as_str())
                        && let Some(obj) = reasoning_block.as_object_mut()
                    {
                        let extras = obj.entry("extras").or_insert_with(|| json!({}));
                        if let Some(extras_obj) = extras.as_object_mut() {
                            extras_obj.insert("signature".to_string(), json!(signature));
                        }
                    }
                }

                if let Some(index) = block.get("index") {
                    reasoning_block["index"] = index.clone();
                }

                let known_fields: HashSet<&str> =
                    ["type", "reasoning_content", "index", "extras"].into();
                populate_extras(&mut reasoning_block, block, &known_fields);

                result.push(reasoning_block);
            }

            "tool_use" => {
                if is_chunk
                    && context
                        .map(|c| c.tool_call_chunks.len() == 1)
                        .unwrap_or(false)
                    && let Some(ctx) = context
                {
                    let chunk = &ctx.tool_call_chunks[0];
                    let mut tool_call_chunk = json!({
                        "type": "tool_call_chunk",
                        "name": chunk.get("name").cloned().unwrap_or(Value::Null),
                        "args": chunk.get("args").and_then(|v| v.as_str()).unwrap_or(""),
                        "id": chunk.get("id").cloned().unwrap_or(Value::Null),
                    });

                    if let Some(index) = chunk.get("index").or_else(|| block.get("index")) {
                        tool_call_chunk["index"] = index.clone();
                    }

                    result.push(tool_call_chunk);
                } else {
                    let mut tool_call_block = json!({
                        "type": "tool_call",
                        "name": block.get("name").and_then(|v| v.as_str()).unwrap_or(""),
                        "args": block.get("input").cloned().unwrap_or(json!({})),
                        "id": block.get("id").and_then(|v| v.as_str()).unwrap_or(""),
                    });

                    if let Some(index) = block.get("index") {
                        tool_call_block["index"] = index.clone();
                    }

                    result.push(tool_call_block);
                }
            }

            "input_json_delta" => {
                if let Some(ctx) = context
                    && ctx.tool_call_chunks.len() == 1
                {
                    let chunk = &ctx.tool_call_chunks[0];
                    let mut tool_call_chunk = json!({
                        "type": "tool_call_chunk",
                        "name": chunk.get("name").cloned().unwrap_or(Value::Null),
                        "args": chunk.get("args").and_then(|v| v.as_str()).unwrap_or(""),
                        "id": chunk.get("id").cloned().unwrap_or(Value::Null),
                    });

                    if let Some(index) = chunk.get("index").or_else(|| block.get("index")) {
                        tool_call_chunk["index"] = index.clone();
                    }

                    result.push(tool_call_chunk);
                    continue;
                }

                let mut tool_call_chunk = json!({
                    "type": "tool_call_chunk",
                    "name": Value::Null,
                    "args": block.get("partial_json").and_then(|v| v.as_str()).unwrap_or(""),
                    "id": Value::Null,
                });

                if let Some(index) = block.get("index") {
                    tool_call_chunk["index"] = index.clone();
                }

                result.push(tool_call_chunk);
            }

            _ => {
                if KNOWN_BLOCK_TYPES.contains(&block_type) {
                    result.push(block.clone());
                } else {
                    let mut non_standard = json!({
                        "type": "non_standard",
                        "value": block.clone(),
                    });
                    if let Some(index) = block.get("index") {
                        non_standard["index"] = index.clone();
                        if let Some(value) = non_standard.get_mut("value")
                            && let Some(obj) = value.as_object_mut()
                        {
                            obj.remove("index");
                        }
                    }
                    result.push(non_standard);
                }
            }
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_convert_text_block() {
        let content = vec![json!({"type": "text", "text": "Hello from Converse"})];
        let result = convert_to_standard_blocks(&content, false);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0]["type"], "text");
        assert_eq!(result[0]["text"], "Hello from Converse");
    }

    #[test]
    fn test_convert_reasoning_content() {
        let content = vec![json!({
            "type": "reasoning_content",
            "reasoning_content": {
                "text": "Let me think about this...",
                "signature": "sig_123"
            }
        })];
        let result = convert_to_standard_blocks(&content, false);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0]["type"], "reasoning");
        assert_eq!(result[0]["reasoning"], "Let me think about this...");
        assert_eq!(result[0]["extras"]["signature"], "sig_123");
    }

    #[test]
    fn test_convert_tool_use() {
        let content = vec![json!({
            "type": "tool_use",
            "id": "tool_1",
            "name": "calculator",
            "input": {"expression": "2+2"}
        })];
        let result = convert_to_standard_blocks(&content, false);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0]["type"], "tool_call");
        assert_eq!(result[0]["name"], "calculator");
        assert_eq!(result[0]["id"], "tool_1");
        assert_eq!(result[0]["args"]["expression"], "2+2");
    }

    #[test]
    fn test_convert_input_image() {
        let content = vec![json!({
            "image": {
                "format": "png",
                "source": {"bytes": "aGVsbG8="}
            }
        })];
        let result = convert_input_to_standard_blocks(&content);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0]["type"], "image");
        assert_eq!(result[0]["mime_type"], "image/png");
    }

    #[test]
    fn test_convert_input_pdf_document() {
        let content = vec![json!({
            "document": {
                "format": "pdf",
                "source": {"bytes": "cGRmZGF0YQ=="}
            }
        })];
        let result = convert_input_to_standard_blocks(&content);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0]["type"], "file");
        assert_eq!(result[0]["mime_type"], "application/pdf");
    }

    #[test]
    fn test_convert_input_text_document() {
        let content = vec![json!({
            "document": {
                "format": "txt",
                "source": {"text": "Hello, world!"}
            }
        })];
        let result = convert_input_to_standard_blocks(&content);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0]["type"], "text-plain");
        assert_eq!(result[0]["text"], "Hello, world!");
    }

    #[test]
    fn test_convert_text_with_citations() {
        let content = vec![json!({
            "type": "text",
            "text": "The answer is 42.",
            "citations": [{"title": "Guide", "source_content": [{"text": "42 is the answer"}]}]
        })];
        let result = convert_to_standard_blocks(&content, false);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0]["type"], "text");
        let annotations = result[0]["annotations"].as_array().unwrap();
        assert_eq!(annotations.len(), 1);
        assert_eq!(annotations[0]["type"], "citation");
        assert_eq!(annotations[0]["title"], "Guide");
        assert_eq!(annotations[0]["cited_text"], "42 is the answer");
    }
}
