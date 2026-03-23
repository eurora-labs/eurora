use std::fmt;
use std::sync::Arc;

use agent_chain::async_trait;
use agent_chain::error::{Error, Result};
use agent_chain::messages::content::{ContentBlock, ImageContentBlock, TextContentBlock};
use agent_chain::tools::base::{ArgsSchema, ToolInput, ToolOutput};
use agent_chain::{BaseChatModel, BaseTool, HumanMessage};
use serde_json::Value;

struct VisionTool {
    name: &'static str,
    description: &'static str,
    schema: ArgsSchema,
    model: Arc<dyn BaseChatModel + Send + Sync>,
    system_prompt: &'static str,
}

impl fmt::Debug for VisionTool {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("VisionTool")
            .field("name", &self.name)
            .finish()
    }
}

fn parse_image_input(image: &str) -> ImageContentBlock {
    if image.starts_with("data:") {
        if let Some((header, data)) = image.split_once(',') {
            let mime = header
                .strip_prefix("data:")
                .and_then(|s| s.strip_suffix(";base64"))
                .unwrap_or("image/png");
            ImageContentBlock::from_base64(data, mime)
        } else {
            ImageContentBlock::from_url(image)
        }
    } else {
        ImageContentBlock::from_url(image)
    }
}

fn get_string_arg(args: &std::collections::HashMap<String, Value>, key: &str) -> Option<String> {
    args.get(key).and_then(|v| v.as_str()).map(String::from)
}

fn require_string_arg(
    args: &std::collections::HashMap<String, Value>,
    key: &str,
) -> Result<String> {
    get_string_arg(args, key)
        .ok_or_else(|| Error::ToolException(format!("Missing required argument: {key}")))
}

async fn invoke_vision(
    model: &(dyn BaseChatModel + Send + Sync),
    image_block: ImageContentBlock,
    prompt: String,
) -> Result<String> {
    let content = vec![
        ContentBlock::Image(image_block),
        ContentBlock::Text(TextContentBlock::new(&prompt)),
    ];
    let message: agent_chain::AnyMessage = HumanMessage::builder()
        .content(agent_chain::messages::content::ContentBlocks::from(content))
        .build()
        .into();

    let response = model
        .invoke(vec![message], None)
        .await
        .map_err(|e| Error::ToolException(format!("Vision model invocation failed: {e}")))?;

    let text = response.text();
    if text.is_empty() {
        return Err(Error::ToolException(
            "Vision model returned empty response".into(),
        ));
    }
    Ok(text)
}

#[async_trait]
impl BaseTool for VisionTool {
    fn name(&self) -> &str {
        self.name
    }

    fn description(&self) -> &str {
        self.description
    }

    fn args_schema(&self) -> Option<&ArgsSchema> {
        Some(&self.schema)
    }

    async fn tool_run(
        &self,
        input: ToolInput,
        _run_manager: Option<&agent_chain::callbacks::CallbackManagerForToolRun>,
        _config: &agent_chain::runnables::RunnableConfig,
    ) -> Result<ToolOutput> {
        let args = match input {
            ToolInput::Dict(d) => d,
            ToolInput::String(s) => {
                let v: Value = serde_json::from_str(&s).map_err(|e| {
                    Error::ToolException(format!("Failed to parse tool input: {e}"))
                })?;
                v.as_object()
                    .map(|obj| obj.clone().into_iter().collect())
                    .unwrap_or_default()
            }
            ToolInput::ToolCall(tc) => tc
                .args
                .as_object()
                .map(|obj| obj.clone().into_iter().collect())
                .unwrap_or_default(),
        };

        let image_str = require_string_arg(&args, "image")?;
        let image_block = parse_image_input(&image_str);

        let prompt = match self.name {
            "ask_about_image" => {
                let question = require_string_arg(&args, "question")?;
                match get_string_arg(&args, "context") {
                    Some(ctx) => format!("Context: {ctx}\n\nQuestion: {question}"),
                    None => question,
                }
            }
            _ => {
                let base = self.system_prompt.to_string();
                match get_string_arg(&args, "context") {
                    Some(ctx) => format!("{base}\n\nAdditional context: {ctx}"),
                    None => base,
                }
            }
        };

        let result = invoke_vision(self.model.as_ref(), image_block, prompt).await?;
        Ok(ToolOutput::String(result))
    }
}

fn describe_image_schema() -> ArgsSchema {
    ArgsSchema::JsonSchema(serde_json::json!({
        "type": "object",
        "properties": {
            "image": {
                "type": "string",
                "description": "Image URL or base64 data URI (e.g. data:image/png;base64,...)"
            },
            "context": {
                "type": "string",
                "description": "Optional context about why you need this description, to help focus the response"
            }
        },
        "required": ["image"]
    }))
}

fn ask_about_image_schema() -> ArgsSchema {
    ArgsSchema::JsonSchema(serde_json::json!({
        "type": "object",
        "properties": {
            "image": {
                "type": "string",
                "description": "Image URL or base64 data URI (e.g. data:image/png;base64,...)"
            },
            "question": {
                "type": "string",
                "description": "The specific question to ask about the image"
            },
            "context": {
                "type": "string",
                "description": "Optional context to help the vision model understand what you're looking for"
            }
        },
        "required": ["image", "question"]
    }))
}

fn extract_text_schema() -> ArgsSchema {
    ArgsSchema::JsonSchema(serde_json::json!({
        "type": "object",
        "properties": {
            "image": {
                "type": "string",
                "description": "Image URL or base64 data URI (e.g. data:image/png;base64,...)"
            }
        },
        "required": ["image"]
    }))
}

pub fn vision_tools(model: Arc<dyn BaseChatModel + Send + Sync>) -> Vec<Arc<dyn BaseTool>> {
    vec![
        Arc::new(VisionTool {
            name: "describe_image",
            description: "Describe an image in detail. Returns a comprehensive text description of the image contents. Use this when you need to understand what's in an image.",
            schema: describe_image_schema(),
            model: model.clone(),
            system_prompt: "Describe this image in detail. Include key visual elements, objects, text, colors, layout, and any notable features.",
        }),
        Arc::new(VisionTool {
            name: "ask_about_image",
            description: "Ask a specific question about an image. Use this when you need targeted information from an image rather than a full description.",
            schema: ask_about_image_schema(),
            model: model.clone(),
            system_prompt: "",
        }),
        Arc::new(VisionTool {
            name: "extract_text",
            description: "Extract all readable text from an image (OCR). Use this for screenshots, documents, photos of signs, or any image containing text.",
            schema: extract_text_schema(),
            model,
            system_prompt: "Extract all readable text from this image. Preserve the original layout and structure as much as possible. If no text is found, state that clearly.",
        }),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_image_input_url() {
        let block = parse_image_input("https://example.com/image.png");
        assert_eq!(block.url.as_deref(), Some("https://example.com/image.png"));
        assert!(block.base64.is_none());
    }

    #[test]
    fn test_parse_image_input_base64() {
        let block = parse_image_input("data:image/jpeg;base64,/9j/4AAQ");
        assert_eq!(block.base64.as_deref(), Some("/9j/4AAQ"));
        assert_eq!(block.mime_type.as_deref(), Some("image/jpeg"));
        assert!(block.url.is_none());
    }

    #[test]
    fn test_parse_image_input_base64_png() {
        let block = parse_image_input("data:image/png;base64,iVBOR");
        assert_eq!(block.base64.as_deref(), Some("iVBOR"));
        assert_eq!(block.mime_type.as_deref(), Some("image/png"));
    }

    #[test]
    fn test_vision_tools_count() {
        use agent_chain::openai::ChatOpenAI;
        let model: Arc<dyn BaseChatModel + Send + Sync> = Arc::new(
            ChatOpenAI::builder()
                .model("gpt-4o")
                .api_key("test")
                .build(),
        );
        let tools = vision_tools(model);
        assert_eq!(tools.len(), 3);
        assert_eq!(tools[0].name(), "describe_image");
        assert_eq!(tools[1].name(), "ask_about_image");
        assert_eq!(tools[2].name(), "extract_text");
    }

    #[test]
    fn test_schemas_have_required_fields() {
        use agent_chain::openai::ChatOpenAI;
        let model: Arc<dyn BaseChatModel + Send + Sync> = Arc::new(
            ChatOpenAI::builder()
                .model("gpt-4o")
                .api_key("test")
                .build(),
        );
        let tools = vision_tools(model);

        for tool in &tools {
            let schema = tool.args_schema().expect("should have schema");
            let json = schema.to_json_schema();
            assert_eq!(json["type"], "object");
            let required = json["required"].as_array().expect("should have required");
            assert!(
                required.contains(&Value::String("image".to_string())),
                "Tool {} should require 'image'",
                tool.name()
            );
        }

        let ask_schema = tools[1].args_schema().unwrap().to_json_schema();
        let required = ask_schema["required"].as_array().unwrap();
        assert!(required.contains(&Value::String("question".to_string())));
    }
}
