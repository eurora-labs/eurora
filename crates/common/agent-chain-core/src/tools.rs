//! **Tools** are classes that an Agent uses to interact with the world.
//!
//! Each tool has a **description**. Agent uses the description to choose the right
//! tool for the job.
//!
//! This module provides the core tool abstractions, mirroring
//! `langchain_core.tools`.
//!
//! # Overview
//!
//! Tools are the primary way for LLM agents to interact with external systems,
//! APIs, and data sources. This module provides:
//!
//! - [`BaseTool`] - The base trait that all tools must implement
//! - [`Tool`] - A simple single-input string-to-string tool
//! - [`StructuredTool`] - A tool that accepts multiple typed arguments
//! - [`ToolDefinition`] - Schema definition for LLM function calling
//! - Rendering utilities for displaying tool information
//! - Retriever tool creation utilities
//!
//! # Example
//!
//! ```rust,ignore
//! use agent_chain_core::tools::{Tool, StructuredTool, BaseTool, ToolInput};
//!
//! // Create a simple tool
//! let echo_tool = Tool::from_function(
//!     |input| Ok(format!("Echo: {}", input)),
//!     "echo",
//!     "Echoes back the input",
//! );
//!
//! // Use the tool
//! let result = echo_tool.run(ToolInput::from("Hello"), None)?;
//! ```

pub mod base;
pub mod convert;
pub mod render;
pub mod retriever;
pub mod simple;
pub mod structured;

// Re-export from base
pub use base::{
    // Core types
    ArgsSchema,
    BaseTool,
    BaseToolkit,
    DynTool,
    // Constants
    FILTERED_ARGS,
    HandleToolError,
    HandleValidationError,
    InjectedToolArg,
    InjectedToolCallId,
    ResponseFormat,
    // Error types
    SchemaAnnotationError,
    TOOL_MESSAGE_BLOCK_TYPES,
    ToolDefinition,
    ToolException,
    ToolInput,
    ToolOutput,
    // Utility functions
    format_output,
    handle_tool_error_impl,
    handle_validation_error_impl,
    is_message_content_block,
    is_message_content_type,
    is_tool_call,
    prep_run_args,
    stringify_content,
};

// Re-export from simple
pub use simple::{AsyncToolFunc, Tool, ToolFunc};

// Re-export from structured
pub use structured::{
    AsyncStructuredToolFunc, StructuredTool, StructuredToolFunc, create_args_schema,
};

// Re-export from convert
pub use convert::{
    ToolConfig, convert_runnable_to_tool, create_simple_tool, create_simple_tool_async,
    create_structured_tool, create_structured_tool_async, create_tool_with_config,
    get_description_from_runnable, tool_from_schema,
};

// Re-export from render
pub use render::{ToolsRenderer, render_text_description, render_text_description_and_args};

// Re-export from retriever
pub use retriever::{
    RetrieverInput, RetrieverToolBuilder, create_async_retriever_tool, create_retriever_tool,
    create_retriever_tool_with_options,
};

// Legacy re-export for backward compatibility with the old tools.rs
// The old Tool trait is now BaseTool
pub use base::BaseTool as LegacyTool;

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::sync::Arc;

    #[test]
    fn test_module_exports() {
        // Test that key types are accessible
        let _: fn() -> ArgsSchema = ArgsSchema::default;
        let _: fn() -> ResponseFormat = ResponseFormat::default;
    }

    #[test]
    fn test_create_simple_tool() {
        let tool = create_simple_tool("test", "A test tool", |input| Ok(format!("Got: {}", input)));

        assert_eq!(tool.name(), "test");
    }

    #[test]
    fn test_create_structured_tool() {
        let schema = create_args_schema(
            "test",
            {
                let mut props = HashMap::new();
                props.insert("x".to_string(), serde_json::json!({"type": "number"}));
                props
            },
            vec!["x".to_string()],
            None,
        );

        let tool = create_structured_tool("test", "A test tool", schema, |args| {
            let x = args.get("x").and_then(|v| v.as_f64()).unwrap_or(0.0);
            Ok(serde_json::json!(x * 2.0))
        });

        assert_eq!(tool.name(), "test");
    }

    #[test]
    fn test_render_tools() {
        let tools: Vec<Arc<dyn BaseTool>> = vec![
            Arc::new(create_simple_tool("tool1", "First tool", Ok)),
            Arc::new(create_simple_tool("tool2", "Second tool", Ok)),
        ];

        let rendered = render_text_description(&tools);
        assert!(rendered.contains("tool1"));
        assert!(rendered.contains("tool2"));
    }
}
