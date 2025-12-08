//! Core types for LLM requests and responses.
//!
//! This module defines standardized types that are used across all providers,
//! including request/response structures, messages, and common data types.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{collections::HashMap, fmt};

#[cfg(feature = "specta")]
use specta::Type;

/// A chat request containing messages and parameters.
#[cfg_attr(feature = "specta", derive(Type))]
#[derive(Debug, Clone, Serialize, Deserialize)]

pub struct ChatRequest {
    /// The conversation messages
    pub messages: Vec<Message>,
    /// Request parameters (temperature, max_tokens, etc.)
    pub parameters: Parameters,
    /// Additional metadata and provider-specific extensions
    pub metadata: Metadata,
}

/// A completion request for non-chat text generation.
#[cfg_attr(feature = "specta", derive(Type))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionRequest {
    /// The text prompt to complete
    pub prompt: String,
    /// Request parameters
    pub parameters: Parameters,
    /// Additional metadata
    pub metadata: Metadata,
}

/// Common parameters used across providers.
#[cfg_attr(feature = "specta", derive(Type))]
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Parameters {
    /// Controls randomness in the response (0.0 to 2.0)
    pub temperature: Option<f32>,
    /// Maximum number of tokens to generate
    pub max_tokens: Option<u32>,
    /// Nucleus sampling parameter (0.0 to 1.0)
    pub top_p: Option<f32>,
    /// Alternative to temperature, called Top-k sampling
    pub top_k: Option<u32>,
    /// Sequences where the API will stop generating further tokens
    pub stop_sequences: Vec<String>,
    /// Number between -2.0 and 2.0. Positive values penalize new tokens based on their existing frequency
    pub frequency_penalty: Option<f32>,
    /// Number between -2.0 and 2.0. Positive values penalize new tokens based on whether they appear in the text so far
    pub presence_penalty: Option<f32>,
}

/// Metadata for requests, including provider-specific extensions.
#[cfg_attr(feature = "specta", derive(Type))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Metadata {
    /// Provider-specific extensions
    pub extensions: HashMap<String, Value>,
    /// Optional request ID for tracking
    pub request_id: Option<String>,
    /// Optional user ID for tracking
    pub user_id: Option<String>,
    /// Timestamp when the request was created
    pub created_at: DateTime<Utc>,
}

/// A message in a conversation.
#[cfg_attr(feature = "specta", derive(Type))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    /// The role of the message sender
    pub role: Role,
    /// The content of the message
    pub content: MessageContent,
}

/// The role of a message sender.
#[cfg_attr(feature = "specta", derive(Type))]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    /// Message from the user
    User,
    /// Message from the AI assistant
    Assistant,
    /// System message (instructions, context)
    System,
    /// Message from a tool/function call
    Tool,
}

impl fmt::Display for Role {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Role::User => "user",
            Role::Assistant => "assistant",
            Role::System => "system",
            Role::Tool => "tool",
        })
    }
}

impl TryFrom<String> for Role {
    type Error = String;
    fn try_from(role: String) -> Result<Self, Self::Error> {
        match role.as_str() {
            "user" => Ok(Role::User),
            "assistant" => Ok(Role::Assistant),
            "system" => Ok(Role::System),
            "tool" => Ok(Role::Tool),
            _ => Err(format!("Invalid role: {role}")),
        }
    }
}

/// Content of a message, which can be text or multimodal.
#[cfg_attr(feature = "specta", derive(Type))]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MessageContent {
    /// Simple text content
    Text(String),
    /// Multimodal content with text and other media
    Multimodal(Vec<ContentPart>),
    /// Tool-related content (calls and responses)
    Tool(ToolContent),
}

/// Tool-related message content.
#[cfg_attr(feature = "specta", derive(Type))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolContent {
    /// Tool calls made by the assistant
    pub tool_calls: Option<Vec<ToolCall>>,
    /// Tool call ID if this is a tool response
    pub tool_call_id: Option<String>,
    /// Optional text content alongside tool data
    pub text: Option<String>,
}

impl MessageContent {
    /// Create text content
    pub fn text(content: impl Into<String>) -> Self {
        Self::Text(content.into())
    }

    /// Create multimodal content
    pub fn multimodal(parts: Vec<ContentPart>) -> Self {
        Self::Multimodal(parts)
    }

    /// Create tool content with tool calls
    pub fn tool_calls(tool_calls: Vec<ToolCall>) -> Self {
        Self::Tool(ToolContent {
            tool_calls: Some(tool_calls),
            tool_call_id: None,
            text: None,
        })
    }

    /// Create tool response content
    pub fn tool_response(content: impl Into<String>, tool_call_id: impl Into<String>) -> Self {
        Self::Tool(ToolContent {
            tool_calls: None,
            tool_call_id: Some(tool_call_id.into()),
            text: Some(content.into()),
        })
    }
}

/// A part of multimodal message content.
#[cfg_attr(feature = "specta", derive(Type))]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentPart {
    /// Text content
    Text { text: String },
    /// Image content
    Image {
        /// Image data or URL
        image_source: ImageSource,
        /// Optional detail level for image processing
        detail: Option<String>,
    },
    /// Audio content
    Audio {
        /// Audio data or URL
        audio_url: String,
        /// Audio format (mp3, wav, etc.)
        format: Option<String>,
    },
}
impl ContentPart {
    /// Create text content part
    pub fn text(text: impl Into<String>) -> Self {
        Self::Text { text: text.into() }
    }

    /// Create image content part
    pub fn image(source: ImageSource) -> Self {
        Self::Image {
            image_source: source,
            detail: None,
        }
    }

    #[cfg(feature = "dynamic-image")]
    pub fn image_dynamic(image: image::DynamicImage) -> Self {
        Self::Image {
            image_source: ImageSource::DynamicImage(image),
            detail: None,
        }
    }

    pub fn image_url(url: impl Into<String>) -> Self {
        Self::Image {
            image_source: ImageSource::Url(url.into()),
            detail: None,
        }
    }

    /// Create image content part with detail level
    pub fn image_with_detail(url: impl Into<String>, detail: impl Into<String>) -> Self {
        let detail_str = detail.into();
        Self::Image {
            image_source: ImageSource::Url(url.into()),
            detail: Some(detail_str),
        }
    }

    /// Create audio content part
    pub fn audio(url: impl Into<String>, format: Option<String>) -> Self {
        Self::Audio {
            audio_url: url.into(),
            format,
        }
    }
}

#[cfg_attr(feature = "specta", derive(Type))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ImageSource {
    /// The URL or base64-encoded image data
    Url(String),

    #[cfg(feature = "dynamic-image")]
    #[serde(skip_serializing, skip_deserializing)]
    /// The image data
    DynamicImage(image::DynamicImage),
}

#[cfg(feature = "dynamic-image")]
impl ImageSource {
    pub fn dynamic_image(image: image::DynamicImage) -> Self {
        Self::DynamicImage(image)
    }
}

#[cfg(feature = "dynamic-image")]
impl From<image::DynamicImage> for ImageSource {
    fn from(image: image::DynamicImage) -> Self {
        Self::DynamicImage(image)
    }
}

/// Converts an ImageSource to a String representation.
///
/// - `Url` variants are returned as-is
/// - `DynamicImage` variants are converted to base64-encoded PNG data URLs
///
/// Note: This conversion is lossy - the original type cannot be determined from the resulting string.
impl From<ImageSource> for String {
    fn from(source: ImageSource) -> Self {
        match source {
            ImageSource::Url(url) => url,

            #[cfg(feature = "dynamic-image")]
            ImageSource::DynamicImage(image) => crate::util::dynamic_image::image_to_base64(&image),

            #[cfg(not(feature = "dynamic-image"))]
            #[allow(unreachable_patterns)]
            _ => panic!(
                "ImageSource::DynamicImage variant requires the 'dynamic-image' feature to be enabled"
            ),
        }
    }
}

/// A tool/function call made by the AI.
#[cfg_attr(feature = "specta", derive(Type))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    /// Unique identifier for this tool call
    pub id: String,
    /// The type of tool call (usually "function")
    #[serde(rename = "type")]
    pub call_type: String,
    /// The function being called
    pub function: FunctionCall,
}

/// A function call within a tool call.
#[cfg_attr(feature = "specta", derive(Type))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionCall {
    /// Name of the function to call
    pub name: String,
    /// Arguments to pass to the function (JSON string)
    pub arguments: String,
}

/// Definition of a tool/function that can be called.
#[cfg_attr(feature = "specta", derive(Type))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tool {
    /// Type of tool (usually "function")
    #[serde(rename = "type")]
    pub tool_type: String,
    /// The function definition
    pub function: Function,
}

/// Definition of a function that can be called.
#[cfg_attr(feature = "specta", derive(Type))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Function {
    /// Name of the function
    pub name: String,
    /// Description of what the function does
    pub description: String,
    /// JSON schema for the function parameters
    pub parameters: Value,
}

/// Usage statistics for a request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Usage {
    /// Number of tokens in the prompt
    pub prompt_tokens: u32,
    /// Number of tokens in the completion
    pub completion_tokens: u32,
    /// Total number of tokens used
    pub total_tokens: u32,
}

/// Reason why the model stopped generating.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FinishReason {
    /// The model reached a natural stopping point
    Stop,
    /// The model reached the maximum token limit
    Length,
    /// The model generated a stop sequence
    StopSequence,
    /// The model made a tool call
    ToolCalls,
    /// Content was filtered
    ContentFilter,
    /// An error occurred
    Error,
}

/// An embedding vector.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Embedding {
    /// The embedding vector
    pub embedding: Vec<f32>,
    /// Index of the input text this embedding corresponds to
    pub index: usize,
}

/// Request for image generation.
#[cfg_attr(feature = "specta", derive(Type))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageRequest {
    /// Text prompt for image generation
    pub prompt: String,
    /// Optional negative prompt (what to avoid)
    pub negative_prompt: Option<String>,
    /// Number of images to generate
    pub n: Option<u32>,
    /// Image size specification
    pub size: Option<String>,
    /// Image quality setting
    pub quality: Option<String>,
    /// Response format (url or b64_json)
    pub response_format: Option<String>,
    /// Additional metadata
    pub metadata: Metadata,
}

/// Request for speech-to-text conversion.
#[cfg_attr(feature = "specta", derive(Type))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpeechToTextRequest {
    /// Audio data (base64 encoded or file path)
    pub audio: String,
    /// Audio format (mp3, wav, etc.)
    pub format: String,
    /// Language of the audio (optional)
    pub language: Option<String>,
    /// Additional metadata
    pub metadata: Metadata,
}

/// Request for text-to-speech conversion.
#[cfg_attr(feature = "specta", derive(Type))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextToSpeechRequest {
    /// Text to convert to speech
    pub text: String,
    /// Voice to use
    pub voice: String,
    /// Audio format for output
    pub format: Option<String>,
    /// Speed of speech (0.25 to 4.0)
    pub speed: Option<f32>,
    /// Additional metadata
    pub metadata: Metadata,
}

/// Trait for chat response types.
pub trait ChatResponse: Send + Sync {
    /// Get the text content of the response
    fn content(&self) -> String;

    /// Get usage statistics if available
    fn usage(&self) -> Option<Usage>;

    /// Get the reason why generation finished
    fn finish_reason(&self) -> Option<FinishReason>;

    /// Get response metadata
    fn metadata(&self) -> Metadata;

    /// Get tool calls if any were made
    fn tool_calls(&self) -> Option<Vec<ToolCall>> {
        None
    }

    /// Convert response to a Message for conversation history
    fn as_message(&self) -> Message {
        let content = if let Some(tool_calls) = self.tool_calls() {
            MessageContent::Tool(ToolContent {
                tool_calls: Some(tool_calls),
                tool_call_id: None,
                text: if self.content().is_empty() {
                    None
                } else {
                    Some(self.content())
                },
            })
        } else {
            MessageContent::Text(self.content())
        };

        Message {
            role: Role::Assistant,
            content,
        }
    }
}

/// Trait for completion response types.
pub trait CompletionResponse: Send + Sync {
    /// Get the completion text
    fn text(&self) -> String;

    /// Get usage statistics if available
    fn usage(&self) -> Option<Usage>;

    /// Get the reason why generation finished
    fn finish_reason(&self) -> Option<FinishReason>;

    /// Get response metadata
    fn metadata(&self) -> Metadata;
}

/// Trait for image generation response types.
pub trait ImageResponse: Send + Sync {
    /// Get the generated images
    fn images(&self) -> Vec<GeneratedImage>;

    /// Get response metadata
    fn metadata(&self) -> Metadata;
}

/// A generated image.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratedImage {
    /// Image URL or base64 data
    pub url: Option<String>,
    /// Base64 encoded image data
    pub b64_json: Option<String>,
    /// Revised prompt used for generation
    pub revised_prompt: Option<String>,
}

/// Trait for speech-to-text response types.
pub trait SpeechToTextResponse: Send + Sync {
    /// Get the transcribed text
    fn text(&self) -> String;

    /// Get the detected language if available
    fn language(&self) -> Option<String>;

    /// Get response metadata
    fn metadata(&self) -> Metadata;
}

/// Trait for text-to-speech response types.
pub trait TextToSpeechResponse: Send + Sync {
    /// Get the audio data
    fn audio_data(&self) -> Vec<u8>;

    /// Get the audio format
    fn format(&self) -> String;

    /// Get response metadata
    fn metadata(&self) -> Metadata;
}

// Convenience constructors
impl Message {
    /// Create a user message with text content
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: Role::User,
            content: MessageContent::Text(content.into()),
        }
    }

    /// Create an assistant message with text content
    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: Role::Assistant,
            content: MessageContent::Text(content.into()),
        }
    }

    /// Create a system message with text content
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: Role::System,
            content: MessageContent::Text(content.into()),
        }
    }

    /// Create a tool response message
    pub fn tool_response(content: impl Into<String>, tool_call_id: impl Into<String>) -> Self {
        Self {
            role: Role::Tool,
            content: MessageContent::Tool(ToolContent {
                tool_calls: None,
                tool_call_id: Some(tool_call_id.into()),
                text: Some(content.into()),
            }),
        }
    }

    /// Create an assistant message with tool calls
    pub fn assistant_with_tools(content: impl Into<String>, tool_calls: Vec<ToolCall>) -> Self {
        let content_str = content.into();
        Self {
            role: Role::Assistant,
            content: MessageContent::Tool(ToolContent {
                tool_calls: Some(tool_calls),
                tool_call_id: None,
                text: if content_str.is_empty() {
                    None
                } else {
                    Some(content_str)
                },
            }),
        }
    }

    /// Create a user message with multimodal content
    pub fn user_multimodal(content: Vec<ContentPart>) -> Self {
        Self {
            role: Role::User,
            content: MessageContent::Multimodal(content),
        }
    }

    /// Create an assistant message with multimodal content
    pub fn assistant_multimodal(content: Vec<ContentPart>) -> Self {
        Self {
            role: Role::Assistant,
            content: MessageContent::Multimodal(content),
        }
    }
}

impl Default for Metadata {
    fn default() -> Self {
        Self {
            extensions: HashMap::new(),
            request_id: None,
            user_id: None,
            created_at: Utc::now(),
        }
    }
}

impl ChatRequest {
    /// Create a new chat request builder
    pub fn builder() -> ChatRequestBuilder {
        ChatRequestBuilder::new()
    }
}

/// Builder for ChatRequest
pub struct ChatRequestBuilder {
    messages: Vec<Message>,
    parameters: Parameters,
    metadata: Metadata,
}

impl Default for ChatRequestBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl ChatRequestBuilder {
    pub fn new() -> Self {
        Self {
            messages: Vec::new(),
            parameters: Parameters::default(),
            metadata: Metadata::default(),
        }
    }

    pub fn message(mut self, message: Message) -> Self {
        self.messages.push(message);
        self
    }

    pub fn messages(mut self, messages: Vec<Message>) -> Self {
        self.messages = messages;
        self
    }

    pub fn temperature(mut self, temperature: f32) -> Self {
        self.parameters.temperature = Some(temperature);
        self
    }

    pub fn max_tokens(mut self, max_tokens: u32) -> Self {
        self.parameters.max_tokens = Some(max_tokens);
        self
    }

    pub fn top_p(mut self, top_p: f32) -> Self {
        self.parameters.top_p = Some(top_p);
        self
    }

    pub fn stop_sequences(mut self, stop_sequences: Vec<String>) -> Self {
        self.parameters.stop_sequences = stop_sequences;
        self
    }

    pub fn request_id(mut self, request_id: String) -> Self {
        self.metadata.request_id = Some(request_id);
        self
    }

    pub fn user_id(mut self, user_id: String) -> Self {
        self.metadata.user_id = Some(user_id);
        self
    }

    pub fn extension(mut self, key: String, value: Value) -> Self {
        self.metadata.extensions.insert(key, value);
        self
    }
    /// Add a user message with text content
    pub fn user_message(mut self, content: impl Into<String>) -> Self {
        self.messages.push(Message::user(content));
        self
    }

    /// Add an assistant message with text content
    pub fn assistant_message(mut self, content: impl Into<String>) -> Self {
        self.messages.push(Message::assistant(content));
        self
    }

    /// Add a system message with text content
    pub fn system_message(mut self, content: impl Into<String>) -> Self {
        self.messages.push(Message::system(content));
        self
    }

    /// Add a tool response message
    pub fn tool_response(
        mut self,
        content: impl Into<String>,
        tool_call_id: impl Into<String>,
    ) -> Self {
        self.messages
            .push(Message::tool_response(content, tool_call_id));
        self
    }

    /// Add an assistant message with tool calls
    pub fn assistant_with_tools(
        mut self,
        content: impl Into<String>,
        tool_calls: Vec<ToolCall>,
    ) -> Self {
        self.messages
            .push(Message::assistant_with_tools(content, tool_calls));
        self
    }

    /// Add a user message with multimodal content
    pub fn user_multimodal(mut self, content: Vec<ContentPart>) -> Self {
        self.messages.push(Message::user_multimodal(content));
        self
    }

    /// Add an assistant message with multimodal content
    pub fn assistant_multimodal(mut self, content: Vec<ContentPart>) -> Self {
        self.messages.push(Message::assistant_multimodal(content));
        self
    }

    pub fn build(self) -> ChatRequest {
        ChatRequest {
            messages: self.messages,
            parameters: self.parameters,
            metadata: self.metadata,
        }
    }
}
