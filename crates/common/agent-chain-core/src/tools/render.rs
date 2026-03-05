use std::fmt::Write;
use std::sync::Arc;

use super::base::BaseTool;

pub type ToolsRenderer = fn(&[Arc<dyn BaseTool>]) -> String;

pub fn render_text_description(tools: &[Arc<dyn BaseTool>]) -> String {
    let mut out = String::new();
    for (i, tool) in tools.iter().enumerate() {
        if i > 0 {
            out.push('\n');
        }
        write!(out, "{} - {}", tool.name(), tool.description()).unwrap();
    }
    out
}

pub fn render_text_description_and_args(tools: &[Arc<dyn BaseTool>]) -> String {
    let mut out = String::new();
    for (i, tool) in tools.iter().enumerate() {
        if i > 0 {
            out.push('\n');
        }
        let args_schema = serde_json::to_string(&tool.args()).unwrap_or_else(|_| "{}".to_string());
        write!(
            out,
            "{} - {}, args: {}",
            tool.name(),
            tool.description(),
            args_schema
        )
        .unwrap();
    }
    out
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
