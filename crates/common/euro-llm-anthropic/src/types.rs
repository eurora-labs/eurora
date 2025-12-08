//! Anthropic-specific request and response types.

use chrono::Utc;
use euro_llm_core::{ChatResponse, FinishReason, FunctionCall, Metadata, ToolCall, Usage};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Anthropic messages request.
#[derive(Debug, Clone, Serialize)]
pub struct AnthropicMessagesRequest {
    pub model: String,
    pub max_tokens: u32,
    pub messages: Vec<AnthropicMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_k: Option<u32>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub stop_sequences: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<AnthropicTool>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<AnthropicToolChoice>,
}

/// Anthropic message format.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnthropicMessage {
    pub role: String,
    pub content: AnthropicContent,
}

/// Anthropic content can be text or array of content blocks.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum AnthropicContent {
    Text(String),
    Blocks(Vec<AnthropicContentBlock>),
}

/// Anthropic content block.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum AnthropicContentBlock {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "image")]
    Image { source: AnthropicImageSource },
    #[serde(rename = "tool_use")]
    ToolUse {
        id: String,
        name: String,
        input: serde_json::Value,
    },
    #[serde(rename = "tool_result")]
    ToolResult {
        tool_use_id: String,
        content: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        is_error: Option<bool>,
    },
}

/// Anthropic image source.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnthropicImageSource {
    #[serde(rename = "type")]
    pub source_type: String, // "base64"
    pub media_type: String, // "image/jpeg", "image/png", etc.
    pub data: String,       // base64 encoded image data
}

/// Anthropic tool definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnthropicTool {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
}

/// Anthropic tool choice.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum AnthropicToolChoice {
    Auto,
    Any,
    Tool { name: String },
}

/// Anthropic messages response.
#[derive(Debug, Clone, Deserialize)]
pub struct AnthropicMessagesResponse {
    pub id: String,
    #[serde(rename = "type")]
    pub response_type: String,
    pub role: String,
    pub content: Vec<AnthropicContentBlock>,
    pub model: String,
    pub stop_reason: Option<String>,
    pub stop_sequence: Option<String>,
    pub usage: AnthropicUsage,
}

/// Anthropic usage statistics.
#[derive(Debug, Clone, Deserialize)]
pub struct AnthropicUsage {
    pub input_tokens: u32,
    pub output_tokens: u32,
}

/// Anthropic streaming response chunk.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type")]
pub enum AnthropicStreamChunk {
    #[serde(rename = "message_start")]
    MessageStart { message: AnthropicStreamMessage },
    #[serde(rename = "content_block_start")]
    ContentBlockStart {
        index: u32,
        content_block: AnthropicContentBlock,
    },
    #[serde(rename = "content_block_delta")]
    ContentBlockDelta {
        index: u32,
        delta: AnthropicContentDelta,
    },
    #[serde(rename = "content_block_stop")]
    ContentBlockStop { index: u32 },
    #[serde(rename = "message_delta")]
    MessageDelta {
        delta: AnthropicMessageDelta,
        usage: AnthropicUsage,
    },
    #[serde(rename = "message_stop")]
    MessageStop,
    #[serde(rename = "ping")]
    Ping,
    #[serde(rename = "error")]
    Error {
        error: crate::error::AnthropicErrorDetail,
    },
}

/// Anthropic streaming message.
#[derive(Debug, Clone, Deserialize)]
pub struct AnthropicStreamMessage {
    pub id: String,
    #[serde(rename = "type")]
    pub message_type: String,
    pub role: String,
    pub content: Vec<serde_json::Value>,
    pub model: String,
    pub stop_reason: Option<String>,
    pub stop_sequence: Option<String>,
    pub usage: AnthropicUsage,
}

/// Anthropic content delta for streaming.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type")]
pub enum AnthropicContentDelta {
    #[serde(rename = "text_delta")]
    TextDelta { text: String },
    #[serde(rename = "input_json_delta")]
    InputJsonDelta { partial_json: String },
}

/// Anthropic message delta for streaming.
#[derive(Debug, Clone, Deserialize)]
pub struct AnthropicMessageDelta {
    pub stop_reason: Option<String>,
    pub stop_sequence: Option<String>,
}

/// Wrapper for Anthropic messages response that includes converted generic data
#[derive(Debug, Clone)]
pub struct AnthropicMessagesResponseWrapper {
    pub response: AnthropicMessagesResponse,
    pub converted_usage: Usage,
    pub converted_metadata: Metadata,
    pub converted_tool_calls: Option<Vec<ToolCall>>,
}

impl AnthropicMessagesResponseWrapper {
    pub fn new(response: AnthropicMessagesResponse, request_id: Option<String>) -> Self {
        let converted_usage = Usage {
            prompt_tokens: response.usage.input_tokens,
            completion_tokens: response.usage.output_tokens,
            total_tokens: response.usage.input_tokens + response.usage.output_tokens,
        };

        let converted_metadata = Metadata {
            extensions: HashMap::new(),
            request_id,
            user_id: None,
            created_at: Utc::now(), // Anthropic doesn't provide timestamp
        };

        let converted_tool_calls = extract_tool_calls(&response.content);

        Self {
            response,
            converted_usage,
            converted_metadata,
            converted_tool_calls,
        }
    }
}

/// Extract tool calls from Anthropic content blocks.
fn extract_tool_calls(content: &[AnthropicContentBlock]) -> Option<Vec<ToolCall>> {
    let tool_calls: Vec<ToolCall> = content
        .iter()
        .filter_map(|block| match block {
            AnthropicContentBlock::ToolUse { id, name, input } => Some(ToolCall {
                id: id.clone(),
                call_type: "function".to_string(),
                function: FunctionCall {
                    name: name.clone(),
                    arguments: input.to_string(),
                },
            }),
            _ => None,
        })
        .collect();

    if tool_calls.is_empty() {
        None
    } else {
        Some(tool_calls)
    }
}

/// Extract text content from Anthropic content blocks.
fn extract_text_content(content: &[AnthropicContentBlock]) -> String {
    content
        .iter()
        .filter_map(|block| match block {
            AnthropicContentBlock::Text { text } => Some(text.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("")
}

// Implement ChatResponse for AnthropicMessagesResponseWrapper
impl ChatResponse for AnthropicMessagesResponseWrapper {
    fn content(&self) -> String {
        extract_text_content(&self.response.content)
    }

    fn usage(&self) -> Option<Usage> {
        Some(self.converted_usage.clone())
    }

    fn finish_reason(&self) -> Option<FinishReason> {
        self.response
            .stop_reason
            .as_ref()
            .and_then(|reason| match reason.as_str() {
                "end_turn" => Some(FinishReason::Stop),
                "max_tokens" => Some(FinishReason::Length),
                "stop_sequence" => Some(FinishReason::Stop),
                "tool_use" => Some(FinishReason::ToolCalls),
                _ => None,
            })
    }

    fn metadata(&self) -> Metadata {
        self.converted_metadata.clone()
    }

    fn tool_calls(&self) -> Option<Vec<ToolCall>> {
        self.converted_tool_calls.clone()
    }
}

// Implement ChatResponse for AnthropicMessagesResponse
impl ChatResponse for AnthropicMessagesResponse {
    fn content(&self) -> String {
        extract_text_content(&self.content)
    }

    fn usage(&self) -> Option<Usage> {
        Some(Usage {
            prompt_tokens: self.usage.input_tokens,
            completion_tokens: self.usage.output_tokens,
            total_tokens: self.usage.input_tokens + self.usage.output_tokens,
        })
    }

    fn finish_reason(&self) -> Option<FinishReason> {
        self.stop_reason
            .as_ref()
            .and_then(|reason| match reason.as_str() {
                "end_turn" => Some(FinishReason::Stop),
                "max_tokens" => Some(FinishReason::Length),
                "stop_sequence" => Some(FinishReason::StopSequence),
                "tool_use" => Some(FinishReason::ToolCalls),
                _ => None,
            })
    }

    fn metadata(&self) -> Metadata {
        Metadata {
            extensions: HashMap::new(),
            request_id: Some(self.id.clone()),
            user_id: None,
            created_at: Utc::now(), // Anthropic doesn't provide timestamp
        }
    }

    fn tool_calls(&self) -> Option<Vec<ToolCall>> {
        extract_tool_calls(&self.content)
    }
}

// Conversion utilities
impl From<&euro_llm_core::Message> for AnthropicMessage {
    fn from(message: &euro_llm_core::Message) -> Self {
        let role = match message.role {
            euro_llm_core::Role::User => "user".to_string(),
            euro_llm_core::Role::Assistant => "assistant".to_string(),
            // Anthropic handles system messages differently - they go in the system field
            euro_llm_core::Role::System => "system".to_string(),
            euro_llm_core::Role::Tool => "user".to_string(), // Tool results become user messages
        };

        let content = match &message.content {
            euro_llm_core::MessageContent::Text(text) => AnthropicContent::Text(text.clone()),
            euro_llm_core::MessageContent::Multimodal(parts) => {
                let blocks: Vec<AnthropicContentBlock> = parts
                    .iter()
                    .map(|part| match part {
                        euro_llm_core::ContentPart::Text { text } => {
                            AnthropicContentBlock::Text { text: text.clone() }
                        }
                        euro_llm_core::ContentPart::Image { image_source, .. } => {
                            let url: String = image_source.clone().into();
                            // Parse the image URL to determine if it's a data URI or external URL
                            if url.starts_with("data:") {
                                // Parse data URI to extract media type and base64 data
                                // Format: data:image/jpeg;base64,<data>
                                let parts: Vec<&str> = url.splitn(2, ',').collect();
                                if parts.len() == 2 {
                                    let header = parts[0];
                                    let data = parts[1];
                                    let media_type = header
                                        .strip_prefix("data:")
                                        .and_then(|s| s.split(';').next())
                                        .unwrap_or("image/jpeg");
                                    AnthropicContentBlock::Image {
                                        source: AnthropicImageSource {
                                            source_type: "base64".to_string(),
                                            media_type: media_type.to_string(),
                                            data: data.to_string(),
                                        },
                                    }
                                } else {
                                    // Invalid data URI, convert to text
                                    AnthropicContentBlock::Text {
                                        text: "[Invalid image data URI]".to_string(),
                                    }
                                }
                            } else {
                                // External URL - needs to be downloaded and converted to base64
                                // For now, return a placeholder
                                AnthropicContentBlock::Text {
                                    text: format!("[Image URL not supported: {url}]"),
                                }
                            }
                        }

                        euro_llm_core::ContentPart::Audio { audio_url, .. } => {
                            // Anthropic doesn't support audio in the same way, convert to text description
                            AnthropicContentBlock::Text {
                                text: format!("[Audio content: {audio_url}]"),
                            }
                        }
                    })
                    .collect();
                AnthropicContent::Blocks(blocks)
            }
            euro_llm_core::MessageContent::Tool(tool_content) => {
                // Handle tool content - use text if available, otherwise create a placeholder
                let text = tool_content.text.as_deref().unwrap_or("[Tool response]");
                AnthropicContent::Text(text.to_string())
            }
        };

        Self { role, content }
    }
}

impl From<&euro_llm_core::Tool> for AnthropicTool {
    fn from(tool: &euro_llm_core::Tool) -> Self {
        Self {
            name: tool.function.name.clone(),
            description: tool.function.description.clone(),
            input_schema: tool.function.parameters.clone(),
        }
    }
}

impl From<AnthropicUsage> for Usage {
    fn from(anthropic_usage: AnthropicUsage) -> Self {
        Self {
            prompt_tokens: anthropic_usage.input_tokens,
            completion_tokens: anthropic_usage.output_tokens,
            total_tokens: anthropic_usage.input_tokens + anthropic_usage.output_tokens,
        }
    }
}

impl From<&AnthropicUsage> for Usage {
    fn from(anthropic_usage: &AnthropicUsage) -> Self {
        Self {
            prompt_tokens: anthropic_usage.input_tokens,
            completion_tokens: anthropic_usage.output_tokens,
            total_tokens: anthropic_usage.input_tokens + anthropic_usage.output_tokens,
        }
    }
}
