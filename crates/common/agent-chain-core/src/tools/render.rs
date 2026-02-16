//! Utilities to render tools.
//!
//! Mirrors `langchain_core.tools.render`.

use std::sync::Arc;

use super::base::BaseTool;

/// Type alias for a tools renderer function.
pub type ToolsRenderer = fn(&[Arc<dyn BaseTool>]) -> String;

/// Render the tool name and description in plain text.
///
/// Output will be in the format of:
/// ```text
/// search: This tool is used for search
/// calculator: This tool is used for math
/// ```
pub fn render_text_description(tools: &[Arc<dyn BaseTool>]) -> String {
    let descriptions: Vec<String> = tools
        .iter()
        .map(|tool| format!("{} - {}", tool.name(), tool.description()))
        .collect();

    descriptions.join("\n")
}

/// Render the tool name, description, and args in plain text.
///
/// Output will be in the format of:
/// ```text
/// search: This tool is used for search, args: {"query": {"type": "string"}}
/// calculator: This tool is used for math, args: {"expression": {"type": "string"}}
/// ```
pub fn render_text_description_and_args(tools: &[Arc<dyn BaseTool>]) -> String {
    let tool_strings: Vec<String> = tools
        .iter()
        .map(|tool| {
            let args_schema =
                serde_json::to_string(&tool.args()).unwrap_or_else(|_| "{}".to_string());
            format!(
                "{} - {}, args: {}",
                tool.name(),
                tool.description(),
                args_schema
            )
        })
        .collect();

    tool_strings.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::simple::Tool;

    fn create_test_tools() -> Vec<Arc<dyn BaseTool>> {
        vec![
            Arc::new(Tool::from_function(
                |input| Ok(format!("Searched: {}", input)),
                "search",
                "Search for information",
            )) as Arc<dyn BaseTool>,
            Arc::new(Tool::from_function(
                |input| Ok(format!("Calculated: {}", input)),
                "calculator",
                "Perform calculations",
            )) as Arc<dyn BaseTool>,
        ]
    }

    #[test]
    fn test_render_text_description() {
        let tools = create_test_tools();
        let rendered = render_text_description(&tools);

        assert!(rendered.contains("search - Search for information"));
        assert!(rendered.contains("calculator - Perform calculations"));
    }

    #[test]
    fn test_render_text_description_and_args() {
        let tools = create_test_tools();
        let rendered = render_text_description_and_args(&tools);

        assert!(rendered.contains("search -"));
        assert!(rendered.contains("args:"));
    }
}
