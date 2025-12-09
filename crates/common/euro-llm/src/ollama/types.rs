//! Ollama-specific request and response types.

use crate::core::{self, ChatResponse, CompletionResponse, FinishReason, Metadata, Usage};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Ollama chat completion request.
#[derive(Debug, Clone, Serialize)]
pub struct OllamaChatRequest {
    pub model: String,
    pub messages: Vec<OllamaMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub keep_alive: Option<String>,
}

/// Ollama message format.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OllamaMessage {
    pub role: String,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub images: Option<Vec<String>>, // Base64 encoded images
}

/// Ollama chat completion response.
#[derive(Debug, Clone, Deserialize)]
pub struct OllamaChatResponse {
    pub model: String,
    pub created_at: String,
    pub message: OllamaMessage,
    pub done: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_duration: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub load_duration: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_eval_count: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_eval_duration: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub eval_count: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub eval_duration: Option<u64>,
}

/// Ollama generate (completion) request.
#[derive(Debug, Clone, Serialize)]
pub struct OllamaCompletionRequest {
    pub model: String,
    pub prompt: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub keep_alive: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<Vec<u32>>, // Context from previous requests
}

/// Ollama generate (completion) response.
#[derive(Debug, Clone, Deserialize)]
pub struct OllamaCompletionResponse {
    pub model: String,
    pub created_at: String,
    pub response: String,
    pub done: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<Vec<u32>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_duration: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub load_duration: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_eval_count: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_eval_duration: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub eval_count: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub eval_duration: Option<u64>,
}

/// Ollama embeddings request.
#[derive(Debug, Clone, Serialize)]
pub struct OllamaEmbeddingsRequest {
    pub model: String,
    pub prompt: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub keep_alive: Option<String>,
}

/// Ollama embeddings response.
#[derive(Debug, Clone, Deserialize)]
pub struct OllamaEmbeddingsResponse {
    pub embedding: Vec<f32>,
}

/// Ollama streaming response chunk.
#[derive(Debug, Clone, Deserialize)]
pub struct OllamaStreamChunk {
    pub model: String,
    pub created_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<OllamaMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response: Option<String>, // For completion streaming
    pub done: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_duration: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub load_duration: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_eval_count: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_eval_duration: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub eval_count: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub eval_duration: Option<u64>,
}

/// Ollama usage statistics (derived from timing information).
#[derive(Debug, Clone)]
pub struct OllamaUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

/// Ollama choice (for compatibility with other providers).
#[derive(Debug, Clone)]
pub struct OllamaChoice {
    pub index: u32,
    pub message: OllamaMessage,
    pub finish_reason: Option<String>,
}

/// Wrapper for Ollama chat response that includes converted generic data.
#[derive(Debug, Clone)]
pub struct OllamaChatResponseWrapper {
    pub response: OllamaChatResponse,
    pub converted_usage: Option<Usage>,
    pub converted_metadata: Metadata,
}

/// Wrapper for Ollama completion response that includes converted generic data.
#[derive(Debug, Clone)]
pub struct OllamaCompletionResponseWrapper {
    pub response: OllamaCompletionResponse,
    pub converted_usage: Option<Usage>,
    pub converted_metadata: Metadata,
}

impl OllamaChatResponseWrapper {
    pub fn new(response: OllamaChatResponse, request_id: Option<String>) -> Self {
        let converted_usage =
            if response.prompt_eval_count.is_some() || response.eval_count.is_some() {
                Some(Usage {
                    prompt_tokens: response.prompt_eval_count.unwrap_or(0),
                    completion_tokens: response.eval_count.unwrap_or(0),
                    total_tokens: response.prompt_eval_count.unwrap_or(0)
                        + response.eval_count.unwrap_or(0),
                })
            } else {
                None
            };

        let converted_metadata = Metadata {
            extensions: {
                let mut ext = HashMap::new();
                if let Some(total_duration) = response.total_duration {
                    ext.insert(
                        "total_duration_ns".to_string(),
                        serde_json::Value::Number(total_duration.into()),
                    );
                }
                if let Some(load_duration) = response.load_duration {
                    ext.insert(
                        "load_duration_ns".to_string(),
                        serde_json::Value::Number(load_duration.into()),
                    );
                }
                if let Some(prompt_eval_duration) = response.prompt_eval_duration {
                    ext.insert(
                        "prompt_eval_duration_ns".to_string(),
                        serde_json::Value::Number(prompt_eval_duration.into()),
                    );
                }
                if let Some(eval_duration) = response.eval_duration {
                    ext.insert(
                        "eval_duration_ns".to_string(),
                        serde_json::Value::Number(eval_duration.into()),
                    );
                }
                ext
            },
            request_id,
            user_id: None,
            created_at: parse_ollama_timestamp(&response.created_at).unwrap_or_else(Utc::now),
        };

        Self {
            response,
            converted_usage,
            converted_metadata,
        }
    }
}

impl OllamaCompletionResponseWrapper {
    pub fn new(response: OllamaCompletionResponse, request_id: Option<String>) -> Self {
        let converted_usage =
            if response.prompt_eval_count.is_some() || response.eval_count.is_some() {
                Some(Usage {
                    prompt_tokens: response.prompt_eval_count.unwrap_or(0),
                    completion_tokens: response.eval_count.unwrap_or(0),
                    total_tokens: response.prompt_eval_count.unwrap_or(0)
                        + response.eval_count.unwrap_or(0),
                })
            } else {
                None
            };

        let converted_metadata = Metadata {
            extensions: {
                let mut ext = HashMap::new();
                if let Some(total_duration) = response.total_duration {
                    ext.insert(
                        "total_duration_ns".to_string(),
                        serde_json::Value::Number(total_duration.into()),
                    );
                }
                if let Some(load_duration) = response.load_duration {
                    ext.insert(
                        "load_duration_ns".to_string(),
                        serde_json::Value::Number(load_duration.into()),
                    );
                }
                if let Some(prompt_eval_duration) = response.prompt_eval_duration {
                    ext.insert(
                        "prompt_eval_duration_ns".to_string(),
                        serde_json::Value::Number(prompt_eval_duration.into()),
                    );
                }
                if let Some(eval_duration) = response.eval_duration {
                    ext.insert(
                        "eval_duration_ns".to_string(),
                        serde_json::Value::Number(eval_duration.into()),
                    );
                }
                ext
            },
            request_id,
            user_id: None,
            created_at: parse_ollama_timestamp(&response.created_at).unwrap_or_else(Utc::now),
        };

        Self {
            response,
            converted_usage,
            converted_metadata,
        }
    }
}

// Implement ChatResponse for OllamaChatResponseWrapper
impl ChatResponse for OllamaChatResponseWrapper {
    fn content(&self) -> String {
        self.response.message.content.clone()
    }

    fn usage(&self) -> Option<Usage> {
        self.converted_usage.clone()
    }

    fn finish_reason(&self) -> Option<FinishReason> {
        if self.response.done {
            Some(FinishReason::Stop)
        } else {
            None
        }
    }

    fn metadata(&self) -> Metadata {
        self.converted_metadata.clone()
    }

    fn tool_calls(&self) -> Option<Vec<core::ToolCall>> {
        // Ollama doesn't support tool calls in the same way as OpenAI
        None
    }
}

// Implement CompletionResponse for OllamaCompletionResponseWrapper
impl CompletionResponse for OllamaCompletionResponseWrapper {
    fn text(&self) -> String {
        self.response.response.clone()
    }

    fn usage(&self) -> Option<Usage> {
        self.converted_usage.clone()
    }

    fn finish_reason(&self) -> Option<FinishReason> {
        if self.response.done {
            Some(FinishReason::Stop)
        } else {
            None
        }
    }

    fn metadata(&self) -> Metadata {
        self.converted_metadata.clone()
    }
}

// Implement ChatResponse for OllamaChatResponse (direct implementation)
impl ChatResponse for OllamaChatResponse {
    fn content(&self) -> String {
        self.message.content.clone()
    }

    fn usage(&self) -> Option<Usage> {
        // Convert from Ollama usage if available
        Some(Usage {
            prompt_tokens: self.prompt_eval_count.unwrap_or(0),
            completion_tokens: self.eval_count.unwrap_or(0),
            total_tokens: self.prompt_eval_count.unwrap_or(0) + self.eval_count.unwrap_or(0),
        })
    }

    fn finish_reason(&self) -> Option<FinishReason> {
        if self.done {
            Some(FinishReason::Stop)
        } else {
            None
        }
    }

    fn metadata(&self) -> Metadata {
        let mut extensions = HashMap::new();
        if let Some(total_duration) = self.total_duration {
            extensions.insert(
                "total_duration_ns".to_string(),
                serde_json::Value::Number(total_duration.into()),
            );
        }
        if let Some(load_duration) = self.load_duration {
            extensions.insert(
                "load_duration_ns".to_string(),
                serde_json::Value::Number(load_duration.into()),
            );
        }

        Metadata {
            extensions,
            request_id: None,
            user_id: None,
            created_at: parse_ollama_timestamp(&self.created_at).unwrap_or_else(Utc::now),
        }
    }

    fn tool_calls(&self) -> Option<Vec<core::ToolCall>> {
        None
    }
}

// Implement CompletionResponse for OllamaCompletionResponse (direct implementation)
impl CompletionResponse for OllamaCompletionResponse {
    fn text(&self) -> String {
        self.response.clone()
    }

    fn usage(&self) -> Option<Usage> {
        // Convert from Ollama usage if available
        Some(Usage {
            prompt_tokens: self.prompt_eval_count.unwrap_or(0),
            completion_tokens: self.eval_count.unwrap_or(0),
            total_tokens: self.prompt_eval_count.unwrap_or(0) + self.eval_count.unwrap_or(0),
        })
    }

    fn finish_reason(&self) -> Option<FinishReason> {
        if self.done {
            Some(FinishReason::Stop)
        } else {
            None
        }
    }

    fn metadata(&self) -> Metadata {
        let mut extensions = HashMap::new();
        if let Some(total_duration) = self.total_duration {
            extensions.insert(
                "total_duration_ns".to_string(),
                serde_json::Value::Number(total_duration.into()),
            );
        }
        if let Some(load_duration) = self.load_duration {
            extensions.insert(
                "load_duration_ns".to_string(),
                serde_json::Value::Number(load_duration.into()),
            );
        }

        Metadata {
            extensions,
            request_id: None,
            user_id: None,
            created_at: parse_ollama_timestamp(&self.created_at).unwrap_or_else(Utc::now),
        }
    }
}

// Conversion utilities
impl From<&core::Message> for OllamaMessage {
    fn from(message: &core::Message) -> Self {
        let role = match message.role {
            core::Role::User => "user".to_string(),
            core::Role::Assistant => "assistant".to_string(),
            core::Role::System => "system".to_string(),
            core::Role::Tool => "tool".to_string(),
        };

        let content = match &message.content {
            core::MessageContent::Text(text) => text.clone(),
            core::MessageContent::Multimodal(parts) => {
                // Extract text parts and collect images
                let text_parts: Vec<String> = parts
                    .iter()
                    .filter_map(|part| match part {
                        core::ContentPart::Text { text } => Some(text.clone()),
                        _ => None,
                    })
                    .collect();
                text_parts.join("\n")
            }
            core::MessageContent::Tool(tool_content) => {
                // Handle tool content - use text if available, otherwise create a placeholder
                tool_content
                    .text
                    .as_deref()
                    .unwrap_or("[Tool response]")
                    .to_string()
            }
        };

        // Extract images from multimodal content
        let images = match &message.content {
            core::MessageContent::Multimodal(parts) => {
                let image_data: Vec<String> = parts
                    .iter()
                    .filter_map(|part| match part {
                        core::ContentPart::Image { image_source, .. } => {
                            let url: String = image_source.clone().into();
                            Some(url)
                        }
                        _ => None,
                    })
                    .collect();
                if image_data.is_empty() {
                    None
                } else {
                    Some(image_data)
                }
            }
            _ => None,
        };

        Self {
            role,
            content,
            images,
        }
    }
}

impl From<OllamaUsage> for Usage {
    fn from(ollama_usage: OllamaUsage) -> Self {
        Self {
            prompt_tokens: ollama_usage.prompt_tokens,
            completion_tokens: ollama_usage.completion_tokens,
            total_tokens: ollama_usage.total_tokens,
        }
    }
}

impl From<&OllamaUsage> for Usage {
    fn from(ollama_usage: &OllamaUsage) -> Self {
        Self {
            prompt_tokens: ollama_usage.prompt_tokens,
            completion_tokens: ollama_usage.completion_tokens,
            total_tokens: ollama_usage.total_tokens,
        }
    }
}

/// Parse Ollama timestamp format (RFC3339).
fn parse_ollama_timestamp(timestamp: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(timestamp)
        .map(|dt| dt.with_timezone(&Utc))
        .ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_conversion() {
        let core_message = core::Message::user("Hello, world!");
        let ollama_message = OllamaMessage::from(&core_message);

        assert_eq!(ollama_message.role, "user");
        assert_eq!(ollama_message.content, "Hello, world!");
        assert!(ollama_message.images.is_none());
    }

    #[test]
    fn test_usage_conversion() {
        let ollama_usage = OllamaUsage {
            prompt_tokens: 10,
            completion_tokens: 20,
            total_tokens: 30,
        };

        let core_usage = Usage::from(ollama_usage);
        assert_eq!(core_usage.prompt_tokens, 10);
        assert_eq!(core_usage.completion_tokens, 20);
        assert_eq!(core_usage.total_tokens, 30);
    }

    #[test]
    fn test_timestamp_parsing() {
        let timestamp = "2023-12-07T14:30:00Z";
        let parsed = parse_ollama_timestamp(timestamp);
        assert!(parsed.is_some());
    }

    #[test]
    fn test_chat_response_wrapper() {
        let response = OllamaChatResponse {
            model: "llama2".to_string(),
            created_at: "2023-12-07T14:30:00Z".to_string(),
            message: OllamaMessage {
                role: "assistant".to_string(),
                content: "Hello!".to_string(),
                images: None,
            },
            done: true,
            total_duration: Some(1000000),
            load_duration: None,
            prompt_eval_count: Some(5),
            prompt_eval_duration: Some(500000),
            eval_count: Some(3),
            eval_duration: Some(300000),
        };

        let wrapper = OllamaChatResponseWrapper::new(response, Some("test-123".to_string()));

        assert_eq!(wrapper.content(), "Hello!");
        assert!(wrapper.usage().is_some());
        assert_eq!(wrapper.usage().unwrap().prompt_tokens, 5);
        assert_eq!(wrapper.usage().unwrap().completion_tokens, 3);
        assert_eq!(wrapper.metadata().request_id, Some("test-123".to_string()));
    }
}
