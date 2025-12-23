//! Agent Chain Core - A Rust implementation of LangChain core library.
//!
//! This crate provides:
//! - Message types for LLM conversations (human, AI, system, tool)
//! - Tool trait and `#[tool]` macro for function calling
//! - Chat model abstractions and provider integrations
//! - Support for multiple providers (Anthropic, OpenAI, etc.)
//!
//! # Architecture
//!
//! The architecture follows LangChain's pattern:
//!
//! - **Core layer** ([`chat_model`]): Base `ChatModel` trait that all providers implement
//! - **Message layer** ([`messages`]): Message types for conversations
//! - **Tools layer** ([`tools`]): Tool definitions and the `#[tool]` macro
//!
//! # Quick Start
//!
//! ```ignore
//! use agent_chain_core::{init_chat_model, HumanMessage};
//!
//! // Initialize a model - provider is inferred from name
//! let model = init_chat_model("claude-sonnet-4-5-20250929", None)?;
//!
//! // Or specify explicitly
//! let model = init_chat_model("my-custom-model", Some("openai"))?;
//!
//! // Use the model
//! let messages = vec![HumanMessage::new("Hello!").into()];
//! let response = model.generate(messages, None).await?;
//! ```
//!
//! # Feature Flags
//!
//! - `default`: Includes all providers
//! - `specta`: Specta derive support

pub mod chat_models;
pub mod error;
pub mod messages;
pub mod tools;

// Re-export error types
pub use error::{Error, Result};

// Re-export core chat model types
pub use chat_models::{
    BoundChatModel, ChatChunk, ChatModel, ChatModelExt, ChatResult, ChatResultMetadata, ChatStream,
    DynBoundChatModel, DynChatModelExt, LangSmithParams, ToolChoice, UsageMetadata,
};

// Re-export message types
pub use messages::{
    AIMessage, AnyMessage, BaseMessage, ContentPart, HasId, HumanMessage, ImageDetail, ImageSource,
    MessageContent, SystemMessage, ToolCall, ToolMessage,
};

// Re-export tool types
pub use tools::{Tool, ToolDefinition, tool};

// Re-export async_trait for use in generated code
pub use async_trait::async_trait;
