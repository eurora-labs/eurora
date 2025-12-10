//! OpenAI-specific request and response types.

use crate::*;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// OpenAI chat completion request.
#[derive(Debug, Clone, Serialize)]
pub struct OpenAIChatRequest {
    pub model: String,
    pub messages: Vec<OpenAIMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frequency_penalty: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub presence_penalty: Option<f32>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub stop: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<OpenAITool>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<String>,
}

/// OpenAI message format.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAIMessage {
    pub role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<OpenAIToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

/// OpenAI tool call format.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAIToolCall {
    pub id: String,
    #[serde(rename = "type")]
    pub call_type: String,
    pub function: OpenAIFunctionCall,
}

/// OpenAI function call format.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAIFunctionCall {
    pub name: String,
    pub arguments: String,
}

/// OpenAI tool definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAITool {
    #[serde(rename = "type")]
    pub tool_type: String,
    pub function: OpenAIFunction,
}

/// OpenAI function definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAIFunction {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

/// OpenAI chat completion response.
#[derive(Debug, Clone, Deserialize)]
pub struct OpenAIChatResponse {
    pub id: String,
    pub object: String,
    pub created: u64,
    pub model: String,
    pub choices: Vec<OpenAIChatChoice>,
    pub usage: Option<OpenAIUsage>,
    pub system_fingerprint: Option<String>,
}

/// OpenAI chat choice.
#[derive(Debug, Clone, Deserialize)]
pub struct OpenAIChatChoice {
    pub index: u32,
    pub message: OpenAIMessage,
    pub finish_reason: Option<String>,
    pub logprobs: Option<serde_json::Value>,
}

/// OpenAI usage statistics.
#[derive(Debug, Clone, Deserialize)]
pub struct OpenAIUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

/// OpenAI embeddings usage statistics (no completion_tokens).
#[derive(Debug, Clone, Deserialize)]
pub struct OpenAIEmbeddingsUsage {
    pub prompt_tokens: u32,
    pub total_tokens: u32,
}

/// OpenAI completion request.
#[derive(Debug, Clone, Serialize)]
pub struct OpenAICompletionRequest {
    pub model: String,
    pub prompt: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frequency_penalty: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub presence_penalty: Option<f32>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub stop: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<String>,
}

/// OpenAI completion response.
#[derive(Debug, Clone, Deserialize)]
pub struct OpenAICompletionResponse {
    pub id: String,
    pub object: String,
    pub created: u64,
    pub model: String,
    pub choices: Vec<OpenAICompletionChoice>,
    pub usage: Option<OpenAIUsage>,
}

/// OpenAI completion choice.
#[derive(Debug, Clone, Deserialize)]
pub struct OpenAICompletionChoice {
    pub index: u32,
    pub text: String,
    pub finish_reason: Option<String>,
    pub logprobs: Option<serde_json::Value>,
}

/// OpenAI embeddings request.
#[derive(Debug, Clone, Serialize)]
pub struct OpenAIEmbeddingsRequest {
    pub model: String,
    pub input: serde_json::Value, // Can be string or array of strings
    #[serde(skip_serializing_if = "Option::is_none")]
    pub encoding_format: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dimensions: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<String>,
}

/// OpenAI embeddings response.
#[derive(Debug, Clone, Deserialize)]
pub struct OpenAIEmbeddingsResponse {
    pub object: String,
    pub data: Vec<OpenAIEmbedding>,
    pub model: String,
    pub usage: OpenAIEmbeddingsUsage,
}

/// OpenAI embedding data.
#[derive(Debug, Clone, Deserialize)]
pub struct OpenAIEmbedding {
    pub object: String,
    pub index: usize,
    pub embedding: Vec<f32>,
}

/// OpenAI streaming response chunk.
#[derive(Debug, Clone, Deserialize)]
pub struct OpenAIStreamChunk {
    pub id: String,
    pub object: String,
    pub created: u64,
    pub model: String,
    pub choices: Vec<OpenAIStreamChoice>,
}

/// OpenAI streaming choice.
#[derive(Debug, Clone, Deserialize)]
pub struct OpenAIStreamChoice {
    pub index: u32,
    pub delta: OpenAIStreamDelta,
    pub finish_reason: Option<String>,
}

/// OpenAI streaming delta.
#[derive(Debug, Clone, Deserialize)]
pub struct OpenAIStreamDelta {
    pub role: Option<String>,
    pub content: Option<String>,
    pub tool_calls: Option<Vec<OpenAIStreamToolCall>>,
}

/// OpenAI streaming tool call.
#[derive(Debug, Clone, Deserialize)]
pub struct OpenAIStreamToolCall {
    pub index: u32,
    pub id: Option<String>,
    #[serde(rename = "type")]
    pub call_type: Option<String>,
    pub function: Option<OpenAIStreamFunction>,
}

/// OpenAI streaming function.
#[derive(Debug, Clone, Deserialize)]
pub struct OpenAIStreamFunction {
    pub name: Option<String>,
    pub arguments: Option<String>,
}

/// Wrapper for OpenAI chat response that includes converted generic data
#[derive(Debug, Clone)]
pub struct OpenAIChatResponseWrapper {
    pub response: OpenAIChatResponse,
    pub converted_usage: Option<Usage>,
    pub converted_metadata: Metadata,
    pub converted_tool_calls: Option<Vec<ToolCall>>,
}

/// Wrapper for OpenAI completion response that includes converted generic data
#[derive(Debug, Clone)]
pub struct OpenAICompletionResponseWrapper {
    pub response: OpenAICompletionResponse,
    pub converted_usage: Option<Usage>,
    pub converted_metadata: Metadata,
}

impl OpenAIChatResponseWrapper {
    pub fn new(response: OpenAIChatResponse, request_id: Option<String>) -> Self {
        let converted_usage = response.usage.as_ref().map(|usage| Usage {
            prompt_tokens: usage.prompt_tokens,
            completion_tokens: usage.completion_tokens,
            total_tokens: usage.total_tokens,
        });

        let converted_metadata = Metadata {
            extensions: HashMap::new(),
            request_id,
            user_id: None,
            created_at: DateTime::from_timestamp(response.created as i64, 0)
                .unwrap_or_else(Utc::now),
        };

        let converted_tool_calls = response
            .choices
            .first()
            .and_then(|choice| choice.message.tool_calls.as_ref())
            .map(|tool_calls| {
                tool_calls
                    .iter()
                    .map(|tc| ToolCall {
                        id: tc.id.clone(),
                        call_type: tc.call_type.clone(),
                        function: FunctionCall {
                            name: tc.function.name.clone(),
                            arguments: tc.function.arguments.clone(),
                        },
                    })
                    .collect()
            });

        Self {
            response,
            converted_usage,
            converted_metadata,
            converted_tool_calls,
        }
    }
}

impl OpenAICompletionResponseWrapper {
    pub fn new(response: OpenAICompletionResponse, request_id: Option<String>) -> Self {
        let converted_usage = response.usage.as_ref().map(|usage| Usage {
            prompt_tokens: usage.prompt_tokens,
            completion_tokens: usage.completion_tokens,
            total_tokens: usage.total_tokens,
        });

        let converted_metadata = Metadata {
            extensions: HashMap::new(),
            request_id,
            user_id: None,
            created_at: DateTime::from_timestamp(response.created as i64, 0)
                .unwrap_or_else(Utc::now),
        };

        Self {
            response,
            converted_usage,
            converted_metadata,
        }
    }
}

// Implement ChatResponse for OpenAIChatResponseWrapper
impl ChatResponse for OpenAIChatResponseWrapper {
    fn content(&self) -> String {
        self.response
            .choices
            .first()
            .and_then(|choice| match &choice.message.content {
                Some(serde_json::Value::String(s)) => Some(s.clone()),
                _ => None,
            })
            .unwrap_or_default()
    }

    fn usage(&self) -> Option<Usage> {
        self.converted_usage.clone()
    }

    fn finish_reason(&self) -> Option<FinishReason> {
        self.response
            .choices
            .first()
            .and_then(|choice| choice.finish_reason.as_ref())
            .and_then(|reason| match reason.as_str() {
                "stop" => Some(FinishReason::Stop),
                "length" => Some(FinishReason::Length),
                "tool_calls" => Some(FinishReason::ToolCalls),
                "content_filter" => Some(FinishReason::ContentFilter),
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

// Implement CompletionResponse for OpenAICompletionResponseWrapper
impl CompletionResponse for OpenAICompletionResponseWrapper {
    fn text(&self) -> String {
        self.response
            .choices
            .first()
            .map(|choice| choice.text.clone())
            .unwrap_or_default()
    }

    fn usage(&self) -> Option<Usage> {
        self.converted_usage.clone()
    }

    fn finish_reason(&self) -> Option<FinishReason> {
        self.response
            .choices
            .first()
            .and_then(|choice| choice.finish_reason.as_ref())
            .and_then(|reason| match reason.as_str() {
                "stop" => Some(FinishReason::Stop),
                "length" => Some(FinishReason::Length),
                _ => None,
            })
    }

    fn metadata(&self) -> Metadata {
        self.converted_metadata.clone()
    }
}

// Implement ChatResponse for OpenAIChatResponse
impl ChatResponse for OpenAIChatResponse {
    fn content(&self) -> String {
        self.choices
            .first()
            .and_then(|choice| choice.message.content.as_ref())
            .and_then(|content| content.as_str())
            .unwrap_or("")
            .to_string()
    }

    fn usage(&self) -> Option<Usage> {
        self.usage.as_ref().map(|usage| Usage {
            prompt_tokens: usage.prompt_tokens,
            completion_tokens: usage.completion_tokens,
            total_tokens: usage.total_tokens,
        })
    }

    fn finish_reason(&self) -> Option<FinishReason> {
        self.choices
            .first()
            .and_then(|choice| choice.finish_reason.as_ref())
            .and_then(|reason| match reason.as_str() {
                "stop" => Some(FinishReason::Stop),
                "length" => Some(FinishReason::Length),
                "tool_calls" => Some(FinishReason::ToolCalls),
                "content_filter" => Some(FinishReason::ContentFilter),
                _ => None,
            })
    }

    fn metadata(&self) -> Metadata {
        Metadata {
            extensions: HashMap::new(),
            request_id: Some(self.id.clone()),
            user_id: None,
            created_at: DateTime::from_timestamp(self.created as i64, 0).unwrap_or_else(Utc::now),
        }
    }

    fn tool_calls(&self) -> Option<Vec<ToolCall>> {
        self.choices
            .first()
            .and_then(|choice| choice.message.tool_calls.as_ref())
            .map(|tool_calls| {
                tool_calls
                    .iter()
                    .map(|tc| ToolCall {
                        id: tc.id.clone(),
                        call_type: tc.call_type.clone(),
                        function: FunctionCall {
                            name: tc.function.name.clone(),
                            arguments: tc.function.arguments.clone(),
                        },
                    })
                    .collect()
            })
    }
}

// Implement CompletionResponse for OpenAICompletionResponse
impl CompletionResponse for OpenAICompletionResponse {
    fn text(&self) -> String {
        self.choices
            .first()
            .map(|choice| choice.text.clone())
            .unwrap_or_default()
    }

    fn usage(&self) -> Option<Usage> {
        self.usage.as_ref().map(|usage| Usage {
            prompt_tokens: usage.prompt_tokens,
            completion_tokens: usage.completion_tokens,
            total_tokens: usage.total_tokens,
        })
    }

    fn finish_reason(&self) -> Option<FinishReason> {
        self.choices
            .first()
            .and_then(|choice| choice.finish_reason.as_ref())
            .and_then(|reason| match reason.as_str() {
                "stop" => Some(FinishReason::Stop),
                "length" => Some(FinishReason::Length),
                _ => None,
            })
    }

    fn metadata(&self) -> Metadata {
        Metadata {
            extensions: HashMap::new(),
            request_id: Some(self.id.clone()),
            user_id: None,
            created_at: DateTime::from_timestamp(self.created as i64, 0).unwrap_or_else(Utc::now),
        }
    }
}

// Conversion utilities
impl From<&Message> for OpenAIMessage {
    fn from(message: &Message) -> Self {
        let role = match message.role {
            Role::User => "user".to_string(),
            Role::Assistant => "assistant".to_string(),
            Role::System => "system".to_string(),
            Role::Tool => "tool".to_string(),
        };

        let content = match &message.content {
            MessageContent::Text(text) => Some(serde_json::Value::String(text.clone())),
            MessageContent::Multimodal(parts) => {
                let content_array: Vec<serde_json::Value> = parts
                    .iter()
                    .map(|part| match part {
                        ContentPart::Text { text } => serde_json::json!({
                            "type": "text",
                            "text": text
                        }),
                        ContentPart::Image {
                            image_source,
                            detail,
                        } => {
                            let url: String = image_source.clone().into();
                            serde_json::json!({
                                "type": "image_url",
                                "image_url": {
                                    "url": url,
                                    "detail": detail.as_deref().unwrap_or("auto")
                                }
                            })
                        }
                        ContentPart::Audio { audio_url, format } => {
                            serde_json::json!({
                                "type": "audio",
                                "audio": {
                                    "mime_type": format
                                    .as_deref()
                                    .map(|f| format!("audio/{f}"))
                                    .unwrap_or_else(|| "audio/mpeg".to_string()),
                                    "segments": [
                                        {
                                            "url": audio_url,
                                            // Add `"caption": "<caption>"` here if available
                                        }
                                    ]
                                }
                            })
                        }
                    })
                    .collect();
                Some(serde_json::Value::Array(content_array))
            }
            MessageContent::Tool(tool_content) => {
                // Handle tool content - use text if available, otherwise create a placeholder
                let text = tool_content.text.as_deref().unwrap_or("[Tool response]");
                Some(serde_json::Value::String(text.to_string()))
            }
        };

        // Extract tool information from MessageContent::Tool if present
        let (tool_calls, tool_call_id) = match &message.content {
            MessageContent::Tool(tool_content) => {
                let tool_calls = tool_content.tool_calls.as_ref().map(|calls| {
                    calls
                        .iter()
                        .map(|call| OpenAIToolCall {
                            id: call.id.clone(),
                            call_type: call.call_type.clone(),
                            function: OpenAIFunctionCall {
                                name: call.function.name.clone(),
                                arguments: call.function.arguments.clone(),
                            },
                        })
                        .collect()
                });
                (tool_calls, tool_content.tool_call_id.clone())
            }
            _ => (None, None),
        };

        Self {
            role,
            content,
            name: None, // Name field removed from core Message
            tool_calls,
            tool_call_id,
        }
    }
}

impl From<&Tool> for OpenAITool {
    fn from(tool: &Tool) -> Self {
        Self {
            tool_type: tool.tool_type.clone(),
            function: OpenAIFunction {
                name: tool.function.name.clone(),
                description: tool.function.description.clone(),
                parameters: tool.function.parameters.clone(),
            },
        }
    }
}

// Conversion from OpenAI types to core types
impl From<OpenAIUsage> for Usage {
    fn from(openai_usage: OpenAIUsage) -> Self {
        Self {
            prompt_tokens: openai_usage.prompt_tokens,
            completion_tokens: openai_usage.completion_tokens,
            total_tokens: openai_usage.total_tokens,
        }
    }
}

impl From<&OpenAIUsage> for Usage {
    fn from(openai_usage: &OpenAIUsage) -> Self {
        Self {
            prompt_tokens: openai_usage.prompt_tokens,
            completion_tokens: openai_usage.completion_tokens,
            total_tokens: openai_usage.total_tokens,
        }
    }
}

impl From<OpenAIToolCall> for ToolCall {
    fn from(openai_tool_call: OpenAIToolCall) -> Self {
        Self {
            id: openai_tool_call.id,
            call_type: openai_tool_call.call_type,
            function: FunctionCall {
                name: openai_tool_call.function.name,
                arguments: openai_tool_call.function.arguments,
            },
        }
    }
}

impl From<&OpenAIToolCall> for ToolCall {
    fn from(openai_tool_call: &OpenAIToolCall) -> Self {
        Self {
            id: openai_tool_call.id.clone(),
            call_type: openai_tool_call.call_type.clone(),
            function: FunctionCall {
                name: openai_tool_call.function.name.clone(),
                arguments: openai_tool_call.function.arguments.clone(),
            },
        }
    }
}
