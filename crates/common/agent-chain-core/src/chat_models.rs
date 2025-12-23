//! Core ChatModel trait and related types.
//!
//! This module provides the base abstraction for chat models, following the
//! LangChain pattern of having a common interface for different providers.

use std::pin::Pin;
use std::sync::Arc;

use async_trait::async_trait;
use futures::Stream;
use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::messages::{AIMessage, BaseMessage};
use crate::tools::{Tool, ToolDefinition};

/// Output from a chat model generation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatResult {
    /// The generated message.
    pub message: AIMessage,
    /// Additional metadata from the model.
    #[serde(default)]
    pub metadata: ChatResultMetadata,
}

/// Metadata from a chat model generation.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ChatResultMetadata {
    /// The model that was used.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// Stop reason from the model.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_reason: Option<String>,
    /// Token usage information.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<UsageMetadata>,
}

/// Token usage metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageMetadata {
    /// Number of input tokens.
    pub input_tokens: u32,
    /// Number of output tokens.
    pub output_tokens: u32,
    /// Total tokens (input + output).
    pub total_tokens: u32,
}

impl UsageMetadata {
    /// Create a new usage metadata.
    pub fn new(input_tokens: u32, output_tokens: u32) -> Self {
        Self {
            input_tokens,
            output_tokens,
            total_tokens: input_tokens + output_tokens,
        }
    }
}

/// A chunk of output from streaming.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatChunk {
    /// The content delta.
    pub content: String,
    /// Whether this is the final chunk.
    pub is_final: bool,
    /// Metadata (only present on final chunk).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<ChatResultMetadata>,
}

/// Type alias for streaming output.
pub type ChatStream = Pin<Box<dyn Stream<Item = Result<ChatChunk>> + Send>>;

/// Parameters for tracing and monitoring.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LangSmithParams {
    /// Provider name (e.g., "anthropic", "openai").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ls_provider: Option<String>,
    /// Model name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ls_model_name: Option<String>,
    /// Model type (always "chat" for chat models).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ls_model_type: Option<String>,
    /// Temperature setting.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ls_temperature: Option<f64>,
    /// Max tokens setting.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ls_max_tokens: Option<u32>,
    /// Stop sequences.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ls_stop: Option<Vec<String>>,
}

/// Base trait for all chat models.
///
/// This trait follows the LangChain pattern where each provider implements
/// the core generation methods. The trait provides both sync-style (via async)
/// and streaming interfaces.
///
/// # Example Implementation
///
/// ```ignore
/// use agent_chain_core::chat_model::{ChatModel, ChatResult};
/// use agent_chain_core::messages::BaseMessage;
///
/// struct MyChatModel {
///     model: String,
/// }
///
/// #[async_trait::async_trait]
/// impl ChatModel for MyChatModel {
///     fn llm_type(&self) -> &str {
///         "my-chat-model"
///     }
///
///     async fn generate(
///         &self,
///         messages: Vec<BaseMessage>,
///         stop: Option<Vec<String>>,
///     ) -> Result<ChatResult> {
///         // Implementation here
///         todo!()
///     }
/// }
/// ```
#[async_trait]
pub trait ChatModel: Send + Sync {
    /// Return the type identifier for this chat model.
    ///
    /// This is used for logging and tracing purposes.
    fn llm_type(&self) -> &str;

    /// Get the model name/identifier.
    fn model_name(&self) -> &str;

    /// Generate a response from the model.
    ///
    /// # Arguments
    ///
    /// * `messages` - The conversation history.
    /// * `stop` - Optional stop sequences.
    ///
    /// # Returns
    ///
    /// A `ChatResult` containing the generated message and metadata.
    async fn generate(
        &self,
        messages: Vec<BaseMessage>,
        stop: Option<Vec<String>>,
    ) -> Result<ChatResult>;

    /// Generate a response from the model with tools.
    ///
    /// This is the preferred method when tool calling is needed.
    /// Default implementation ignores tools and calls `generate`.
    /// Providers should override this to enable tool calling.
    ///
    /// # Arguments
    ///
    /// * `messages` - The conversation history.
    /// * `tools` - Tool definitions for the model to use.
    /// * `tool_choice` - Optional configuration for tool selection.
    /// * `stop` - Optional stop sequences.
    ///
    /// # Returns
    ///
    /// A `ChatResult` containing the generated message and metadata.
    async fn generate_with_tools(
        &self,
        messages: Vec<BaseMessage>,
        tools: &[ToolDefinition],
        tool_choice: Option<&ToolChoice>,
        stop: Option<Vec<String>>,
    ) -> Result<ChatResult> {
        // Default implementation ignores tools
        let _ = tools;
        let _ = tool_choice;
        self.generate(messages, stop).await
    }

    /// Generate a streaming response from the model.
    ///
    /// Default implementation calls `generate` and wraps the result in a stream.
    /// Providers should override this for native streaming support.
    ///
    /// # Arguments
    ///
    /// * `messages` - The conversation history.
    /// * `stop` - Optional stop sequences.
    ///
    /// # Returns
    ///
    /// A stream of `ChatChunk`s.
    async fn stream(
        &self,
        messages: Vec<BaseMessage>,
        stop: Option<Vec<String>>,
    ) -> Result<ChatStream> {
        let result = self.generate(messages, stop).await?;
        let chunk = ChatChunk {
            content: result.message.content().to_string(),
            is_final: true,
            metadata: Some(result.metadata),
        };
        Ok(Box::pin(futures::stream::once(async move { Ok(chunk) })))
    }

    /// Get parameters for tracing/monitoring.
    fn get_ls_params(&self, stop: Option<&[String]>) -> LangSmithParams {
        let mut params = LangSmithParams {
            ls_model_type: Some("chat".to_string()),
            ..Default::default()
        };
        if let Some(stop) = stop {
            params.ls_stop = Some(stop.to_vec());
        }
        params
    }

    /// Get identifying parameters for serialization.
    fn identifying_params(&self) -> serde_json::Value {
        serde_json::json!({
            "_type": self.llm_type(),
            "model": self.model_name(),
        })
    }
}

/// Configuration for tool choice.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ToolChoice {
    /// Let the model decide whether to use tools.
    Auto,
    /// Model must use at least one tool.
    Any,
    /// Model must use a specific tool.
    Tool {
        /// Name of the tool to use.
        name: String,
    },
    /// Model should not use any tools.
    None,
}

/// A chat model that has been bound with tools (generic version).
///
/// This wraps an underlying chat model and includes tool definitions
/// that will be passed to the model on each invocation.
pub struct BoundChatModel<M: ChatModel> {
    /// The underlying chat model.
    model: M,
    /// Tools bound to this model.
    tools: Vec<Arc<dyn Tool + Send + Sync>>,
    /// Tool choice configuration.
    tool_choice: Option<ToolChoice>,
}

impl<M: ChatModel> BoundChatModel<M> {
    /// Create a new bound chat model.
    pub fn new(model: M, tools: Vec<Arc<dyn Tool + Send + Sync>>) -> Self {
        Self {
            model,
            tools,
            tool_choice: None,
        }
    }

    /// Set the tool choice.
    pub fn with_tool_choice(mut self, tool_choice: ToolChoice) -> Self {
        self.tool_choice = Some(tool_choice);
        self
    }

    /// Get the tool definitions.
    pub fn tool_definitions(&self) -> Vec<ToolDefinition> {
        self.tools.iter().map(|t| t.definition()).collect()
    }

    /// Get a reference to the underlying model.
    pub fn model(&self) -> &M {
        &self.model
    }

    /// Get the tools.
    pub fn tools(&self) -> &[Arc<dyn Tool + Send + Sync>] {
        &self.tools
    }

    /// Get the tool choice.
    pub fn tool_choice(&self) -> Option<&ToolChoice> {
        self.tool_choice.as_ref()
    }

    /// Invoke the model with messages.
    ///
    /// This generates a response using the bound tools.
    pub async fn invoke(&self, messages: Vec<BaseMessage>) -> BaseMessage {
        let tool_definitions = self.tool_definitions();
        match self
            .model
            .generate_with_tools(messages, &tool_definitions, self.tool_choice.as_ref(), None)
            .await
        {
            Ok(result) => result.message.into(),
            Err(e) => {
                // Return an error message
                AIMessage::new(format!("Error: {}", e)).into()
            }
        }
    }
}

impl<M: ChatModel + Clone> Clone for BoundChatModel<M> {
    fn clone(&self) -> Self {
        Self {
            model: self.model.clone(),
            tools: self.tools.clone(),
            tool_choice: self.tool_choice.clone(),
        }
    }
}

/// Extension trait for chat models to add tool binding.
pub trait ChatModelExt: ChatModel + Sized {
    /// Bind tools to this chat model.
    ///
    /// # Arguments
    ///
    /// * `tools` - The tools to bind.
    ///
    /// # Returns
    ///
    /// A `BoundChatModel` that includes the tools.
    fn bind_tools(self, tools: Vec<Arc<dyn Tool + Send + Sync>>) -> BoundChatModel<Self> {
        BoundChatModel::new(self, tools)
    }
}

// Implement ChatModelExt for all ChatModel implementations
impl<T: ChatModel + Sized> ChatModelExt for T {}

/// A dynamically-typed chat model bound with tools.
///
/// This is the dynamic dispatch version of `BoundChatModel`, useful when
/// working with `Arc<dyn ChatModel>` or boxed trait objects.
#[derive(Clone)]
pub struct DynBoundChatModel {
    /// The underlying chat model.
    model: Arc<dyn ChatModel>,
    /// Tools bound to this model.
    tools: Vec<Arc<dyn Tool + Send + Sync>>,
    /// Tool choice configuration.
    tool_choice: Option<ToolChoice>,
}

impl DynBoundChatModel {
    /// Create a new dynamically-typed bound chat model.
    pub fn new(model: Arc<dyn ChatModel>, tools: Vec<Arc<dyn Tool + Send + Sync>>) -> Self {
        Self {
            model,
            tools,
            tool_choice: None,
        }
    }

    /// Set the tool choice.
    pub fn with_tool_choice(mut self, tool_choice: ToolChoice) -> Self {
        self.tool_choice = Some(tool_choice);
        self
    }

    /// Get the tool definitions.
    pub fn tool_definitions(&self) -> Vec<ToolDefinition> {
        self.tools.iter().map(|t| t.definition()).collect()
    }

    /// Get a reference to the underlying model.
    pub fn model(&self) -> &Arc<dyn ChatModel> {
        &self.model
    }

    /// Get the tools.
    pub fn tools(&self) -> &[Arc<dyn Tool + Send + Sync>] {
        &self.tools
    }

    /// Get the tool choice.
    pub fn tool_choice(&self) -> Option<&ToolChoice> {
        self.tool_choice.as_ref()
    }

    /// Invoke the model with messages.
    ///
    /// This generates a response using the bound tools.
    pub async fn invoke(&self, messages: Vec<BaseMessage>) -> BaseMessage {
        let tool_definitions = self.tool_definitions();
        match self
            .model
            .generate_with_tools(messages, &tool_definitions, self.tool_choice.as_ref(), None)
            .await
        {
            Ok(result) => result.message.into(),
            Err(e) => {
                // Return an error message
                AIMessage::new(format!("Error: {}", e)).into()
            }
        }
    }
}

/// Extension methods for `Arc<dyn ChatModel>`.
pub trait DynChatModelExt {
    /// Bind tools to this chat model, returning a dynamically-typed bound model.
    fn bind_tools(self, tools: Vec<Arc<dyn Tool + Send + Sync>>) -> DynBoundChatModel;
}

impl DynChatModelExt for Arc<dyn ChatModel> {
    fn bind_tools(self, tools: Vec<Arc<dyn Tool + Send + Sync>>) -> DynBoundChatModel {
        DynBoundChatModel::new(self, tools)
    }
}
