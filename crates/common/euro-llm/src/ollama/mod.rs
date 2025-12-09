//! Ollama provider for the LLM library.
//!
//! This crate provides an implementation of the LLM core traits for Ollama's API,
//! including support for chat, completion, streaming, and embeddings.

pub mod config;
pub mod error;
pub mod provider;
pub mod types;

// Re-export main types for convenience
pub use config::OllamaConfig;
pub use error::OllamaError;
pub use provider::OllamaProvider;
pub use types::{
    OllamaChatRequest, OllamaChatResponse, OllamaChoice, OllamaCompletionRequest,
    OllamaCompletionResponse, OllamaEmbeddingsRequest, OllamaEmbeddingsResponse, OllamaMessage,
    OllamaStreamChunk, OllamaUsage,
};

// Re-export core traits
pub use crate::core::{ChatProvider, CompletionProvider, EmbeddingProvider, StreamingProvider};
