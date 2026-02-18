//! Anthropic block translator.
//!
//! Converts Anthropic-specific content blocks to the standard LangChain format.
//!
//! This corresponds to `langchain_core/messages/block_translators/anthropic.py` in Python.

use serde_json::{Value, json};
use std::collections::HashSet;

/// Known block types in the standard format.
const KNOWN_BLOCK_TYPES: &[&str] = &[
    "text",
    "image",
    "audio",
    "video",
    "file",
    "text-plain",
    "tool_call",
    "tool_call_chunk",
    "tool_result",
    "reasoning",
    "server_tool_call",
    "server_tool_call_chunk",
    "server_tool_result",
    "citation",
    "non_standard",
    "non_standard_annotation",
];

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

/// Convert a citation to the standard v1 format.
fn convert_citation_to_v1(citation: &Value) -> Value {
    let citation_type = citation.get("type").and_then(|v| v.as_str()).unwrap_or("");

    if citation_type == "web_search_result_location" {
        let mut url_citation = json!({
            "type": "citation",
            "cited_text": citation.get("cited_text").cloned().unwrap_or(json!("")),
            "url": citation.get("url").cloned().unwrap_or(json!("")),
        });

        if let Some(title) = citation.get("title") {
            url_citation["title"] = title.clone();
        }

        let known_fields: HashSet<&str> = ["type", "cited_text", "url", "title", "index", "extras"]
            .iter()
            .copied()
            .collect();

        if let Some(citation_obj) = citation.as_object() {
            for (key, value) in citation_obj {
                if !known_fields.contains(key.as_str())
                    && let Some(obj) = url_citation.as_object_mut()
                {
                    let extras = obj.entry("extras").or_insert_with(|| json!({}));
                    if let Some(extras_obj) = extras.as_object_mut() {
                        extras_obj.insert(key.clone(), value.clone());
                    }
                }
            }
        }

        return url_citation;
    }

    if matches!(
        citation_type,
        "char_location" | "content_block_location" | "page_location" | "search_result_location"
    ) {
        let mut document_citation = json!({
            "type": "citation",
            "cited_text": citation.get("cited_text").cloned().unwrap_or(json!("")),
        });

        if let Some(title) = citation.get("document_title") {
            document_citation["title"] = title.clone();
        } else if let Some(title) = citation.get("title") {
            document_citation["title"] = title.clone();
        }

        let known_fields: HashSet<&str> = [
            "type",
            "cited_text",
            "document_title",
            "title",
            "index",
            "extras",
        ]
        .iter()
        .copied()
        .collect();

        if let Some(citation_obj) = citation.as_object() {
            for (key, value) in citation_obj {
                if !known_fields.contains(key.as_str())
                    && let Some(obj) = document_citation.as_object_mut()
                {
                    let extras = obj.entry("extras").or_insert_with(|| json!({}));
                    if let Some(extras_obj) = extras.as_object_mut() {
                        extras_obj.insert(key.clone(), value.clone());
                    }
                }
            }
        }

        return document_citation;
    }

    json!({
        "type": "non_standard_annotation",
        "value": citation.clone(),
    })
}

/// Context for chunk translation, containing tool_call_chunks from the message.
#[derive(Default)]
pub struct ChunkContext {
    /// Tool call chunks from the message (used for tool_use block translation)
    pub tool_call_chunks: Vec<Value>,
}

/// Convert Anthropic content blocks to standard format.
///
/// # Arguments
/// * `content` - The raw content blocks from Anthropic
/// * `is_chunk` - Whether this is a streaming chunk (affects tool_use handling)
///
/// # Returns
/// A vector of standardized content blocks.
pub fn convert_to_standard_blocks(content: &[Value], is_chunk: bool) -> Vec<Value> {
    convert_to_standard_blocks_with_context(content, is_chunk, None)
}

/// Convert Anthropic content blocks to standard format with additional context.
///
/// # Arguments
/// * `content` - The raw content blocks from Anthropic
/// * `is_chunk` - Whether this is a streaming chunk (affects tool_use handling)
/// * `context` - Optional context containing tool_call_chunks for chunk translation
///
/// # Returns
/// A vector of standardized content blocks.
pub fn convert_to_standard_blocks_with_context(
    content: &[Value],
    is_chunk: bool,
    context: Option<&ChunkContext>,
) -> Vec<Value> {
    let mut result = Vec::new();

    for block in content {
        if !block.is_object() {
            if let Some(s) = block.as_str() {
                result.push(json!({"type": "text", "text": s}));
            }
            continue;
        }

        let block_type = block.get("type").and_then(|v| v.as_str()).unwrap_or("");

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

            "thinking" => {
                let mut reasoning_block = json!({
                    "type": "reasoning",
                    "reasoning": block.get("thinking").and_then(|v| v.as_str()).unwrap_or(""),
                });

                if let Some(index) = block.get("index") {
                    reasoning_block["index"] = index.clone();
                }

                let known_fields: HashSet<&str> = ["type", "thinking", "index", "extras"]
                    .iter()
                    .copied()
                    .collect();
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

                    if let Some(index) = chunk.get("index") {
                        tool_call_chunk["index"] = index.clone();
                    } else if let Some(index) = block.get("index") {
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

                    if let Some(caller) = block.get("caller") {
                        tool_call_block["extras"] = json!({"caller": caller.clone()});
                    }

                    result.push(tool_call_block);
                }
            }

            "input_json_delta" => {
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

            "server_tool_use" => {
                let name = block.get("name").and_then(|v| v.as_str()).unwrap_or("");
                let server_tool_use_name = if name == "code_execution" {
                    "code_interpreter"
                } else {
                    name
                };

                let input = block.get("input");
                let has_partial_json = block.get("partial_json").is_some();
                let is_empty_input =
                    input.map(|v| v == &json!({})).unwrap_or(true) && !has_partial_json;

                if is_chunk && is_empty_input {
                    let mut server_tool_call_chunk = json!({
                        "type": "server_tool_call_chunk",
                        "name": server_tool_use_name,
                        "args": "",
                        "id": block.get("id").and_then(|v| v.as_str()).unwrap_or(""),
                    });

                    if let Some(index) = block.get("index") {
                        server_tool_call_chunk["index"] = index.clone();
                    }

                    let known_fields: HashSet<&str> = ["type", "name", "input", "id", "index"]
                        .iter()
                        .copied()
                        .collect();
                    populate_extras(&mut server_tool_call_chunk, block, &known_fields);

                    result.push(server_tool_call_chunk);
                } else {
                    let mut args = block.get("input").cloned().unwrap_or(json!({}));

                    if args == json!({})
                        && let Some(partial_json) =
                            block.get("partial_json").and_then(|v| v.as_str())
                        && let Ok(parsed) = serde_json::from_str::<Value>(partial_json)
                        && parsed.is_object()
                    {
                        args = parsed;
                    }

                    let mut server_tool_call = json!({
                        "type": "server_tool_call",
                        "name": server_tool_use_name,
                        "args": args,
                        "id": block.get("id").and_then(|v| v.as_str()).unwrap_or(""),
                    });

                    if let Some(index) = block.get("index") {
                        server_tool_call["index"] = index.clone();
                    }

                    let known_fields: HashSet<&str> =
                        ["type", "name", "input", "partial_json", "id", "index"]
                            .iter()
                            .copied()
                            .collect();
                    populate_extras(&mut server_tool_call, block, &known_fields);

                    result.push(server_tool_call);
                }
            }

            "mcp_tool_use" => {
                let input = block.get("input");
                let has_partial_json = block.get("partial_json").is_some();
                let is_empty_input =
                    input.map(|v| v == &json!({})).unwrap_or(true) && !has_partial_json;

                if is_chunk && is_empty_input {
                    let mut server_tool_call_chunk = json!({
                        "type": "server_tool_call_chunk",
                        "name": "remote_mcp",
                        "args": "",
                        "id": block.get("id").and_then(|v| v.as_str()).unwrap_or(""),
                    });

                    if let Some(name) = block.get("name") {
                        server_tool_call_chunk["extras"] = json!({"tool_name": name.clone()});
                    }

                    let known_fields: HashSet<&str> = ["type", "name", "input", "id", "index"]
                        .iter()
                        .copied()
                        .collect();
                    populate_extras(&mut server_tool_call_chunk, block, &known_fields);

                    if let Some(index) = block.get("index") {
                        server_tool_call_chunk["index"] = index.clone();
                    }

                    result.push(server_tool_call_chunk);
                } else {
                    let mut args = block.get("input").cloned().unwrap_or(json!({}));

                    if args == json!({})
                        && let Some(partial_json) =
                            block.get("partial_json").and_then(|v| v.as_str())
                        && let Ok(parsed) = serde_json::from_str::<Value>(partial_json)
                        && parsed.is_object()
                    {
                        args = parsed;
                    }

                    let mut server_tool_call = json!({
                        "type": "server_tool_call",
                        "name": "remote_mcp",
                        "args": args,
                        "id": block.get("id").and_then(|v| v.as_str()).unwrap_or(""),
                    });

                    if let Some(name) = block.get("name") {
                        server_tool_call["extras"] = json!({"tool_name": name.clone()});
                    }

                    let known_fields: HashSet<&str> =
                        ["type", "name", "input", "partial_json", "id", "index"]
                            .iter()
                            .copied()
                            .collect();
                    populate_extras(&mut server_tool_call, block, &known_fields);

                    if let Some(index) = block.get("index") {
                        server_tool_call["index"] = index.clone();
                    }

                    result.push(server_tool_call);
                }
            }

            bt if bt.ends_with("_tool_result") => {
                let mut server_tool_result = json!({
                    "type": "server_tool_result",
                    "tool_call_id": block.get("tool_use_id").and_then(|v| v.as_str()).unwrap_or(""),
                    "status": "success",
                    "extras": {"block_type": bt},
                });

                if let Some(output) = block.get("content") {
                    server_tool_result["output"] = output.clone();

                    if let Some(output_obj) = output.as_object()
                        && output_obj.contains_key("error_code")
                    {
                        server_tool_result["status"] = json!("error");
                    }
                }

                if block.get("is_error") == Some(&json!(true)) {
                    server_tool_result["status"] = json!("error");
                }

                if let Some(index) = block.get("index") {
                    server_tool_result["index"] = index.clone();
                }

                let known_fields: HashSet<&str> =
                    ["type", "tool_use_id", "content", "is_error", "index"]
                        .iter()
                        .copied()
                        .collect();
                populate_extras(&mut server_tool_result, block, &known_fields);

                result.push(server_tool_result);
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

/// Convert Anthropic input content blocks (for HumanMessage) to standard format.
///
/// During the `content_blocks` parsing process, blocks not recognized as v1 are
/// wrapped as `non_standard` with the original block in the `value` field. This
/// function unpacks those blocks before attempting Anthropic-specific conversion.
pub fn convert_input_to_standard_blocks(content: &[Value]) -> Vec<Value> {
    let mut result = Vec::new();

    let unpacked_blocks: Vec<Value> = content
        .iter()
        .map(|block| {
            if block.get("type").and_then(|v| v.as_str()) == Some("non_standard") {
                block.get("value").cloned().unwrap_or_else(|| block.clone())
            } else {
                block.clone()
            }
        })
        .collect();

    for block in &unpacked_blocks {
        if !block.is_object() {
            if let Some(s) = block.as_str() {
                result.push(json!({"type": "text", "text": s}));
            }
            continue;
        }

        let block_type = block.get("type").and_then(|v| v.as_str()).unwrap_or("");

        match block_type {
            "document" => {
                if let Some(source) = block.get("source").and_then(|v| v.as_object()) {
                    let source_type = source.get("type").and_then(|v| v.as_str()).unwrap_or("");

                    match source_type {
                        "base64" => {
                            let mut file_block = json!({
                                "type": "file",
                                "base64": source.get("data").cloned().unwrap_or(json!("")),
                                "mime_type": source.get("media_type").cloned().unwrap_or(json!("")),
                            });
                            let known_fields: HashSet<&str> =
                                ["type", "source"].iter().copied().collect();
                            populate_extras(&mut file_block, block, &known_fields);
                            result.push(file_block);
                        }
                        "url" => {
                            let mut file_block = json!({
                                "type": "file",
                                "url": source.get("url").cloned().unwrap_or(json!("")),
                            });
                            let known_fields: HashSet<&str> =
                                ["type", "source"].iter().copied().collect();
                            populate_extras(&mut file_block, block, &known_fields);
                            result.push(file_block);
                        }
                        "file" => {
                            let mut file_block = json!({
                                "type": "file",
                                "id": source.get("file_id").cloned().unwrap_or(json!("")),
                            });
                            let known_fields: HashSet<&str> =
                                ["type", "source"].iter().copied().collect();
                            populate_extras(&mut file_block, block, &known_fields);
                            result.push(file_block);
                        }
                        "text" => {
                            let mut plain_text_block = json!({
                                "type": "text-plain",
                                "text": source.get("data").cloned().unwrap_or(json!("")),
                                "mime_type": block.get("media_type").cloned().unwrap_or(json!("text/plain")),
                            });
                            let known_fields: HashSet<&str> =
                                ["type", "source"].iter().copied().collect();
                            populate_extras(&mut plain_text_block, block, &known_fields);
                            result.push(plain_text_block);
                        }
                        _ => {
                            result.push(json!({
                                "type": "non_standard",
                                "value": block.clone(),
                            }));
                        }
                    }
                } else {
                    result.push(json!({
                        "type": "non_standard",
                        "value": block.clone(),
                    }));
                }
            }

            "image" => {
                if let Some(source) = block.get("source").and_then(|v| v.as_object()) {
                    let source_type = source.get("type").and_then(|v| v.as_str()).unwrap_or("");

                    match source_type {
                        "base64" => {
                            let mut image_block = json!({
                                "type": "image",
                                "base64": source.get("data").cloned().unwrap_or(json!("")),
                                "mime_type": source.get("media_type").cloned().unwrap_or(json!("")),
                            });
                            let known_fields: HashSet<&str> =
                                ["type", "source"].iter().copied().collect();
                            populate_extras(&mut image_block, block, &known_fields);
                            result.push(image_block);
                        }
                        "url" => {
                            let mut image_block = json!({
                                "type": "image",
                                "url": source.get("url").cloned().unwrap_or(json!("")),
                            });
                            let known_fields: HashSet<&str> =
                                ["type", "source"].iter().copied().collect();
                            populate_extras(&mut image_block, block, &known_fields);
                            result.push(image_block);
                        }
                        "file" => {
                            let mut image_block = json!({
                                "type": "image",
                                "id": source.get("file_id").cloned().unwrap_or(json!("")),
                            });
                            let known_fields: HashSet<&str> =
                                ["type", "source"].iter().copied().collect();
                            populate_extras(&mut image_block, block, &known_fields);
                            result.push(image_block);
                        }
                        _ => {
                            result.push(json!({
                                "type": "non_standard",
                                "value": block.clone(),
                            }));
                        }
                    }
                } else {
                    if KNOWN_BLOCK_TYPES.contains(&block_type) {
                        result.push(block.clone());
                    } else {
                        result.push(json!({
                            "type": "non_standard",
                            "value": block.clone(),
                        }));
                    }
                }
            }

            _ => {
                if KNOWN_BLOCK_TYPES.contains(&block_type) {
                    result.push(block.clone());
                } else {
                    result.push(json!({
                        "type": "non_standard",
                        "value": block.clone(),
                    }));
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
        let content = vec![json!({"type": "text", "text": "Hello"})];
        let result = convert_to_standard_blocks(&content, false);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0]["type"], "text");
        assert_eq!(result[0]["text"], "Hello");
    }

    #[test]
    fn test_convert_thinking_block() {
        let content = vec![json!({
            "type": "thinking",
            "thinking": "foo",
            "signature": "foo_signature"
        })];
        let result = convert_to_standard_blocks(&content, false);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0]["type"], "reasoning");
        assert_eq!(result[0]["reasoning"], "foo");
        assert_eq!(result[0]["extras"]["signature"], "foo_signature");
    }
}
