use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use agent_chain::messages::ContentBlock;
use agent_chain::{AnyMessage, BaseChatModel, BaseTool, SystemMessage};
use base64::{Engine as _, engine::general_purpose};
use be_asset::AssetService;
use be_storage::StorageService;
use futures::stream::{self, StreamExt};
use thread_core::{WireActiveContext, WireToolDescriptor};

use crate::describe_image_tool::{self, DescribeImageTool};
use crate::error::ThreadServiceError;
use crate::llm::Providers;
use crate::message_projection::{collect_thread_images, project_for_text_llm};
use crate::tool_catalog::{TurnCatalog, build_context_system_message};

/// Per-turn LLM context: the messages to invoke the model with, the bound
/// model itself, and the unified tool catalog the agent loop will dispatch
/// from.
pub struct LlmContext {
    pub messages: Vec<AnyMessage>,
    pub chat_model: Arc<dyn BaseChatModel + Send + Sync>,
    pub catalog: Arc<TurnCatalog>,
}

/// Build the per-turn LLM context.
///
/// In text-only mode (no vision provider configured) we resolve every
/// referenced asset inline into the message blocks and hand the chat model a
/// vanilla message history. In vision mode we instead leave images referenced
/// by id, register a `describe_image` tool that the model can call to inspect
/// them lazily, and prepend a system prompt teaching the model how to use it.
///
/// `remote_descriptors` are the tool descriptors the client advertised in
/// its `CapabilityUpdate` frame, and `active_contexts` are the contexts the
/// client said are live. Both are filtered into the merged
/// [`TurnCatalog`] alongside the server-local tools; the LLM is bound with
/// the union so it sees one flat catalog.
pub async fn prepare_llm_context(
    providers: &Providers,
    asset_service: &Arc<AssetService>,
    mut messages: Vec<AnyMessage>,
    remote_descriptors: Vec<WireToolDescriptor>,
    active_contexts: &[WireActiveContext],
) -> Result<LlmContext, ThreadServiceError> {
    if let Some(system_message) = build_context_system_message(active_contexts) {
        messages.insert(0, system_message.into());
    }

    resolve_plain_text_blocks(asset_service, &mut messages).await;

    let Some(vision) = providers.vision.as_ref() else {
        resolve_image_blocks(asset_service, &mut messages).await;
        let catalog = build_catalog(Vec::new(), remote_descriptors, active_contexts)?;
        let chat_model = bind_chat_model(&providers.chat, &catalog)?;
        return Ok(LlmContext {
            messages,
            chat_model,
            catalog,
        });
    };

    let allowed_images = collect_thread_images(&messages);

    let mut server_local: Vec<Arc<dyn BaseTool>> = vision.default_tools.to_vec();

    if !allowed_images.is_empty() {
        let describe = Arc::new(DescribeImageTool::new(
            vision.model.clone(),
            asset_service.clone(),
            allowed_images.clone(),
        )) as Arc<dyn BaseTool>;
        server_local.push(describe);
    }

    let catalog = build_catalog(server_local, remote_descriptors, active_contexts)?;
    let chat_model = bind_chat_model(&providers.chat, &catalog)?;

    project_for_text_llm(&mut messages);

    if !allowed_images.is_empty() {
        let id_list = allowed_images
            .keys()
            .cloned()
            .collect::<Vec<_>>()
            .join(", ");
        let system_prompt = format!(
            "You cannot see attached images directly. To learn anything about an image \
             you MUST call the `{tool}` tool with that image's `image_id` and a \
             concrete `question`. Do not claim to have seen an image without calling the \
             tool first. Available image_ids: {id_list}.",
            tool = describe_image_tool::TOOL_NAME,
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
        catalog,
    })
}

fn build_catalog(
    server_local: Vec<Arc<dyn BaseTool>>,
    remote: Vec<WireToolDescriptor>,
    active_contexts: &[WireActiveContext],
) -> Result<Arc<TurnCatalog>, ThreadServiceError> {
    // `CatalogBuildError` only fires for tool-name collisions, which mean
    // the client advertised a malformed `CapabilityUpdate` — a protocol
    // fault, not a generic invalid-argument. The chat handler surfaces it
    // to the client as `Error { kind: "protocol", ... }`.
    TurnCatalog::build(server_local, remote, active_contexts)
        .map(Arc::new)
        .map_err(|err| ThreadServiceError::ProtocolViolation(err.to_string()))
}

fn bind_chat_model(
    chat: &Arc<dyn BaseChatModel + Send + Sync>,
    catalog: &TurnCatalog,
) -> Result<Arc<dyn BaseChatModel + Send + Sync>, ThreadServiceError> {
    if catalog.is_empty() {
        return Ok(chat.clone());
    }
    let tool_likes = catalog.tool_likes();
    let bound = chat.bind_tools(&tool_likes, None).map_err(|e| {
        ThreadServiceError::Internal(format!("Failed to bind tools to chat model: {e}"))
    })?;
    Ok(Arc::from(bound as Box<dyn BaseChatModel + Send + Sync>))
}

/// Concurrent download fan-out. Storage backends are typically remote (S3),
/// so serial downloads multiply per-asset latency; 8 in-flight balances
/// throughput against connection pressure.
const ASSET_DOWNLOAD_CONCURRENCY: usize = 8;

async fn resolve_plain_text_blocks(asset_service: &AssetService, messages: &mut [AnyMessage]) {
    let mut urls: HashSet<String> = HashSet::new();
    for_each_resolve_block(messages, |block| {
        if let ContentBlock::PlainText(pt) = block
            && pt.text.is_none()
            && let Some(url) = pt.url.as_deref()
        {
            urls.insert(url.to_string());
        }
    });

    let downloaded = download_many(asset_service.storage(), urls, "plain-text").await;

    for_each_resolve_block_mut(messages, |block| {
        let ContentBlock::PlainText(pt) = block else {
            return;
        };
        if pt.text.is_some() {
            return;
        }
        let Some(url) = pt.url.as_deref() else {
            return;
        };
        if let Some(bytes) = downloaded.get(url) {
            pt.text = Some(String::from_utf8_lossy(bytes).into_owned());
        }
    });
}

async fn resolve_image_blocks(asset_service: &AssetService, messages: &mut [AnyMessage]) {
    let mut urls: HashSet<String> = HashSet::new();
    for_each_resolve_block(messages, |block| {
        if let ContentBlock::Image(img) = block
            && img.base64.is_none()
            && let Some(url) = img.url.as_deref()
        {
            urls.insert(url.to_string());
        }
    });

    let downloaded = download_many(asset_service.storage(), urls, "image").await;

    for_each_resolve_block_mut(messages, |block| {
        let ContentBlock::Image(img) = block else {
            return;
        };
        if img.base64.is_some() {
            return;
        }
        let Some(url) = img.url.as_deref() else {
            return;
        };
        if let Some(bytes) = downloaded.get(url) {
            img.base64 = Some(general_purpose::STANDARD.encode(bytes));
            img.url = None;
        }
    });
}

fn for_each_resolve_block(messages: &[AnyMessage], mut f: impl FnMut(&ContentBlock)) {
    for message in messages {
        let blocks: &[ContentBlock] = match message {
            AnyMessage::HumanMessage(m) => &m.content,
            AnyMessage::SystemMessage(m) => &m.content,
            _ => continue,
        };
        for block in blocks {
            f(block);
        }
    }
}

fn for_each_resolve_block_mut(messages: &mut [AnyMessage], mut f: impl FnMut(&mut ContentBlock)) {
    for message in messages {
        let blocks: &mut [ContentBlock] = match message {
            AnyMessage::HumanMessage(m) => &mut m.content,
            AnyMessage::SystemMessage(m) => &mut m.content,
            _ => continue,
        };
        for block in blocks {
            f(block);
        }
    }
}

async fn download_many(
    storage: &StorageService,
    urls: HashSet<String>,
    label: &'static str,
) -> HashMap<String, Vec<u8>> {
    stream::iter(urls)
        .map(|url| async move {
            match storage.download(&url).await {
                Ok(bytes) => Some((url, bytes)),
                Err(e) => {
                    tracing::warn!("Failed to download {label} asset {url}: {e}");
                    None
                }
            }
        })
        .buffer_unordered(ASSET_DOWNLOAD_CONCURRENCY)
        .filter_map(|opt| async move { opt })
        .collect()
        .await
}
