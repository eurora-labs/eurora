//! Agent Chain - A Rust implementation of LangChain-style components.
//!
//! This crate provides:
//! - Message types for LLM threads (human, AI, system, tool)
//! - Tool trait and `#[tool]` macro for function calling
//! - Chat model abstractions and provider integrations
//! - Support for multiple providers (Anthropic, OpenAI, etc.)
//!
//! # Architecture
//!
//! The architecture follows LangChain's pattern:
//!
//! - **Core layer** ([`chat_model`]): Base `ChatModel` trait that all providers implement
//! - **Provider layer** ([`providers`]): Provider-specific implementations (ChatAnthropic, ChatOpenAI)
//! - **Message layer** ([`messages`]): Message types for threads
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
//! let messages = vec![HumanMessage::builder().content("Hello!").build().into()];
//! let response = model.generate(messages, GenerateConfig::default()).await?;
//! ```
//!
//! # Provider Support
//!
//! The following providers are supported:
//!
//! | Provider | Model Prefix | Environment Variable |
//! |----------|--------------|---------------------|
//! | Anthropic | `claude*` | `ANTHROPIC_API_KEY` |
//! | OpenAI | `gpt-*`, `o1*`, `o3*` | `OPENAI_API_KEY` |
//!
//! # Feature Flags
//!
//! - `default`: Includes all providers
//! - `anthropic`: Anthropic/Claude support
//! - `openai`: OpenAI/GPT support
//! - `dynamic-image`: Image processing support
//! - `specta`: Specta derive support

pub mod providers;

// Re-export providers
pub use providers::*;

// Re-export async_trait for use in generated code
pub use async_trait::async_trait;

use std::sync::Arc;

pub use agent_chain_core::*;

// Re-export agent_chain_core as a module for macro-generated code
#[doc(hidden)]
pub use agent_chain_core as _core;

// Re-export the tool macro
pub mod tools {
    //! Tool types and macros for defining tools.
    //!
    //! This module re-exports all tool types from agent-chain-core
    //! and adds the `#[tool]` attribute macro.

    pub use agent_chain_core::tools::*;

    // Re-export the tool attribute macro
    pub use agent_chain_macros::tool;
}

/// Initialize a chat model from the model name with automatic provider inference.
///
/// This function provides a convenient way to create chat models, similar to
/// LangChain's `init_chat_model` function. It supports:
///
/// - Automatic provider inference from model names
/// - Explicit provider specification
/// - Provider prefix syntax (e.g., "openai:gpt-4")
///
/// # Arguments
///
/// * `model` - The model name/identifier. Can include provider prefix (e.g., "openai:gpt-4")
/// * `model_provider` - Optional explicit provider name. Overrides inference.
///
/// # Returns
///
/// A boxed `ChatModel` trait object that can be used for generation.
///
/// # Errors
///
/// Returns an error if:
/// - The provider cannot be inferred and none is specified
/// - The provider is not supported
///
/// # Examples
///
/// ```ignore
/// use agent_chain_core::init_chat_model;
///
/// // Provider inferred from model name
/// let claude = init_chat_model("claude-sonnet-4-5-20250929", None)?;
/// let gpt = init_chat_model("gpt-4o", None)?;
///
/// // Explicit provider
/// let model = init_chat_model("my-custom-model", Some("openai"))?;
///
/// // Provider prefix syntax
/// let model = init_chat_model("anthropic:claude-3-opus", None)?;
/// ```
///
/// # Provider Inference
///
/// The provider is inferred from these model name patterns:
///
/// - `gpt-*`, `o1*`, `o3*` → OpenAI
/// - `claude*` → Anthropic
/// - `gemini*` → Google Vertex AI
/// - `command*` → Cohere
/// - `mistral*` → Mistral AI
/// - `deepseek*` → DeepSeek
/// - `grok*` → xAI
/// - `sonar*` → Perplexity
pub fn init_chat_model(
    model: impl Into<String>,
    model_provider: Option<&str>,
) -> Result<Arc<dyn BaseChatModel>> {
    let model = model.into();
    let (model_name, provider) = parse_model(&model, model_provider)?;

    match provider.as_str() {
        #[cfg(feature = "anthropic")]
        "anthropic" => Ok(Arc::new(anthropic::ChatAnthropic::new(model_name))),
        #[cfg(feature = "openai")]
        "openai" => Ok(Arc::new(openai::ChatOpenAI::new(model_name))),
        _ => Err(Error::unsupported_provider(provider)),
    }
}

/// Builder for creating chat models with additional configuration.
///
/// This provides a more flexible way to create chat models when you need
/// to configure options beyond just the model name.
///
/// # Example
///
/// ```ignore
/// use agent_chain_core::ChatModelBuilder;
///
/// let model = ChatModelBuilder::new("claude-sonnet-4-5-20250929")
///     .provider("anthropic")
///     .temperature(0.7)
///     .max_tokens(1024)
///     .api_key("your-api-key")
///     .build()?;
/// ```
#[derive(Debug, Clone)]
pub struct ChatModelBuilder {
    model: String,
    provider: Option<String>,
    temperature: Option<f64>,
    max_tokens: Option<u32>,
    api_key: Option<String>,
    api_base: Option<String>,
}

impl ChatModelBuilder {
    /// Create a new builder for the given model.
    pub fn new(model: impl Into<String>) -> Self {
        Self {
            model: model.into(),
            provider: None,
            temperature: None,
            max_tokens: None,
            api_key: None,
            api_base: None,
        }
    }

    /// Set the provider explicitly.
    pub fn provider(mut self, provider: impl Into<String>) -> Self {
        self.provider = Some(provider.into());
        self
    }

    /// Set the temperature.
    pub fn temperature(mut self, temp: f64) -> Self {
        self.temperature = Some(temp);
        self
    }

    /// Set the maximum tokens.
    pub fn max_tokens(mut self, max: u32) -> Self {
        self.max_tokens = Some(max);
        self
    }

    /// Set the API key.
    pub fn api_key(mut self, key: impl Into<String>) -> Self {
        self.api_key = Some(key.into());
        self
    }

    /// Set the API base URL.
    pub fn api_base(mut self, base: impl Into<String>) -> Self {
        self.api_base = Some(base.into());
        self
    }

    /// Build the chat model.
    pub fn build(self) -> Result<Arc<dyn BaseChatModel>> {
        let (model_name, provider) = parse_model(&self.model, self.provider.as_deref())?;

        match provider.as_str() {
            #[cfg(feature = "anthropic")]
            "anthropic" => {
                let mut model = anthropic::ChatAnthropic::new(model_name);
                if let Some(temp) = self.temperature {
                    model = model.temperature(temp);
                }
                if let Some(max) = self.max_tokens {
                    model = model.max_tokens(max);
                }
                if let Some(key) = self.api_key {
                    model = model.api_key(key);
                }
                if let Some(base) = self.api_base {
                    model = model.api_base(base);
                }
                Ok(Arc::new(model))
            }
            #[cfg(feature = "openai")]
            "openai" | "azure_openai" => {
                let mut model = openai::ChatOpenAI::new(model_name);
                if let Some(temp) = self.temperature {
                    model = model.temperature(temp);
                }
                if let Some(max) = self.max_tokens {
                    model = model.max_tokens(max);
                }
                if let Some(key) = self.api_key {
                    model = model.api_key(key);
                }
                if let Some(base) = self.api_base {
                    model = model.api_base(base);
                }
                Ok(Arc::new(model))
            }
            _ => Err(Error::unsupported_provider(provider)),
        }
    }
}
