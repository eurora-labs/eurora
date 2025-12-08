//! OpenAI provider for the LLM library.
//!
//! This crate provides an implementation of the LLM core traits for OpenAI's API,
//! including support for chat, completion, streaming, embeddings, and tool calling.

pub mod config;
pub mod error;
pub mod provider;
pub mod types;

// Re-export main types for convenience
pub use config::OpenAIConfig;
pub use error::OpenAIError;
pub use provider::OpenAIProvider;
pub use types::{
    OpenAIChatChoice, OpenAIChatRequest, OpenAIChatResponse, OpenAICompletionChoice,
    OpenAICompletionRequest, OpenAICompletionResponse, OpenAIEmbeddingsRequest,
    OpenAIEmbeddingsResponse, OpenAIMessage, OpenAITool, OpenAIToolCall, OpenAIUsage,
};

// Re-export core traits
pub use euro_llm_core::{
    ChatProvider, CompletionProvider, EmbeddingProvider, StreamingProvider, ToolProvider,
};
