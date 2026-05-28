use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use agent_chain::language_models::ToolLike;
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
use crate::llm::openai_schema;
use crate::message_projection::{collect_thread_images, project_for_text_llm};
use crate::tool_catalog::{
    TurnCatalog, build_context_system_message, build_prelude_system_message,
};

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
/// its `CapabilityUpdate` frame, `active_contexts` are the structured
/// contexts the client said are live, and `prelude_blocks` is the
/// host-authored summary of what the user is currently doing (e.g. a
/// short natural-language block from the active activity strategy).
/// Tool descriptors and active contexts are filtered into the merged
/// [`TurnCatalog`] alongside the server-local tools; the LLM is bound
/// with the union so it sees one flat catalog. Prelude blocks and
/// active contexts each render into their own `SystemMessage`, with the
/// prelude first (what the user is doing) and the contexts second (which
/// tools are pinned to what) so the model reads context before tool
/// guidance.
pub async fn prepare_llm_context(
    providers: &Providers,
    asset_service: &Arc<AssetService>,
    mut messages: Vec<AnyMessage>,
    remote_descriptors: Vec<WireToolDescriptor>,
    active_contexts: &[WireActiveContext],
    prelude_blocks: Vec<ContentBlock>,
) -> Result<LlmContext, ThreadServiceError> {
    // Insertion order matters: the deepest `insert(0, ...)` ends up
    // closest to index 0, so push the *later*-rendered system message
    // first and the *earlier*-rendered one last. After both inserts the
    // head of `messages` reads: prelude (what the user is doing),
    // contexts (which tools are pinned to what), original history.
    if let Some(system_message) = build_context_system_message(active_contexts) {
        messages.insert(0, system_message.into());
    }
    if let Some(prelude_message) = build_prelude_system_message(prelude_blocks) {
        messages.insert(0, prelude_message.into());
    }

    resolve_blocks::<PlainTextBlock>(asset_service, &mut messages).await;

    let Some(vision) = providers.vision.as_ref() else {
        resolve_blocks::<ImageBlock>(asset_service, &mut messages).await;
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
    let tool_likes: Vec<ToolLike> = catalog
        .tool_likes()
        .iter()
        .map(|like| match like {
            ToolLike::Definition(def) => {
                let mut def = def.clone();
                openai_schema::normalize(&mut def.parameters);
                ToolLike::Definition(def)
            }
            other => other.clone(),
        })
        .collect();
    let bound = chat.bind_tools(&tool_likes, None).map_err(|e| {
        ThreadServiceError::Internal(format!("Failed to bind tools to chat model: {e}"))
    })?;
    Ok(Arc::from(bound as Box<dyn BaseChatModel + Send + Sync>))
}

/// Concurrent download fan-out. Storage backends are typically remote (S3),
/// so serial downloads multiply per-asset latency; 8 in-flight balances
/// throughput against connection pressure.
const ASSET_DOWNLOAD_CONCURRENCY: usize = 8;

/// Defines how a [`ContentBlock`] variant participates in the
/// download-and-rewrite pass driven by [`resolve_blocks`].
trait ResolvableBlock {
    /// Human-readable tag included in download-failure log lines.
    const LABEL: &'static str;

    /// The URL still pending resolution, if any. `None` means the block
    /// either has no URL or has already been resolved inline.
    fn pending_url(block: &ContentBlock) -> Option<&str>;

    /// Replace the block's URL reference with the downloaded bytes.
    /// Called only when [`Self::pending_url`] returned `Some(url)` and
    /// the URL was successfully fetched.
    fn apply(block: &mut ContentBlock, bytes: &[u8]);
}

struct PlainTextBlock;
impl ResolvableBlock for PlainTextBlock {
    const LABEL: &'static str = "plain-text";

    fn pending_url(block: &ContentBlock) -> Option<&str> {
        let ContentBlock::PlainText(pt) = block else {
            return None;
        };
        if pt.text.is_some() {
            return None;
        }
        pt.url.as_deref()
    }

    fn apply(block: &mut ContentBlock, bytes: &[u8]) {
        if let ContentBlock::PlainText(pt) = block {
            pt.text = Some(String::from_utf8_lossy(bytes).into_owned());
        }
    }
}

struct ImageBlock;
impl ResolvableBlock for ImageBlock {
    const LABEL: &'static str = "image";

    fn pending_url(block: &ContentBlock) -> Option<&str> {
        let ContentBlock::Image(img) = block else {
            return None;
        };
        if img.base64.is_some() {
            return None;
        }
        img.url.as_deref()
    }

    fn apply(block: &mut ContentBlock, bytes: &[u8]) {
        if let ContentBlock::Image(img) = block {
            img.base64 = Some(general_purpose::STANDARD.encode(bytes));
            img.url = None;
        }
    }
}

async fn resolve_blocks<B: ResolvableBlock>(
    asset_service: &AssetService,
    messages: &mut [AnyMessage],
) {
    let urls: HashSet<String> = iter_blocks(messages)
        .filter_map(|block| B::pending_url(block).map(str::to_owned))
        .collect();

    if urls.is_empty() {
        return;
    }

    let downloaded = download_many(asset_service.storage(), urls, B::LABEL).await;

    for block in iter_blocks_mut(messages) {
        let Some(url) = B::pending_url(block) else {
            continue;
        };
        if let Some(bytes) = downloaded.get(url) {
            B::apply(block, bytes);
        }
    }
}

fn iter_blocks(messages: &[AnyMessage]) -> impl Iterator<Item = &ContentBlock> {
    messages.iter().flat_map(|message| {
        let blocks: &[ContentBlock] = match message {
            AnyMessage::HumanMessage(m) => &m.content,
            AnyMessage::SystemMessage(m) => &m.content,
            _ => &[],
        };
        blocks.iter()
    })
}

fn iter_blocks_mut(messages: &mut [AnyMessage]) -> impl Iterator<Item = &mut ContentBlock> {
    messages.iter_mut().flat_map(|message| {
        let blocks: &mut [ContentBlock] = match message {
            AnyMessage::HumanMessage(m) => &mut m.content,
            AnyMessage::SystemMessage(m) => &mut m.content,
            _ => &mut [],
        };
        blocks.iter_mut()
    })
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
