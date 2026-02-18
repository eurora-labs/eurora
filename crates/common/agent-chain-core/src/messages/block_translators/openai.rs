//! OpenAI block translator.
//!
//! Converts OpenAI-specific content blocks to the standard LangChain format.
//!
//! This corresponds to `langchain_core/messages/block_translators/openai.py` in Python.

use serde_json::{Value, json};
use std::collections::{HashMap, HashSet};
use tracing::warn;

use crate::messages::content::KNOWN_BLOCK_TYPES;
use crate::{is_openai_data_block, parse_data_uri};

/// Simple hex encoding function to avoid adding a dependency.
fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

/// OpenAI API type for formatting data blocks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OpenAiApi {
    /// Chat Completions API format.
    #[default]
    ChatCompletions,
    /// Responses API format.
    Responses,
}

/// Key used to store function call IDs in additional_kwargs.
/// This matches the Python implementation's `_FUNCTION_CALL_IDS_MAP_KEY`.
#[allow(dead_code)]
const FUNCTION_CALL_IDS_MAP_KEY: &str = "__openai_function_call_ids__";

/// Convert `ImageContentBlock` to format expected by OpenAI Chat Completions.
pub fn convert_to_openai_image_block(block: &Value) -> Result<Value, String> {
    if let Some(url) = block.get("url").and_then(|v| v.as_str()) {
        return Ok(json!({
            "type": "image_url",
            "image_url": {
                "url": url
            }
        }));
    }

    let is_base64 = block.get("base64").is_some()
        || block.get("source_type").and_then(|v| v.as_str()) == Some("base64");

    if is_base64 {
        let mime_type = block
            .get("mime_type")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "mime_type key is required for base64 data.".to_string())?;

        let base64_data = if block.get("data").is_some() {
            block.get("data").and_then(|v| v.as_str()).unwrap_or("")
        } else {
            block.get("base64").and_then(|v| v.as_str()).unwrap_or("")
        };

        return Ok(json!({
            "type": "image_url",
            "image_url": {
                "url": format!("data:{};base64,{}", mime_type, base64_data)
            }
        }));
    }

    Err("Unsupported source type. Only 'url' and 'base64' are supported.".to_string())
}

/// Format standard data content block to format expected by OpenAI.
///
/// "Standard data content block" can include old-style LangChain v0 blocks
/// (URLContentBlock, Base64ContentBlock, IDContentBlock) or new ones.
pub fn convert_to_openai_data_block(block: &Value, api: OpenAiApi) -> Result<Value, String> {
    let block_type = block.get("type").and_then(|v| v.as_str()).unwrap_or("");

    match block_type {
        "image" => {
            let chat_completions_block = convert_to_openai_image_block(block)?;

            if api == OpenAiApi::Responses {
                let mut formatted_block = json!({
                    "type": "input_image",
                    "image_url": chat_completions_block["image_url"]["url"]
                });

                if let Some(detail) = chat_completions_block
                    .get("image_url")
                    .and_then(|v| v.get("detail"))
                {
                    formatted_block["detail"] = detail.clone();
                }

                Ok(formatted_block)
            } else {
                Ok(chat_completions_block)
            }
        }

        "file" => {
            let is_base64 = block.get("source_type").and_then(|v| v.as_str()) == Some("base64")
                || block.get("base64").is_some();

            if is_base64 {
                let base64_data = if block.get("source_type").is_some() {
                    block.get("data").and_then(|v| v.as_str()).unwrap_or("")
                } else {
                    block.get("base64").and_then(|v| v.as_str()).unwrap_or("")
                };

                let mime_type = block
                    .get("mime_type")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");

                let mut file = json!({
                    "file_data": format!("data:{};base64,{}", mime_type, base64_data)
                });

                if let Some(filename) = block.get("filename").and_then(|v| v.as_str()) {
                    file["filename"] = json!(filename);
                } else if let Some(extras) = block.get("extras").and_then(|v| v.as_object()) {
                    if let Some(filename) = extras.get("filename").and_then(|v| v.as_str()) {
                        file["filename"] = json!(filename);
                    }
                } else if let Some(metadata) = block.get("metadata").and_then(|v| v.as_object()) {
                    if let Some(filename) = metadata.get("filename").and_then(|v| v.as_str()) {
                        file["filename"] = json!(filename);
                    }
                } else {
                    warn!(
                        "OpenAI may require a filename for file uploads. Specify a filename \
                         in the content block, e.g.: {{'type': 'file', 'mime_type': \
                         '...', 'base64': '...', 'filename': 'my-file.pdf'}}"
                    );
                }

                let formatted_block = json!({"type": "file", "file": file});

                if api == OpenAiApi::Responses {
                    let mut response_block = json!({"type": "input_file"});
                    if let Some(file_obj) = formatted_block.get("file").and_then(|v| v.as_object())
                    {
                        for (key, value) in file_obj {
                            response_block[key] = value.clone();
                        }
                    }
                    Ok(response_block)
                } else {
                    Ok(formatted_block)
                }
            } else if block.get("source_type").and_then(|v| v.as_str()) == Some("id")
                || block.get("file_id").is_some()
            {
                let file_id = if block.get("source_type").is_some() {
                    block.get("id").and_then(|v| v.as_str()).unwrap_or("")
                } else {
                    block.get("file_id").and_then(|v| v.as_str()).unwrap_or("")
                };

                let formatted_block = json!({
                    "type": "file",
                    "file": {"file_id": file_id}
                });

                if api == OpenAiApi::Responses {
                    Ok(json!({
                        "type": "input_file",
                        "file_id": file_id
                    }))
                } else {
                    Ok(formatted_block)
                }
            } else if block.get("url").is_some() {
                if api == OpenAiApi::ChatCompletions {
                    return Err("OpenAI Chat Completions does not support file URLs.".to_string());
                }
                let url = block.get("url").and_then(|v| v.as_str()).unwrap_or("");
                Ok(json!({
                    "type": "input_file",
                    "file_url": url
                }))
            } else {
                Err("Keys base64, url, or file_id required for file blocks.".to_string())
            }
        }

        "audio" => {
            let is_base64 = block.get("base64").is_some()
                || block.get("source_type").and_then(|v| v.as_str()) == Some("base64");

            if is_base64 {
                let base64_data = if block.get("source_type").is_some() {
                    block.get("data").and_then(|v| v.as_str()).unwrap_or("")
                } else {
                    block.get("base64").and_then(|v| v.as_str()).unwrap_or("")
                };

                let mime_type = block
                    .get("mime_type")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let audio_format = mime_type.split('/').next_back().unwrap_or("");

                Ok(json!({
                    "type": "input_audio",
                    "input_audio": {
                        "data": base64_data,
                        "format": audio_format
                    }
                }))
            } else {
                Err("Key base64 is required for audio blocks.".to_string())
            }
        }

        _ => Err(format!("Block of type {} is not supported.", block_type)),
    }
}

/// Extract unknown keys from block to preserve as extras.
fn extract_extras(block: &Value, known_keys: &HashSet<&str>) -> Value {
    let mut extras = json!({});
    if let Some(obj) = block.as_object() {
        for (key, value) in obj {
            if !known_keys.contains(key.as_str()) {
                extras[key] = value.clone();
            }
        }
    }
    extras
}

/// Convert OpenAI image/audio/file content block to respective v1 multimodal block.
///
/// We expect that the incoming block is verified to be in OpenAI Chat Completions format.
/// If parsing fails, passes block through unchanged.
pub fn convert_openai_format_to_data_block(block: &Value) -> Value {
    let block_type = block.get("type").and_then(|v| v.as_str()).unwrap_or("");

    if block_type == "image_url"
        && let Some(image_url) = block.get("image_url").and_then(|v| v.as_object())
        && let Some(url) = image_url.get("url").and_then(|v| v.as_str())
    {
        if let Some(parsed) = parse_data_uri(url) {
            let known_keys: HashSet<&str> = ["type", "image_url"].iter().copied().collect();
            let extras = extract_extras(block, &known_keys);

            let image_url_known_keys: HashSet<&str> = ["url"].iter().copied().collect();
            let image_url_extras = extract_extras(&json!(image_url), &image_url_known_keys);

            let mut all_extras = extras.as_object().cloned().unwrap_or_default();
            if let Some(image_url_obj) = image_url_extras.as_object() {
                for (key, value) in image_url_obj {
                    if key == "detail" {
                        all_extras.insert("detail".to_string(), value.clone());
                    } else {
                        all_extras.insert(format!("image_url_{}", key), value.clone());
                    }
                }
            }

            let mut result = json!({
                "type": "image",
                "base64": parsed.data,
                "mime_type": parsed.mime_type
            });

            if !all_extras.is_empty() {
                result["extras"] = json!(all_extras);
            }

            return result;
        } else {
            let known_keys: HashSet<&str> = ["type", "image_url"].iter().copied().collect();
            let extras = extract_extras(block, &known_keys);

            let image_url_known_keys: HashSet<&str> = ["url"].iter().copied().collect();
            let image_url_extras = extract_extras(&json!(image_url), &image_url_known_keys);

            let mut all_extras = extras.as_object().cloned().unwrap_or_default();
            if let Some(image_url_obj) = image_url_extras.as_object() {
                for (key, value) in image_url_obj {
                    if key == "detail" {
                        all_extras.insert("detail".to_string(), value.clone());
                    } else {
                        all_extras.insert(format!("image_url_{}", key), value.clone());
                    }
                }
            }

            let mut result = json!({
                "type": "image",
                "url": url
            });

            if !all_extras.is_empty() {
                result["extras"] = json!(all_extras);
            }

            return result;
        }
    }

    if block_type == "input_audio"
        && let Some(input_audio) = block.get("input_audio").and_then(|v| v.as_object())
    {
        let known_keys: HashSet<&str> = ["type", "input_audio"].iter().copied().collect();
        let extras = extract_extras(block, &known_keys);

        let audio_known_keys: HashSet<&str> = ["data", "format"].iter().copied().collect();
        let audio_extras = extract_extras(&json!(input_audio), &audio_known_keys);

        let mut all_extras = extras.as_object().cloned().unwrap_or_default();
        if let Some(audio_obj) = audio_extras.as_object() {
            for (key, value) in audio_obj {
                all_extras.insert(format!("audio_{}", key), value.clone());
            }
        }

        let data = input_audio
            .get("data")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let format = input_audio
            .get("format")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        let mut result = json!({
            "type": "audio",
            "base64": data,
            "mime_type": format!("audio/{}", format)
        });

        if !all_extras.is_empty() {
            result["extras"] = json!(all_extras);
        }

        return result;
    }

    if block_type == "file"
        && let Some(file) = block.get("file").and_then(|v| v.as_object())
    {
        if file.get("file_id").is_some() {
            let known_keys: HashSet<&str> = ["type", "file"].iter().copied().collect();
            let extras = extract_extras(block, &known_keys);

            let file_known_keys: HashSet<&str> = ["file_id"].iter().copied().collect();
            let file_extras = extract_extras(&json!(file), &file_known_keys);

            let mut all_extras = extras.as_object().cloned().unwrap_or_default();
            if let Some(file_obj) = file_extras.as_object() {
                for (key, value) in file_obj {
                    all_extras.insert(format!("file_{}", key), value.clone());
                }
            }

            let file_id = file.get("file_id").and_then(|v| v.as_str()).unwrap_or("");

            let mut result = json!({
                "type": "file",
                "file_id": file_id
            });

            if !all_extras.is_empty() {
                result["extras"] = json!(all_extras);
            }

            return result;
        }

        if let Some(file_data) = file.get("file_data").and_then(|v| v.as_str())
            && let Some(parsed) = parse_data_uri(file_data)
        {
            let known_keys: HashSet<&str> = ["type", "file"].iter().copied().collect();
            let extras = extract_extras(block, &known_keys);

            let file_known_keys: HashSet<&str> =
                ["file_data", "filename"].iter().copied().collect();
            let file_extras = extract_extras(&json!(file), &file_known_keys);

            let mut all_extras = extras.as_object().cloned().unwrap_or_default();
            if let Some(file_obj) = file_extras.as_object() {
                for (key, value) in file_obj {
                    all_extras.insert(format!("file_{}", key), value.clone());
                }
            }

            if let Some(filename) = file.get("filename") {
                all_extras.insert("filename".to_string(), filename.clone());
            }

            let mut result = json!({
                "type": "file",
                "base64": parsed.data,
                "mime_type": parsed.mime_type,
            });

            if !all_extras.is_empty() {
                result["extras"] = json!(all_extras);
            }

            return result;
        }
    }

    block.clone()
}

/// Convert annotation to v1 format.
fn convert_annotation_to_v1(annotation: &Value) -> Value {
    let annotation_type = annotation.get("type").and_then(|v| v.as_str());

    if annotation_type == Some("url_citation") {
        let known_fields: HashSet<&str> = [
            "type",
            "url",
            "title",
            "cited_text",
            "start_index",
            "end_index",
        ]
        .iter()
        .copied()
        .collect();

        let mut url_citation = json!({"type": "citation"});

        for field in ["end_index", "start_index", "title"] {
            if let Some(value) = annotation.get(field) {
                url_citation[field] = value.clone();
            }
        }

        if let Some(url) = annotation.get("url") {
            url_citation["url"] = url.clone();
        }

        if let Some(obj) = annotation.as_object() {
            for (field, value) in obj {
                if !known_fields.contains(field.as_str()) {
                    if url_citation.get("extras").is_none() {
                        url_citation["extras"] = json!({});
                    }
                    url_citation["extras"][field] = value.clone();
                }
            }
        }

        return url_citation;
    }

    if annotation_type == Some("file_citation") {
        let known_fields: HashSet<&str> = [
            "type",
            "title",
            "cited_text",
            "start_index",
            "end_index",
            "filename",
        ]
        .iter()
        .copied()
        .collect();

        let mut document_citation = json!({"type": "citation"});

        if let Some(filename) = annotation.get("filename") {
            document_citation["title"] = filename.clone();
        }

        if let Some(obj) = annotation.as_object() {
            for (field, value) in obj {
                if !known_fields.contains(field.as_str()) {
                    if document_citation.get("extras").is_none() {
                        document_citation["extras"] = json!({});
                    }
                    document_citation["extras"][field] = value.clone();
                }
            }
        }

        return document_citation;
    }

    json!({
        "type": "non_standard_annotation",
        "value": annotation.clone()
    })
}

/// Explode reasoning blocks with summary into individual blocks.
fn explode_reasoning(block: &Value) -> Vec<Value> {
    if block.get("summary").is_none() {
        return vec![block.clone()];
    }

    let known_fields: HashSet<&str> = ["type", "reasoning", "id", "index"]
        .iter()
        .copied()
        .collect();

    let mut block = block.clone();

    let unknown_fields: Vec<String> = block
        .as_object()
        .map(|obj| {
            obj.keys()
                .filter(|k| *k != "summary" && !known_fields.contains(k.as_str()))
                .cloned()
                .collect()
        })
        .unwrap_or_default();

    if !unknown_fields.is_empty() {
        block["extras"] = json!({});
        for field in &unknown_fields {
            if let Some(value) = block.get(field).cloned() {
                block["extras"][field] = value;
            }
        }
        if let Some(obj) = block.as_object_mut() {
            for field in &unknown_fields {
                obj.remove(field);
            }
        }
    }

    let summary_clone = block.get("summary").and_then(|v| v.as_array()).cloned();

    let summary = match summary_clone {
        Some(ref s) if !s.is_empty() => s,
        _ => {
            let mut result = json!({});
            if let Some(obj) = block.as_object() {
                for (k, v) in obj {
                    if k != "summary" {
                        result[k] = v.clone();
                    }
                }
            }

            if let Some(index) = result.get("index").and_then(|v| v.as_i64()) {
                let meaningful_idx = format!("{}_0", index);
                result["index"] = json!(format!("lc_rs_{}", hex_encode(meaningful_idx.as_bytes())));
            }

            return vec![result];
        }
    };

    let mut common = json!({});
    if let Some(obj) = block.as_object() {
        for (k, v) in obj {
            if known_fields.contains(k.as_str()) {
                common[k] = v.clone();
            }
        }
    }

    let first_only = block.get("extras").cloned();

    let mut results = Vec::new();

    for (idx, part) in summary.iter().enumerate() {
        let mut new_block = common.clone();
        new_block["reasoning"] = json!(part.get("text").and_then(|v| v.as_str()).unwrap_or(""));

        if idx == 0
            && let Some(ref extras) = first_only
            && let Some(extras_obj) = extras.as_object()
        {
            for (k, v) in extras_obj {
                new_block[k] = v.clone();
            }
        }

        if let Some(block_index) = new_block.get("index").and_then(|v| v.as_i64()) {
            let summary_index = part.get("index").and_then(|v| v.as_i64()).unwrap_or(0);
            let meaningful_idx = format!("{}_{}", block_index, summary_index);
            new_block["index"] = json!(format!("lc_rs_{}", hex_encode(meaningful_idx.as_bytes())));
        }

        results.push(new_block);
    }

    results
}

/// Context for OpenAI message translation.
#[derive(Default)]
pub struct OpenAiContext {
    /// Tool calls from the message.
    pub tool_calls: Vec<Value>,
    /// Tool call chunks from the message (for streaming).
    pub tool_call_chunks: Vec<Value>,
    /// Invalid tool calls from the message.
    pub invalid_tool_calls: Vec<Value>,
    /// Additional kwargs from the message.
    pub additional_kwargs: Value,
    /// Response metadata from the message.
    pub response_metadata: Value,
    /// Message ID.
    pub message_id: Option<String>,
    /// Chunk position (for streaming).
    pub chunk_position: Option<String>,
}

/// Convert OpenAI content blocks to standard format.
///
/// # Arguments
/// * `content` - The raw content blocks from OpenAI
/// * `is_chunk` - Whether this is a streaming chunk
/// * `context` - Optional context containing tool_calls and other message data
///
/// # Returns
/// A vector of standardized content blocks.
pub fn convert_to_standard_blocks(content: &[Value], is_chunk: bool) -> Vec<Value> {
    convert_to_standard_blocks_with_context(content, is_chunk, None)
}

/// Convert OpenAI content blocks to standard format with additional context.
///
/// This is the main entry point for converting OpenAI content blocks to v1 format.
/// It handles:
/// - v0.3 backwards compatibility (reasoning in additional_kwargs, tool_outputs, refusal)
/// - OpenAI Responses API format (reasoning with summary, function_call, etc.)
/// - OpenAI Chat Completions format (plain string content with tool_calls)
///
/// # Arguments
/// * `content` - The raw content blocks from OpenAI
/// * `is_chunk` - Whether this is a streaming chunk
/// * `context` - Optional context containing tool_calls and other message data
///
/// # Returns
/// A vector of standardized content blocks.
pub fn convert_to_standard_blocks_with_context(
    content: &[Value],
    is_chunk: bool,
    context: Option<&OpenAiContext>,
) -> Vec<Value> {
    let processed_content = if let Some(ctx) = context
        && is_v03_format(content, ctx)
    {
        convert_from_v03_format(content, ctx, is_chunk)
    } else {
        content.to_vec()
    };

    convert_to_v1_from_responses(&processed_content, is_chunk, context)
}

/// Check if this is a v0.3 format message.
///
/// v0.3 messages have one or more of these characteristics:
/// - `reasoning` in `additional_kwargs`
/// - `tool_outputs` in `additional_kwargs`
/// - `refusal` in `additional_kwargs`
/// - `__openai_function_call_ids__` in `additional_kwargs`
/// - Message ID starts with "msg_" and response ID starts with "resp_"
fn is_v03_format(content: &[Value], context: &OpenAiContext) -> bool {
    let has_v03_kwargs = [
        "reasoning",
        "tool_outputs",
        "refusal",
        FUNCTION_CALL_IDS_MAP_KEY,
    ]
    .iter()
    .any(|key| context.additional_kwargs.get(key).is_some());

    if has_v03_kwargs {
        return true;
    }

    if let Some(msg_id) = &context.message_id
        && msg_id.starts_with("msg_")
        && let Some(resp_id) = context.response_metadata.get("id").and_then(|v| v.as_str())
        && resp_id.starts_with("resp_")
    {
        let all_dicts = content.iter().all(|v| v.is_object());
        if all_dicts {
            return true;
        }
    }

    false
}

/// Convert v0.3 format message to Responses format.
///
/// This handles backwards compatibility with v0.3 messages which had:
/// - `reasoning` in `additional_kwargs`
/// - `tool_outputs` in `additional_kwargs`
/// - `refusal` in `additional_kwargs`
/// - `__openai_function_call_ids__` mapping in `additional_kwargs`
fn convert_from_v03_format(
    content: &[Value],
    context: &OpenAiContext,
    is_chunk: bool,
) -> Vec<Value> {
    let content_order = vec![
        "reasoning",
        "code_interpreter_call",
        "mcp_call",
        "image_generation_call",
        "text",
        "refusal",
        "function_call",
        "computer_call",
        "mcp_list_tools",
        "mcp_approval_request",
    ];

    let mut buckets: HashMap<&str, Vec<Value>> =
        content_order.iter().map(|k| (*k, Vec::new())).collect();
    let mut unknown_blocks = Vec::new();

    if let Some(reasoning) = context.additional_kwargs.get("reasoning") {
        if is_chunk && context.chunk_position.as_deref() != Some("last") {
            let mut reasoning_with_type = reasoning.clone();
            if reasoning_with_type.is_object() {
                reasoning_with_type["type"] = json!("reasoning");
            }
            buckets
                .entry("reasoning")
                .or_default()
                .push(reasoning_with_type);
        } else {
            buckets
                .entry("reasoning")
                .or_default()
                .push(reasoning.clone());
        }
    }

    if let Some(refusal) = context.additional_kwargs.get("refusal") {
        buckets.entry("refusal").or_default().push(json!({
            "type": "refusal",
            "refusal": refusal
        }));
    }

    for block in content {
        if let Some(obj) = block.as_object()
            && obj.get("type").and_then(|t| t.as_str()) == Some("text")
        {
            let mut block_copy = block.clone();
            if let Some(id) = &context.message_id
                && id.starts_with("msg_")
            {
                block_copy["id"] = json!(id);
            }
            buckets.entry("text").or_default().push(block_copy);
        } else {
            unknown_blocks.push(block.clone());
        }
    }

    let function_call_ids = context
        .additional_kwargs
        .get(FUNCTION_CALL_IDS_MAP_KEY)
        .and_then(|v| v.as_object());

    if is_chunk
        && context.tool_call_chunks.len() == 1
        && context.chunk_position.as_deref() != Some("last")
    {
        if let Some(tool_call_chunk) = context.tool_call_chunks.first() {
            let mut function_call = json!({
                "type": "function_call",
                "name": tool_call_chunk.get("name"),
                "arguments": tool_call_chunk.get("args"),
                "call_id": tool_call_chunk.get("id"),
            });

            if let Some(ids) = function_call_ids
                && let Some(call_id) = tool_call_chunk.get("id").and_then(|v| v.as_str())
                && let Some(id) = ids.get(call_id)
            {
                function_call["id"] = id.clone();
            }

            buckets
                .entry("function_call")
                .or_default()
                .push(function_call);
        }
    } else {
        for tool_call in &context.tool_calls {
            let arguments = if let Some(args) = tool_call.get("args") {
                serde_json::to_string(args).unwrap_or_else(|_| "{}".to_string())
            } else {
                "{}".to_string()
            };

            let mut function_call = json!({
                "type": "function_call",
                "name": tool_call.get("name"),
                "arguments": arguments,
                "call_id": tool_call.get("id"),
            });

            if let Some(ids) = function_call_ids
                && let Some(call_id) = tool_call.get("id").and_then(|v| v.as_str())
                && let Some(id) = ids.get(call_id)
            {
                function_call["id"] = id.clone();
            }

            buckets
                .entry("function_call")
                .or_default()
                .push(function_call);
        }
    }

    if let Some(tool_outputs) = context.additional_kwargs.get("tool_outputs")
        && let Some(outputs_array) = tool_outputs.as_array()
    {
        for block in outputs_array {
            if let Some(obj) = block.as_object()
                && let Some(key) = obj.get("type").and_then(|t| t.as_str())
                && buckets.contains_key(key)
            {
                buckets.entry(key).or_default().push(block.clone());
            } else {
                unknown_blocks.push(block.clone());
            }
        }
    }

    let mut new_content = Vec::new();
    for key in content_order {
        new_content.extend(buckets.remove(key).unwrap_or_default());
    }
    new_content.extend(unknown_blocks);

    new_content
}

/// Convert OpenAI Responses API blocks to v1 format.
///
/// This handles the main conversion logic for:
/// - text blocks (with annotations)
/// - reasoning blocks (with summary explosion)
/// - function_call -> tool_call
/// - web_search_call, file_search_call, code_interpreter_call, mcp_call, etc.
///
fn convert_to_v1_from_responses(
    content: &[Value],
    is_chunk: bool,
    context: Option<&OpenAiContext>,
) -> Vec<Value> {
    let mut result = Vec::new();

    for raw_block in content {
        if !raw_block.is_object() {
            continue;
        }

        let block = raw_block.clone();
        let block_type = block.get("type").and_then(|v| v.as_str()).unwrap_or("");

        match block_type {
            "text" => {
                let mut text_block = block.clone();
                if text_block.get("text").is_none() {
                    text_block["text"] = json!("");
                }

                if let Some(annotations) = block.get("annotations").and_then(|v| v.as_array()) {
                    let converted: Vec<Value> =
                        annotations.iter().map(convert_annotation_to_v1).collect();
                    text_block["annotations"] = json!(converted);
                }

                if let Some(index) = block.get("index").and_then(|v| v.as_i64()) {
                    text_block["index"] = json!(format!("lc_txt_{}", index));
                }

                result.push(text_block);
            }

            "reasoning" => {
                let exploded = explode_reasoning(&block);
                result.extend(exploded);
            }

            "image_generation_call" => {
                if let Some(image_result) = block.get("result").and_then(|v| v.as_str()) {
                    let mut new_block = json!({
                        "type": "image",
                        "base64": image_result
                    });

                    if let Some(output_format) = block.get("output_format").and_then(|v| v.as_str())
                    {
                        new_block["mime_type"] = json!(format!("image/{}", output_format));
                    }

                    if let Some(id) = block.get("id") {
                        new_block["id"] = id.clone();
                    }

                    if let Some(index) = block.get("index").and_then(|v| v.as_i64()) {
                        new_block["index"] = json!(format!("lc_img_{}", index));
                    }

                    for extra_key in [
                        "status",
                        "background",
                        "output_format",
                        "quality",
                        "revised_prompt",
                        "size",
                    ] {
                        if let Some(value) = block.get(extra_key) {
                            if new_block.get("extras").is_none() {
                                new_block["extras"] = json!({});
                            }
                            new_block["extras"][extra_key] = value.clone();
                        }
                    }

                    result.push(new_block);
                }
            }

            "function_call" => {
                let call_id = block.get("call_id").and_then(|v| v.as_str()).unwrap_or("");

                let tool_call_block: Option<Value> = if is_chunk
                    && context
                        .map(|c| c.tool_call_chunks.len() == 1)
                        .unwrap_or(false)
                    && context
                        .map(|c| c.chunk_position.as_deref() != Some("last"))
                        .unwrap_or(true)
                    && let Some(ctx) = context
                {
                    let chunk = &ctx.tool_call_chunks[0];
                    let mut tc = chunk.clone();
                    tc["type"] = json!("tool_call_chunk");
                    Some(tc)
                } else if !call_id.is_empty() {
                    let mut found = None;

                    if let Some(ctx) = context {
                        for tool_call in &ctx.tool_calls {
                            if tool_call.get("id").and_then(|v| v.as_str()) == Some(call_id) {
                                found = Some(json!({
                                    "type": "tool_call",
                                    "name": tool_call.get("name").cloned().unwrap_or(json!("")),
                                    "args": tool_call.get("args").cloned().unwrap_or(json!({})),
                                    "id": tool_call.get("id").cloned()
                                }));
                                break;
                            }
                        }

                        if found.is_none() {
                            for invalid_tool_call in &ctx.invalid_tool_calls {
                                if invalid_tool_call.get("id").and_then(|v| v.as_str())
                                    == Some(call_id)
                                {
                                    found = Some(invalid_tool_call.clone());
                                    break;
                                }
                            }
                        }
                    }

                    found
                } else {
                    None
                };

                if let Some(mut tc) = tool_call_block {
                    if let Some(id) = block.get("id") {
                        if tc.get("extras").is_none() {
                            tc["extras"] = json!({});
                        }
                        tc["extras"]["item_id"] = id.clone();
                    }

                    if let Some(index) = block.get("index").and_then(|v| v.as_i64()) {
                        tc["index"] = json!(format!("lc_tc_{}", index));
                    }

                    result.push(tc);
                }
            }

            "web_search_call" => {
                let id = block.get("id").and_then(|v| v.as_str()).unwrap_or("");

                let mut web_search_call = json!({
                    "type": "server_tool_call",
                    "name": "web_search",
                    "args": {},
                    "id": id
                });

                if let Some(index) = block.get("index").and_then(|v| v.as_i64()) {
                    web_search_call["index"] = json!(format!("lc_wsc_{}", index));
                }

                let mut sources: Option<Value> = None;

                if let Some(action) = block.get("action").and_then(|v| v.as_object()) {
                    if let Some(s) = action.get("sources") {
                        sources = Some(s.clone());
                    }

                    let mut args = json!({});
                    for (k, v) in action {
                        if k != "sources" {
                            args[k] = v.clone();
                        }
                    }
                    web_search_call["args"] = args;
                }

                if let Some(obj) = block.as_object() {
                    for (key, value) in obj {
                        if !["type", "id", "action", "status", "index"].contains(&key.as_str()) {
                            web_search_call[key] = value.clone();
                        }
                    }
                }

                result.push(web_search_call);

                let has_web_search_result = content.iter().any(|other_block| {
                    other_block.get("type").and_then(|v| v.as_str()) == Some("web_search_result")
                        && other_block.get("id").and_then(|v| v.as_str()) == Some(id)
                });

                if !has_web_search_result {
                    let mut web_search_result = json!({
                        "type": "server_tool_result",
                        "tool_call_id": id
                    });

                    if let Some(s) = sources {
                        web_search_result["output"] = json!({"sources": s});
                    }

                    let status = block.get("status").and_then(|v| v.as_str());
                    match status {
                        Some("failed") => {
                            web_search_result["status"] = json!("error");
                        }
                        Some("completed") => {
                            web_search_result["status"] = json!("success");
                        }
                        Some(s) => {
                            web_search_result["extras"] = json!({"status": s});
                        }
                        None => {}
                    }

                    if let Some(index) = block.get("index").and_then(|v| v.as_i64()) {
                        web_search_result["index"] = json!(format!("lc_wsr_{}", index + 1));
                    }

                    result.push(web_search_result);
                }
            }

            "file_search_call" => {
                let id = block.get("id").and_then(|v| v.as_str()).unwrap_or("");

                let mut file_search_call = json!({
                    "type": "server_tool_call",
                    "name": "file_search",
                    "id": id,
                    "args": {
                        "queries": block.get("queries").cloned().unwrap_or(json!([]))
                    }
                });

                if let Some(index) = block.get("index").and_then(|v| v.as_i64()) {
                    file_search_call["index"] = json!(format!("lc_fsc_{}", index));
                }

                if let Some(obj) = block.as_object() {
                    for (key, value) in obj {
                        if !["type", "id", "queries", "results", "status", "index"]
                            .contains(&key.as_str())
                        {
                            file_search_call[key] = value.clone();
                        }
                    }
                }

                result.push(file_search_call);

                let mut file_search_result = json!({
                    "type": "server_tool_result",
                    "tool_call_id": id
                });

                if let Some(output) = block.get("results") {
                    file_search_result["output"] = output.clone();
                }

                let status = block.get("status").and_then(|v| v.as_str());
                match status {
                    Some("failed") => {
                        file_search_result["status"] = json!("error");
                    }
                    Some("completed") => {
                        file_search_result["status"] = json!("success");
                    }
                    Some(s) => {
                        file_search_result["extras"] = json!({"status": s});
                    }
                    None => {}
                }

                if let Some(index) = block.get("index").and_then(|v| v.as_i64()) {
                    file_search_result["index"] = json!(format!("lc_fsr_{}", index + 1));
                }

                result.push(file_search_result);
            }

            "code_interpreter_call" => {
                let id = block.get("id").and_then(|v| v.as_str()).unwrap_or("");

                let mut code_interpreter_call = json!({
                    "type": "server_tool_call",
                    "name": "code_interpreter",
                    "id": id
                });

                if let Some(code) = block.get("code") {
                    code_interpreter_call["args"] = json!({"code": code});
                }

                if let Some(index) = block.get("index").and_then(|v| v.as_i64()) {
                    code_interpreter_call["index"] = json!(format!("lc_cic_{}", index));
                }

                let known_fields: HashSet<&str> =
                    ["type", "id", "outputs", "status", "code", "extras", "index"]
                        .iter()
                        .copied()
                        .collect();

                if let Some(obj) = block.as_object() {
                    for (key, value) in obj {
                        if !known_fields.contains(key.as_str()) {
                            if code_interpreter_call.get("extras").is_none() {
                                code_interpreter_call["extras"] = json!({});
                            }
                            code_interpreter_call["extras"][key] = value.clone();
                        }
                    }
                }

                let mut code_interpreter_result = json!({
                    "type": "server_tool_result",
                    "tool_call_id": id
                });

                if let Some(outputs) = block.get("outputs") {
                    code_interpreter_result["output"] = outputs.clone();
                }

                let status = block.get("status").and_then(|v| v.as_str());
                match status {
                    Some("failed") => {
                        code_interpreter_result["status"] = json!("error");
                    }
                    Some("completed") => {
                        code_interpreter_result["status"] = json!("success");
                    }
                    Some(s) => {
                        code_interpreter_result["extras"] = json!({"status": s});
                    }
                    None => {}
                }

                if let Some(index) = block.get("index").and_then(|v| v.as_i64()) {
                    code_interpreter_result["index"] = json!(format!("lc_cir_{}", index + 1));
                }

                result.push(code_interpreter_call);
                result.push(code_interpreter_result);
            }

            "mcp_call" => {
                let id = block.get("id").and_then(|v| v.as_str()).unwrap_or("");

                let mut mcp_call = json!({
                    "type": "server_tool_call",
                    "name": "remote_mcp",
                    "id": id
                });

                if let Some(arguments) = block.get("arguments").and_then(|v| v.as_str()) {
                    if let Ok(parsed) = serde_json::from_str::<Value>(arguments) {
                        mcp_call["args"] = parsed;
                    } else {
                        mcp_call["extras"] = json!({"arguments": arguments});
                    }
                }

                if let Some(name) = block.get("name") {
                    if mcp_call.get("extras").is_none() {
                        mcp_call["extras"] = json!({});
                    }
                    mcp_call["extras"]["tool_name"] = name.clone();
                }

                if let Some(server_label) = block.get("server_label") {
                    if mcp_call.get("extras").is_none() {
                        mcp_call["extras"] = json!({});
                    }
                    mcp_call["extras"]["server_label"] = server_label.clone();
                }

                if let Some(index) = block.get("index").and_then(|v| v.as_i64()) {
                    mcp_call["index"] = json!(format!("lc_mcp_{}", index));
                }

                let known_fields: HashSet<&str> = [
                    "type",
                    "id",
                    "arguments",
                    "name",
                    "server_label",
                    "output",
                    "error",
                    "extras",
                    "index",
                ]
                .iter()
                .copied()
                .collect();

                if let Some(obj) = block.as_object() {
                    for (key, value) in obj {
                        if !known_fields.contains(key.as_str()) {
                            if mcp_call.get("extras").is_none() {
                                mcp_call["extras"] = json!({});
                            }
                            mcp_call["extras"][key] = value.clone();
                        }
                    }
                }

                result.push(mcp_call);

                let mut mcp_result = json!({
                    "type": "server_tool_result",
                    "tool_call_id": id
                });

                if let Some(output) = block.get("output") {
                    mcp_result["output"] = output.clone();
                }

                if let Some(error) = block.get("error") {
                    if mcp_result.get("extras").is_none() {
                        mcp_result["extras"] = json!({});
                    }
                    mcp_result["extras"]["error"] = error.clone();
                    mcp_result["status"] = json!("error");
                } else {
                    mcp_result["status"] = json!("success");
                }

                if let Some(index) = block.get("index").and_then(|v| v.as_i64()) {
                    mcp_result["index"] = json!(format!("lc_mcpr_{}", index + 1));
                }

                result.push(mcp_result);
            }

            "mcp_list_tools" => {
                let id = block.get("id").and_then(|v| v.as_str()).unwrap_or("");

                let mut mcp_list_tools_call = json!({
                    "type": "server_tool_call",
                    "name": "mcp_list_tools",
                    "args": {},
                    "id": id
                });

                if let Some(server_label) = block.get("server_label") {
                    mcp_list_tools_call["extras"] = json!({});
                    mcp_list_tools_call["extras"]["server_label"] = server_label.clone();
                }

                if let Some(index) = block.get("index").and_then(|v| v.as_i64()) {
                    mcp_list_tools_call["index"] = json!(format!("lc_mlt_{}", index));
                }

                let known_fields: HashSet<&str> = [
                    "type",
                    "id",
                    "name",
                    "server_label",
                    "tools",
                    "error",
                    "extras",
                    "index",
                ]
                .iter()
                .copied()
                .collect();

                if let Some(obj) = block.as_object() {
                    for (key, value) in obj {
                        if !known_fields.contains(key.as_str()) {
                            if mcp_list_tools_call.get("extras").is_none() {
                                mcp_list_tools_call["extras"] = json!({});
                            }
                            mcp_list_tools_call["extras"][key] = value.clone();
                        }
                    }
                }

                result.push(mcp_list_tools_call);

                let mut mcp_list_tools_result = json!({
                    "type": "server_tool_result",
                    "tool_call_id": id
                });

                if let Some(tools) = block.get("tools") {
                    mcp_list_tools_result["output"] = tools.clone();
                }

                if let Some(error) = block.get("error") {
                    if mcp_list_tools_result.get("extras").is_none() {
                        mcp_list_tools_result["extras"] = json!({});
                    }
                    mcp_list_tools_result["extras"]["error"] = error.clone();
                    mcp_list_tools_result["status"] = json!("error");
                } else {
                    mcp_list_tools_result["status"] = json!("success");
                }

                if let Some(index) = block.get("index").and_then(|v| v.as_i64()) {
                    mcp_list_tools_result["index"] = json!(format!("lc_mltr_{}", index + 1));
                }

                result.push(mcp_list_tools_result);
            }

            _ => {
                if KNOWN_BLOCK_TYPES.contains(&block_type) {
                    result.push(block);
                } else {
                    let mut new_block = json!({
                        "type": "non_standard",
                        "value": block.clone()
                    });

                    if let Some(index) =
                        new_block.get("value").and_then(|v| v.get("index")).cloned()
                    {
                        new_block["index"] = json!(format!("lc_ns_{}", index));
                        if let Some(value) = new_block.get_mut("value")
                            && let Some(obj) = value.as_object_mut()
                        {
                            obj.remove("index");
                        }
                    }

                    result.push(new_block);
                }
            }
        }
    }

    result
}

/// Convert OpenAI Chat Completions format blocks to v1 format.
///
/// During the `content_blocks` parsing process, we wrap blocks not recognized as a v1
/// block as a `'non_standard'` block with the original block stored in the `value`
/// field. This function attempts to unpack those blocks and convert any blocks that
/// might be OpenAI format to v1 ContentBlocks.
///
/// If conversion fails, the block is left as a `'non_standard'` block.
pub fn convert_to_v1_from_chat_completions_input(content: &[Value]) -> Vec<Value> {
    let mut converted_blocks = Vec::new();

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

    for block in unpacked_blocks {
        let block_type = block.get("type").and_then(|v| v.as_str()).unwrap_or("");

        if ["image_url", "input_audio", "file"].contains(&block_type)
            && is_openai_data_block(&block, None)
        {
            let converted_block = convert_openai_format_to_data_block(&block);

            let converted_type = converted_block
                .get("type")
                .and_then(|v| v.as_str())
                .unwrap_or("");

            if KNOWN_BLOCK_TYPES.contains(&converted_type) {
                converted_blocks.push(converted_block);
            } else {
                converted_blocks.push(json!({
                    "type": "non_standard",
                    "value": block
                }));
            }
        } else if KNOWN_BLOCK_TYPES.contains(&block_type) {
            converted_blocks.push(block);
        } else {
            converted_blocks.push(json!({
                "type": "non_standard",
                "value": block
            }));
        }
    }

    converted_blocks
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_convert_to_openai_image_block_url() {
        let block = json!({
            "type": "image",
            "url": "https://example.com/image.png"
        });

        let result = convert_to_openai_image_block(&block).unwrap();
        assert_eq!(result["type"], "image_url");
        assert_eq!(result["image_url"]["url"], "https://example.com/image.png");
    }

    #[test]
    fn test_convert_to_openai_image_block_base64() {
        let block = json!({
            "type": "image",
            "base64": "iVBORw0KGgo=",
            "mime_type": "image/png"
        });

        let result = convert_to_openai_image_block(&block).unwrap();
        assert_eq!(result["type"], "image_url");
        assert!(
            result["image_url"]["url"]
                .as_str()
                .unwrap()
                .starts_with("data:image/png;base64,")
        );
    }

    #[test]
    fn test_convert_to_openai_image_block_missing_mime_type() {
        let block = json!({
            "type": "image",
            "base64": "iVBORw0KGgo="
        });

        let result = convert_to_openai_image_block(&block);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("mime_type"));
    }

    #[test]
    fn test_convert_to_openai_data_block_audio() {
        let block = json!({
            "type": "audio",
            "base64": "audio_data",
            "mime_type": "audio/wav"
        });

        let result = convert_to_openai_data_block(&block, OpenAiApi::ChatCompletions).unwrap();
        assert_eq!(result["type"], "input_audio");
        assert_eq!(result["input_audio"]["data"], "audio_data");
        assert_eq!(result["input_audio"]["format"], "wav");
    }

    #[test]
    fn test_convert_to_openai_data_block_file_base64() {
        let block = json!({
            "type": "file",
            "base64": "file_data",
            "mime_type": "application/pdf",
            "filename": "test.pdf"
        });

        let result = convert_to_openai_data_block(&block, OpenAiApi::ChatCompletions).unwrap();
        assert_eq!(result["type"], "file");
        assert!(
            result["file"]["file_data"]
                .as_str()
                .unwrap()
                .contains("base64")
        );
        assert_eq!(result["file"]["filename"], "test.pdf");
    }

    #[test]
    fn test_convert_to_openai_data_block_file_id() {
        let block = json!({
            "type": "file",
            "file_id": "file-123"
        });

        let result = convert_to_openai_data_block(&block, OpenAiApi::ChatCompletions).unwrap();
        assert_eq!(result["type"], "file");
        assert_eq!(result["file"]["file_id"], "file-123");
    }

    #[test]
    fn test_convert_openai_format_to_data_block_image_url() {
        let block = json!({
            "type": "image_url",
            "image_url": {
                "url": "https://example.com/image.png",
                "detail": "high"
            }
        });

        let result = convert_openai_format_to_data_block(&block);
        assert_eq!(result["type"], "image");
        assert_eq!(result["url"], "https://example.com/image.png");
        assert_eq!(result["extras"]["detail"], "high");
    }

    #[test]
    fn test_convert_openai_format_to_data_block_image_base64() {
        let block = json!({
            "type": "image_url",
            "image_url": {
                "url": "data:image/png;base64,iVBORw0KGgo="
            }
        });

        let result = convert_openai_format_to_data_block(&block);
        assert_eq!(result["type"], "image");
        assert_eq!(result["base64"], "iVBORw0KGgo=");
        assert_eq!(result["mime_type"], "image/png");
    }

    #[test]
    fn test_convert_openai_format_to_data_block_audio() {
        let block = json!({
            "type": "input_audio",
            "input_audio": {
                "data": "audio_data",
                "format": "wav"
            }
        });

        let result = convert_openai_format_to_data_block(&block);
        assert_eq!(result["type"], "audio");
        assert_eq!(result["base64"], "audio_data");
        assert_eq!(result["mime_type"], "audio/wav");
    }

    #[test]
    fn test_convert_openai_format_to_data_block_file_id() {
        let block = json!({
            "type": "file",
            "file": {
                "file_id": "file-123"
            }
        });

        let result = convert_openai_format_to_data_block(&block);
        assert_eq!(result["type"], "file");
        assert_eq!(result["file_id"], "file-123");
    }

    #[test]
    fn test_convert_annotation_to_v1_url_citation() {
        let annotation = json!({
            "type": "url_citation",
            "url": "https://example.com",
            "title": "Example",
            "start_index": 0,
            "end_index": 10,
            "custom_field": "value"
        });

        let result = convert_annotation_to_v1(&annotation);
        assert_eq!(result["type"], "citation");
        assert_eq!(result["url"], "https://example.com");
        assert_eq!(result["title"], "Example");
        assert_eq!(result["extras"]["custom_field"], "value");
    }

    #[test]
    fn test_convert_annotation_to_v1_file_citation() {
        let annotation = json!({
            "type": "file_citation",
            "filename": "document.pdf"
        });

        let result = convert_annotation_to_v1(&annotation);
        assert_eq!(result["type"], "citation");
        assert_eq!(result["title"], "document.pdf");
    }

    #[test]
    fn test_convert_annotation_to_v1_non_standard() {
        let annotation = json!({
            "type": "unknown_type",
            "data": "value"
        });

        let result = convert_annotation_to_v1(&annotation);
        assert_eq!(result["type"], "non_standard_annotation");
        assert_eq!(result["value"]["type"], "unknown_type");
    }

    #[test]
    fn test_convert_to_standard_blocks_text() {
        let content = vec![json!({
            "type": "text",
            "text": "Hello, world!",
            "index": 0
        })];

        let result = convert_to_standard_blocks(&content, false);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0]["type"], "text");
        assert_eq!(result[0]["text"], "Hello, world!");
        assert_eq!(result[0]["index"], "lc_txt_0");
    }

    #[test]
    fn test_convert_to_standard_blocks_reasoning() {
        let content = vec![json!({
            "type": "reasoning",
            "reasoning": "Thinking...",
            "id": "rs_123"
        })];

        let result = convert_to_standard_blocks(&content, false);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0]["type"], "reasoning");
        assert_eq!(result[0]["reasoning"], "Thinking...");
    }

    #[test]
    fn test_convert_to_standard_blocks_web_search_call() {
        let content = vec![json!({
            "type": "web_search_call",
            "id": "ws_123",
            "status": "completed",
            "action": {
                "query": "test query",
                "sources": [{"url": "https://example.com"}]
            },
            "index": 0
        })];

        let result = convert_to_standard_blocks(&content, false);
        assert_eq!(result.len(), 2);

        assert_eq!(result[0]["type"], "server_tool_call");
        assert_eq!(result[0]["name"], "web_search");
        assert_eq!(result[0]["id"], "ws_123");
        assert_eq!(result[0]["args"]["query"], "test query");

        assert_eq!(result[1]["type"], "server_tool_result");
        assert_eq!(result[1]["tool_call_id"], "ws_123");
        assert_eq!(result[1]["status"], "success");
    }

    #[test]
    fn test_convert_to_standard_blocks_non_standard() {
        let content = vec![json!({
            "type": "unknown_block_type",
            "data": "value",
            "index": 5
        })];

        let result = convert_to_standard_blocks(&content, false);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0]["type"], "non_standard");
        assert_eq!(result[0]["index"], "lc_ns_5");
        assert!(result[0]["value"].get("index").is_none());
    }

    #[test]
    fn test_convert_to_v1_from_chat_completions_input() {
        let content = vec![
            json!({
                "type": "image_url",
                "image_url": {
                    "url": "https://example.com/image.png"
                }
            }),
            json!({
                "type": "text",
                "text": "Hello"
            }),
        ];

        let result = convert_to_v1_from_chat_completions_input(&content);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0]["type"], "image");
        assert_eq!(result[0]["url"], "https://example.com/image.png");
        assert_eq!(result[1]["type"], "text");
    }

    #[test]
    fn test_explode_reasoning_no_summary() {
        let block = json!({
            "type": "reasoning",
            "reasoning": "Simple thought",
            "id": "rs_123"
        });

        let result = explode_reasoning(&block);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0]["reasoning"], "Simple thought");
    }

    #[test]
    fn test_explode_reasoning_with_summary() {
        let block = json!({
            "type": "reasoning",
            "id": "rs_123",
            "index": 0,
            "summary": [
                {"text": "First thought", "index": 0},
                {"text": "Second thought", "index": 1}
            ]
        });

        let result = explode_reasoning(&block);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0]["reasoning"], "First thought");
        assert_eq!(result[1]["reasoning"], "Second thought");
    }
}
