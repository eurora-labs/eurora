//! Utilities to render tools.
//!
//! This module provides functions for rendering tool descriptions
//! in various text formats, mirroring `langchain_core.tools.render`.

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

/// Render tools as a JSON array of tool definitions.
pub fn render_json(tools: &[Arc<dyn BaseTool>]) -> String {
    let definitions: Vec<_> = tools.iter().map(|t| t.definition()).collect();
    serde_json::to_string_pretty(&definitions).unwrap_or_else(|_| "[]".to_string())
}

/// Render tools as a compact JSON array.
pub fn render_json_compact(tools: &[Arc<dyn BaseTool>]) -> String {
    let definitions: Vec<_> = tools.iter().map(|t| t.definition()).collect();
    serde_json::to_string(&definitions).unwrap_or_else(|_| "[]".to_string())
}

/// Render a single tool as a formatted string.
pub fn render_tool(tool: &dyn BaseTool) -> String {
    format!(
        "Tool: {}\nDescription: {}\nArguments: {}",
        tool.name(),
        tool.description(),
        serde_json::to_string_pretty(&tool.args()).unwrap_or_else(|_| "{}".to_string())
    )
}

/// Render tools in a format suitable for system prompts.
pub fn render_for_prompt(tools: &[Arc<dyn BaseTool>]) -> String {
    let mut output = String::from("Available tools:\n\n");

    for (i, tool) in tools.iter().enumerate() {
        output.push_str(&format!("{}. {}\n", i + 1, tool.name()));
        output.push_str(&format!("   Description: {}\n", tool.description()));

        let args = tool.args();
        if !args.is_empty() {
            output.push_str("   Arguments:\n");
            for (name, schema) in args {
                let type_str = schema.get("type").and_then(|t| t.as_str()).unwrap_or("any");
                let desc = schema
                    .get("description")
                    .and_then(|d| d.as_str())
                    .unwrap_or("");
                output.push_str(&format!("     - {} ({}): {}\n", name, type_str, desc));
            }
        }
        output.push('\n');
    }

    output
}

/// Render tools as a numbered list.
pub fn render_numbered_list(tools: &[Arc<dyn BaseTool>]) -> String {
    tools
        .iter()
        .enumerate()
        .map(|(i, tool)| format!("{}. {} - {}", i + 1, tool.name(), tool.description()))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Render tools with their full schemas.
pub fn render_with_schemas(tools: &[Arc<dyn BaseTool>]) -> String {
    let mut output = String::new();

    for tool in tools {
        output.push_str(&format!("## {}\n\n", tool.name()));
        output.push_str(&format!("{}\n\n", tool.description()));
        output.push_str("### Schema\n\n");
        output.push_str("```json\n");
        output.push_str(&serde_json::to_string_pretty(&tool.definition()).unwrap_or_default());
        output.push_str("\n```\n\n");
    }

    output
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

    #[test]
    fn test_render_json() {
        let tools = create_test_tools();
        let rendered = render_json(&tools);

        assert!(rendered.contains("\"name\": \"search\""));
        assert!(rendered.contains("\"name\": \"calculator\""));
    }

    #[test]
    fn test_render_for_prompt() {
        let tools = create_test_tools();
        let rendered = render_for_prompt(&tools);

        assert!(rendered.contains("Available tools:"));
        assert!(rendered.contains("1. search"));
        assert!(rendered.contains("2. calculator"));
    }

    #[test]
    fn test_render_numbered_list() {
        let tools = create_test_tools();
        let rendered = render_numbered_list(&tools);

        assert!(rendered.starts_with("1."));
        assert!(rendered.contains("2. calculator"));
    }

    #[test]
    fn test_render_tool() {
        let tool = Tool::from_function(Ok, "test", "A test tool");

        let rendered = render_tool(&tool);

        assert!(rendered.contains("Tool: test"));
        assert!(rendered.contains("Description: A test tool"));
    }
}
