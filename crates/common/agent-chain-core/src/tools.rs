pub mod base;
pub mod convert;
pub mod render;
pub mod retriever;
pub mod simple;
pub mod structured;

pub use base::{
    ArgsSchema, BaseTool, BaseToolkit, DynTool, ErrorHandler, FILTERED_ARGS, ResponseFormat,
    TOOL_MESSAGE_BLOCK_TYPES, ToolDefinition, ToolInput, ToolOutput, ToolRunnable, format_output,
    is_message_content_block, is_message_content_type, is_tool_call, prep_run_args, stringify,
};

pub use simple::{AsyncToolFunc, Tool, ToolFunc};

pub use structured::{
    AsyncStructuredToolFunc, StructuredTool, StructuredToolFunc, create_args_schema,
};

pub use convert::{
    ToolConfig, convert_runnable_to_tool, get_description_from_runnable, tool_from_schema,
};

pub use render::{ToolsRenderer, render_text_description, render_text_description_and_args};

pub use retriever::{RetrieverInput, RetrieverTool};

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
    fn test_simple_tool() {
        let tool =
            Tool::from_function(|input| Ok(format!("Got: {}", input)), "test", "A test tool");
        assert_eq!(tool.name(), "test");
    }

    #[test]
    fn test_structured_tool() {
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

        let tool = StructuredTool::from_function(
            |args| {
                let x = args.get("x").and_then(|v| v.as_f64()).unwrap_or(0.0);
                Ok(serde_json::json!(x * 2.0))
            },
            "test",
            "A test tool",
            schema,
        );

        assert_eq!(tool.name(), "test");
    }

    #[test]
    fn test_render_tools() {
        let tools: Vec<Arc<dyn BaseTool>> = vec![
            Arc::new(Tool::from_function(Ok, "tool1", "First tool")),
            Arc::new(Tool::from_function(Ok, "tool2", "Second tool")),
        ];

        let rendered = render_text_description(&tools);
        assert!(rendered.contains("tool1"));
        assert!(rendered.contains("tool2"));
    }
}
