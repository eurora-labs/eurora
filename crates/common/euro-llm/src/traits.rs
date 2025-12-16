//! Core traits for LLM providers.
//!
//! This module defines the foundational traits that LLM providers implement,
//! following the Interface Segregation Principle - providers only implement
//! the capabilities they support.

use super::{config::ProviderConfig, error::ProviderError, types::*};
use async_trait::async_trait;
use futures::Stream;

/// Base trait for chat-based LLM providers.
///
/// This trait defines the core chat functionality that most LLM providers support.
/// Providers implement this trait to provide conversational AI capabilities.
#[async_trait]
pub trait ChatProvider: Send + Sync {
    /// Provider-specific configuration type
    type Config: ProviderConfig;

    /// Provider-specific response type
    type Response: ChatResponse;

    /// Provider-specific error type
    type Error: ProviderError;

    /// Send a chat request and receive a response.
    ///
    /// # Arguments
    /// * `request` - The chat request containing messages and parameters
    ///
    /// # Returns
    /// A result containing the provider's response or an error
    async fn chat(&self, request: ChatRequest) -> Result<Self::Response, Self::Error>;
}

/// Trait for providers that support text completion (non-chat).
///
/// This is separate from ChatProvider to allow providers to implement
/// only the capabilities they support.
#[async_trait]
pub trait CompletionProvider: Send + Sync {
    /// Provider-specific configuration type
    type Config: ProviderConfig;

    /// Provider-specific response type
    type Response: CompletionResponse;

    /// Provider-specific error type
    type Error: ProviderError;

    /// Complete a text prompt.
    ///
    /// # Arguments
    /// * `request` - The completion request containing the prompt and parameters
    ///
    /// # Returns
    /// A result containing the completion response or an error
    async fn complete(&self, request: CompletionRequest) -> Result<Self::Response, Self::Error>;
}

/// Optional trait for providers that support streaming responses.
///
/// This extends ChatProvider to add streaming capabilities.
#[async_trait]
pub trait StreamingProvider: ChatProvider {
    /// Stream item type for incremental responses
    type StreamItem: Send + 'static;

    /// Stream type for the response
    type Stream: Stream<Item = Result<Self::StreamItem, Self::Error>> + Send + 'static;

    /// Send a chat request and receive a streaming response.
    ///
    /// # Arguments
    /// * `request` - The chat request containing messages and parameters
    ///
    /// # Returns
    /// A result containing a stream of response chunks or an error
    async fn chat_stream(&self, request: ChatRequest) -> Result<Self::Stream, Self::Error>;
}

/// Optional trait for providers that support tool/function calling.
///
/// This extends ChatProvider to add tool calling capabilities.
#[async_trait]
pub trait ToolProvider: ChatProvider {
    /// Stream type for tool-enabled streaming responses.
    /// Each item in the stream is a [`StreamEvent`] representing either
    /// text content or tool call information.
    type ToolStream: Stream<Item = Result<StreamEvent, Self::Error>> + Send + 'static;

    /// Send a chat request with available tools.
    ///
    /// # Arguments
    /// * `request` - The chat request containing messages and parameters
    /// * `tools` - Available tools that the model can call
    ///
    /// # Returns
    /// A result containing the response (potentially with tool calls) or an error
    async fn chat_with_tools(
        &self,
        request: ChatRequest,
        tools: &[Tool],
    ) -> Result<Self::Response, Self::Error>;

    /// Send a chat request with tools and receive a streaming response.
    ///
    /// This method enables streaming responses while supporting tool/function calls.
    /// The stream emits [`StreamEvent`] items that can be either:
    /// - `ContentDelta`: Incremental text content from the assistant
    /// - `ToolCallStart`: Beginning of a tool call with ID and function name
    /// - `ToolCallDelta`: Incremental tool call arguments
    /// - `Done`: Signal that the stream has completed
    ///
    /// # Arguments
    /// * `request` - The chat request containing messages and parameters
    /// * `tools` - Available tools that the model can call
    ///
    /// # Returns
    /// A result containing a stream of [`StreamEvent`] items or an error
    ///
    /// # Example
    /// ```ignore
    /// use futures::StreamExt;
    ///
    /// let mut stream = provider.chat_stream_with_tools(request, &tools).await?;
    /// let mut accumulated_args = String::new();
    ///
    /// while let Some(event) = stream.next().await {
    ///     match event? {
    ///         StreamEvent::ContentDelta { content } => {
    ///             print!("{}", content);
    ///         }
    ///         StreamEvent::ToolCallStart { index, id, name, .. } => {
    ///             println!("Tool call started: {} ({})", name, id);
    ///         }
    ///         StreamEvent::ToolCallDelta { arguments_delta, .. } => {
    ///             accumulated_args.push_str(&arguments_delta);
    ///         }
    ///         StreamEvent::Done { finish_reason } => {
    ///             println!("Stream finished: {:?}", finish_reason);
    ///         }
    ///     }
    /// }
    /// ```
    async fn chat_stream_with_tools(
        &self,
        request: ChatRequest,
        tools: &[Tool],
    ) -> Result<Self::ToolStream, Self::Error>;
}

/// Trait for providers that support text embeddings.
///
/// This is a separate capability from chat/completion as not all providers
/// support embeddings, and embedding-only providers don't need chat capabilities.
#[async_trait]
pub trait EmbeddingProvider: Send + Sync {
    /// Provider-specific configuration type
    type Config: ProviderConfig;

    /// Provider-specific error type
    type Error: ProviderError;

    /// Generate embeddings for the given texts.
    ///
    /// # Arguments
    /// * `texts` - The texts to generate embeddings for
    ///
    /// # Returns
    /// A result containing the embeddings or an error
    async fn embed(&self, texts: &[String]) -> Result<Vec<Embedding>, Self::Error>;
}

/// Optional trait for providers that support image generation.
#[async_trait]
pub trait ImageProvider: Send + Sync {
    /// Provider-specific configuration type
    type Config: ProviderConfig;

    /// Provider-specific response type
    type Response: ImageResponse;

    /// Provider-specific error type
    type Error: ProviderError;

    /// Generate images from a text prompt.
    ///
    /// # Arguments
    /// * `request` - The image generation request
    ///
    /// # Returns
    /// A result containing the generated images or an error
    async fn generate_image(&self, request: ImageRequest) -> Result<Self::Response, Self::Error>;
}

/// Optional trait for providers that support speech-to-text.
#[async_trait]
pub trait SpeechToTextProvider: Send + Sync {
    /// Provider-specific configuration type
    type Config: ProviderConfig;

    /// Provider-specific response type
    type Response: SpeechToTextResponse;

    /// Provider-specific error type
    type Error: ProviderError;

    /// Transcribe audio to text.
    ///
    /// # Arguments
    /// * `request` - The speech-to-text request containing audio data
    ///
    /// # Returns
    /// A result containing the transcription or an error
    async fn speech_to_text(
        &self,
        request: SpeechToTextRequest,
    ) -> Result<Self::Response, Self::Error>;
}

/// Optional trait for providers that support text-to-speech.
#[async_trait]
pub trait TextToSpeechProvider: Send + Sync {
    /// Provider-specific configuration type
    type Config: ProviderConfig;

    /// Provider-specific response type
    type Response: TextToSpeechResponse;

    /// Provider-specific error type
    type Error: ProviderError;

    /// Convert text to speech.
    ///
    /// # Arguments
    /// * `request` - The text-to-speech request
    ///
    /// # Returns
    /// A result containing the audio data or an error
    async fn text_to_speech(
        &self,
        request: TextToSpeechRequest,
    ) -> Result<Self::Response, Self::Error>;
}
