use std::collections::HashSet;
use std::fmt::Write;

use serde_json::{Value, json};

use crate::messages::content::KNOWN_BLOCK_TYPES;

pub struct OpenAiContext {
    pub tool_calls: Vec<Value>,
    pub tool_call_chunks: Vec<Value>,
    pub invalid_tool_calls: Vec<Value>,
    pub additional_kwargs: Value,
    pub response_metadata: Value,
    pub message_id: Option<String>,
    pub chunk_position: Option<String>,
}

fn to_hex(bytes: &[u8]) -> String {
    let mut hex_string = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        let _ = write!(hex_string, "{byte:02x}");
    }
    hex_string
}

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

fn extract_extras(block: &Value, known_keys: &HashSet<&str>) -> Value {
    let mut extras = json!({});
    if let Some(obj) = block.as_object() {
        for (key, value) in obj {
            if !known_keys.contains(key.as_str())
                && let Some(extras_obj) = extras.as_object_mut()
            {
                extras_obj.insert(key.clone(), value.clone());
            }
        }
    }
    extras
}

fn parse_data_uri(uri: &str) -> Option<(String, String)> {
    let stripped = uri.strip_prefix("data:")?;
    let (mime_and_encoding, data) = stripped.split_once(',')?;
    let mime_type = mime_and_encoding.strip_suffix(";base64")?;
    if mime_type.is_empty() || data.is_empty() {
        return None;
    }
    Some((mime_type.to_string(), data.to_string()))
}

fn is_openai_data_block(block: &Value) -> bool {
    let block_type = block.get("type").and_then(|v| v.as_str()).unwrap_or("");

    match block_type {
        "image_url" => {
            if let Some(image_url) = block.get("image_url").and_then(|v| v.as_object()) {
                image_url.get("url").and_then(|v| v.as_str()).is_some()
            } else {
                false
            }
        }
        "input_audio" => {
            if let Some(audio) = block.get("input_audio").and_then(|v| v.as_object()) {
                audio.get("data").and_then(|v| v.as_str()).is_some()
                    && audio.get("format").and_then(|v| v.as_str()).is_some()
            } else {
                false
            }
        }
        "file" => {
            if let Some(file) = block.get("file").and_then(|v| v.as_object()) {
                file.get("file_data").and_then(|v| v.as_str()).is_some()
                    || file.get("file_id").and_then(|v| v.as_str()).is_some()
            } else {
                false
            }
        }
        _ => false,
    }
}

fn convert_openai_format_to_data_block(block: &Value) -> Value {
    let block_type = block.get("type").and_then(|v| v.as_str()).unwrap_or("");

    if block_type == "image_url"
        && let Some(image_url) = block.get("image_url").and_then(|v| v.as_object())
        && let Some(url) = image_url.get("url").and_then(|v| v.as_str())
    {
        let top_known: HashSet<&str> = ["type", "image_url"].iter().copied().collect();
        let mut all_extras = extract_extras(block, &top_known);

        let url_known: HashSet<&str> = ["url"].iter().copied().collect();
        let url_extras = extract_extras(&json!(image_url), &url_known);
        if let Some(url_extras_obj) = url_extras.as_object() {
            for (key, value) in url_extras_obj {
                let prefixed_key = if key == "detail" {
                    "detail".to_string()
                } else {
                    format!("image_url_{key}")
                };
                if let Some(all_obj) = all_extras.as_object_mut() {
                    all_obj.insert(prefixed_key, value.clone());
                }
            }
        }

        if let Some((mime_type, data)) = parse_data_uri(url) {
            let mut image_block = json!({
                "type": "image",
                "base64": data,
                "mime_type": mime_type,
            });
            if let Some(extras_obj) = all_extras.as_object()
                && !extras_obj.is_empty()
            {
                image_block["extras"] = all_extras;
            }
            return image_block;
        }

        let mut image_block = json!({
            "type": "image",
            "url": url,
        });
        if let Some(extras_obj) = all_extras.as_object()
            && !extras_obj.is_empty()
        {
            image_block["extras"] = all_extras;
        }
        return image_block;
    }

    if block_type == "input_audio"
        && let Some(audio) = block.get("input_audio").and_then(|v| v.as_object())
    {
        let top_known: HashSet<&str> = ["type", "input_audio"].iter().copied().collect();
        let mut all_extras = extract_extras(block, &top_known);

        let audio_known: HashSet<&str> = ["data", "format"].iter().copied().collect();
        let audio_extras = extract_extras(&json!(audio), &audio_known);
        if let Some(audio_extras_obj) = audio_extras.as_object() {
            for (key, value) in audio_extras_obj {
                if let Some(all_obj) = all_extras.as_object_mut() {
                    all_obj.insert(format!("audio_{key}"), value.clone());
                }
            }
        }

        let audio_data = audio.get("data").and_then(|v| v.as_str()).unwrap_or("");
        let audio_format = audio.get("format").and_then(|v| v.as_str()).unwrap_or("");

        let mut audio_block = json!({
            "type": "audio",
            "base64": audio_data,
            "mime_type": format!("audio/{audio_format}"),
        });
        if let Some(extras_obj) = all_extras.as_object()
            && !extras_obj.is_empty()
        {
            audio_block["extras"] = all_extras;
        }
        return audio_block;
    }

    if block_type == "file"
        && let Some(file) = block.get("file").and_then(|v| v.as_object())
    {
        let top_known: HashSet<&str> = ["type", "file"].iter().copied().collect();

        if let Some(file_id) = file.get("file_id").and_then(|v| v.as_str()) {
            let mut all_extras = extract_extras(block, &top_known);
            let file_known: HashSet<&str> = ["file_id"].iter().copied().collect();
            let file_extras = extract_extras(&json!(file), &file_known);
            if let Some(file_extras_obj) = file_extras.as_object() {
                for (key, value) in file_extras_obj {
                    if let Some(all_obj) = all_extras.as_object_mut() {
                        all_obj.insert(format!("file_{key}"), value.clone());
                    }
                }
            }

            let mut file_block = json!({
                "type": "file",
                "file_id": file_id,
            });
            if let Some(extras_obj) = all_extras.as_object()
                && !extras_obj.is_empty()
            {
                file_block["extras"] = all_extras;
            }
            return file_block;
        }

        if let Some(file_data) = file.get("file_data").and_then(|v| v.as_str())
            && let Some((_mime_type, data)) = parse_data_uri(file_data)
        {
            let mut all_extras = extract_extras(block, &top_known);
            let file_known: HashSet<&str> = ["file_data", "filename"].iter().copied().collect();
            let file_extras = extract_extras(&json!(file), &file_known);
            if let Some(file_extras_obj) = file_extras.as_object() {
                for (key, value) in file_extras_obj {
                    if let Some(all_obj) = all_extras.as_object_mut() {
                        all_obj.insert(format!("file_{key}"), value.clone());
                    }
                }
            }

            let mut file_block = json!({
                "type": "file",
                "base64": data,
                "mime_type": "application/pdf",
            });
            if let Some(filename) = file.get("filename").and_then(|v| v.as_str()) {
                file_block["filename"] = json!(filename);
            }
            if let Some(extras_obj) = all_extras.as_object()
                && !extras_obj.is_empty()
            {
                file_block["extras"] = all_extras;
            }
            return file_block;
        }
    }

    block.clone()
}

fn convert_annotation_to_v1(annotation: &Value) -> Value {
    let annotation_type = annotation
        .get("type")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    if annotation_type == "url_citation" {
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

        let mut citation = json!({"type": "citation"});
        if let Some(url) = annotation.get("url") {
            citation["url"] = url.clone();
        }
        for field in ["end_index", "start_index", "title"] {
            if let Some(value) = annotation.get(field) {
                citation[field] = value.clone();
            }
        }

        if let Some(annotation_obj) = annotation.as_object() {
            for (key, value) in annotation_obj {
                if !known_fields.contains(key.as_str()) {
                    let extras = citation
                        .as_object_mut()
                        .expect("citation should be an object")
                        .entry("extras")
                        .or_insert_with(|| json!({}));
                    if let Some(extras_obj) = extras.as_object_mut() {
                        extras_obj.insert(key.clone(), value.clone());
                    }
                }
            }
        }

        return citation;
    }

    if annotation_type == "file_citation" {
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

        let mut citation = json!({"type": "citation"});
        if let Some(filename) = annotation.get("filename") {
            citation["title"] = filename.clone();
        }

        if let Some(annotation_obj) = annotation.as_object() {
            for (key, value) in annotation_obj {
                if !known_fields.contains(key.as_str()) {
                    let extras = citation
                        .as_object_mut()
                        .expect("citation should be an object")
                        .entry("extras")
                        .or_insert_with(|| json!({}));
                    if let Some(extras_obj) = extras.as_object_mut() {
                        extras_obj.insert(key.clone(), value.clone());
                    }
                }
            }
        }

        return citation;
    }

    json!({
        "type": "non_standard_annotation",
        "value": annotation.clone(),
    })
}

fn explode_reasoning(block: &Value) -> Vec<Value> {
    if block.get("summary").is_none() {
        return vec![block.clone()];
    }

    let known_fields: HashSet<&str> = ["type", "reasoning", "id", "index"]
        .iter()
        .copied()
        .collect();

    let mut extras = json!({});
    if let Some(block_obj) = block.as_object() {
        for (key, value) in block_obj {
            if key != "summary"
                && !known_fields.contains(key.as_str())
                && let Some(extras_obj) = extras.as_object_mut()
            {
                extras_obj.insert(key.clone(), value.clone());
            }
        }
    }
    let has_extras = extras.as_object().map(|o| !o.is_empty()).unwrap_or(false);

    let summary = match block.get("summary").and_then(|v| v.as_array()) {
        Some(arr) => arr,
        None => {
            // summary is present but not an array (or is empty/null)
            let mut new_block = json!({});
            if let Some(block_obj) = block.as_object() {
                for (key, value) in block_obj {
                    if key != "summary" {
                        new_block[key] = value.clone();
                    }
                }
            }
            if has_extras {
                new_block["extras"] = extras;
            }
            if let Some(index) = new_block.get("index").and_then(|v| v.as_i64()) {
                let meaningful_idx = format!("{index}_0");
                new_block["index"] = json!(format!("lc_rs_{}", to_hex(meaningful_idx.as_bytes())));
            }
            return vec![new_block];
        }
    };

    if summary.is_empty() {
        let mut new_block = json!({});
        if let Some(block_obj) = block.as_object() {
            for (key, value) in block_obj {
                if key != "summary" {
                    new_block[key] = value.clone();
                }
            }
        }
        if has_extras {
            new_block["extras"] = extras;
        }
        if let Some(index) = new_block.get("index").and_then(|v| v.as_i64()) {
            let meaningful_idx = format!("{index}_0");
            new_block["index"] = json!(format!("lc_rs_{}", to_hex(meaningful_idx.as_bytes())));
        }
        return vec![new_block];
    }

    let mut common = json!({});
    if let Some(block_obj) = block.as_object() {
        for (key, value) in block_obj {
            if known_fields.contains(key.as_str()) {
                common[key] = value.clone();
            }
        }
    }

    let mut result = Vec::new();
    for (idx, part) in summary.iter().enumerate() {
        let mut new_block = common.clone();
        let reasoning_text = part.get("text").and_then(|v| v.as_str()).unwrap_or("");
        new_block["reasoning"] = json!(reasoning_text);

        if idx == 0
            && has_extras
            && let Some(new_obj) = new_block.as_object_mut()
            && let Some(extras_obj) = extras.as_object()
        {
            for (key, value) in extras_obj {
                new_obj.insert(key.clone(), value.clone());
            }
        }

        if let Some(block_index) = new_block.get("index").and_then(|v| v.as_i64()) {
            let summary_index = part.get("index").and_then(|v| v.as_i64()).unwrap_or(0);
            let meaningful_idx = format!("{block_index}_{summary_index}");
            new_block["index"] = json!(format!("lc_rs_{}", to_hex(meaningful_idx.as_bytes())));
        }

        result.push(new_block);
    }

    result
}

fn set_status(result_block: &mut Value, status: Option<&str>) {
    match status {
        Some("failed") => {
            result_block["status"] = json!("error");
        }
        Some("completed") => {
            result_block["status"] = json!("success");
        }
        Some(other) => {
            let extras = result_block
                .as_object_mut()
                .expect("result_block should be an object")
                .entry("extras")
                .or_insert_with(|| json!({}));
            if let Some(extras_obj) = extras.as_object_mut() {
                extras_obj.insert("status".to_string(), json!(other));
            }
        }
        None => {}
    }
}

const FUNCTION_CALL_IDS_MAP_KEY: &str = "__openai_function_call_ids__";

fn is_chatopenai_v03(content: &[Value], context: &OpenAiContext) -> bool {
    let all_dicts = content.iter().all(|b| b.is_object());
    if !all_dicts {
        return false;
    }

    let additional_kwargs = &context.additional_kwargs;
    let has_special_kwarg = [
        "reasoning",
        "tool_outputs",
        "refusal",
        FUNCTION_CALL_IDS_MAP_KEY,
    ]
    .iter()
    .any(|key| additional_kwargs.get(key).is_some());

    if has_special_kwarg {
        return true;
    }

    if let Some(msg_id) = &context.message_id
        && msg_id.starts_with("msg_")
        && let Some(resp_id) = context.response_metadata.get("id").and_then(|v| v.as_str())
        && resp_id.starts_with("resp_")
    {
        return true;
    }

    false
}

fn convert_from_v03(content: &[Value], context: &OpenAiContext) -> Vec<Value> {
    let content_order = [
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

    let mut buckets: Vec<(&str, Vec<Value>)> =
        content_order.iter().map(|key| (*key, Vec::new())).collect();
    let mut unknown_blocks: Vec<Value> = Vec::new();

    let is_chunk = context.chunk_position.as_deref() != Some("last");

    // Reasoning from additional_kwargs
    if let Some(reasoning) = context.additional_kwargs.get("reasoning") {
        if is_chunk {
            let mut reasoning_block = reasoning.clone();
            if let Some(obj) = reasoning_block.as_object_mut() {
                obj.insert("type".to_string(), json!("reasoning"));
            }
            buckets[0].1.push(reasoning_block);
        } else {
            buckets[0].1.push(reasoning.clone());
        }
    }

    // Refusal from additional_kwargs
    if let Some(refusal) = context
        .additional_kwargs
        .get("refusal")
        .and_then(|v| v.as_str())
        && !refusal.is_empty()
    {
        buckets[5]
            .1
            .push(json!({"type": "refusal", "refusal": refusal}));
    }

    // Text blocks from content
    for block in content {
        if block.get("type").and_then(|v| v.as_str()) == Some("text") {
            let mut block_copy = block.clone();
            if let Some(msg_id) = &context.message_id
                && msg_id.starts_with("msg_")
            {
                block_copy["id"] = json!(msg_id);
            }
            buckets[4].1.push(block_copy);
        } else {
            unknown_blocks.push(block.clone());
        }
    }

    // Function calls
    let function_call_ids = context.additional_kwargs.get(FUNCTION_CALL_IDS_MAP_KEY);

    if is_chunk && context.tool_call_chunks.len() == 1 {
        let tool_call_chunk = &context.tool_call_chunks[0];
        let mut function_call = json!({
            "type": "function_call",
            "name": tool_call_chunk.get("name").cloned().unwrap_or(Value::Null),
            "arguments": tool_call_chunk.get("args").and_then(|v| v.as_str()).unwrap_or(""),
            "call_id": tool_call_chunk.get("id").and_then(|v| v.as_str()).unwrap_or(""),
        });
        if let Some(ids_map) = function_call_ids
            && let Some(tc_id) = tool_call_chunk.get("id").and_then(|v| v.as_str())
            && let Some(mapped_id) = ids_map.get(tc_id)
        {
            function_call["id"] = mapped_id.clone();
        }
        buckets[6].1.push(function_call);
    } else {
        for tool_call in &context.tool_calls {
            let args_str = match tool_call.get("args") {
                Some(args) if args.is_object() || args.is_array() => {
                    serde_json::to_string(args).unwrap_or_default()
                }
                Some(args) if args.is_string() => args.as_str().unwrap_or("").to_string(),
                _ => "{}".to_string(),
            };
            let call_id = tool_call.get("id").and_then(|v| v.as_str()).unwrap_or("");
            let mut function_call = json!({
                "type": "function_call",
                "name": tool_call.get("name").and_then(|v| v.as_str()).unwrap_or(""),
                "arguments": args_str,
                "call_id": call_id,
            });
            if let Some(ids_map) = function_call_ids
                && let Some(mapped_id) = ids_map.get(call_id)
            {
                function_call["id"] = mapped_id.clone();
            }
            buckets[6].1.push(function_call);
        }
    }

    // Tool outputs from additional_kwargs
    if let Some(tool_outputs) = context
        .additional_kwargs
        .get("tool_outputs")
        .and_then(|v| v.as_array())
    {
        for output_block in tool_outputs {
            if let Some(block_type) = output_block.get("type").and_then(|v| v.as_str()) {
                let placed = buckets.iter_mut().any(|(key, bucket)| {
                    if *key == block_type {
                        bucket.push(output_block.clone());
                        true
                    } else {
                        false
                    }
                });
                if !placed {
                    unknown_blocks.push(output_block.clone());
                }
            } else {
                unknown_blocks.push(output_block.clone());
            }
        }
    }

    // Reassemble in canonical order
    let mut new_content = Vec::new();
    for (_key, bucket) in &buckets {
        new_content.extend(bucket.iter().cloned());
    }
    new_content.extend(unknown_blocks);

    new_content
}

pub fn convert_to_standard_blocks_with_context(
    content: &[Value],
    is_chunk: bool,
    context: Option<&OpenAiContext>,
) -> Vec<Value> {
    let content = match context {
        Some(ctx) if is_chatopenai_v03(content, ctx) => convert_from_v03(content, ctx),
        _ => content.to_vec(),
    };
    convert_to_standard_blocks(&content, is_chunk)
}

pub fn convert_to_standard_blocks(content: &[Value], is_chunk: bool) -> Vec<Value> {
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

                if let Some(annotations) = block.get("annotations").and_then(|v| v.as_array()) {
                    let converted: Vec<Value> =
                        annotations.iter().map(convert_annotation_to_v1).collect();
                    text_block["annotations"] = json!(converted);
                }

                if let Some(index) = block.get("index") {
                    text_block["index"] = json!(format!("lc_txt_{index}"));
                }

                let known_fields: HashSet<&str> =
                    ["type", "text", "annotations", "index", "extras", "id"]
                        .iter()
                        .copied()
                        .collect();
                populate_extras(&mut text_block, block, &known_fields);

                result.push(text_block);
            }

            "reasoning" => {
                for reasoning_block in explode_reasoning(block) {
                    result.push(reasoning_block);
                }
            }

            "image_generation_call" => {
                if let Some(image_result) = block.get("result").and_then(|v| v.as_str()) {
                    let mut new_block = json!({
                        "type": "image",
                        "base64": image_result,
                    });

                    if let Some(output_format) = block.get("output_format").and_then(|v| v.as_str())
                    {
                        new_block["mime_type"] = json!(format!("image/{output_format}"));
                    }
                    if let Some(id) = block.get("id") {
                        new_block["id"] = id.clone();
                    }
                    if let Some(index) = block.get("index") {
                        new_block["index"] = json!(format!("lc_img_{index}"));
                    }

                    let extra_keys = [
                        "status",
                        "background",
                        "output_format",
                        "quality",
                        "revised_prompt",
                        "size",
                    ];
                    for extra_key in extra_keys {
                        if let Some(value) = block.get(extra_key) {
                            let extras = new_block
                                .as_object_mut()
                                .expect("new_block should be an object")
                                .entry("extras")
                                .or_insert_with(|| json!({}));
                            if let Some(extras_obj) = extras.as_object_mut() {
                                extras_obj.insert(extra_key.to_string(), value.clone());
                            }
                        }
                    }

                    result.push(new_block);
                }
            }

            "function_call" => {
                let call_id = block.get("call_id").and_then(|v| v.as_str()).unwrap_or("");

                if is_chunk {
                    let mut tool_call_chunk = json!({
                        "type": "tool_call_chunk",
                        "name": block.get("name").cloned().unwrap_or(Value::Null),
                        "args": block.get("arguments").and_then(|v| v.as_str()).unwrap_or(""),
                        "id": call_id,
                    });

                    if let Some(id) = block.get("id") {
                        let extras = tool_call_chunk
                            .as_object_mut()
                            .expect("tool_call_chunk should be an object")
                            .entry("extras")
                            .or_insert_with(|| json!({}));
                        if let Some(extras_obj) = extras.as_object_mut() {
                            extras_obj.insert("item_id".to_string(), id.clone());
                        }
                    }

                    if let Some(index) = block.get("index") {
                        tool_call_chunk["index"] = json!(format!("lc_tc_{index}"));
                    }

                    result.push(tool_call_chunk);
                } else {
                    let args = block
                        .get("arguments")
                        .and_then(|v| v.as_str())
                        .and_then(|s| serde_json::from_str::<Value>(s).ok())
                        .unwrap_or(json!({}));

                    let mut tool_call_block = json!({
                        "type": "tool_call",
                        "name": block.get("name").and_then(|v| v.as_str()).unwrap_or(""),
                        "args": args,
                        "id": call_id,
                    });

                    if let Some(id) = block.get("id") {
                        let extras = tool_call_block
                            .as_object_mut()
                            .expect("tool_call_block should be an object")
                            .entry("extras")
                            .or_insert_with(|| json!({}));
                        if let Some(extras_obj) = extras.as_object_mut() {
                            extras_obj.insert("item_id".to_string(), id.clone());
                        }
                    }

                    if let Some(index) = block.get("index") {
                        tool_call_block["index"] = json!(format!("lc_tc_{index}"));
                    }

                    result.push(tool_call_block);
                }
            }

            "web_search_call" => {
                let block_id = block.get("id").and_then(|v| v.as_str()).unwrap_or("");

                let mut web_search_call = json!({
                    "type": "server_tool_call",
                    "name": "web_search",
                    "args": {},
                    "id": block_id,
                });

                if let Some(index) = block.get("index") {
                    web_search_call["index"] = json!(format!("lc_wsc_{index}"));
                }

                let mut sources: Option<Value> = None;
                if let Some(action) = block.get("action").and_then(|v| v.as_object()) {
                    if let Some(s) = action.get("sources") {
                        sources = Some(s.clone());
                    }
                    let mut args = json!({});
                    for (key, value) in action {
                        if key != "sources" {
                            args[key] = value.clone();
                        }
                    }
                    web_search_call["args"] = args;
                }

                let skip_keys: HashSet<&str> = ["type", "id", "action", "status", "index"]
                    .iter()
                    .copied()
                    .collect();
                if let Some(block_obj) = block.as_object() {
                    for (key, value) in block_obj {
                        if !skip_keys.contains(key.as_str()) {
                            web_search_call[key] = value.clone();
                        }
                    }
                }

                result.push(web_search_call);

                // Check if content already has a matching web_search_result
                let has_existing_result = content.iter().any(|other_block| {
                    other_block.get("type").and_then(|v| v.as_str()) == Some("web_search_result")
                        && other_block.get("id").and_then(|v| v.as_str()) == Some(block_id)
                });

                if !has_existing_result {
                    let mut web_search_result = json!({
                        "type": "server_tool_result",
                        "tool_call_id": block_id,
                    });
                    if let Some(sources_val) = sources {
                        web_search_result["output"] = json!({"sources": sources_val});
                    }

                    let status = block.get("status").and_then(|v| v.as_str());
                    set_status(&mut web_search_result, status);

                    if let Some(index) = block.get("index").and_then(|v| v.as_i64()) {
                        web_search_result["index"] = json!(format!("lc_wsr_{}", index + 1));
                    }

                    result.push(web_search_result);
                }
            }

            "file_search_call" => {
                let block_id = block.get("id").and_then(|v| v.as_str()).unwrap_or("");

                let mut file_search_call = json!({
                    "type": "server_tool_call",
                    "name": "file_search",
                    "id": block_id,
                    "args": {
                        "queries": block.get("queries").cloned().unwrap_or(json!([])),
                    },
                });

                if let Some(index) = block.get("index") {
                    file_search_call["index"] = json!(format!("lc_fsc_{index}"));
                }

                let skip_keys: HashSet<&str> =
                    ["type", "id", "queries", "results", "status", "index"]
                        .iter()
                        .copied()
                        .collect();
                if let Some(block_obj) = block.as_object() {
                    for (key, value) in block_obj {
                        if !skip_keys.contains(key.as_str()) {
                            file_search_call[key] = value.clone();
                        }
                    }
                }

                result.push(file_search_call);

                let mut file_search_result = json!({
                    "type": "server_tool_result",
                    "tool_call_id": block_id,
                });

                if let Some(results_val) = block.get("results") {
                    file_search_result["output"] = results_val.clone();
                }

                let status = block.get("status").and_then(|v| v.as_str());
                set_status(&mut file_search_result, status);

                if let Some(index) = block.get("index").and_then(|v| v.as_i64()) {
                    file_search_result["index"] = json!(format!("lc_fsr_{}", index + 1));
                }

                result.push(file_search_result);
            }

            "code_interpreter_call" => {
                let block_id = block.get("id").and_then(|v| v.as_str()).unwrap_or("");

                let mut code_interpreter_call = json!({
                    "type": "server_tool_call",
                    "name": "code_interpreter",
                    "id": block_id,
                });

                if let Some(code) = block.get("code") {
                    code_interpreter_call["args"] = json!({"code": code.clone()});
                }
                if let Some(index) = block.get("index") {
                    code_interpreter_call["index"] = json!(format!("lc_cic_{index}"));
                }

                let known_fields: HashSet<&str> =
                    ["type", "id", "outputs", "status", "code", "extras", "index"]
                        .iter()
                        .copied()
                        .collect();
                populate_extras(&mut code_interpreter_call, block, &known_fields);

                let mut code_interpreter_result = json!({
                    "type": "server_tool_result",
                    "tool_call_id": block_id,
                });

                if let Some(outputs) = block.get("outputs") {
                    code_interpreter_result["output"] = outputs.clone();
                }

                let status = block.get("status").and_then(|v| v.as_str());
                set_status(&mut code_interpreter_result, status);

                if let Some(index) = block.get("index").and_then(|v| v.as_i64()) {
                    code_interpreter_result["index"] = json!(format!("lc_cir_{}", index + 1));
                }

                result.push(code_interpreter_call);
                result.push(code_interpreter_result);
            }

            "mcp_call" => {
                let block_id = block.get("id").and_then(|v| v.as_str()).unwrap_or("");

                let mut mcp_call = json!({
                    "type": "server_tool_call",
                    "name": "remote_mcp",
                    "id": block_id,
                });

                if let Some(arguments) = block.get("arguments").and_then(|v| v.as_str()) {
                    match serde_json::from_str::<Value>(arguments) {
                        Ok(parsed) => {
                            mcp_call["args"] = parsed;
                        }
                        Err(_) => {
                            let extras = mcp_call
                                .as_object_mut()
                                .expect("mcp_call should be an object")
                                .entry("extras")
                                .or_insert_with(|| json!({}));
                            if let Some(extras_obj) = extras.as_object_mut() {
                                extras_obj.insert("arguments".to_string(), json!(arguments));
                            }
                        }
                    }
                }

                if let Some(name) = block.get("name") {
                    let extras = mcp_call
                        .as_object_mut()
                        .expect("mcp_call should be an object")
                        .entry("extras")
                        .or_insert_with(|| json!({}));
                    if let Some(extras_obj) = extras.as_object_mut() {
                        extras_obj.insert("tool_name".to_string(), name.clone());
                    }
                }

                if let Some(server_label) = block.get("server_label") {
                    let extras = mcp_call
                        .as_object_mut()
                        .expect("mcp_call should be an object")
                        .entry("extras")
                        .or_insert_with(|| json!({}));
                    if let Some(extras_obj) = extras.as_object_mut() {
                        extras_obj.insert("server_label".to_string(), server_label.clone());
                    }
                }

                if let Some(index) = block.get("index") {
                    mcp_call["index"] = json!(format!("lc_mcp_{index}"));
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
                populate_extras(&mut mcp_call, block, &known_fields);

                result.push(mcp_call);

                let mut mcp_result = json!({
                    "type": "server_tool_result",
                    "tool_call_id": block_id,
                });

                if let Some(output) = block.get("output") {
                    mcp_result["output"] = output.clone();
                }

                if let Some(error) = block.get("error") {
                    let extras = mcp_result
                        .as_object_mut()
                        .expect("mcp_result should be an object")
                        .entry("extras")
                        .or_insert_with(|| json!({}));
                    if let Some(extras_obj) = extras.as_object_mut() {
                        extras_obj.insert("error".to_string(), error.clone());
                    }
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
                let block_id = block.get("id").and_then(|v| v.as_str()).unwrap_or("");

                let mut mcp_list_tools_call = json!({
                    "type": "server_tool_call",
                    "name": "mcp_list_tools",
                    "args": {},
                    "id": block_id,
                });

                if let Some(server_label) = block.get("server_label") {
                    mcp_list_tools_call["extras"] = json!({"server_label": server_label.clone()});
                }
                if let Some(index) = block.get("index") {
                    mcp_list_tools_call["index"] = json!(format!("lc_mlt_{index}"));
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
                populate_extras(&mut mcp_list_tools_call, block, &known_fields);

                result.push(mcp_list_tools_call);

                let mut mcp_list_tools_result = json!({
                    "type": "server_tool_result",
                    "tool_call_id": block_id,
                });

                if let Some(tools) = block.get("tools") {
                    mcp_list_tools_result["output"] = tools.clone();
                }

                if let Some(error) = block.get("error") {
                    let extras = mcp_list_tools_result
                        .as_object_mut()
                        .expect("mcp_list_tools_result should be an object")
                        .entry("extras")
                        .or_insert_with(|| json!({}));
                    if let Some(extras_obj) = extras.as_object_mut() {
                        extras_obj.insert("error".to_string(), error.clone());
                    }
                    mcp_list_tools_result["status"] = json!("error");
                } else {
                    mcp_list_tools_result["status"] = json!("success");
                }

                if let Some(index) = block.get("index").and_then(|v| v.as_i64()) {
                    mcp_list_tools_result["index"] = json!(format!("lc_mltr_{}", index + 1));
                }

                result.push(mcp_list_tools_result);
            }

            "refusal" => {
                result.push(block.clone());
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
                        non_standard["index"] = json!(format!("lc_ns_{index}"));
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

pub fn convert_to_v1_from_chat_completions_input(content: &[Value]) -> Vec<Value> {
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

    let mut converted_blocks = Vec::new();
    for block in &unpacked_blocks {
        let block_type = block.get("type").and_then(|v| v.as_str()).unwrap_or("");

        if block_type == "text-plain" {
            let text = block.get("text").and_then(|v| v.as_str()).unwrap_or("");
            converted_blocks.push(json!({"type": "text", "text": text}));
            continue;
        }

        if matches!(block_type, "image_url" | "input_audio" | "file") && is_openai_data_block(block)
        {
            let converted = convert_openai_format_to_data_block(block);
            let converted_type = converted.get("type").and_then(|v| v.as_str()).unwrap_or("");
            if KNOWN_BLOCK_TYPES.contains(&converted_type) {
                converted_blocks.push(converted);
            } else {
                converted_blocks.push(json!({"type": "non_standard", "value": block.clone()}));
            }
        } else if block_type.is_empty() || KNOWN_BLOCK_TYPES.contains(&block_type) {
            converted_blocks.push(block.clone());
        } else {
            converted_blocks.push(json!({"type": "non_standard", "value": block.clone()}));
        }
    }

    converted_blocks
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_data_uri() {
        let (mime, data) = parse_data_uri("data:image/jpeg;base64,/9j/4AAQ").expect("should parse");
        assert_eq!(mime, "image/jpeg");
        assert_eq!(data, "/9j/4AAQ");

        assert!(parse_data_uri("not a data uri").is_none());
        assert!(parse_data_uri("data:;base64,abc").is_none());
    }

    #[test]
    fn test_is_openai_data_block_image() {
        let block = json!({
            "type": "image_url",
            "image_url": {"url": "https://example.com/img.png"}
        });
        assert!(is_openai_data_block(&block));

        let bad_block = json!({"type": "image_url"});
        assert!(!is_openai_data_block(&bad_block));
    }

    #[test]
    fn test_is_openai_data_block_audio() {
        let block = json!({
            "type": "input_audio",
            "input_audio": {"data": "base64data", "format": "mp3"}
        });
        assert!(is_openai_data_block(&block));
    }

    #[test]
    fn test_is_openai_data_block_file() {
        let block = json!({
            "type": "file",
            "file": {"file_id": "file-123"}
        });
        assert!(is_openai_data_block(&block));

        let block2 = json!({
            "type": "file",
            "file": {"file_data": "data:application/pdf;base64,abc"}
        });
        assert!(is_openai_data_block(&block2));
    }

    #[test]
    fn test_convert_text_block() {
        let content = vec![json!({
            "type": "text",
            "text": "Hello world",
        })];
        let result = convert_to_standard_blocks(&content, false);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0]["type"], "text");
        assert_eq!(result[0]["text"], "Hello world");
    }

    #[test]
    fn test_convert_text_block_with_index() {
        let content = vec![json!({
            "type": "text",
            "text": "Hello",
            "index": 0,
        })];
        let result = convert_to_standard_blocks(&content, false);
        assert_eq!(result[0]["index"], "lc_txt_0");
    }

    #[test]
    fn test_convert_text_block_with_url_citation() {
        let content = vec![json!({
            "type": "text",
            "text": "cited text",
            "annotations": [{
                "type": "url_citation",
                "url": "https://example.com",
                "title": "Example",
                "start_index": 0,
                "end_index": 10,
            }],
        })];
        let result = convert_to_standard_blocks(&content, false);
        assert_eq!(result[0]["annotations"][0]["type"], "citation");
        assert_eq!(result[0]["annotations"][0]["url"], "https://example.com");
        assert_eq!(result[0]["annotations"][0]["title"], "Example");
    }

    #[test]
    fn test_convert_text_block_with_file_citation() {
        let content = vec![json!({
            "type": "text",
            "text": "some text",
            "annotations": [{
                "type": "file_citation",
                "filename": "doc.pdf",
            }],
        })];
        let result = convert_to_standard_blocks(&content, false);
        assert_eq!(result[0]["annotations"][0]["type"], "citation");
        assert_eq!(result[0]["annotations"][0]["title"], "doc.pdf");
    }

    #[test]
    fn test_convert_function_call_non_chunk() {
        let content = vec![json!({
            "type": "function_call",
            "name": "get_weather",
            "arguments": "{\"city\":\"NYC\"}",
            "call_id": "call_123",
            "id": "item_abc",
        })];
        let result = convert_to_standard_blocks(&content, false);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0]["type"], "tool_call");
        assert_eq!(result[0]["name"], "get_weather");
        assert_eq!(result[0]["args"]["city"], "NYC");
        assert_eq!(result[0]["id"], "call_123");
        assert_eq!(result[0]["extras"]["item_id"], "item_abc");
    }

    #[test]
    fn test_convert_function_call_chunk() {
        let content = vec![json!({
            "type": "function_call",
            "name": "get_weather",
            "arguments": "{\"city\":",
            "call_id": "call_123",
            "index": 0,
        })];
        let result = convert_to_standard_blocks(&content, true);
        assert_eq!(result[0]["type"], "tool_call_chunk");
        assert_eq!(result[0]["name"], "get_weather");
        assert_eq!(result[0]["args"], "{\"city\":");
        assert_eq!(result[0]["index"], "lc_tc_0");
    }

    #[test]
    fn test_convert_web_search_call() {
        let content = vec![json!({
            "type": "web_search_call",
            "id": "ws_123",
            "action": {
                "query": "rust programming",
                "sources": [{"url": "https://example.com"}],
            },
            "status": "completed",
            "index": 0,
        })];
        let result = convert_to_standard_blocks(&content, false);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0]["type"], "server_tool_call");
        assert_eq!(result[0]["name"], "web_search");
        assert_eq!(result[0]["args"]["query"], "rust programming");
        assert_eq!(result[1]["type"], "server_tool_result");
        assert_eq!(result[1]["tool_call_id"], "ws_123");
        assert_eq!(
            result[1]["output"]["sources"][0]["url"],
            "https://example.com"
        );
        assert_eq!(result[1]["status"], "success");
        assert_eq!(result[1]["index"], "lc_wsr_1");
    }

    #[test]
    fn test_convert_file_search_call() {
        let content = vec![json!({
            "type": "file_search_call",
            "id": "fs_123",
            "queries": ["search term"],
            "results": [{"file": "doc.pdf"}],
            "status": "completed",
            "index": 0,
        })];
        let result = convert_to_standard_blocks(&content, false);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0]["type"], "server_tool_call");
        assert_eq!(result[0]["name"], "file_search");
        assert_eq!(result[1]["type"], "server_tool_result");
        assert_eq!(result[1]["status"], "success");
    }

    #[test]
    fn test_convert_code_interpreter_call() {
        let content = vec![json!({
            "type": "code_interpreter_call",
            "id": "ci_123",
            "code": "print('hello')",
            "outputs": [{"type": "logs", "logs": "hello"}],
            "status": "completed",
            "index": 0,
        })];
        let result = convert_to_standard_blocks(&content, false);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0]["type"], "server_tool_call");
        assert_eq!(result[0]["name"], "code_interpreter");
        assert_eq!(result[0]["args"]["code"], "print('hello')");
        assert_eq!(result[1]["type"], "server_tool_result");
        assert_eq!(result[1]["status"], "success");
    }

    #[test]
    fn test_convert_mcp_call() {
        let content = vec![json!({
            "type": "mcp_call",
            "id": "mcp_123",
            "name": "my_tool",
            "arguments": "{\"key\":\"value\"}",
            "server_label": "my_server",
            "output": "result",
            "index": 0,
        })];
        let result = convert_to_standard_blocks(&content, false);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0]["type"], "server_tool_call");
        assert_eq!(result[0]["name"], "remote_mcp");
        assert_eq!(result[0]["args"]["key"], "value");
        assert_eq!(result[0]["extras"]["tool_name"], "my_tool");
        assert_eq!(result[0]["extras"]["server_label"], "my_server");
        assert_eq!(result[1]["type"], "server_tool_result");
        assert_eq!(result[1]["output"], "result");
        assert_eq!(result[1]["status"], "success");
    }

    #[test]
    fn test_convert_mcp_call_with_error() {
        let content = vec![json!({
            "type": "mcp_call",
            "id": "mcp_123",
            "arguments": "{}",
            "error": "something went wrong",
        })];
        let result = convert_to_standard_blocks(&content, false);
        assert_eq!(result[1]["status"], "error");
        assert_eq!(result[1]["extras"]["error"], "something went wrong");
    }

    #[test]
    fn test_convert_mcp_list_tools() {
        let content = vec![json!({
            "type": "mcp_list_tools",
            "id": "mlt_123",
            "server_label": "my_server",
            "tools": [{"name": "tool1"}],
            "index": 0,
        })];
        let result = convert_to_standard_blocks(&content, false);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0]["type"], "server_tool_call");
        assert_eq!(result[0]["name"], "mcp_list_tools");
        assert_eq!(result[0]["extras"]["server_label"], "my_server");
        assert_eq!(result[1]["type"], "server_tool_result");
        assert_eq!(result[1]["output"][0]["name"], "tool1");
        assert_eq!(result[1]["status"], "success");
    }

    #[test]
    fn test_convert_image_generation_call() {
        let content = vec![json!({
            "type": "image_generation_call",
            "result": "base64imagedata",
            "output_format": "png",
            "id": "img_123",
            "status": "completed",
            "quality": "hd",
            "index": 0,
        })];
        let result = convert_to_standard_blocks(&content, false);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0]["type"], "image");
        assert_eq!(result[0]["base64"], "base64imagedata");
        assert_eq!(result[0]["mime_type"], "image/png");
        assert_eq!(result[0]["id"], "img_123");
        assert_eq!(result[0]["index"], "lc_img_0");
        assert_eq!(result[0]["extras"]["status"], "completed");
        assert_eq!(result[0]["extras"]["quality"], "hd");
    }

    #[test]
    fn test_convert_reasoning_simple() {
        let content = vec![json!({
            "type": "reasoning",
            "reasoning": "thinking...",
        })];
        let result = convert_to_standard_blocks(&content, false);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0]["type"], "reasoning");
        assert_eq!(result[0]["reasoning"], "thinking...");
    }

    #[test]
    fn test_convert_reasoning_with_summary() {
        let content = vec![json!({
            "type": "reasoning",
            "id": "rs_123",
            "summary": [
                {"text": "first thought", "index": 0},
                {"text": "second thought", "index": 1},
            ],
            "index": 0,
        })];
        let result = convert_to_standard_blocks(&content, false);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0]["reasoning"], "first thought");
        assert_eq!(result[1]["reasoning"], "second thought");
    }

    #[test]
    fn test_convert_reasoning_empty_summary() {
        let content = vec![json!({
            "type": "reasoning",
            "id": "rs_123",
            "summary": [],
            "index": 0,
        })];
        let result = convert_to_standard_blocks(&content, false);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0]["type"], "reasoning");
        assert!(result[0].get("summary").is_none());
    }

    #[test]
    fn test_convert_unknown_block_to_non_standard() {
        let content = vec![json!({
            "type": "computer_call",
            "id": "cc_123",
            "index": 5,
        })];
        let result = convert_to_standard_blocks(&content, false);
        assert_eq!(result[0]["type"], "non_standard");
        assert_eq!(result[0]["index"], "lc_ns_5");
        assert!(result[0]["value"].get("index").is_none());
    }

    #[test]
    fn test_convert_known_block_passthrough() {
        let content = vec![json!({
            "type": "tool_call",
            "name": "my_tool",
            "args": {"key": "value"},
            "id": "tc_123",
        })];
        let result = convert_to_standard_blocks(&content, false);
        assert_eq!(result[0], content[0]);
    }

    #[test]
    fn test_convert_openai_input_image_url() {
        let content = vec![json!({
            "type": "image_url",
            "image_url": {"url": "https://example.com/img.png"}
        })];
        let result = convert_to_v1_from_chat_completions_input(&content);
        assert_eq!(result[0]["type"], "image");
        assert_eq!(result[0]["url"], "https://example.com/img.png");
    }

    #[test]
    fn test_convert_openai_input_image_base64() {
        let content = vec![json!({
            "type": "image_url",
            "image_url": {"url": "data:image/png;base64,abc123"}
        })];
        let result = convert_to_v1_from_chat_completions_input(&content);
        assert_eq!(result[0]["type"], "image");
        assert_eq!(result[0]["base64"], "abc123");
        assert_eq!(result[0]["mime_type"], "image/png");
    }

    #[test]
    fn test_convert_openai_input_audio() {
        let content = vec![json!({
            "type": "input_audio",
            "input_audio": {"data": "audiodata", "format": "mp3"}
        })];
        let result = convert_to_v1_from_chat_completions_input(&content);
        assert_eq!(result[0]["type"], "audio");
        assert_eq!(result[0]["base64"], "audiodata");
        assert_eq!(result[0]["mime_type"], "audio/mp3");
    }

    #[test]
    fn test_convert_openai_input_file_id() {
        let content = vec![json!({
            "type": "file",
            "file": {"file_id": "file-123"}
        })];
        let result = convert_to_v1_from_chat_completions_input(&content);
        assert_eq!(result[0]["type"], "file");
        assert_eq!(result[0]["file_id"], "file-123");
    }

    #[test]
    fn test_convert_openai_input_file_base64() {
        let content = vec![json!({
            "type": "file",
            "file": {
                "file_data": "data:application/pdf;base64,pdfdata",
                "filename": "doc.pdf",
            }
        })];
        let result = convert_to_v1_from_chat_completions_input(&content);
        assert_eq!(result[0]["type"], "file");
        assert_eq!(result[0]["base64"], "pdfdata");
        assert_eq!(result[0]["mime_type"], "application/pdf");
        assert_eq!(result[0]["filename"], "doc.pdf");
    }

    #[test]
    fn test_convert_openai_input_non_standard_unwrap() {
        let content = vec![json!({
            "type": "non_standard",
            "value": {
                "type": "image_url",
                "image_url": {"url": "https://example.com/img.png"},
            }
        })];
        let result = convert_to_v1_from_chat_completions_input(&content);
        assert_eq!(result[0]["type"], "image");
        assert_eq!(result[0]["url"], "https://example.com/img.png");
    }

    #[test]
    fn test_convert_openai_input_unknown_becomes_non_standard() {
        let content = vec![json!({
            "type": "custom_thing",
            "data": "something"
        })];
        let result = convert_to_v1_from_chat_completions_input(&content);
        assert_eq!(result[0]["type"], "non_standard");
    }

    #[test]
    fn test_convert_image_url_with_detail_extras() {
        let content = vec![json!({
            "type": "image_url",
            "image_url": {"url": "https://example.com/img.png", "detail": "high"}
        })];
        let result = convert_to_v1_from_chat_completions_input(&content);
        assert_eq!(result[0]["type"], "image");
        assert_eq!(result[0]["extras"]["detail"], "high");
    }

    #[test]
    fn test_string_content_becomes_text() {
        let content = vec![json!("hello world")];
        let result = convert_to_standard_blocks(&content, false);
        assert_eq!(result[0]["type"], "text");
        assert_eq!(result[0]["text"], "hello world");
    }

    #[test]
    fn test_annotation_unknown_type() {
        let annotation = json!({"type": "unknown_annotation", "data": "something"});
        let result = convert_annotation_to_v1(&annotation);
        assert_eq!(result["type"], "non_standard_annotation");
        assert_eq!(result["value"]["type"], "unknown_annotation");
    }

    #[test]
    fn test_web_search_no_duplicate_result() {
        let content = vec![
            json!({
                "type": "web_search_call",
                "id": "ws_123",
                "status": "completed",
            }),
            json!({
                "type": "web_search_result",
                "id": "ws_123",
            }),
        ];
        let result = convert_to_standard_blocks(&content, false);
        let result_count = result
            .iter()
            .filter(|b| b.get("type").and_then(|v| v.as_str()) == Some("server_tool_result"))
            .count();
        assert_eq!(result_count, 0);
    }

    #[test]
    fn test_set_status_failed() {
        let mut block = json!({"type": "server_tool_result"});
        set_status(&mut block, Some("failed"));
        assert_eq!(block["status"], "error");
    }

    #[test]
    fn test_set_status_unknown() {
        let mut block = json!({"type": "server_tool_result"});
        set_status(&mut block, Some("in_progress"));
        assert_eq!(block["extras"]["status"], "in_progress");
    }

    #[test]
    fn test_text_plain_block_converted_to_text() {
        let content = vec![json!({
            "type": "text-plain",
            "text": "some plain text content",
            "mime_type": "text/plain",
        })];
        let result = convert_to_v1_from_chat_completions_input(&content);
        assert_eq!(result[0]["type"], "text");
        assert_eq!(result[0]["text"], "some plain text content");
    }

    #[test]
    fn test_text_plain_block_with_url_converted_to_text() {
        let content = vec![json!({
            "type": "text-plain",
            "url": "https://example.com/file.txt",
            "mime_type": "text/plain",
        })];
        let result = convert_to_v1_from_chat_completions_input(&content);
        // Should not pass through as text-plain since OpenAI doesn't support it
        assert_ne!(result[0]["type"], "text-plain");
    }
}
