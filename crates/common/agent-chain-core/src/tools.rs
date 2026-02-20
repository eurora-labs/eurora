pub mod base;
pub mod convert;
pub mod render;
pub mod retriever;
pub mod simple;
pub mod structured;

pub use base::{
    ArgsSchema, BaseTool, BaseToolkit, DynTool, FILTERED_ARGS, HandleToolError,
    HandleValidationError, InjectedToolArg, InjectedToolCallId, ResponseFormat,
    SchemaAnnotationError, TOOL_MESSAGE_BLOCK_TYPES, ToolDefinition, ToolException, ToolInput,
    ToolOutput, ToolRunnable, format_output, handle_tool_error_impl, handle_validation_error_impl,
    is_message_content_block, is_message_content_type, is_tool_call, prep_run_args,
    stringify_content,
};

pub use simple::{AsyncToolFunc, Tool, ToolFunc};

pub use structured::{
    AsyncStructuredToolFunc, StructuredTool, StructuredToolFunc, create_args_schema,
};

pub use convert::{
    ToolConfig, convert_runnable_to_tool, create_simple_tool, create_simple_tool_async,
    create_structured_tool, create_structured_tool_async, create_tool_with_config,
    get_description_from_runnable, tool_from_schema,
};

pub use render::{ToolsRenderer, render_text_description, render_text_description_and_args};

pub use retriever::{
    RetrieverInput, RetrieverToolBuilder, create_async_retriever_tool, create_retriever_tool,
    create_retriever_tool_with_options,
};

pub use base::BaseTool as LegacyTool;

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::sync::Arc;

    #[test]
    fn test_module_exports() {
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
