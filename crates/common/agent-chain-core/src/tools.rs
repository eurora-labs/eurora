macro_rules! impl_base_tool_getters {
    () => {
        fn name(&self) -> &str {
            &self.meta.name
        }

        fn description(&self) -> &str {
            &self.meta.description
        }

        fn return_direct(&self) -> bool {
            self.meta.return_direct
        }

        fn verbose(&self) -> bool {
            self.meta.verbose
        }

        fn tags(&self) -> Option<&[String]> {
            self.meta.tags.as_deref()
        }

        fn metadata(&self) -> Option<&std::collections::HashMap<String, serde_json::Value>> {
            self.meta.metadata.as_ref()
        }

        fn handle_tool_error(&self) -> &$crate::tools::base::ErrorHandler {
            &self.meta.handle_tool_error
        }

        fn handle_validation_error(&self) -> &$crate::tools::base::ErrorHandler {
            &self.meta.handle_validation_error
        }

        fn response_format(&self) -> $crate::tools::base::ResponseFormat {
            self.meta.response_format
        }

        fn extras(&self) -> Option<&std::collections::HashMap<String, serde_json::Value>> {
            self.meta.extras.as_ref()
        }

        fn callbacks(&self) -> Option<&$crate::callbacks::Callbacks> {
            self.meta.callbacks.as_ref()
        }
    };
}

pub mod base;
pub mod convert;
pub mod render;
pub mod retriever;
pub mod simple;
pub mod structured;

pub use base::{
    ArgsSchema, BaseTool, BaseToolkit, DynTool, ErrorHandler, FILTERED_ARGS, ResponseFormat,
    TOOL_MESSAGE_BLOCK_TYPES, ToolDefinition, ToolInput, ToolMeta, ToolOutput, ToolRunnable,
    format_output, is_message_content_block, is_message_content_type, is_tool_call, prep_run_args,
    stringify,
};

pub use simple::{AsyncToolFunc, Tool, ToolFunc};

pub use structured::{
    AsyncStructuredToolFunc, StructuredTool, StructuredToolFunc, create_args_schema,
};

pub use convert::{
    PropertyDef, ToolConfig, convert_runnable_to_tool, get_description_from_runnable,
    tool_from_schema,
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
