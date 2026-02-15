//! Derivations of standard content blocks from Google (GenAI) content.
//!
//! Mirrors `langchain_core/messages/block_translators/google_genai.py`.
//!
//! Google GenAI uses a Part-based format:
//! - Text parts: `{"type": "text", "text": "..."}`
//! - Inline data: `{"type": "inline_data", "data": "...", "mime_type": "..."}`
//! - Function calls: `{"type": "function_call", "name": "...", "args": {...}}`
//! - Thinking: `{"type": "thinking", "thinking": "..."}`
//! - Executable code: `{"type": "executable_code", ...}`
//! - Code execution result: `{"type": "code_execution_result", ...}`

use std::collections::HashSet;

use base64::Engine;
use serde_json::{Value, json};

use crate::messages::content::KNOWN_BLOCK_TYPES;

/// Convert bytes (as a JSON value) to a base64 string.
fn bytes_to_b64_str(bytes_value: &Value) -> String {
    if let Some(s) = bytes_value.as_str() {
        s.to_string()
    } else if let Some(arr) = bytes_value.as_array() {
        let bytes: Vec<u8> = arr
            .iter()
            .filter_map(|v| v.as_u64().map(|n| n as u8))
            .collect();
        base64::engine::general_purpose::STANDARD.encode(&bytes)
    } else {
        String::new()
    }
}

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
                    .unwrap()
                    .entry("extras")
                    .or_insert_with(|| json!({}));
                if let Some(extras_obj) = extras.as_object_mut() {
                    extras_obj.insert(key.clone(), value.clone());
                }
            }
        }
    }
}

/// Translate Google AI grounding metadata to LangChain Citations.
pub fn translate_grounding_metadata_to_citations(grounding_metadata: &Value) -> Vec<Value> {
    if grounding_metadata.is_null() {
        return Vec::new();
    }

    let grounding_chunks = grounding_metadata
        .get("grounding_chunks")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let grounding_supports = grounding_metadata
        .get("grounding_supports")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let web_search_queries = grounding_metadata
        .get("web_search_queries")
        .cloned()
        .unwrap_or(json!([]));

    let mut citations = Vec::new();

    for support in &grounding_supports {
        let segment = support.get("segment").cloned().unwrap_or(json!({}));
        let chunk_indices = support
            .get("grounding_chunk_indices")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();

        let start_index = segment.get("start_index").cloned();
        let end_index = segment.get("end_index").cloned();
        let cited_text = segment
            .get("text")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        for chunk_index_val in &chunk_indices {
            let chunk_index = chunk_index_val.as_u64().unwrap_or(0) as usize;
            if chunk_index < grounding_chunks.len() {
                let chunk = &grounding_chunks[chunk_index];

                let web_info = chunk.get("web").cloned().unwrap_or(json!({}));
                let maps_info = chunk.get("maps").cloned().unwrap_or(json!({}));

                let url = maps_info
                    .get("uri")
                    .or_else(|| web_info.get("uri"))
                    .and_then(|v| v.as_str());
                let title = maps_info
                    .get("title")
                    .or_else(|| web_info.get("title"))
                    .and_then(|v| v.as_str());

                let mut citation = json!({"type": "citation"});

                if let Some(url) = url {
                    citation["url"] = json!(url);
                }
                if let Some(title) = title {
                    citation["title"] = json!(title);
                }
                if let Some(ref start) = start_index {
                    citation["start_index"] = start.clone();
                }
                if let Some(ref end) = end_index {
                    citation["end_index"] = end.clone();
                }
                if let Some(ref text) = cited_text {
                    citation["cited_text"] = json!(text);
                }

                let mut extras = json!({
                    "web_search_queries": web_search_queries,
                    "grounding_chunk_index": chunk_index,
                });
                if let Some(confidence) = support.get("confidence_scores") {
                    extras["confidence_scores"] = confidence.clone();
                } else {
                    extras["confidence_scores"] = json!([]);
                }
                if let Some(place_id) = maps_info.get("placeId").and_then(|v| v.as_str()) {
                    extras["place_id"] = json!(place_id);
                }

                citation["extras"] = extras;
                citations.push(citation);
            }
        }
    }

    citations
}

/// Convert Google GenAI format input blocks to v1 format.
///
/// During the `content_blocks` parsing process, blocks not recognized as v1
/// are wrapped as `non_standard`. This function unpacks those and converts
/// GenAI-format blocks to v1 ContentBlocks.
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
        let block_type = obj.get("type").and_then(|v| v.as_str());

        // {"text": "..."} -> TextContentBlock
        if num_keys == 1 {
            if let Some(text) = obj.get("text").and_then(|v| v.as_str()) {
                result.push(json!({"type": "text", "text": text}));
                continue;
            }
        }

        // {"document": {"format": ..., "source": ...}} -> FileContentBlock / PlainTextContentBlock
        if num_keys == 1 {
            if let Some(document) = obj.get("document").and_then(|v| v.as_object()) {
                if let Some(format) = document.get("format").and_then(|v| v.as_str()) {
                    let source = document.get("source").and_then(|v| v.as_object());
                    match format {
                        "pdf" => {
                            if let Some(bytes_val) = source.and_then(|s| s.get("bytes")) {
                                let b64 = if bytes_val.is_string() {
                                    bytes_val.as_str().unwrap_or("").to_string()
                                } else {
                                    bytes_to_b64_str(bytes_val)
                                };
                                let mut file_block = json!({
                                    "type": "file",
                                    "base64": b64,
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
                            if let Some(text) =
                                source.and_then(|s| s.get("text")).and_then(|t| t.as_str())
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
            }
        }

        // {"image": {"format": ..., "source": {"bytes": ...}}} -> ImageContentBlock
        if num_keys == 1 {
            if let Some(image) = obj.get("image").and_then(|v| v.as_object()) {
                if let Some(format) = image.get("format").and_then(|v| v.as_str()) {
                    if let Some(bytes_val) = image
                        .get("source")
                        .and_then(|s| s.as_object())
                        .and_then(|s| s.get("bytes"))
                    {
                        let b64 = if bytes_val.is_string() {
                            bytes_val.as_str().unwrap_or("").to_string()
                        } else {
                            bytes_to_b64_str(bytes_val)
                        };
                        let mut image_block = json!({
                            "type": "image",
                            "base64": b64,
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
            }
        }

        // {"type": "file_data", "file_uri": ...} -> FileContentBlock
        if block_type == Some("file_data") {
            if let Some(uri) = obj.get("file_uri").and_then(|v| v.as_str()) {
                let mut file_block = json!({"type": "file", "url": uri});
                if let Some(mime) = obj.get("mime_type").and_then(|v| v.as_str()) {
                    file_block["mime_type"] = json!(mime);
                }
                result.push(file_block);
                continue;
            }
        }

        // {"type": "function_call", ...} -> ToolCall
        if block_type == Some("function_call") {
            if let Some(name) = obj.get("name").and_then(|v| v.as_str()) {
                result.push(json!({
                    "type": "tool_call",
                    "name": name,
                    "args": obj.get("args").cloned().unwrap_or(json!({})),
                    "id": obj.get("id").and_then(|v| v.as_str()).unwrap_or(""),
                }));
                continue;
            }
        }

        // {"type": "executable_code", ...} -> ServerToolCall
        if block_type == Some("executable_code") {
            result.push(json!({
                "type": "server_tool_call",
                "name": "code_interpreter",
                "args": {
                    "code": obj.get("executable_code").and_then(|v| v.as_str()).unwrap_or(""),
                    "language": obj.get("language").and_then(|v| v.as_str()).unwrap_or("python"),
                },
                "id": obj.get("id").and_then(|v| v.as_str()).unwrap_or(""),
            }));
            continue;
        }

        // {"type": "code_execution_result", ...} -> ServerToolResult
        if block_type == Some("code_execution_result") {
            let outcome = obj.get("outcome").and_then(|v| v.as_i64()).unwrap_or(1);
            let status = if outcome == 1 { "success" } else { "error" };
            let mut server_result = json!({
                "type": "server_tool_result",
                "tool_call_id": obj.get("tool_call_id").and_then(|v| v.as_str()).unwrap_or(""),
                "status": status,
                "output": obj.get("code_execution_result").and_then(|v| v.as_str()).unwrap_or(""),
            });
            server_result["extras"] = json!({"outcome": outcome});
            result.push(server_result);
            continue;
        }

        // Known v1 block type — pass through
        if let Some(bt) = block_type {
            if KNOWN_BLOCK_TYPES.contains(&bt) {
                result.push(block.clone());
                continue;
            }
        }

        // Unknown — wrap as non_standard
        result.push(json!({"type": "non_standard", "value": block}));
    }

    result
}

/// Convert Google GenAI content blocks to standard format.
///
/// This handles both `AIMessage` content (list of parts) and string content.
pub fn convert_to_standard_blocks(content: &[Value], _is_chunk: bool) -> Vec<Value> {
    let mut result = Vec::new();

    for block in content {
        if let Some(text) = block.as_str() {
            result.push(json!({"type": "text", "text": text}));
            continue;
        }

        let obj = match block.as_object() {
            Some(o) => o,
            None => {
                result.push(json!({"type": "non_standard", "value": block}));
                continue;
            }
        };

        let block_type = obj.get("type").and_then(|v| v.as_str());

        match block_type {
            Some("text") => {
                result.push(block.clone());
            }

            Some("image_url") => {
                // image_url format from previous implementations
                if let Some(image_url) = obj.get("image_url").and_then(|v| v.as_object()) {
                    if let Some(url) = image_url.get("url").and_then(|v| v.as_str()) {
                        // Check if it's a data URI
                        if let Some(rest) = url.strip_prefix("data:") {
                            if let Some((mime_type, data)) = rest.split_once(";base64,") {
                                result.push(json!({
                                    "type": "image",
                                    "base64": data,
                                    "mime_type": mime_type,
                                }));
                            } else {
                                result.push(json!({"type": "non_standard", "value": block}));
                            }
                        } else {
                            // Try as raw base64
                            result.push(json!({
                                "type": "image",
                                "base64": url,
                            }));
                        }
                    } else {
                        result.push(json!({"type": "non_standard", "value": block}));
                    }
                } else {
                    result.push(json!({"type": "non_standard", "value": block}));
                }
            }

            Some("function_call") => {
                result.push(json!({
                    "type": "tool_call",
                    "name": obj.get("name").and_then(|v| v.as_str()).unwrap_or(""),
                    "args": obj.get("args").cloned().unwrap_or(json!({})),
                    "id": obj.get("id").and_then(|v| v.as_str()).unwrap_or(""),
                }));
            }

            Some("file_data") => {
                let mut file_block = json!({
                    "type": "file",
                    "url": obj.get("file_uri").and_then(|v| v.as_str()).unwrap_or(""),
                });
                if let Some(mime) = obj.get("mime_type").and_then(|v| v.as_str()) {
                    file_block["mime_type"] = json!(mime);
                }
                result.push(file_block);
            }

            Some("thinking") => {
                let mut reasoning_block = json!({
                    "type": "reasoning",
                    "reasoning": obj.get("thinking").and_then(|v| v.as_str()).unwrap_or(""),
                });
                if let Some(signature) = obj.get("signature").and_then(|v| v.as_str()) {
                    reasoning_block["extras"] = json!({"signature": signature});
                }
                result.push(reasoning_block);
            }

            Some("executable_code") => {
                result.push(json!({
                    "type": "server_tool_call",
                    "name": "code_interpreter",
                    "args": {
                        "code": obj.get("executable_code").and_then(|v| v.as_str()).unwrap_or(""),
                        "language": obj.get("language").and_then(|v| v.as_str()).unwrap_or("python"),
                    },
                    "id": obj.get("id").and_then(|v| v.as_str()).unwrap_or(""),
                }));
            }

            Some("code_execution_result") => {
                let outcome = obj.get("outcome").and_then(|v| v.as_i64()).unwrap_or(1);
                let status = if outcome == 1 { "success" } else { "error" };
                let mut server_result = json!({
                    "type": "server_tool_result",
                    "tool_call_id": obj.get("tool_call_id").and_then(|v| v.as_str()).unwrap_or(""),
                    "status": status,
                    "output": obj.get("code_execution_result").and_then(|v| v.as_str()).unwrap_or(""),
                });
                server_result["extras"] =
                    json!({"block_type": "code_execution_result", "outcome": outcome});
                result.push(server_result);
            }

            Some(bt) if KNOWN_BLOCK_TYPES.contains(&bt) => {
                result.push(block.clone());
            }

            _ => {
                result.push(json!({"type": "non_standard", "value": block}));
            }
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_convert_text() {
        let content = vec![json!({"type": "text", "text": "Hello from Gemini"})];
        let result = convert_to_standard_blocks(&content, false);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0]["type"], "text");
        assert_eq!(result[0]["text"], "Hello from Gemini");
    }

    #[test]
    fn test_convert_function_call() {
        let content = vec![json!({
            "type": "function_call",
            "name": "search",
            "args": {"query": "rust"},
            "id": "fc_1"
        })];
        let result = convert_to_standard_blocks(&content, false);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0]["type"], "tool_call");
        assert_eq!(result[0]["name"], "search");
        assert_eq!(result[0]["args"]["query"], "rust");
        assert_eq!(result[0]["id"], "fc_1");
    }

    #[test]
    fn test_convert_thinking() {
        let content = vec![json!({
            "type": "thinking",
            "thinking": "Let me analyze...",
            "signature": "sig_abc"
        })];
        let result = convert_to_standard_blocks(&content, false);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0]["type"], "reasoning");
        assert_eq!(result[0]["reasoning"], "Let me analyze...");
        assert_eq!(result[0]["extras"]["signature"], "sig_abc");
    }

    #[test]
    fn test_convert_executable_code() {
        let content = vec![json!({
            "type": "executable_code",
            "executable_code": "print('hello')",
            "language": "python",
            "id": "exec_1"
        })];
        let result = convert_to_standard_blocks(&content, false);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0]["type"], "server_tool_call");
        assert_eq!(result[0]["name"], "code_interpreter");
        assert_eq!(result[0]["args"]["code"], "print('hello')");
        assert_eq!(result[0]["args"]["language"], "python");
    }

    #[test]
    fn test_convert_code_execution_result() {
        let content = vec![json!({
            "type": "code_execution_result",
            "code_execution_result": "hello",
            "outcome": 1
        })];
        let result = convert_to_standard_blocks(&content, false);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0]["type"], "server_tool_result");
        assert_eq!(result[0]["output"], "hello");
        assert_eq!(result[0]["status"], "success");
    }

    #[test]
    fn test_convert_code_execution_result_error() {
        let content = vec![json!({
            "type": "code_execution_result",
            "code_execution_result": "NameError",
            "outcome": 2
        })];
        let result = convert_to_standard_blocks(&content, false);
        assert_eq!(result[0]["status"], "error");
    }

    #[test]
    fn test_convert_file_data() {
        let content = vec![json!({
            "type": "file_data",
            "file_uri": "gs://bucket/file.pdf",
            "mime_type": "application/pdf"
        })];
        let result = convert_to_standard_blocks(&content, false);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0]["type"], "file");
        assert_eq!(result[0]["url"], "gs://bucket/file.pdf");
        assert_eq!(result[0]["mime_type"], "application/pdf");
    }

    #[test]
    fn test_convert_image_url_data_uri() {
        let content = vec![json!({
            "type": "image_url",
            "image_url": {"url": "data:image/png;base64,iVBOR"}
        })];
        let result = convert_to_standard_blocks(&content, false);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0]["type"], "image");
        assert_eq!(result[0]["base64"], "iVBOR");
        assert_eq!(result[0]["mime_type"], "image/png");
    }

    #[test]
    fn test_convert_input_inline_data_image() {
        let content = vec![json!({
            "image": {
                "format": "jpeg",
                "source": {"bytes": "abc123"}
            }
        })];
        let result = convert_input_to_standard_blocks(&content);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0]["type"], "image");
        assert_eq!(result[0]["mime_type"], "image/jpeg");
    }

    #[test]
    fn test_grounding_metadata_to_citations() {
        let metadata = json!({
            "web_search_queries": ["rust programming"],
            "grounding_chunks": [{
                "web": {
                    "uri": "https://rust-lang.org",
                    "title": "Rust Language"
                }
            }],
            "grounding_supports": [{
                "segment": {
                    "start_index": 0,
                    "end_index": 20,
                    "text": "Rust is a fast language"
                },
                "grounding_chunk_indices": [0]
            }]
        });
        let citations = translate_grounding_metadata_to_citations(&metadata);
        assert_eq!(citations.len(), 1);
        assert_eq!(citations[0]["type"], "citation");
        assert_eq!(citations[0]["url"], "https://rust-lang.org");
        assert_eq!(citations[0]["title"], "Rust Language");
        assert_eq!(citations[0]["cited_text"], "Rust is a fast language");
    }
}
