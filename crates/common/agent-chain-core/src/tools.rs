//! Tools module for LLM function calling.
//!
//! This module provides the `Tool` trait and `#[tool]` macro for creating
//! tools that can be invoked by AI models.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::messages::{BaseMessage, ToolCall};

// Re-export the tool macro
pub use agent_chain_macros::tool;

/// Represents a tool's definition for LLM function calling.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    /// The name of the tool
    pub name: String,
    /// A description of what the tool does
    pub description: String,
    /// JSON schema for the tool's parameters
    pub parameters: serde_json::Value,
}

/// A trait for tools that can be invoked by an AI model.
#[async_trait]
pub trait Tool: Send + Sync {
    /// Get the name of the tool.
    fn name(&self) -> &str;

    /// Get the description of the tool.
    fn description(&self) -> &str;

    /// Get the JSON schema for the tool's parameters.
    fn parameters_schema(&self) -> serde_json::Value;

    /// Invoke the tool with the given tool call.
    async fn invoke(&self, tool_call: ToolCall) -> BaseMessage;

    /// Invoke the tool directly with arguments (without a full ToolCall).
    /// Returns the result as a JSON value.
    ///
    /// This is a convenience method for when you have the args directly.
    /// The default implementation creates a temporary ToolCall and invokes it.
    async fn invoke_args(&self, args: serde_json::Value) -> serde_json::Value {
        let tool_call = ToolCall::new(self.name(), args);
        let result = self.invoke(tool_call).await;
        serde_json::Value::String(result.content().to_string())
    }

    /// Get the tool definition for LLM function calling.
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: self.name().to_string(),
            description: self.description().to_string(),
            parameters: self.parameters_schema(),
        }
    }
}
