use std::collections::HashMap;
use std::sync::Arc;

use agent_chain::messages::ContentBlock;
use agent_chain::{AnyMessage, BaseChatModel, BaseTool, SystemMessage, language_models::ToolLike};
use base64::{Engine as _, engine::general_purpose};
use be_asset::AssetService;

use crate::describe_image_tool::{self, DescribeImageTool};
use crate::error::ThreadServiceError;
use crate::llm::Providers;
use crate::message_projection::{collect_thread_images, project_for_text_llm};

/// Per-turn LLM context: the messages to invoke the model with, the bound
/// model itself, and the tool registry the agent loop will dispatch from.
pub struct LlmContext {
    pub messages: Vec<AnyMessage>,
    pub chat_model: Arc<dyn BaseChatModel + Send + Sync>,
    pub tools: HashMap<String, Arc<dyn BaseTool>>,
}

/// Build the per-turn LLM context.
///
/// In text-only mode (no vision provider configured) we resolve every
/// referenced asset inline into the message blocks and hand the chat model a
/// vanilla message history. In vision mode we instead leave images referenced
/// by id, register a `describe_image` tool that the model can call to inspect
/// them lazily, and prepend a system prompt teaching the model how to use it.
pub async fn prepare_llm_context(
    providers: &Providers,
    asset_service: &Arc<AssetService>,
    mut messages: Vec<AnyMessage>,
) -> Result<LlmContext, ThreadServiceError> {
    resolve_plain_text_blocks(asset_service, &mut messages).await;

    let Some(vision) = providers.vision.as_ref() else {
        resolve_image_blocks(asset_service, &mut messages).await;
        return Ok(LlmContext {
            messages,
            chat_model: providers.chat.clone(),
            tools: HashMap::new(),
        });
    };

    let allowed_images = collect_thread_images(&messages);

    let mut tools: HashMap<String, Arc<dyn BaseTool>> = vision
        .default_tools
        .iter()
        .map(|tool| (tool.name().to_string(), tool.clone()))
        .collect();

    if !allowed_images.is_empty() {
        let describe = Arc::new(DescribeImageTool::new(
            vision.model.clone(),
            asset_service.clone(),
            allowed_images.clone(),
        )) as Arc<dyn BaseTool>;
        tools.insert(describe_image_tool::TOOL_NAME.to_string(), describe);
    }

    let tool_likes: Vec<ToolLike> = tools.values().cloned().map(ToolLike::Tool).collect();
    let bound = providers.chat.bind_tools(&tool_likes, None).map_err(|e| {
        ThreadServiceError::Internal(format!("Failed to bind tools to chat model: {e}"))
    })?;
    let chat_model: Arc<dyn BaseChatModel + Send + Sync> =
        Arc::from(bound as Box<dyn BaseChatModel + Send + Sync>);

    project_for_text_llm(&mut messages);

    if !allowed_images.is_empty() {
        let id_list = allowed_images
            .keys()
            .cloned()
            .collect::<Vec<_>>()
            .join(", ");
        let system_prompt = format!(
            "You cannot see attached images directly. To learn anything about an image \
             you MUST call the `describe_image` tool with that image's `image_id` and a \
             concrete `question`. Do not claim to have seen an image without calling the \
             tool first. Available image_ids: {id_list}."
        );
        messages.insert(
            0,
            SystemMessage::builder()
                .content(system_prompt)
                .build()
                .into(),
        );
    }

    Ok(LlmContext {
        messages,
        chat_model,
        tools,
    })
}

async fn resolve_plain_text_blocks(asset_service: &AssetService, messages: &mut [AnyMessage]) {
    let storage = asset_service.storage();
    for message in messages.iter_mut() {
        let content = match message {
            AnyMessage::HumanMessage(m) => &mut m.content,
            AnyMessage::SystemMessage(m) => &mut m.content,
            _ => continue,
        };
        for block in content.iter_mut() {
            let ContentBlock::PlainText(pt) = block else {
                continue;
            };
            if pt.text.is_some() {
                continue;
            }
            let Some(url) = pt.url.as_deref() else {
                continue;
            };
            match storage.download(url).await {
                Ok(bytes) => {
                    pt.text = Some(String::from_utf8_lossy(&bytes).into_owned());
                }
                Err(e) => {
                    tracing::warn!("Failed to download plain-text asset {url}: {e}");
                }
            }
        }
    }
}

async fn resolve_image_blocks(asset_service: &AssetService, messages: &mut [AnyMessage]) {
    let storage = asset_service.storage();
    for message in messages.iter_mut() {
        let content = match message {
            AnyMessage::HumanMessage(m) => &mut m.content,
            AnyMessage::SystemMessage(m) => &mut m.content,
            _ => continue,
        };
        for block in content.iter_mut() {
            let ContentBlock::Image(img) = block else {
                continue;
            };
            if img.base64.is_some() {
                continue;
            }
            let Some(url) = img.url.as_deref() else {
                continue;
            };
            match storage.download(url).await {
                Ok(bytes) => {
                    img.base64 = Some(general_purpose::STANDARD.encode(&bytes));
                    img.url = None;
                }
                Err(e) => {
                    tracing::warn!("Failed to download image asset {url}: {e}");
                }
            }
        }
    }
}
