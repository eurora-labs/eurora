use std::collections::{BTreeMap, HashMap};
use std::fmt;
use std::sync::Arc;

use agent_chain::async_trait;
use agent_chain::callbacks::manager::CallbackManagerForToolRun;
use agent_chain::error::{Error, Result};
use agent_chain::messages::{ContentBlock, ImageContentBlock, TextContentBlock};
use agent_chain::runnables::RunnableConfig;
use agent_chain::tools::{ArgsSchema, BaseTool, ToolInput, ToolOutput};
use agent_chain::{BaseChatModel, HumanMessage, SystemMessage};
use base64::{Engine as _, engine::general_purpose};
use be_asset::AssetService;
use serde_json::{Value, json};

pub(crate) const TOOL_NAME: &str = "describe_image";

const TOOL_DESCRIPTION: &str = "You cannot see attached images directly. Use this tool to ask a \
    vision model about an image that appears in the conversation. Pass the `image_id` shown next \
    to the image placeholder and a specific `question` about the image. Concrete questions \
    produce better answers. You may call this tool multiple times per image (different \
    questions) and across different images. The response is plain text truncated to \
    4000 characters.";

const VISION_SYSTEM_PROMPT: &str = "You are a vision assistant. Answer the user's question about \
    the attached image concisely and factually. Do not speculate beyond what is visible; if the \
    image does not contain the requested information, say so plainly.";

const MAX_DESCRIPTION_CHARS: usize = 4_000;
const DEFAULT_MIME_TYPE: &str = "image/png";

pub(crate) struct DescribeImageTool {
    vision_model: Arc<dyn BaseChatModel + Send + Sync>,
    asset_service: Arc<AssetService>,
    allowed_images: HashMap<String, ImageContentBlock>,
    args_schema: ArgsSchema,
}

impl DescribeImageTool {
    pub(crate) fn new(
        vision_model: Arc<dyn BaseChatModel + Send + Sync>,
        asset_service: Arc<AssetService>,
        allowed_images: BTreeMap<String, ImageContentBlock>,
    ) -> Self {
        let allowed_ids: Vec<String> = allowed_images.keys().cloned().collect();
        let args_schema = ArgsSchema::JsonSchema(json!({
            "type": "object",
            "properties": {
                "image_id": {
                    "type": "string",
                    "description": "The image_id shown in the attached-image placeholder. \
                                    Pass the value exactly, with no surrounding quotes or brackets.",
                    "enum": allowed_ids,
                },
                "question": {
                    "type": "string",
                    "description": "A specific question to ask the vision model about the image."
                }
            },
            "required": ["image_id", "question"],
            "additionalProperties": false
        }));

        Self {
            vision_model,
            asset_service,
            allowed_images: allowed_images.into_iter().collect(),
            args_schema,
        }
    }

    async fn run(&self, args: DescribeImageArgs) -> Result<String> {
        let block = self.allowed_images.get(&args.image_id).ok_or_else(|| {
            Error::ToolException(format!(
                "Unknown image_id '{}'. Use one of the image_ids listed in the conversation \
                 placeholders.",
                args.image_id
            ))
        })?;

        let (mime_type, base64_data) = self.resolve_image_bytes(block).await?;

        let vision_image = ImageContentBlock::builder()
            .base64(base64_data)
            .mime_type(mime_type)
            .build()
            .map_err(|e| Error::ToolException(format!("Failed to construct image block: {e}")))?;

        let messages = vec![
            SystemMessage::builder()
                .content(VISION_SYSTEM_PROMPT.to_string())
                .build()
                .into(),
            HumanMessage::builder()
                .content(vec![
                    ContentBlock::Image(vision_image),
                    ContentBlock::Text(TextContentBlock::builder().text(args.question).build()),
                ])
                .build()
                .into(),
        ];

        let response = self
            .vision_model
            .invoke(messages, None)
            .await
            .map_err(|e| Error::ToolException(format!("Vision model request failed: {e}")))?;

        Ok(truncate_description(response.content.to_string()))
    }

    async fn resolve_image_bytes(&self, block: &ImageContentBlock) -> Result<(String, String)> {
        let mime_type = block
            .mime_type
            .clone()
            .unwrap_or_else(|| DEFAULT_MIME_TYPE.to_string());

        if let Some(b64) = block.base64.as_ref() {
            return Ok((mime_type, b64.clone()));
        }

        let url = block.url.as_deref().ok_or_else(|| {
            Error::ToolException(
                "Image has no resolvable storage location (no url or base64).".into(),
            )
        })?;

        let bytes = self
            .asset_service
            .storage()
            .download(url)
            .await
            .map_err(|e| Error::ToolException(format!("Failed to download image asset: {e}")))?;

        Ok((mime_type, general_purpose::STANDARD.encode(&bytes)))
    }
}

impl fmt::Debug for DescribeImageTool {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DescribeImageTool")
            .field("allowed_images", &self.allowed_images.keys())
            .finish_non_exhaustive()
    }
}

#[async_trait]
impl BaseTool for DescribeImageTool {
    fn name(&self) -> &str {
        TOOL_NAME
    }

    fn description(&self) -> &str {
        TOOL_DESCRIPTION
    }

    fn args_schema(&self) -> Option<&ArgsSchema> {
        Some(&self.args_schema)
    }

    async fn tool_run(
        &self,
        input: ToolInput,
        _run_manager: Option<&CallbackManagerForToolRun>,
        _config: &RunnableConfig,
    ) -> Result<ToolOutput> {
        let args = DescribeImageArgs::from_input(input)?;
        let description = self.run(args).await?;
        Ok(ToolOutput::String(description))
    }
}

#[derive(Debug, PartialEq, Eq)]
struct DescribeImageArgs {
    image_id: String,
    question: String,
}

impl DescribeImageArgs {
    fn from_input(input: ToolInput) -> Result<Self> {
        let value = match input {
            ToolInput::ToolCall(tc) => tc.args,
            ToolInput::Dict(map) => Value::Object(map.into_iter().collect()),
            ToolInput::String(s) => serde_json::from_str::<Value>(&s).map_err(|e| {
                Error::ToolException(format!("describe_image input was not valid JSON: {e}"))
            })?,
        };
        Self::from_value(&value)
    }

    fn from_value(value: &Value) -> Result<Self> {
        let image_id = required_string(value, "image_id")?;
        let question = required_string(value, "question")?;
        Ok(Self { image_id, question })
    }
}

fn required_string(value: &Value, field: &str) -> Result<String> {
    match value.get(field).and_then(|v| v.as_str()) {
        Some(s) if !s.is_empty() => Ok(s.to_string()),
        Some(_) => Err(Error::ToolException(format!(
            "describe_image argument '{field}' must be a non-empty string"
        ))),
        None => Err(Error::ToolException(format!(
            "describe_image argument '{field}' is required"
        ))),
    }
}

fn truncate_description(description: String) -> String {
    if description.chars().count() <= MAX_DESCRIPTION_CHARS {
        return description;
    }
    let truncated: String = description.chars().take(MAX_DESCRIPTION_CHARS).collect();
    format!("{truncated}\n\n[Description truncated at {MAX_DESCRIPTION_CHARS} characters]")
}

#[cfg(test)]
mod tests {
    use super::*;
    use agent_chain::messages::ToolCall;

    #[test]
    fn from_input_parses_tool_call_args() {
        let tc = ToolCall::builder()
            .name(TOOL_NAME)
            .args(json!({"image_id": "abc", "question": "What is this?"}))
            .build();
        let args = DescribeImageArgs::from_input(ToolInput::ToolCall(tc)).unwrap();
        assert_eq!(args.image_id, "abc");
        assert_eq!(args.question, "What is this?");
    }

    #[test]
    fn from_input_parses_string_json() {
        let input =
            ToolInput::String(r#"{"image_id": "abc", "question": "What is this?"}"#.to_string());
        let args = DescribeImageArgs::from_input(input).unwrap();
        assert_eq!(args.image_id, "abc");
    }

    #[test]
    fn from_input_rejects_missing_field() {
        let tc = ToolCall::builder()
            .name(TOOL_NAME)
            .args(json!({"image_id": "abc"}))
            .build();
        let err = DescribeImageArgs::from_input(ToolInput::ToolCall(tc)).unwrap_err();
        assert!(err.to_string().contains("question"));
    }

    #[test]
    fn from_input_rejects_empty_string() {
        let tc = ToolCall::builder()
            .name(TOOL_NAME)
            .args(json!({"image_id": "", "question": "What?"}))
            .build();
        let err = DescribeImageArgs::from_input(ToolInput::ToolCall(tc)).unwrap_err();
        assert!(err.to_string().contains("image_id"));
    }

    #[test]
    fn from_input_rejects_invalid_json_string() {
        let err = DescribeImageArgs::from_input(ToolInput::String("not json".into())).unwrap_err();
        assert!(err.to_string().contains("valid JSON"));
    }

    #[test]
    fn truncate_description_noop_under_limit() {
        let description = "short".to_string();
        assert_eq!(truncate_description(description.clone()), description);
    }

    #[test]
    fn truncate_description_cuts_over_limit() {
        let description = "a".repeat(MAX_DESCRIPTION_CHARS + 100);
        let truncated = truncate_description(description);
        assert!(truncated.contains("[Description truncated"));
        assert!(truncated.chars().count() < MAX_DESCRIPTION_CHARS + 100);
    }
}
