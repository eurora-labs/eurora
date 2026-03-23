use agent_chain::SystemMessage;
use agent_chain::openai::BuiltinTool;
use agent_chain::{
    AIMessage, AnyMessage, BaseChatModel, BaseTool, HumanMessage, language_models::ToolLike,
    messages::ToolCall, ollama::ChatOllama, openai::ChatOpenAI,
};
use be_asset::AssetService;
use be_authz::{extract_claims, parse_user_id};
use be_local_settings::{OllamaConfig, OpenAIConfig, ProviderSettings, SettingsReceiver};
use be_remote_db::{Asset, DatabaseManager, MessageType, PaginationParams};
use chrono::{DateTime, Utc};
use prost_types::Timestamp;
use proto_gen::agent_chain::ProtoAiMessageChunk;
pub use proto_gen::thread::proto_thread_service_server::{
    ProtoThreadService, ProtoThreadServiceServer,
};
use proto_gen::thread::{
    AddHiddenHumanMessageRequest, AddHiddenHumanMessageResponse, AddHumanMessageRequest,
    AddHumanMessageResponse, AddSystemMessageRequest, AddSystemMessageResponse, ChatStreamRequest,
    CreateThreadRequest, CreateThreadResponse, DeleteThreadRequest, DeleteThreadResponse,
    GenerateThreadTitleRequest, GenerateThreadTitleResponse, GetMessageTreeRequest,
    GetMessageTreeResponse, GetMessagesRequest, GetMessagesResponse, GetThreadResponse,
    ListThreadsRequest, ListThreadsResponse, MessageSiblingInfo, MessageTreeNode,
    SearchMessageResult, SearchMessagesRequest, SearchMessagesResponse, SearchThreadResult,
    SearchThreadsRequest, SearchThreadsResponse, SwitchBranchRequest, Thread,
};
use secrecy::ExposeSecret;
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::{Arc, RwLock};
use tokio_stream::{Stream, StreamExt};
use tonic::{Request, Response, Status};
use uuid::Uuid;

use crate::converters::{
    convert_db_message_to_base_message, convert_db_message_to_base_message_async,
    create_image_assets, extract_message_content,
};
use crate::error::ThreadServiceError;
use crate::tools::firecrawl_tools;
use crate::vision_tools::vision_tools;

const BASE_NEBUL_URL: &str = "https://api.inference.nebul.io/v1";

fn resolve_host_url(url: &str) -> String {
    let local_mode = std::env::var("RUNNING_EURORA_FULLY_LOCAL")
        .map(|v| v.eq_ignore_ascii_case("true"))
        .unwrap_or(false);
    if local_mode {
        url.replace("://localhost", "://host.docker.internal")
            .replace("://127.0.0.1", "://host.docker.internal")
    } else {
        url.to_string()
    }
}

struct Providers {
    chat: Arc<dyn BaseChatModel + Send + Sync>,
    title: Arc<dyn BaseChatModel + Send + Sync>,
    tools: HashMap<String, Arc<dyn BaseTool>>,
}

fn build_ollama(
    config: &OllamaConfig,
    model_override: Option<&str>,
) -> Box<dyn BaseChatModel + Send + Sync> {
    let model = model_override.unwrap_or(&config.model);
    let base_url = resolve_host_url(config.base_url.as_str());
    Box::new(
        ChatOllama::builder()
            .model(model)
            .base_url(&base_url)
            .build(),
    )
}

fn build_openai(
    config: &OpenAIConfig,
    model_override: Option<&str>,
    web_search: bool,
) -> Box<dyn BaseChatModel + Send + Sync> {
    let model = model_override.unwrap_or(&config.model);
    let is_openai_native = config.base_url.as_str().contains("openai.com");
    let mut provider = ChatOpenAI::builder()
        .model(model)
        .api_key(config.api_key.expose_secret())
        .api_base(config.base_url.as_str())
        .use_responses_api(is_openai_native)
        .build();
    if web_search && is_openai_native {
        provider = provider.with_builtin_tools(vec![BuiltinTool::WebSearch]);
    }
    Box::new(provider)
}

fn build_chat_provider_from(settings: &ProviderSettings) -> Box<dyn BaseChatModel + Send + Sync> {
    match settings {
        ProviderSettings::Ollama(c) => build_ollama(c, None),
        ProviderSettings::OpenAI(c) => build_openai(c, None, true),
    }
}

fn build_title_provider_from(settings: &ProviderSettings) -> Box<dyn BaseChatModel + Send + Sync> {
    match settings {
        ProviderSettings::Ollama(c) => build_ollama(c, None),
        ProviderSettings::OpenAI(c) => {
            let title_model = c.title_model.as_deref();
            build_openai(c, title_model, false)
        }
    }
}

fn build_tool_map(
    vision_model: Option<Arc<dyn BaseChatModel + Send + Sync>>,
) -> HashMap<String, Arc<dyn BaseTool>> {
    let mut tools: Vec<Arc<dyn BaseTool>> = firecrawl_tools();
    if let Some(vm) = vision_model {
        tools.extend(vision_tools(vm));
    }
    tools
        .into_iter()
        .map(|tool| (tool.name().to_string(), tool))
        .collect()
}

fn build_env_vision_model() -> Option<Arc<dyn BaseChatModel + Send + Sync>> {
    let vision_model = std::env::var("VISION_MODEL").ok()?;
    let local_mode = std::env::var("RUNNING_EURORA_FULLY_LOCAL")
        .map(|v| v.eq_ignore_ascii_case("true"))
        .unwrap_or(false);

    let model: Box<dyn BaseChatModel + Send + Sync> = if local_mode {
        let host = std::env::var("OLLAMA_HOST")
            .unwrap_or_else(|_| "http://host.docker.internal:11434".to_string());
        Box::new(
            ChatOllama::builder()
                .model(&vision_model)
                .base_url(resolve_host_url(&host))
                .build(),
        )
    } else {
        let api_key = std::env::var("NEBUL_API_KEY").ok()?;
        Box::new(
            ChatOpenAI::builder()
                .model(&vision_model)
                .api_key(&api_key)
                .api_base(BASE_NEBUL_URL)
                .build(),
        )
    };

    tracing::info!("Vision model configured: {vision_model}");
    Some(Arc::from(model))
}

fn build_env_fallback() -> Option<Providers> {
    let local_mode = std::env::var("RUNNING_EURORA_FULLY_LOCAL")
        .map(|v| v.eq_ignore_ascii_case("true"))
        .unwrap_or(false);

    let vision_model = build_env_vision_model();

    if local_mode {
        let model = std::env::var("OLLAMA_MODEL").unwrap_or_else(|_| "llama3.2".to_string());
        let host = std::env::var("OLLAMA_HOST")
            .unwrap_or_else(|_| "http://host.docker.internal:11434".to_string());
        let chat: Arc<dyn BaseChatModel + Send + Sync> =
            Arc::new(ChatOllama::builder().model(&model).base_url(&host).build());
        let title: Arc<dyn BaseChatModel + Send + Sync> =
            Arc::new(ChatOllama::builder().model(&model).base_url(&host).build());
        Some(Providers {
            chat,
            title,
            tools: build_tool_map(vision_model),
        })
    } else {
        let tools = build_tool_map(vision_model);
        let chat_model = ChatOpenAI::builder()
            .model(std::env::var("NEBUL_MODEL").expect("Nebul model should be set"))
            .reasoning_effort("medium")
            .api_base(BASE_NEBUL_URL)
            .api_key(std::env::var("NEBUL_API_KEY").expect("Nebul API key should be set"))
            .use_responses_api(false)
            .build();
        let tool_likes: Vec<ToolLike> = tools.values().cloned().map(ToolLike::Tool).collect();
        let bound = chat_model
            .bind_tools(&tool_likes, None)
            .expect("Failed to bind tools");
        let chat: Arc<dyn BaseChatModel + Send + Sync> =
            Arc::from(bound as Box<dyn BaseChatModel + Send + Sync>);

        let title = Arc::new(
            ChatOpenAI::builder()
                .model(std::env::var("NEBUL_TITLE_MODEL").expect("Nebul title model should be set"))
                .api_base(BASE_NEBUL_URL)
                .api_key(std::env::var("NEBUL_API_KEY").expect("Nebul API key should be set"))
                .build(),
        );

        Some(Providers { chat, title, tools })
    }
}

pub struct ThreadService {
    db: Arc<DatabaseManager>,
    asset_service: Arc<AssetService>,
    providers: Arc<RwLock<Option<Providers>>>,
}

impl ThreadService {
    pub fn new(
        db: Arc<DatabaseManager>,
        asset_service: Arc<AssetService>,
        mut settings_rx: SettingsReceiver,
    ) -> Self {
        let env_fallback = build_env_fallback();
        tracing::info!(
            "Creating new ThreadService instance (env fallback: {})",
            env_fallback.is_some()
        );

        let initial = settings_rx.borrow_and_update().clone();
        let initial_providers = initial
            .map(|s| Providers {
                chat: build_chat_provider_from(&s).into(),
                title: build_title_provider_from(&s).into(),
                tools: build_tool_map(None),
            })
            .or(env_fallback);

        let providers = Arc::new(RwLock::new(initial_providers));

        let providers_handle = providers.clone();
        tokio::spawn(async move {
            loop {
                if settings_rx.changed().await.is_err() {
                    tracing::info!("Settings channel closed, stopping provider watcher");
                    break;
                }
                let new_settings = settings_rx.borrow_and_update().clone();
                let new_providers = new_settings.map(|s| {
                    tracing::info!("Provider settings changed, rebuilding providers");
                    Providers {
                        chat: build_chat_provider_from(&s).into(),
                        title: build_title_provider_from(&s).into(),
                        tools: build_tool_map(None),
                    }
                });
                let mut lock = providers_handle.write().unwrap_or_else(|e| e.into_inner());
                *lock = new_providers;
            }
        });

        Self { db, asset_service, providers }
    }

    fn get_chat_provider(&self) -> Result<Arc<dyn BaseChatModel + Send + Sync>, Status> {
        let lock = self.providers.read().unwrap_or_else(|e| e.into_inner());
        lock.as_ref().map(|p| p.chat.clone()).ok_or_else(|| {
            Status::failed_precondition(
                "No provider settings configured and no environment fallback available",
            )
        })
    }

    fn get_title_provider(&self) -> Result<Arc<dyn BaseChatModel + Send + Sync>, Status> {
        let lock = self.providers.read().unwrap_or_else(|e| e.into_inner());
        lock.as_ref().map(|p| p.title.clone()).ok_or_else(|| {
            Status::failed_precondition(
                "No provider settings configured and no environment fallback available",
            )
        })
    }

    fn get_tools(&self) -> HashMap<String, Arc<dyn BaseTool>> {
        let lock = self.providers.read().unwrap_or_else(|e| e.into_inner());
        lock.as_ref().map(|p| p.tools.clone()).unwrap_or_default()
    }

    fn db_thread_to_proto(thread: be_remote_db::Thread) -> Thread {
        Thread {
            id: thread.id.to_string(),
            user_id: thread.user_id.to_string(),
            title: thread.title.clone().unwrap_or_default(),
            created_at: Some(datetime_to_timestamp(thread.created_at)),
            updated_at: Some(datetime_to_timestamp(thread.updated_at)),
            active_leaf_id: thread.active_leaf_id.map(|id| id.to_string()),
        }
    }
}

fn datetime_to_timestamp(dt: DateTime<Utc>) -> Timestamp {
    Timestamp {
        seconds: dt.timestamp(),
        nanos: dt.timestamp_subsec_nanos() as i32,
    }
}

type ChatResult<T> = Result<Response<T>, Status>;
type ChatStreamResult = Pin<Box<dyn Stream<Item = Result<ProtoAiMessageChunk, Status>> + Send>>;

#[tonic::async_trait]
impl ProtoThreadService for ThreadService {
    type ChatStreamStream = ChatStreamResult;

    async fn create_thread(
        &self,
        request: Request<CreateThreadRequest>,
    ) -> Result<Response<CreateThreadResponse>, Status> {
        tracing::info!("CreateThread request received");

        let claims = extract_claims(&request)?;
        let user_id = parse_user_id(claims)?;

        let req = request.into_inner();

        let title = if req.title.is_empty() {
            "New Chat".to_string()
        } else {
            req.title
        };

        let thread = self
            .db
            .create_thread()
            .user_id(user_id)
            .title(title)
            .call()
            .await
            .map_err(ThreadServiceError::from)?;

        tracing::info!("Created thread {}", thread.id);

        Ok(Response::new(CreateThreadResponse {
            thread: Some(Self::db_thread_to_proto(thread)),
        }))
    }

    async fn list_threads(
        &self,
        request: Request<ListThreadsRequest>,
    ) -> Result<Response<ListThreadsResponse>, Status> {
        tracing::info!("ListThreads request received");

        let claims = extract_claims(&request)?;
        let user_id = parse_user_id(claims)?;

        let req = request.into_inner();

        let threads = self
            .db
            .list_threads()
            .user_id(user_id)
            .params(PaginationParams::new(req.offset, req.limit, "DESC"))
            .call()
            .await
            .map_err(ThreadServiceError::from)?;

        tracing::info!("Listed {} threads", threads.len());

        Ok(Response::new(ListThreadsResponse {
            threads: threads.into_iter().map(Self::db_thread_to_proto).collect(),
        }))
    }

    async fn add_human_message(
        &self,
        request: Request<AddHumanMessageRequest>,
    ) -> Result<Response<AddHumanMessageResponse>, Status> {
        tracing::info!("AddHumanMessage request received");

        let claims = extract_claims(&request)?;
        let user_id = parse_user_id(claims)?;
        let req = request.into_inner();

        let thread_id =
            Uuid::parse_str(&req.thread_id).map_err(|e| ThreadServiceError::InvalidUuid {
                field: "thread_id",
                source: e,
            })?;

        let proto_message = req
            .message
            .ok_or_else(|| Status::invalid_argument("message field is required"))?;

        let human_message: HumanMessage = proto_message.into();
        
        // Extract text and images from message content
        let extracted = extract_message_content(&human_message);
        
        // Create assets for any images and get references
        let image_refs = if !extracted.images.is_empty() {
            create_image_assets(&extracted.images, &self.asset_service, user_id)
                .await
                .map_err(|e| Status::internal(format!("Failed to create image assets: {}", e)))?
        } else {
            Vec::new()
        };
        
        // Build additional_kwargs with image asset refs
        let additional_kwargs = if !image_refs.is_empty() {
            serde_json::to_value(serde_json::json!({
                "image_assets": image_refs
            }))
            .map_err(|e| Status::internal(format!("Failed to serialize additional_kwargs: {}", e)))?
        } else {
            serde_json::json!({})
        };

        let message = self
            .db
            .create_message()
            .thread_id(thread_id)
            .user_id(user_id)
            .message_type(MessageType::Human)
            .content(extracted.text)
            .additional_kwargs(additional_kwargs)
            .call()
            .await
            .map_err(ThreadServiceError::from)?;

        tracing::info!("Added human message to thread {}", thread_id);

        Ok(Response::new(AddHumanMessageResponse {
            message: Some(message.into()),
        }))
    }

    async fn add_hidden_human_message(
        &self,
        request: Request<AddHiddenHumanMessageRequest>,
    ) -> Result<Response<AddHiddenHumanMessageResponse>, Status> {
        tracing::info!("AddHiddenHumanMessage request received");

        let claims = extract_claims(&request)?;
        let user_id = parse_user_id(claims)?;
        let req = request.into_inner();

        let thread_id =
            Uuid::parse_str(&req.thread_id).map_err(|e| ThreadServiceError::InvalidUuid {
                field: "thread_id",
                source: e,
            })?;

        let proto_message = req
            .message
            .ok_or_else(|| Status::invalid_argument("message field is required"))?;

        let human_message: HumanMessage = proto_message.into();
        
        // Extract text and images from message content
        let extracted = extract_message_content(&human_message);
        
        // Create assets for any images and get references
        let image_refs = if !extracted.images.is_empty() {
            create_image_assets(&extracted.images, &self.asset_service, user_id)
                .await
                .map_err(|e| Status::internal(format!("Failed to create image assets: {}", e)))?
        } else {
            Vec::new()
        };
        
        // Build additional_kwargs with image asset refs
        let additional_kwargs = if !image_refs.is_empty() {
            serde_json::to_value(serde_json::json!({
                "image_assets": image_refs
            }))
            .map_err(|e| Status::internal(format!("Failed to serialize additional_kwargs: {}", e)))?
        } else {
            serde_json::json!({})
        };

        let message = self
            .db
            .create_message()
            .thread_id(thread_id)
            .user_id(user_id)
            .message_type(MessageType::Human)
            .content(extracted.text)
            .additional_kwargs(additional_kwargs)
            .hidden_from_ui(true)
            .call()
            .await
            .map_err(ThreadServiceError::from)?;

        tracing::info!("Added hidden human message to thread {}", thread_id);

        Ok(Response::new(AddHiddenHumanMessageResponse {
            message: Some(message.into()),
        }))
    }

    async fn add_system_message(
        &self,
        request: Request<AddSystemMessageRequest>,
    ) -> Result<Response<AddSystemMessageResponse>, Status> {
        tracing::info!("AddSystemMessage request received");

        let claims = extract_claims(&request)?;
        let user_id = parse_user_id(claims)?;
        let req = request.into_inner();

        let thread_id =
            Uuid::parse_str(&req.thread_id).map_err(|e| ThreadServiceError::InvalidUuid {
                field: "thread_id",
                source: e,
            })?;

        let proto_message = req
            .message
            .ok_or_else(|| Status::invalid_argument("message field is required"))?;

        let system_message: SystemMessage = proto_message.into();
        let content = system_message.content.as_text();

        let message = self
            .db
            .create_message()
            .thread_id(thread_id)
            .user_id(user_id)
            .message_type(MessageType::System)
            .content(content)
            .call()
            .await
            .map_err(ThreadServiceError::from)?;

        tracing::info!("Added system message to thread {}", thread_id);

        Ok(Response::new(AddSystemMessageResponse {
            message: Some(message.into()),
        }))
    }

    async fn chat_stream(
        &self,
        request: Request<ChatStreamRequest>,
    ) -> ChatResult<Self::ChatStreamStream> {
        tracing::info!("ChatStream request received");

        let claims = extract_claims(&request)?;
        let user_id = parse_user_id(claims)?;
        let req = request.into_inner();

        let thread_id =
            Uuid::parse_str(&req.thread_id).map_err(|e| ThreadServiceError::InvalidUuid {
                field: "thread_id",
                source: e,
            })?;

        let is_edit = req.parent_message_id.is_some();
        let parent_id = req
            .parent_message_id
            .filter(|s| !s.is_empty())
            .map(|s| Uuid::parse_str(&s))
            .transpose()
            .map_err(|e| ThreadServiceError::InvalidUuid {
                field: "parent_message_id",
                source: e,
            })?;

        if is_edit {
            let effective_parent = if parent_id.is_some() {
                parent_id
            } else {
                let first_visible = self
                    .db
                    .list_messages()
                    .thread_id(thread_id)
                    .user_id(user_id)
                    .include_hidden(false)
                    .params(PaginationParams::new(0, 1, "ASC"))
                    .call()
                    .await
                    .map_err(ThreadServiceError::from)?;
                first_visible.first().and_then(|msg| msg.parent_message_id)
            };
            self.db
                .set_active_leaf()
                .id(thread_id)
                .user_id(user_id)
                .maybe_active_leaf_id(effective_parent)
                .call()
                .await
                .map_err(ThreadServiceError::from)?;
        }

        tracing::debug!("ChatStream: thread_id = {}", thread_id);

        let mut hidden_messages = self
            .db
            .list_messages()
            .thread_id(thread_id)
            .user_id(user_id)
            .include_visible(false)
            .params(PaginationParams::new(0, 2, "DESC"))
            .call()
            .await
            .map_err(ThreadServiceError::from)?;

        hidden_messages.reverse();

        let mut visible_messages = self
            .db
            .list_messages()
            .thread_id(thread_id)
            .user_id(user_id)
            .include_hidden(false)
            .params(PaginationParams::new(0, 3, "DESC"))
            .call()
            .await
            .map_err(ThreadServiceError::from)?;
        visible_messages.reverse();

        hidden_messages.extend(visible_messages);

        // Use async converter with asset cache for image support
        let mut asset_cache: HashMap<Uuid, Asset> = HashMap::new();
        let mut messages: Vec<AnyMessage> = Vec::with_capacity(hidden_messages.len());
        for msg in hidden_messages {
            match convert_db_message_to_base_message_async(msg, &self.db, &mut asset_cache).await {
                Ok(m) => messages.push(m),
                Err(e) => tracing::warn!("Skipping unconvertible message: {e}"),
            }
        }

        let mut human_additional_kwargs = HashMap::new();
        if let Some(ref chips_json) = req.asset_chips_json
            && let Ok(chips_value) = serde_json::from_str::<serde_json::Value>(chips_json)
        {
            human_additional_kwargs.insert("asset_chips".to_string(), chips_value);
        }

        // Parse and validate image asset IDs
        let image_asset_refs: Vec<crate::converters::ImageAssetRef> = 
            if let Some(ref image_ids_json) = req.image_asset_ids_json {
                match serde_json::from_str::<Vec<crate::converters::ImageAssetRef>>(image_ids_json) {
                    Ok(refs) => {
                        // Validate each asset exists and belongs to user
                        let mut valid_refs = Vec::new();
                        for r in refs {
                            match self.db.get_asset().id(r.asset_id).call().await {
                                Ok(asset) => {
                                    if asset.user_id == user_id {
                                        valid_refs.push(r);
                                    } else {
                                        tracing::warn!(
                                            "Image asset {} does not belong to user {}",
                                            r.asset_id, user_id
                                        );
                                    }
                                }
                                Err(e) => {
                                    tracing::warn!("Image asset {} not found: {}", r.asset_id, e);
                                }
                            }
                        }
                        valid_refs
                    }
                    Err(e) => {
                        tracing::warn!("Failed to parse image_asset_ids_json: {}", e);
                        Vec::new()
                    }
                }
            } else {
                Vec::new()
            };

        // Store image asset refs in additional_kwargs for persistence
        if !image_asset_refs.is_empty() {
            human_additional_kwargs.insert(
                "image_assets".to_string(),
                serde_json::to_value(&image_asset_refs).unwrap_or(serde_json::json!([])),
            );
        }

        // Build human message with image content blocks for current request
        let human_message = if image_asset_refs.is_empty() {
            HumanMessage::builder()
                .content(req.content.clone())
                .additional_kwargs(human_additional_kwargs)
                .build()
        } else {
            use agent_chain::messages::content::{ContentBlock, ContentBlocks, ImageContentBlock, TextContentBlock};
            
            let mut content_blocks: Vec<ContentBlock> = vec![
                ContentBlock::Text(TextContentBlock::new(&req.content))
            ];
            
            for r in &image_asset_refs {
                if let Ok(asset) = self.db.get_asset().id(r.asset_id).call().await {
                    content_blocks.push(ContentBlock::Image(ImageContentBlock {
                        url: Some(asset.storage_uri.clone()),
                        mime_type: Some(r.mime_type.clone()),
                        ..Default::default()
                    }));
                }
            }
            
            HumanMessage::builder()
                .content(ContentBlocks::from(content_blocks))
                .additional_kwargs(human_additional_kwargs)
                .build()
        };

        messages.push(human_message.clone().into());

        let content = human_message.content.as_text();

        let additional_kwargs =
            serde_json::to_value(&human_message.additional_kwargs).map_err(|e| {
                ThreadServiceError::Internal(format!(
                    "Failed to serialize additional_kwargs: {}",
                    e
                ))
            })?;

        let _message = self
            .db
            .create_message()
            .thread_id(thread_id)
            .user_id(user_id)
            .message_type(MessageType::Human)
            .content(content)
            .additional_kwargs(additional_kwargs)
            .call()
            .await
            .map_err(ThreadServiceError::from)?;

        let chat_provider = self.get_chat_provider()?;
        let tools = self.get_tools();

        let db = self.db.clone();
        let output_stream = async_stream::try_stream! {
            let mut full_content = String::new();
            let mut full_reasoning = String::new();
            let mut total_input_tokens: i64 = 0;
            let mut total_output_tokens: i64 = 0;
            let mut total_reasoning_tokens: i64 = 0;
            let mut total_cache_creation_tokens: i64 = 0;
            let mut total_cache_read_tokens: i64 = 0;
            const MAX_TOOL_ROUNDS: usize = 5;

            for round in 0..=MAX_TOOL_ROUNDS {
                let provider_stream = chat_provider
                    .stream(messages.clone(), None, None)
                    .await
                    .map_err(|e| {
                        tracing::error!("Error in chat_stream: {}", e);
                        Status::internal(e.to_string())
                    })?;

                tokio::pin!(provider_stream);
                let mut round_content = String::new();
                let mut tool_calls: Vec<ToolCall> = Vec::new();

                while let Some(result) = provider_stream.next().await {
                    match result {
                        Ok(chunk) => {
                            let content = chunk.content.to_string();
                            if !content.is_empty() {
                                round_content.push_str(&content);
                            }
                            if let Some(reasoning) = chunk.additional_kwargs.get("reasoning_content").and_then(|v| v.as_str()) {
                                full_reasoning.push_str(reasoning);
                            }
                            if !chunk.tool_calls.is_empty() {
                                tool_calls.extend(chunk.tool_calls.clone());
                            }
                            if let Some(ref usage) = chunk.usage_metadata {
                                total_input_tokens += usage.input_tokens;
                                total_output_tokens += usage.output_tokens;
                                if let Some(ref details) = usage.output_token_details {
                                    total_reasoning_tokens += details.reasoning.unwrap_or(0);
                                }
                                if let Some(ref details) = usage.input_token_details {
                                    total_cache_creation_tokens += details.cache_creation.unwrap_or(0);
                                    total_cache_read_tokens += details.cache_read.unwrap_or(0);
                                }
                            }
                            yield chunk.into();
                        }
                        Err(e) => {
                            Err(Status::internal(e.to_string()))?;
                        }
                    }
                }

                full_content.push_str(&round_content);

                if tool_calls.is_empty() || round == MAX_TOOL_ROUNDS {
                    break;
                }

                messages.push(
                    AIMessage::builder()
                        .content(&round_content)
                        .tool_calls(tool_calls.clone())
                        .build()
                        .into(),
                );

                for tc in tool_calls {
                    let tool_name = tc.name.clone();
                    let result_msg = if let Some(tool) = tools.get(&tool_name) {
                        tool.invoke_tool_call(tc).await
                    } else {
                        tracing::error!("Unknown tool: {}", tool_name);
                        agent_chain::messages::ToolMessage::builder()
                            .content(format!("Error: unknown tool '{}'", tool_name))
                            .tool_call_id("")
                            .status(agent_chain::messages::ToolStatus::Error)
                            .build()
                            .into()
                    };
                    messages.push(result_msg);
                }
            }

            if !full_content.is_empty() {
                let reasoning_blocks = match full_reasoning.is_empty() {
                    true => None,
                    false => Some(serde_json::json!([{
                        "type": "thinking",
                        "content": full_reasoning,
                    }])),
                };

                let save_result = db.create_message()
                    .thread_id(thread_id)
                    .user_id(user_id)
                    .message_type(MessageType::Ai)
                    .content(full_content.clone())
                    .maybe_reasoning_blocks(reasoning_blocks)
                    .call()
                    .await;

                match save_result
                {
                    Ok(ai_message) => {
                        if (total_input_tokens > 0 || total_output_tokens > 0) && let Err(e) = {
                                db
                                .record_token_usage()
                                .user_id(user_id)
                                .thread_id(thread_id)
                                .message_id(ai_message.id)
                                .input_tokens(total_input_tokens)
                                .output_tokens(total_output_tokens)
                                .reasoning_tokens(total_reasoning_tokens)
                                .cache_creation_tokens(total_cache_creation_tokens)
                                .cache_read_tokens(total_cache_read_tokens)
                                .call()
                                .await
                            }
                            {
                                tracing::error!("Failed to record token usage: {}", e);
                            }
                    }
                    Err(e) => {
                        tracing::error!("Failed to save AI message to database: {}", e);
                    }
                }
            }
        };

        Ok(Response::new(
            Box::pin(output_stream) as Self::ChatStreamStream
        ))
    }

    async fn get_messages(
        &self,
        request: Request<GetMessagesRequest>,
    ) -> Result<Response<GetMessagesResponse>, Status> {
        tracing::info!("Get messages request received");
        let claims = extract_claims(&request)?;
        let user_id = parse_user_id(claims)?;
        let req = request.into_inner();

        let thread_id =
            Uuid::parse_str(&req.thread_id).map_err(|e| ThreadServiceError::InvalidUuid {
                field: "thread_id",
                source: e,
            })?;

        let messages = self
            .db
            .list_messages()
            .thread_id(thread_id)
            .user_id(user_id)
            .params(PaginationParams::new(req.offset, req.limit, "ASC"))
            .include_hidden(false)
            .call()
            .await
            .map_err(ThreadServiceError::from)?;

        let message_ids: Vec<Uuid> = messages.iter().map(|m| m.id).collect();
        let sibling_rows = self
            .db
            .get_sibling_info()
            .thread_id(thread_id)
            .user_id(user_id)
            .message_ids(&message_ids)
            .call()
            .await
            .map_err(ThreadServiceError::from)?;

        let sibling_info = sibling_rows
            .into_iter()
            .map(|s| MessageSiblingInfo {
                message_id: s.message_id.to_string(),
                sibling_count: u32::try_from(s.sibling_count).unwrap_or(0),
                sibling_index: u32::try_from(s.sibling_index).unwrap_or(0),
            })
            .collect();

        Ok(Response::new(GetMessagesResponse {
            messages: messages.into_iter().map(|m| m.into()).collect(),
            sibling_info,
        }))
    }

    async fn get_thread(
        &self,
        request: tonic::Request<proto_gen::thread::GetThreadRequest>,
    ) -> Result<Response<GetThreadResponse>, Status> {
        tracing::info!("Get thread request received");
        let claims = extract_claims(&request)?;
        let user_id = parse_user_id(claims)?;
        let req = request.into_inner();

        let thread_id =
            Uuid::parse_str(&req.thread_id).map_err(|e| ThreadServiceError::InvalidUuid {
                field: "thread_id",
                source: e,
            })?;

        let thread = self
            .db
            .get_thread()
            .id(thread_id)
            .user_id(user_id)
            .call()
            .await
            .map_err(ThreadServiceError::from)?;

        Ok(Response::new(GetThreadResponse {
            thread: thread.try_into().ok(),
        }))
    }

    async fn delete_thread(
        &self,
        request: Request<DeleteThreadRequest>,
    ) -> Result<Response<DeleteThreadResponse>, Status> {
        let claims = extract_claims(&request)?;
        let user_id = parse_user_id(claims)?;
        let req = request.into_inner();

        let thread_id =
            Uuid::parse_str(&req.thread_id).map_err(|e| ThreadServiceError::InvalidUuid {
                field: "thread_id",
                source: e,
            })?;

        self.db
            .delete_thread()
            .id(thread_id)
            .user_id(user_id)
            .call()
            .await
            .map_err(ThreadServiceError::from)?;

        tracing::info!("Deleted thread {}", thread_id);

        Ok(Response::new(DeleteThreadResponse {}))
    }

    async fn generate_thread_title(
        &self,
        request: tonic::Request<GenerateThreadTitleRequest>,
    ) -> Result<Response<GenerateThreadTitleResponse>, Status> {
        tracing::info!("Generate thread title request received");
        let claims = extract_claims(&request)?;
        let user_id = parse_user_id(claims)?;
        let req = request.into_inner();

        let thread_id =
            Uuid::parse_str(&req.thread_id).map_err(|e| ThreadServiceError::InvalidUuid {
                field: "thread_id",
                source: e,
            })?;

        let hidden_messages = self
            .db
            .list_messages()
            .thread_id(thread_id)
            .user_id(user_id)
            .include_hidden(true)
            .include_visible(false)
            .params(PaginationParams::new(0, 2, "DESC"))
            .call()
            .await
            .map_err(ThreadServiceError::from)?;

        let mut messages: Vec<AnyMessage> = vec![
            SystemMessage::builder()
                .content(
                    "Generate a title for the following conversation. Your task is:
                - Return a concise title, max 6 words.
                - No quotation marks.
                - Use sentence case.
                - Summarize the main topic, not the tone.
                - If the topic is unclear, use a generic title.
                - Do NOT answer or respond to the messages. Only output a title.
                Output only the title text."
                        .to_string(),
                )
                .build()
                .into(),
        ];

        messages.extend(hidden_messages.into_iter().filter_map(|msg| {
            convert_db_message_to_base_message(msg)
                .map_err(|e| tracing::warn!("Skipping unconvertible message: {e}"))
                .ok()
        }));

        messages.push(HumanMessage::builder().content(req.content).build().into());

        let title_provider = self.get_title_provider()?;
        let mut title = match title_provider.invoke(messages, None).await {
            Ok(message) => message.content.to_string(),
            Err(_) => "New Chat".to_string(),
        };
        let title_words: Vec<&str> = title.split_whitespace().collect();
        title = title_words[..title_words.len().min(6)].join(" ");
        title = match title.is_empty() {
            true => {
                tracing::warn!("Failed to generate title");
                "New Chat".to_string()
            }
            false => title,
        };

        if let Some(first) = title.chars().next() {
            let rest = &title[first.len_utf8()..];
            title = first.to_uppercase().collect::<String>() + rest;
        }

        let thread = self
            .db
            .update_thread()
            .id(thread_id)
            .user_id(user_id)
            .title(title.clone())
            .call()
            .await
            .map_err(ThreadServiceError::from)?;

        Ok(Response::new(GenerateThreadTitleResponse {
            thread: Some(Self::db_thread_to_proto(thread)),
        }))
    }

    async fn switch_branch(
        &self,
        request: Request<SwitchBranchRequest>,
    ) -> Result<Response<GetMessagesResponse>, Status> {
        let claims = extract_claims(&request)?;
        let user_id = parse_user_id(claims)?;
        let req = request.into_inner();

        let thread_id =
            Uuid::parse_str(&req.thread_id).map_err(|e| ThreadServiceError::InvalidUuid {
                field: "thread_id",
                source: e,
            })?;

        let message_id =
            Uuid::parse_str(&req.message_id).map_err(|e| ThreadServiceError::InvalidUuid {
                field: "message_id",
                source: e,
            })?;

        let target_id = if req.direction == 0 {
            message_id
        } else if req.direction == -1 || req.direction == 1 {
            self.db
                .get_adjacent_sibling()
                .thread_id(thread_id)
                .user_id(user_id)
                .message_id(message_id)
                .direction(req.direction)
                .call()
                .await
                .map_err(ThreadServiceError::from)?
                .ok_or_else(|| Status::not_found("No adjacent sibling found"))?
        } else {
            return Err(Status::invalid_argument("direction must be -1, 0, or 1"));
        };

        let new_leaf = self
            .db
            .find_deepest_leaf(thread_id, user_id, target_id)
            .await
            .map_err(ThreadServiceError::from)?;

        self.db
            .set_active_leaf()
            .id(thread_id)
            .user_id(user_id)
            .active_leaf_id(new_leaf)
            .call()
            .await
            .map_err(ThreadServiceError::from)?;

        let messages = self
            .db
            .list_messages()
            .thread_id(thread_id)
            .user_id(user_id)
            .params(PaginationParams::new(0, 100, "ASC"))
            .include_hidden(false)
            .call()
            .await
            .map_err(ThreadServiceError::from)?;

        let message_ids: Vec<Uuid> = messages.iter().map(|m| m.id).collect();
        let sibling_rows = self
            .db
            .get_sibling_info()
            .thread_id(thread_id)
            .user_id(user_id)
            .message_ids(&message_ids)
            .call()
            .await
            .map_err(ThreadServiceError::from)?;

        let sibling_info = sibling_rows
            .into_iter()
            .map(|s| MessageSiblingInfo {
                message_id: s.message_id.to_string(),
                sibling_count: u32::try_from(s.sibling_count).unwrap_or(0),
                sibling_index: u32::try_from(s.sibling_index).unwrap_or(0),
            })
            .collect();

        Ok(Response::new(GetMessagesResponse {
            messages: messages.into_iter().map(|m| m.into()).collect(),
            sibling_info,
        }))
    }

    async fn get_message_tree(
        &self,
        request: Request<GetMessageTreeRequest>,
    ) -> Result<Response<GetMessageTreeResponse>, Status> {
        let claims = extract_claims(&request)?;
        let user_id = parse_user_id(claims)?;
        let req = request.into_inner();

        let thread_id =
            Uuid::parse_str(&req.thread_id).map_err(|e| ThreadServiceError::InvalidUuid {
                field: "thread_id",
                source: e,
            })?;

        const MAX_TREE_DEPTH: u32 = 100;
        let start_level = req.start_level.min(MAX_TREE_DEPTH);
        let end_level = req.end_level.min(MAX_TREE_DEPTH);

        let result = if req.parent_node_ids.is_empty() {
            self.db
                .list_messages_by_level(thread_id, user_id, start_level as i32, end_level as i32)
                .await
                .map_err(ThreadServiceError::from)?
        } else {
            let parent_ids: Vec<Uuid> = req
                .parent_node_ids
                .iter()
                .map(|id| Uuid::parse_str(id))
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| ThreadServiceError::InvalidUuid {
                    field: "parent_node_ids",
                    source: e,
                })?;
            let depth = end_level.saturating_sub(start_level) + 1;
            self.db
                .list_messages_by_level_from_parents(
                    thread_id,
                    user_id,
                    &parent_ids,
                    start_level as i32,
                    depth as i32,
                )
                .await
                .map_err(ThreadServiceError::from)?
        };

        let has_more = result.has_more;
        let tree_nodes = result
            .nodes
            .into_iter()
            .map(|n| {
                let additional_kwargs = if n.additional_kwargs.is_null()
                    || n.additional_kwargs
                        .as_object()
                        .is_some_and(|o| o.is_empty())
                {
                    None
                } else {
                    Some(serde_json::to_string(&n.additional_kwargs).unwrap_or_default())
                };

                let reasoning_blocks = n
                    .reasoning_blocks
                    .filter(|v| !v.is_null())
                    .map(|v| serde_json::to_string(&v).unwrap_or_default());

                MessageTreeNode {
                    id: n.id.to_string(),
                    parent_message_id: n.parent_message_id.map(|id| id.to_string()),
                    message_type: n.message_type.to_string(),
                    content: n.content,
                    level: n.level as u32,
                    sibling_count: u32::try_from(n.sibling_count).unwrap_or(0),
                    sibling_index: u32::try_from(n.sibling_index).unwrap_or(0),
                    additional_kwargs,
                    reasoning_blocks,
                }
            })
            .collect();

        Ok(Response::new(GetMessageTreeResponse {
            nodes: tree_nodes,
            has_more,
        }))
    }

    async fn search_threads(
        &self,
        request: Request<SearchThreadsRequest>,
    ) -> Result<Response<SearchThreadsResponse>, Status> {
        let claims = extract_claims(&request)?;
        let user_id = parse_user_id(claims)?;
        let req = request.into_inner();

        if req.query.trim().len() < 2 {
            return Ok(Response::new(SearchThreadsResponse { results: vec![] }));
        }

        let results = self
            .db
            .search_threads(user_id, &req.query, req.limit as i64, req.offset as i64)
            .await
            .map_err(ThreadServiceError::from)?;

        let results = results
            .into_iter()
            .map(|r| SearchThreadResult {
                id: r.id.to_string(),
                title: r.title.unwrap_or_default(),
                rank: r.rank,
                updated_at: Some(Timestamp {
                    seconds: r.updated_at.timestamp(),
                    nanos: r.updated_at.timestamp_subsec_nanos() as i32,
                }),
            })
            .collect();

        Ok(Response::new(SearchThreadsResponse { results }))
    }

    async fn search_messages(
        &self,
        request: Request<SearchMessagesRequest>,
    ) -> Result<Response<SearchMessagesResponse>, Status> {
        let claims = extract_claims(&request)?;
        let user_id = parse_user_id(claims)?;
        let req = request.into_inner();

        if req.query.trim().len() < 2 {
            return Ok(Response::new(SearchMessagesResponse { results: vec![] }));
        }

        let results = self
            .db
            .search_messages(user_id, &req.query, req.limit as i64, req.offset as i64)
            .await
            .map_err(ThreadServiceError::from)?;

        let results = results
            .into_iter()
            .map(|r| SearchMessageResult {
                id: r.id.to_string(),
                thread_id: r.thread_id.to_string(),
                message_type: r.message_type.to_string(),
                content: String::new(),
                rank: r.rank,
                created_at: Some(Timestamp {
                    seconds: r.created_at.timestamp(),
                    nanos: r.created_at.timestamp_subsec_nanos() as i32,
                }),
                snippet: r.snippet,
            })
            .collect();

        Ok(Response::new(SearchMessagesResponse { results }))
    }
}
