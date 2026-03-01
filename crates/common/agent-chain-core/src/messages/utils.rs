use super::ai::{AIMessage, AIMessageChunk};
use super::base::{BaseMessage, BaseMessageChunk};
use super::chat::{ChatMessage, ChatMessageChunk};
use super::function::{FunctionMessage, FunctionMessageChunk};
use super::human::{HumanMessage, HumanMessageChunk};
use super::modifier::RemoveMessage;
use super::system::{SystemMessage, SystemMessageChunk};
use super::tool::{ToolCall, ToolMessage, ToolMessageChunk};

pub type AnyMessage = BaseMessage;

pub type MessageLikeRepresentation = serde_json::Value;

pub(crate) fn msg_to_chunk(message: &BaseMessage) -> BaseMessageChunk {
    match message {
        BaseMessage::Human(m) => BaseMessageChunk::Human(
            HumanMessageChunk::builder()
                .content(m.content.clone())
                .maybe_id(m.id.clone())
                .maybe_name(m.name.clone())
                .additional_kwargs(m.additional_kwargs.clone())
                .response_metadata(m.response_metadata.clone())
                .build(),
        ),
        BaseMessage::AI(m) => {
            let mut chunk = AIMessageChunk::builder()
                .content(m.content.clone())
                .maybe_id(m.id.clone())
                .maybe_name(m.name.clone())
                .tool_calls(m.tool_calls.clone())
                .invalid_tool_calls(m.invalid_tool_calls.clone())
                .maybe_usage_metadata(m.usage_metadata.clone())
                .additional_kwargs(m.additional_kwargs.clone())
                .response_metadata(m.response_metadata.clone())
                .build();
            chunk.init_tool_calls();
            BaseMessageChunk::AI(chunk)
        }
        BaseMessage::System(m) => BaseMessageChunk::System(
            SystemMessageChunk::builder()
                .content(m.content.clone())
                .maybe_id(m.id.clone())
                .maybe_name(m.name.clone())
                .additional_kwargs(m.additional_kwargs.clone())
                .response_metadata(m.response_metadata.clone())
                .build(),
        ),
        BaseMessage::Tool(m) => BaseMessageChunk::Tool(
            ToolMessageChunk::builder()
                .content(m.content.clone())
                .tool_call_id(m.tool_call_id.clone())
                .maybe_id(m.id.clone())
                .maybe_name(m.name.clone())
                .status(m.status.clone())
                .maybe_artifact(m.artifact.clone())
                .additional_kwargs(m.additional_kwargs.clone())
                .response_metadata(m.response_metadata.clone())
                .build(),
        ),
        BaseMessage::Chat(m) => BaseMessageChunk::Chat(
            ChatMessageChunk::builder()
                .content(m.content.clone())
                .role(m.role.clone())
                .maybe_id(m.id.clone())
                .maybe_name(m.name.clone())
                .additional_kwargs(m.additional_kwargs.clone())
                .response_metadata(m.response_metadata.clone())
                .build(),
        ),
        BaseMessage::Function(m) => BaseMessageChunk::Function(
            FunctionMessageChunk::builder()
                .content(m.content.clone())
                .name(m.name.clone())
                .maybe_id(m.id.clone())
                .additional_kwargs(m.additional_kwargs.clone())
                .response_metadata(m.response_metadata.clone())
                .build(),
        ),
        BaseMessage::Remove(_) => {
            panic!("Cannot convert RemoveMessage to chunk")
        }
    }
}

pub(crate) fn chunk_to_msg(chunk: &BaseMessageChunk) -> BaseMessage {
    chunk.to_message()
}

pub fn get_buffer_string(messages: &[BaseMessage], human_prefix: &str, ai_prefix: &str) -> String {
    messages
        .iter()
        .map(|m| {
            let role = match m {
                BaseMessage::Human(_) => human_prefix,
                BaseMessage::System(_) => "System",
                BaseMessage::AI(_) => ai_prefix,
                BaseMessage::Tool(_) => "Tool",
                BaseMessage::Chat(c) => &c.role,
                BaseMessage::Function(_) => "Function",
                BaseMessage::Remove(_) => "Remove",
            };
            let mut message = format!("{}: {}", role, m.text());
            if let BaseMessage::AI(ai_msg) = m
                && let Some(function_call) = ai_msg.additional_kwargs.get("function_call")
            {
                message.push_str(&function_call.to_string());
            }
            message
        })
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn message_to_dict(message: &BaseMessage) -> serde_json::Value {
    let mut data = serde_json::to_value(message).unwrap_or_default();

    let msg_type = message.message_type();

    if let Some(obj) = data.as_object_mut() {
        obj.remove("type");
    }

    serde_json::json!({
        "type": msg_type,
        "data": data
    })
}

pub fn messages_to_dict(messages: &[BaseMessage]) -> Vec<serde_json::Value> {
    messages.iter().map(message_to_dict).collect()
}

pub fn message_from_dict(message: &serde_json::Value) -> Result<BaseMessage, String> {
    let msg_type = message
        .get("type")
        .and_then(|t| t.as_str())
        .ok_or_else(|| "Message dict must contain 'type' key".to_string())?;

    let data = message
        .get("data")
        .ok_or_else(|| "Message dict must contain 'data' key".to_string())?;

    let mut merged_data = data.clone();
    if let Some(obj) = merged_data.as_object_mut() {
        obj.insert(
            "type".to_string(),
            serde_json::Value::String(msg_type.to_string()),
        );
    }

    serde_json::from_value(merged_data).map_err(|e| {
        format!(
            "Failed to deserialize message of type '{}': {}",
            msg_type, e
        )
    })
}

pub fn messages_from_dict(messages: &[serde_json::Value]) -> Result<Vec<BaseMessage>, String> {
    messages.iter().map(message_from_dict).collect()
}

pub fn convert_to_messages(messages: &[serde_json::Value]) -> Result<Vec<BaseMessage>, String> {
    let mut result = Vec::new();

    for message in messages {
        result.push(convert_to_message(message)?);
    }

    Ok(result)
}

pub fn convert_to_message(message: &serde_json::Value) -> Result<BaseMessage, String> {
    if let Some(_msg_type) = message.get("type").and_then(|t| t.as_str()) {
        if message.get("data").is_some() {
            return message_from_dict(message);
        }
        let msg_kwargs = message.as_object().ok_or("Expected object")?;
        let msg_type = msg_kwargs
            .get("type")
            .and_then(|t| t.as_str())
            .unwrap_or("");
        let content = msg_kwargs
            .get("content")
            .and_then(|c| c.as_str())
            .unwrap_or("");
        let name = msg_kwargs.get("name").and_then(|n| n.as_str());
        let tool_call_id = msg_kwargs.get("tool_call_id").and_then(|t| t.as_str());
        let tool_calls = msg_kwargs.get("tool_calls").and_then(|t| t.as_array());
        let id = msg_kwargs.get("id").and_then(|i| i.as_str());
        return create_message_from_role(msg_type, content, name, tool_call_id, tool_calls, id);
    }

    if let Some(obj) = message.as_object() {
        let msg_type = obj
            .get("role")
            .and_then(|r| r.as_str())
            .or_else(|| obj.get("type").and_then(|t| t.as_str()));

        if let Some(msg_type) = msg_type {
            let content = obj.get("content").and_then(|c| c.as_str()).unwrap_or("");
            let name = obj.get("name").and_then(|n| n.as_str());
            let tool_call_id = obj.get("tool_call_id").and_then(|t| t.as_str());
            let tool_calls = obj.get("tool_calls").and_then(|t| t.as_array());
            let id = obj.get("id").and_then(|i| i.as_str());
            return create_message_from_role(msg_type, content, name, tool_call_id, tool_calls, id);
        }
    }

    if let Some(s) = message.as_str() {
        return Ok(BaseMessage::Human(
            HumanMessage::builder().content(s).build(),
        ));
    }

    if let Some(arr) = message.as_array() {
        if arr.len() == 2 {
            let role = arr[0].as_str().ok_or("First element must be role string")?;
            let content = arr[1]
                .as_str()
                .ok_or("Second element must be content string")?;
            return create_message_from_role(role, content, None, None, None, None);
        } else {
            return Err("Array message must have exactly 2 elements [role, content]".to_string());
        }
    }

    Err(format!("Cannot convert to message: {:?}", message))
}

fn create_message_from_role(
    role: &str,
    content: &str,
    name: Option<&str>,
    tool_call_id: Option<&str>,
    tool_calls: Option<&Vec<serde_json::Value>>,
    id: Option<&str>,
) -> Result<BaseMessage, String> {
    let parsed_tool_calls: Vec<ToolCall> = if let Some(tcs) = tool_calls {
        tcs.iter()
            .filter_map(|tc| {
                if let Some(function) = tc.get("function") {
                    let args_raw = function
                        .get("arguments")
                        .and_then(|a| a.as_str())
                        .unwrap_or("{}");
                    let args = serde_json::from_str(args_raw)
                        .unwrap_or(serde_json::Value::Object(serde_json::Map::new()));
                    Some(ToolCall {
                        name: function
                            .get("name")
                            .and_then(|n| n.as_str())
                            .unwrap_or("")
                            .to_string(),
                        args,
                        id: tc.get("id").and_then(|i| i.as_str()).map(|s| s.to_string()),
                        call_type: Some("tool_call".to_string()),
                    })
                } else {
                    serde_json::from_value::<ToolCall>(tc.clone()).ok()
                }
            })
            .collect()
    } else {
        Vec::new()
    };

    match role {
        "human" | "user" => Ok(BaseMessage::Human(
            HumanMessage::builder()
                .content(content)
                .maybe_name(name.map(|n| n.to_string()))
                .maybe_id(id.map(|i| i.to_string()))
                .build(),
        )),
        "ai" | "assistant" => Ok(BaseMessage::AI(
            AIMessage::builder()
                .content(content)
                .maybe_name(name.map(|n| n.to_string()))
                .maybe_id(id.map(|i| i.to_string()))
                .tool_calls(parsed_tool_calls)
                .build(),
        )),
        "system" => Ok(BaseMessage::System(
            SystemMessage::builder()
                .content(content)
                .maybe_name(name.map(|n| n.to_string()))
                .maybe_id(id.map(|i| i.to_string()))
                .build(),
        )),
        "developer" => {
            let mut msg = SystemMessage::builder()
                .content(content)
                .maybe_name(name.map(|n| n.to_string()))
                .maybe_id(id.map(|i| i.to_string()))
                .build();
            msg.additional_kwargs.insert(
                "__openai_role__".to_string(),
                serde_json::Value::String("developer".to_string()),
            );
            Ok(BaseMessage::System(msg))
        }
        "function" => {
            let fn_name = name.ok_or("Function messages require a name")?;
            Ok(BaseMessage::Function(
                FunctionMessage::builder()
                    .name(fn_name)
                    .content(content)
                    .maybe_id(id.map(|i| i.to_string()))
                    .build(),
            ))
        }
        "tool" => {
            let tc_id = tool_call_id.ok_or("Tool messages require a tool_call_id")?;
            Ok(BaseMessage::Tool(
                ToolMessage::builder()
                    .content(content)
                    .tool_call_id(tc_id)
                    .maybe_name(name.map(|n| n.to_string()))
                    .maybe_id(id.map(|i| i.to_string()))
                    .build(),
            ))
        }
        "remove" => {
            let msg_id = id.unwrap_or("");
            Ok(BaseMessage::Remove(
                RemoveMessage::builder().id(msg_id).build(),
            ))
        }
        _ => Ok(BaseMessage::Chat(
            ChatMessage::builder()
                .content(content)
                .role(role)
                .maybe_name(name.map(|n| n.to_string()))
                .maybe_id(id.map(|i| i.to_string()))
                .build(),
        )),
    }
}

#[derive(Debug, Clone)]
pub enum ExcludeToolCalls {
    All,
    Ids(Vec<String>),
}

#[allow(clippy::too_many_arguments)]
pub fn filter_messages(
    messages: &[BaseMessage],
    include_names: Option<&[&str]>,
    exclude_names: Option<&[&str]>,
    include_types: Option<&[&str]>,
    exclude_types: Option<&[&str]>,
    include_ids: Option<&[&str]>,
    exclude_ids: Option<&[&str]>,
    exclude_tool_calls: Option<&ExcludeToolCalls>,
) -> Vec<BaseMessage> {
    let mut filtered: Vec<BaseMessage> = Vec::new();

    for msg in messages {
        if let Some(exclude_names) = exclude_names
            && let Some(name) = msg.name()
            && exclude_names.contains(&name.as_str())
        {
            continue;
        }

        if let Some(exclude_types) = exclude_types
            && exclude_types.contains(&msg.message_type())
        {
            continue;
        }

        if let Some(exclude_ids) = exclude_ids
            && let Some(id) = msg.id()
            && exclude_ids.contains(&id.as_str())
        {
            continue;
        }

        let mut msg = msg.clone();
        match exclude_tool_calls {
            Some(ExcludeToolCalls::All) => {
                if let BaseMessage::AI(ref ai_msg) = msg
                    && !ai_msg.tool_calls.is_empty()
                {
                    continue;
                }
                if matches!(msg, BaseMessage::Tool(_)) {
                    continue;
                }
            }
            Some(ExcludeToolCalls::Ids(ids)) => {
                if let BaseMessage::AI(ref ai_msg) = msg
                    && !ai_msg.tool_calls.is_empty()
                {
                    let remaining_tool_calls: Vec<ToolCall> = ai_msg
                        .tool_calls
                        .iter()
                        .filter(|tc| tc.id.as_ref().is_none_or(|id| !ids.contains(id)))
                        .cloned()
                        .collect();
                    if remaining_tool_calls.is_empty() {
                        continue;
                    }
                    if remaining_tool_calls.len() != ai_msg.tool_calls.len() {
                        msg = BaseMessage::AI(
                            AIMessage::builder()
                                .content(ai_msg.content.clone())
                                .maybe_id(ai_msg.id.clone())
                                .maybe_name(ai_msg.name.clone())
                                .tool_calls(remaining_tool_calls)
                                .invalid_tool_calls(ai_msg.invalid_tool_calls.clone())
                                .maybe_usage_metadata(ai_msg.usage_metadata.clone())
                                .additional_kwargs(ai_msg.additional_kwargs.clone())
                                .response_metadata(ai_msg.response_metadata.clone())
                                .build(),
                        );
                    }
                }
                if let BaseMessage::Tool(ref tool_msg) = msg
                    && ids.contains(&tool_msg.tool_call_id)
                {
                    continue;
                }
            }
            None => {}
        }

        let no_include_criteria =
            include_names.is_none() && include_types.is_none() && include_ids.is_none();

        let matches_include_names = include_names.is_some_and(|names| {
            msg.name()
                .is_some_and(|name| names.contains(&name.as_str()))
        });

        let matches_include_types =
            include_types.is_some_and(|types| types.contains(&msg.message_type()));

        let matches_include_ids =
            include_ids.is_some_and(|ids| msg.id().is_some_and(|id| ids.contains(&id.as_str())));

        if no_include_criteria
            || matches_include_names
            || matches_include_types
            || matches_include_ids
        {
            filtered.push(msg);
        }
    }

    filtered
}

pub fn merge_message_runs(messages: &[BaseMessage], chunk_separator: &str) -> Vec<BaseMessage> {
    if messages.is_empty() {
        return Vec::new();
    }

    let mut merged: Vec<BaseMessage> = Vec::new();

    for msg in messages {
        let last = if merged.is_empty() {
            None
        } else {
            merged.pop()
        };

        let Some(last) = last else {
            merged.push(msg.clone());
            continue;
        };

        if matches!(msg, BaseMessage::Tool(_))
            || std::mem::discriminant(&last) != std::mem::discriminant(msg)
        {
            merged.push(last);
            merged.push(msg.clone());
        } else {
            let last_chunk = msg_to_chunk(&last);
            let mut curr_chunk = msg_to_chunk(msg);

            match &mut curr_chunk {
                BaseMessageChunk::AI(c) => c.response_metadata.clear(),
                BaseMessageChunk::Human(c) => c.response_metadata.clear(),
                BaseMessageChunk::System(c) => c.response_metadata.clear(),
                BaseMessageChunk::Tool(c) => c.response_metadata.clear(),
                BaseMessageChunk::Chat(c) => c.response_metadata.clear(),
                BaseMessageChunk::Function(c) => c.response_metadata.clear(),
            }

            if !chunk_separator.is_empty() {
                let last_content = last_chunk.content();
                let curr_content = curr_chunk.content();
                if !last_content.is_empty() && !curr_content.is_empty() {
                    let last_is_str =
                        matches!(last_content, super::content::MessageContent::Text(_));
                    let curr_is_str =
                        matches!(curr_content, super::content::MessageContent::Text(_));
                    if last_is_str && curr_is_str {
                        match &mut curr_chunk {
                            BaseMessageChunk::AI(c) => {
                                if let super::content::MessageContent::Text(ref mut s) = c.content {
                                    *s = format!("{}{}", chunk_separator, s);
                                }
                            }
                            BaseMessageChunk::Human(c) => {
                                if let super::content::MessageContent::Text(ref mut s) = c.content {
                                    *s = format!("{}{}", chunk_separator, s);
                                }
                            }
                            BaseMessageChunk::System(c) => {
                                if let super::content::MessageContent::Text(ref mut s) = c.content {
                                    *s = format!("{}{}", chunk_separator, s);
                                }
                            }
                            BaseMessageChunk::Chat(c) => {
                                if let super::content::MessageContent::Text(ref mut s) = c.content {
                                    *s = format!("{}{}", chunk_separator, s);
                                }
                            }
                            BaseMessageChunk::Function(c) => {
                                if let super::content::MessageContent::Text(ref mut s) = c.content {
                                    *s = format!("{}{}", chunk_separator, s);
                                }
                            }
                            BaseMessageChunk::Tool(c) => {
                                if let super::content::MessageContent::Text(ref mut s) = c.content {
                                    *s = format!("{}{}", chunk_separator, s);
                                }
                            }
                        }
                    }
                }
            }

            let mut merged_chunk = last_chunk + curr_chunk;

            if let BaseMessageChunk::AI(ref mut ai_chunk) = merged_chunk {
                ai_chunk.init_tool_calls();
            }

            merged.push(chunk_to_msg(&merged_chunk));
        }
    }

    merged
}

pub fn message_chunk_to_message(chunk: &BaseMessageChunk) -> BaseMessage {
    chunk.to_message()
}

#[derive(Debug, Clone)]
pub struct CountTokensConfig {
    pub chars_per_token: f64,
    pub extra_tokens_per_message: f64,
    pub count_name: bool,
}

impl Default for CountTokensConfig {
    fn default() -> Self {
        Self {
            chars_per_token: 4.0,
            extra_tokens_per_message: 3.0,
            count_name: true,
        }
    }
}

pub fn count_tokens_approximately(messages: &[BaseMessage], config: &CountTokensConfig) -> usize {
    let mut token_count: f64 = 0.0;

    for message in messages {
        let mut message_chars: usize = 0;

        message_chars += message.text().len();

        if let BaseMessage::AI(ai_msg) = message
            && !ai_msg.tool_calls.is_empty()
        {
            let tool_calls_str = format!("{:?}", ai_msg.tool_calls);
            message_chars += tool_calls_str.len();
        }

        if let BaseMessage::Tool(tool_msg) = message {
            message_chars += tool_msg.tool_call_id.len();
        }

        let role = get_message_openai_role(message);
        message_chars += role.len();

        if config.count_name
            && let Some(name) = message.name()
        {
            message_chars += name.len();
        }

        token_count += (message_chars as f64 / config.chars_per_token).ceil();

        token_count += config.extra_tokens_per_message;
    }

    token_count.ceil() as usize
}

fn get_message_openai_role(message: &BaseMessage) -> &'static str {
    match message {
        BaseMessage::AI(_) => "assistant",
        BaseMessage::Human(_) => "user",
        BaseMessage::Tool(_) => "tool",
        BaseMessage::System(msg) => {
            if msg
                .additional_kwargs
                .get("__openai_role__")
                .and_then(|v| v.as_str())
                == Some("developer")
            {
                "developer"
            } else {
                "system"
            }
        }
        BaseMessage::Function(_) => "function",
        BaseMessage::Chat(c) => match c.role.as_str() {
            "user" => "user",
            "assistant" => "assistant",
            "system" => "system",
            "function" => "function",
            "tool" => "tool",
            _ => "user",
        },
        BaseMessage::Remove(_) => "remove",
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TextFormat {
    #[default]
    String,
    Block,
}

pub fn convert_to_openai_messages(
    messages: &[BaseMessage],
    text_format: TextFormat,
    include_id: bool,
) -> Vec<serde_json::Value> {
    let mut oai_messages = Vec::new();
    for msg in messages {
        oai_messages.extend(convert_single_to_openai_message(
            msg,
            text_format,
            include_id,
        ));
    }
    oai_messages
}

fn convert_single_to_openai_message(
    message: &BaseMessage,
    text_format: TextFormat,
    include_id: bool,
) -> Vec<serde_json::Value> {
    let role = get_message_openai_role(message);
    let mut oai_msg = serde_json::json!({ "role": role });

    if let Some(name) = message.name() {
        oai_msg["name"] = serde_json::json!(name);
    }

    if let BaseMessage::Tool(tool_msg) = message {
        oai_msg["tool_call_id"] = serde_json::json!(tool_msg.tool_call_id);
    }

    if let BaseMessage::AI(ai_msg) = message
        && !ai_msg.tool_calls.is_empty()
    {
        oai_msg["tool_calls"] = serde_json::json!(convert_to_openai_tool_calls(&ai_msg.tool_calls));
    }

    if let Some(additional_kwargs) = message.additional_kwargs()
        && let Some(refusal) = additional_kwargs.get("refusal")
    {
        oai_msg["refusal"] = refusal.clone();
    }

    if include_id && let Some(id) = message.id() {
        oai_msg["id"] = serde_json::json!(id);
    }

    let raw_content = message.content();
    let content_list: Option<Vec<serde_json::Value>> = match raw_content {
        super::content::MessageContent::Parts(_) => Some(raw_content.as_json_values()),
        super::content::MessageContent::Text(s) => serde_json::from_str(s).ok(),
    };

    let mut tool_messages: Vec<serde_json::Value> = Vec::new();

    if let Some(blocks) = content_list {
        let mut content_blocks: Vec<serde_json::Value> = Vec::new();

        for block in &blocks {
            let block_type = block.get("type").and_then(|t| t.as_str()).unwrap_or("");

            match block_type {
                "text" => {
                    content_blocks.push(serde_json::json!({
                        "type": "text",
                        "text": block.get("text").and_then(|t| t.as_str()).unwrap_or(""),
                    }));
                }
                "image_url" => {
                    content_blocks.push(serde_json::json!({
                        "type": "image_url",
                        "image_url": block.get("image_url").cloned().unwrap_or_default(),
                    }));
                }
                "input_audio" => {
                    content_blocks.push(block.clone());
                }
                "tool_use" => {
                    if let BaseMessage::AI(ai_msg) = message {
                        let block_id = block.get("id").and_then(|i| i.as_str()).unwrap_or("");
                        let already_in_tool_calls = ai_msg
                            .tool_calls
                            .iter()
                            .any(|tc| tc.id.as_deref() == Some(block_id));
                        if !already_in_tool_calls {
                            let tool_calls_arr = oai_msg
                                .get("tool_calls")
                                .and_then(|v| v.as_array())
                                .cloned()
                                .unwrap_or_default();
                            let mut new_tool_calls = tool_calls_arr;
                            new_tool_calls.push(serde_json::json!({
                                "type": "function",
                                "id": block_id,
                                "function": {
                                    "name": block.get("name").and_then(|n| n.as_str()).unwrap_or(""),
                                    "arguments": serde_json::to_string(
                                        block.get("input").unwrap_or(&serde_json::json!({}))
                                    ).unwrap_or_default(),
                                }
                            }));
                            oai_msg["tool_calls"] = serde_json::json!(new_tool_calls);
                        }
                    }
                }
                "tool_result" => {
                    let tool_use_id = block
                        .get("tool_use_id")
                        .and_then(|t| t.as_str())
                        .unwrap_or("");
                    let tool_content = block.get("content").and_then(|c| c.as_str()).unwrap_or("");
                    let is_error = block
                        .get("is_error")
                        .and_then(|e| e.as_bool())
                        .unwrap_or(false);
                    let status = if is_error { "error" } else { "success" };
                    let tool_msg = ToolMessage::builder()
                        .content(tool_content)
                        .tool_call_id(tool_use_id)
                        .status(super::tool::ToolStatus::from(status.to_string()))
                        .build();
                    tool_messages.extend(convert_single_to_openai_message(
                        &BaseMessage::Tool(tool_msg),
                        text_format,
                        include_id,
                    ));
                }
                "image" | "source" => {
                    if let Some(source) = block.get("source") {
                        let media_type = source
                            .get("media_type")
                            .and_then(|m| m.as_str())
                            .unwrap_or("");
                        let src_type = source.get("type").and_then(|t| t.as_str()).unwrap_or("");
                        let data = source.get("data").and_then(|d| d.as_str()).unwrap_or("");
                        content_blocks.push(serde_json::json!({
                            "type": "image_url",
                            "image_url": {
                                "url": format!("data:{};{},{}", media_type, src_type, data),
                            }
                        }));
                    }
                }
                "thinking" | "reasoning" => {
                    content_blocks.push(block.clone());
                }
                _ => {
                    if let Some(s) = block.as_str() {
                        content_blocks.push(serde_json::json!({"type": "text", "text": s}));
                    } else {
                        content_blocks.push(block.clone());
                    }
                }
            }
        }

        match text_format {
            TextFormat::String => {
                if content_blocks
                    .iter()
                    .all(|b| b.get("type").and_then(|t| t.as_str()) == Some("text"))
                {
                    let joined: String = content_blocks
                        .iter()
                        .filter_map(|b| b.get("text").and_then(|t| t.as_str()))
                        .collect::<Vec<_>>()
                        .join("\n");
                    oai_msg["content"] = serde_json::json!(joined);
                } else {
                    oai_msg["content"] = serde_json::json!(content_blocks);
                }
            }
            TextFormat::Block => {
                oai_msg["content"] = serde_json::json!(content_blocks);
            }
        }
    } else {
        match text_format {
            TextFormat::String => {
                oai_msg["content"] = serde_json::json!(raw_content);
            }
            TextFormat::Block => {
                if raw_content.is_empty() {
                    oai_msg["content"] = serde_json::json!([]);
                } else {
                    oai_msg["content"] =
                        serde_json::json!([{ "type": "text", "text": raw_content }]);
                }
            }
        }
    }

    let has_content = oai_msg.get("content").is_some_and(|c| {
        if let Some(s) = c.as_str() {
            !s.is_empty()
        } else if let Some(arr) = c.as_array() {
            !arr.is_empty()
        } else {
            true
        }
    });

    if has_content || tool_messages.is_empty() {
        let mut result = vec![oai_msg];
        result.extend(tool_messages);
        result
    } else {
        tool_messages
    }
}

fn convert_to_openai_tool_calls(tool_calls: &[ToolCall]) -> Vec<serde_json::Value> {
    tool_calls
        .iter()
        .map(|tc| {
            serde_json::json!({
                "type": "function",
                "id": tc.id,
                "function": {
                    "name": tc.name,
                    "arguments": serde_json::to_string(&tc.args).unwrap_or_default(),
                }
            })
        })
        .collect()
}

fn is_message_type(message: &BaseMessage, types: &[String]) -> bool {
    types.iter().any(|t| t == message.message_type())
}

fn default_text_splitter(text: &str) -> Vec<String> {
    let splits: Vec<&str> = text.split('\n').collect();
    if splits.len() <= 1 {
        return vec![text.to_string()];
    }
    let mut result: Vec<String> = splits[..splits.len() - 1]
        .iter()
        .map(|s| format!("{}\n", s))
        .collect();
    result.push(splits.last().unwrap_or(&"").to_string());
    result
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TrimStrategy {
    First,
    #[default]
    Last,
}

#[derive(Debug, Clone)]
pub struct TrimMessagesConfig<F, S = fn(&str) -> Vec<String>>
where
    F: Fn(&[BaseMessage]) -> usize,
    S: Fn(&str) -> Vec<String>,
{
    pub max_tokens: usize,
    pub token_counter: F,
    pub strategy: TrimStrategy,
    pub allow_partial: bool,
    pub include_system: bool,
    pub end_on: Option<Vec<String>>,
    pub start_on: Option<Vec<String>>,
    pub text_splitter: Option<S>,
}

#[bon::bon]
impl<F> TrimMessagesConfig<F>
where
    F: Fn(&[BaseMessage]) -> usize,
{
    #[builder]
    pub fn new(
        max_tokens: usize,
        token_counter: F,
        #[builder(default)] strategy: TrimStrategy,
        #[builder(default)] allow_partial: bool,
        #[builder(default)] include_system: bool,
        end_on: Option<Vec<String>>,
        start_on: Option<Vec<String>>,
    ) -> Self {
        Self {
            max_tokens,
            token_counter,
            strategy,
            allow_partial,
            include_system,
            end_on,
            start_on,
            text_splitter: None,
        }
    }

    pub fn with_text_splitter<S2: Fn(&str) -> Vec<String>>(
        self,
        text_splitter: S2,
    ) -> TrimMessagesConfig<F, S2> {
        TrimMessagesConfig {
            max_tokens: self.max_tokens,
            token_counter: self.token_counter,
            strategy: self.strategy,
            allow_partial: self.allow_partial,
            include_system: self.include_system,
            end_on: self.end_on,
            start_on: self.start_on,
            text_splitter: Some(text_splitter),
        }
    }
}

pub fn trim_messages<F, S>(
    messages: &[BaseMessage],
    config: &TrimMessagesConfig<F, S>,
) -> Vec<BaseMessage>
where
    F: Fn(&[BaseMessage]) -> usize,
    S: Fn(&str) -> Vec<String>,
{
    if messages.is_empty() {
        return Vec::new();
    }

    if config.start_on.is_some() && config.strategy == TrimStrategy::First {
        panic!("start_on parameter is only valid with strategy='last'");
    }
    if config.include_system && config.strategy == TrimStrategy::First {
        panic!("include_system parameter is only valid with strategy='last'");
    }

    match config.strategy {
        TrimStrategy::First => trim_messages_first(messages, config, false),
        TrimStrategy::Last => trim_messages_last(messages, config),
    }
}

fn trim_messages_first<F, S>(
    messages: &[BaseMessage],
    config: &TrimMessagesConfig<F, S>,
    reverse_partial: bool,
) -> Vec<BaseMessage>
where
    F: Fn(&[BaseMessage]) -> usize,
    S: Fn(&str) -> Vec<String>,
{
    let mut messages: Vec<BaseMessage> = messages.to_vec();

    if messages.is_empty() {
        return messages;
    }

    if (config.token_counter)(&messages) <= config.max_tokens {
        if let Some(ref end_on) = config.end_on {
            while !messages.is_empty()
                && !is_message_type(messages.last().expect("checked non-empty"), end_on)
            {
                messages.pop();
            }
        }
        return messages;
    }

    let mut left = 0;
    let mut right = messages.len();

    while left < right {
        let mid = (left + right).div_ceil(2);
        if (config.token_counter)(&messages[..mid]) <= config.max_tokens {
            left = mid;
        } else {
            right = mid - 1;
        }
    }

    let mut idx = left;

    if config.allow_partial && idx < messages.len() {
        let mut included_partial = false;

        let excluded_content = messages[idx].content();
        let content_blocks_opt: Option<Vec<serde_json::Value>> = match excluded_content {
            super::content::MessageContent::Parts(_) => Some(excluded_content.as_json_values()),
            super::content::MessageContent::Text(s) => serde_json::from_str(s).ok(),
        };
        if let Some(mut content_blocks) = content_blocks_opt
            && content_blocks.len() > 1
        {
            if reverse_partial {
                content_blocks.reverse();
            }
            let num_blocks = content_blocks.len();
            for remove_count in 1..num_blocks {
                let mut partial_blocks = content_blocks[..num_blocks - remove_count].to_vec();
                if reverse_partial {
                    partial_blocks.reverse();
                }
                let partial_content = serde_json::to_string(&partial_blocks).unwrap_or_default();
                let partial_msg = create_message_with_content(&messages[idx], &partial_content);
                let mut test = messages[..idx].to_vec();
                test.push(partial_msg);
                if (config.token_counter)(&test) <= config.max_tokens {
                    messages = test;
                    idx += 1;
                    included_partial = true;
                    break;
                }
            }
        }

        if !included_partial {
            let content_str = messages[idx].text();
            if !content_str.is_empty() {
                let mut split_texts = if let Some(ref splitter) = config.text_splitter {
                    splitter(&content_str)
                } else {
                    default_text_splitter(&content_str)
                };
                if split_texts.len() > 1 {
                    if reverse_partial {
                        split_texts.reverse();
                    }
                    let mut s_left = 0;
                    let mut s_right = split_texts.len();
                    while s_left < s_right {
                        let mid = (s_left + s_right).div_ceil(2);
                        let partial_content: String = split_texts[..mid].concat();
                        let partial_msg =
                            create_message_with_content(&messages[idx], &partial_content);
                        let mut test = messages[..idx].to_vec();
                        test.push(partial_msg);
                        if (config.token_counter)(&test) <= config.max_tokens {
                            s_left = mid;
                        } else {
                            s_right = mid - 1;
                        }
                    }
                    if s_left > 0 {
                        let mut content_splits = split_texts[..s_left].to_vec();
                        if reverse_partial {
                            content_splits.reverse();
                        }
                        let partial_content: String = content_splits.concat();
                        let partial_msg =
                            create_message_with_content(&messages[idx], &partial_content);
                        let end = idx;
                        messages = messages[..end].to_vec();
                        messages.push(partial_msg);
                        idx += 1;
                    }
                }
            }
        }
    }

    if let Some(ref end_on) = config.end_on {
        while idx > 0 && !is_message_type(&messages[idx - 1], end_on) {
            idx -= 1;
        }
    }

    messages[..idx].to_vec()
}

fn trim_messages_last<F, S>(
    messages: &[BaseMessage],
    config: &TrimMessagesConfig<F, S>,
) -> Vec<BaseMessage>
where
    F: Fn(&[BaseMessage]) -> usize,
    S: Fn(&str) -> Vec<String>,
{
    let mut messages: Vec<BaseMessage> = messages.to_vec();

    if messages.is_empty() {
        return messages;
    }

    if let Some(ref end_on) = config.end_on {
        while !messages.is_empty()
            && !is_message_type(messages.last().expect("checked non-empty"), end_on)
        {
            messages.pop();
        }
    }

    let system_message = if config.include_system
        && !messages.is_empty()
        && matches!(messages.first(), Some(BaseMessage::System(_)))
    {
        Some(messages.remove(0))
    } else {
        None
    };

    let remaining_tokens = if let Some(ref sys_msg) = system_message {
        let sys_tokens = (config.token_counter)(std::slice::from_ref(sys_msg));
        config.max_tokens.saturating_sub(sys_tokens)
    } else {
        config.max_tokens
    };

    messages.reverse();

    #[allow(clippy::type_complexity)]
    let splitter_wrapper: Option<Box<dyn Fn(&str) -> Vec<String> + '_>> = config
        .text_splitter
        .as_ref()
        .map(|s| Box::new(move |text: &str| s(text)) as Box<dyn Fn(&str) -> Vec<String>>);
    let reverse_config = TrimMessagesConfig {
        max_tokens: remaining_tokens,
        token_counter: &config.token_counter,
        strategy: TrimStrategy::First,
        allow_partial: config.allow_partial,
        include_system: false,
        end_on: config.start_on.clone(),
        start_on: None,
        text_splitter: splitter_wrapper,
    };

    let mut result = trim_messages_first(&messages, &reverse_config, true);

    result.reverse();

    if let Some(sys_msg) = system_message {
        result.insert(0, sys_msg);
    }

    result
}

fn create_message_with_content(original: &BaseMessage, content: &str) -> BaseMessage {
    match original {
        BaseMessage::Human(m) => BaseMessage::Human(
            HumanMessage::builder()
                .content(content)
                .maybe_id(m.id.clone())
                .build(),
        ),
        BaseMessage::AI(m) => BaseMessage::AI(
            AIMessage::builder()
                .content(content)
                .maybe_id(m.id.clone())
                .build(),
        ),
        BaseMessage::System(m) => BaseMessage::System(
            SystemMessage::builder()
                .content(content)
                .maybe_id(m.id.clone())
                .build(),
        ),
        BaseMessage::Tool(m) => {
            let new_msg = ToolMessage::builder()
                .content(content)
                .tool_call_id(&m.tool_call_id)
                .maybe_id(m.id.clone())
                .build();
            BaseMessage::Tool(new_msg)
        }
        BaseMessage::Chat(m) => BaseMessage::Chat(
            ChatMessage::builder()
                .content(content)
                .role(&m.role)
                .maybe_id(m.id.clone())
                .build(),
        ),
        BaseMessage::Function(m) => BaseMessage::Function(
            FunctionMessage::builder()
                .name(&m.name)
                .content(content)
                .maybe_id(m.id.clone())
                .build(),
        ),
        BaseMessage::Remove(m) => BaseMessage::Remove(RemoveMessage::builder().id(&m.id).build()),
    }
}

use crate::runnables::base::RunnableLambdaWithConfig;
use std::sync::Arc;

pub fn filter_messages_runnable(
    include_names: Option<Vec<String>>,
    exclude_names: Option<Vec<String>>,
    include_types: Option<Vec<String>>,
    exclude_types: Option<Vec<String>>,
    include_ids: Option<Vec<String>>,
    exclude_ids: Option<Vec<String>>,
    exclude_tool_calls: Option<ExcludeToolCalls>,
) -> RunnableLambdaWithConfig<Vec<BaseMessage>, Vec<BaseMessage>> {
    RunnableLambdaWithConfig::from_func_named(move |messages: Vec<BaseMessage>| {
        let include_names_refs: Option<Vec<&str>> = include_names
            .as_ref()
            .map(|v| v.iter().map(|s| s.as_str()).collect());
        let exclude_names_refs: Option<Vec<&str>> = exclude_names
            .as_ref()
            .map(|v| v.iter().map(|s| s.as_str()).collect());
        let include_types_refs: Option<Vec<&str>> = include_types
            .as_ref()
            .map(|v| v.iter().map(|s| s.as_str()).collect());
        let exclude_types_refs: Option<Vec<&str>> = exclude_types
            .as_ref()
            .map(|v| v.iter().map(|s| s.as_str()).collect());
        let include_ids_refs: Option<Vec<&str>> = include_ids
            .as_ref()
            .map(|v| v.iter().map(|s| s.as_str()).collect());
        let exclude_ids_refs: Option<Vec<&str>> = exclude_ids
            .as_ref()
            .map(|v| v.iter().map(|s| s.as_str()).collect());

        Ok(filter_messages(
            &messages,
            include_names_refs.as_deref(),
            exclude_names_refs.as_deref(),
            include_types_refs.as_deref(),
            exclude_types_refs.as_deref(),
            include_ids_refs.as_deref(),
            exclude_ids_refs.as_deref(),
            exclude_tool_calls.as_ref(),
        ))
    }, "filter_messages")
}

pub fn merge_message_runs_runnable(
    chunk_separator: Option<String>,
) -> RunnableLambdaWithConfig<Vec<BaseMessage>, Vec<BaseMessage>> {
    let separator = chunk_separator.unwrap_or_else(|| "\n".to_string());
    RunnableLambdaWithConfig::from_func_named(move |messages: Vec<BaseMessage>| {
        Ok(merge_message_runs(&messages, &separator))
    }, "merge_message_runs")
}

pub fn trim_messages_runnable<F, S>(
    config: TrimMessagesConfig<F, S>,
) -> RunnableLambdaWithConfig<Vec<BaseMessage>, Vec<BaseMessage>>
where
    F: Fn(&[BaseMessage]) -> usize + Send + Sync + 'static,
    S: Fn(&str) -> Vec<String> + Send + Sync + 'static,
{
    let config = Arc::new(config);
    RunnableLambdaWithConfig::from_func_named(move |messages: Vec<BaseMessage>| {
        Ok(trim_messages(&messages, &config))
    }, "trim_messages")
}
