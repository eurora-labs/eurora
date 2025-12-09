//! Anthropic provider for the LLM library.
//!
//! This crate provides an implementation of the LLM core traits for Anthropic's API,
//! including support for chat, streaming, and tool calling with Claude models.

pub mod config;
pub mod error;
pub mod provider;
pub mod types;

// Re-export main types for convenience
pub use config::AnthropicConfig;
pub use error::AnthropicError;
pub use provider::AnthropicProvider;
pub use types::{
    AnthropicContent, AnthropicContentBlock, AnthropicMessage, AnthropicMessagesRequest,
    AnthropicMessagesResponse, AnthropicStreamChunk, AnthropicTool, AnthropicToolChoice,
    AnthropicUsage,
};

// Re-export core traits
pub use crate::core::{ChatProvider, StreamingProvider, ToolProvider};
