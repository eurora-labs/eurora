use crate::describe_image_tool::{self, DescribeImageTool};
use crate::message_projection::{collect_thread_images, project_for_text_llm};
use agent_chain::SystemMessage;
use agent_chain::messages::{ContentBlock, ImageContentBlock, PlainTextContentBlock};
use agent_chain::{
    AnyMessage, BaseChatModel, BaseTool, HumanMessage, language_models::ToolLike,
    ollama::ChatOllama, openai::ChatOpenAI,
};
use agent_chain_core::proto::{
    BaseMessageWithSibling, ChatStreamResponse, ProtoContentBlock, chat_stream_response::Payload,
};
use be_asset::AssetService;
use be_authz::{extract_claims, parse_user_id};
use be_remote_db::{DatabaseManager, MessageType, PaginationParams};
use chrono::{DateTime, Utc};
use prost_types::Timestamp;
pub use proto_gen::thread::proto_thread_service_server::{
    ProtoThreadService, ProtoThreadServiceServer,
};
use proto_gen::thread::{
    ChatStreamRequest, CreateThreadRequest, CreateThreadResponse, DeleteThreadRequest,
    DeleteThreadResponse, GenerateThreadTitleRequest, GenerateThreadTitleResponse,
    GetMessagesRequest, GetMessagesResponse, GetThreadResponse, ListThreadsRequest,
    ListThreadsResponse, ProtoThread, SavePreliminaryContentBlocksRequest,
    SavePreliminaryContentBlocksResponse, SearchMessageResult, SearchMessagesRequest,
    SearchMessagesResponse, SearchThreadResult, SearchThreadsRequest, SearchThreadsResponse,
    SwitchBranchRequest,
};
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use tokio::sync::mpsc;
use tokio_stream::Stream;
use tokio_util::sync::CancellationToken;
use tonic::{Request, Response, Status};
use uuid::Uuid;

use crate::agent_loop::run_agent_loop;
use crate::converters::convert_db_message_to_base_message;
use crate::error::ThreadServiceError;
use crate::tools::firecrawl_tools;

const BASE_NEBUL_URL: &str = "https://api.inference.nebul.io/v1";
const CONTEXT_MESSAGE_LIMIT: u32 = 5;
const MAX_CONTENT_BLOCKS: usize = 50;
const MAX_TOOL_ROUNDS: usize = 15;

struct Providers {
    chat: Arc<dyn BaseChatModel + Send + Sync>,
    title: Arc<dyn BaseChatModel + Send + Sync>,
    vision: Option<VisionConfig>,
}

struct VisionConfig {
    model: Arc<dyn BaseChatModel + Send + Sync>,
    default_tools: Vec<Arc<dyn BaseTool>>,
}

struct LlmContext {
    messages: Vec<AnyMessage>,
    chat_model: Arc<dyn BaseChatModel + Send + Sync>,
    tools: HashMap<String, Arc<dyn BaseTool>>,
}

fn build_providers() -> Providers {
    let local_mode = std::env::var("RUNNING_EURORA_FULLY_LOCAL")
        .map(|v| v.eq_ignore_ascii_case("true"))
        .unwrap_or(false);

    if local_mode {
        let model = std::env::var("OLLAMA_MODEL").unwrap_or_else(|_| "llama3.2".to_string());
        let host = std::env::var("OLLAMA_HOST")
            .unwrap_or_else(|_| "http://host.docker.internal:11434".to_string());
        let chat: Arc<dyn BaseChatModel + Send + Sync> =
            Arc::new(ChatOllama::builder().model(&model).base_url(&host).build());
        let title: Arc<dyn BaseChatModel + Send + Sync> =
            Arc::new(ChatOllama::builder().model(&model).base_url(&host).build());
        Providers {
            chat,
            title,
            vision: None,
        }
    } else {
        let api_key =
            std::env::var("NEBUL_API_KEY").expect("NEBUL_API_KEY environment variable must be set");

        let chat: Arc<dyn BaseChatModel + Send + Sync> = Arc::new(
            ChatOpenAI::builder()
                .model(std::env::var("NEBUL_MODEL").expect("NEBUL_MODEL must be set"))
                .reasoning_effort("medium")
                .api_base(BASE_NEBUL_URL)
                .api_key(&api_key)
                .use_responses_api(false)
                .build(),
        );

        let title: Arc<dyn BaseChatModel + Send + Sync> = Arc::new(
            ChatOpenAI::builder()
                .model(std::env::var("NEBUL_TITLE_MODEL").expect("NEBUL_TITLE_MODEL must be set"))
                .api_base(BASE_NEBUL_URL)
                .api_key(&api_key)
                .build(),
        );

        let vision_model: Arc<dyn BaseChatModel + Send + Sync> = Arc::new(
            ChatOpenAI::builder()
                .model(std::env::var("NEBUL_VISION_MODEL").expect("NEBUL_VISION_MODEL must be set"))
                .api_base(BASE_NEBUL_URL)
                .api_key(&api_key)
                .use_responses_api(false)
                .build(),
        );

        Providers {
            chat,
            title,
            vision: Some(VisionConfig {
                model: vision_model,
                default_tools: firecrawl_tools(),
            }),
        }
    }
}

pub struct ThreadService {
    db: Arc<DatabaseManager>,
    asset_service: Arc<AssetService>,
    providers: Providers,
}

impl ThreadService {
    pub fn new(db: Arc<DatabaseManager>, asset_service: Arc<AssetService>) -> Self {
        let providers = build_providers();

        Self {
            db,
            asset_service,
            providers,
        }
    }

    fn get_title_provider(&self) -> Arc<dyn BaseChatModel + Send + Sync> {
        self.providers.title.clone()
    }

    async fn prepare_llm_context(
        &self,
        mut messages: Vec<AnyMessage>,
    ) -> Result<LlmContext, ThreadServiceError> {
        self.resolve_plain_text_blocks(&mut messages).await;

        let Some(vision) = self.providers.vision.as_ref() else {
            self.resolve_image_blocks(&mut messages).await;
            return Ok(LlmContext {
                messages,
                chat_model: self.providers.chat.clone(),
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
                self.asset_service.clone(),
                allowed_images.clone(),
            )) as Arc<dyn BaseTool>;
            tools.insert(describe_image_tool::TOOL_NAME.to_string(), describe);
        }

        let tool_likes: Vec<ToolLike> = tools.values().cloned().map(ToolLike::Tool).collect();
        let bound = self
            .providers
            .chat
            .bind_tools(&tool_likes, None)
            .map_err(|e| {
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

    async fn upload_block_content(
        &self,
        name: &str,
        content: &[u8],
        mime_type: &str,
        extras: &Option<HashMap<String, serde_json::Value>>,
        user_id: Uuid,
    ) -> Result<(String, String), Status> {
        let asset_response = self
            .asset_service
            .create_asset(
                proto_gen::asset::CreateAssetRequest {
                    name: name.to_string(),
                    content: content.to_vec(),
                    mime_type: mime_type.to_string(),
                    metadata: extras.as_ref().and_then(|e| serde_json::to_string(e).ok()),
                    activity_id: None,
                },
                user_id,
            )
            .await
            .map_err(|e| Status::internal(format!("Failed to save content block as asset: {e}")))?;

        let asset_id = asset_response
            .asset
            .as_ref()
            .map(|a| a.id.clone())
            .unwrap_or_default();
        let storage_uri = asset_response
            .asset
            .as_ref()
            .map(|a| a.storage_uri.clone())
            .unwrap_or_default();

        Ok((asset_id, storage_uri))
    }

    async fn resolve_plain_text_blocks(&self, messages: &mut [AnyMessage]) {
        let storage = self.asset_service.storage();
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

    async fn resolve_image_blocks(&self, messages: &mut [AnyMessage]) {
        use base64::{Engine as _, engine::general_purpose};

        let storage = self.asset_service.storage();
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

    fn db_thread_to_proto(thread: be_remote_db::Thread) -> ProtoThread {
        ProtoThread {
            id: thread.id.to_string(),
            user_id: thread.user_id.to_string(),
            title: thread.title.clone().unwrap_or_default(),
            created_at: Some(datetime_to_timestamp(thread.created_at)),
            updated_at: Some(datetime_to_timestamp(thread.updated_at)),
            active_leaf_id: thread.active_leaf_id.map(|id| id.to_string()),
        }
    }

    fn build_tree_from_rows(
        rows: Vec<be_remote_db::BranchMessageRow>,
    ) -> Vec<BaseMessageWithSibling> {
        let mut result: Vec<BaseMessageWithSibling> = Vec::new();
        let mut current_branch_id: Option<Uuid> = None;

        for row in rows {
            let is_active = row.message.id == row.branch_message_id;
            let parent_id = row
                .message
                .parent_message_id
                .map(|id| id.to_string())
                .unwrap_or_default();
            let proto_message = row.message.into();
            let sibling = BaseMessageWithSibling {
                parent_id: parent_id.clone(),
                message: Some(proto_message),
                children: vec![],
                sibling_index: row.sibling_index as i32,
                depth: row.branch_depth,
            };

            let is_new_group = current_branch_id != Some(row.branch_message_id);
            if is_new_group {
                current_branch_id = Some(row.branch_message_id);
                result.push(BaseMessageWithSibling {
                    parent_id,
                    message: None,
                    children: vec![],
                    sibling_index: 0,
                    depth: row.branch_depth,
                });
            }

            let group = result.last_mut().expect("group always exists");
            if is_active {
                group.sibling_index = group.children.len() as i32;
                group.message = sibling.message.clone();
            }
            group.children.push(sibling);
        }

        result
    }

    async fn fetch_branch_with_siblings(
        &self,
        thread_id: Uuid,
        user_id: Uuid,
        params: PaginationParams,
    ) -> Result<Vec<BaseMessageWithSibling>, Status> {
        let rows = self
            .db
            .list_branch_with_siblings()
            .thread_id(thread_id)
            .user_id(user_id)
            .params(params)
            .call()
            .await
            .map_err(ThreadServiceError::from)?;

        Ok(Self::build_tree_from_rows(rows))
    }

    fn build_full_tree(messages: Vec<be_remote_db::Message>) -> Vec<BaseMessageWithSibling> {
        let mut children_by_parent: HashMap<Option<Uuid>, Vec<be_remote_db::Message>> =
            HashMap::new();
        for msg in messages {
            children_by_parent
                .entry(msg.parent_message_id)
                .or_default()
                .push(msg);
        }

        fn build_subtree(
            parent_id: Option<Uuid>,
            children_by_parent: &HashMap<Option<Uuid>, Vec<be_remote_db::Message>>,
            depth: i32,
        ) -> Vec<BaseMessageWithSibling> {
            let Some(siblings) = children_by_parent.get(&parent_id) else {
                return vec![];
            };
            siblings
                .iter()
                .enumerate()
                .map(|(idx, msg)| {
                    let children = build_subtree(Some(msg.id), children_by_parent, depth + 1);
                    BaseMessageWithSibling {
                        parent_id: msg
                            .parent_message_id
                            .map(|id| id.to_string())
                            .unwrap_or_default(),
                        message: Some(msg.clone().into()),
                        children,
                        sibling_index: idx as i32,
                        depth,
                    }
                })
                .collect()
        }

        build_subtree(None, &children_by_parent, 0)
    }

    async fn fetch_full_tree(
        &self,
        thread_id: Uuid,
        user_id: Uuid,
    ) -> Result<Vec<BaseMessageWithSibling>, Status> {
        let messages = self
            .db
            .list_all_thread_messages(thread_id, user_id, 1000, 0)
            .await
            .map_err(ThreadServiceError::from)?;

        Ok(Self::build_full_tree(messages))
    }
}

fn datetime_to_timestamp(dt: DateTime<Utc>) -> Timestamp {
    Timestamp {
        seconds: dt.timestamp(),
        nanos: dt.timestamp_subsec_nanos() as i32,
    }
}

type ChatResult<T> = Result<Response<T>, Status>;
type ChatStreamResult = Pin<Box<dyn Stream<Item = Result<ChatStreamResponse, Status>> + Send>>;

struct CancellableStream {
    rx: mpsc::Receiver<Result<ChatStreamResponse, Status>>,
    token: CancellationToken,
}

impl Stream for CancellableStream {
    type Item = Result<ChatStreamResponse, Status>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.rx.poll_recv(cx)
    }
}

impl Drop for CancellableStream {
    fn drop(&mut self) {
        self.token.cancel();
    }
}

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
                let first_message = self
                    .db
                    .list_messages()
                    .thread_id(thread_id)
                    .user_id(user_id)
                    .params(PaginationParams::new(0, 1, "ASC"))
                    .call()
                    .await
                    .map_err(ThreadServiceError::from)?;
                first_message.first().and_then(|msg| msg.parent_message_id)
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

        let mut recent_messages = self
            .db
            .list_messages()
            .thread_id(thread_id)
            .user_id(user_id)
            .params(PaginationParams::new(0, CONTEXT_MESSAGE_LIMIT, "DESC"))
            .call()
            .await
            .map_err(ThreadServiceError::from)?;

        recent_messages.reverse();

        let mut messages: Vec<AnyMessage> = recent_messages
            .into_iter()
            .filter_map(|msg| {
                convert_db_message_to_base_message(msg)
                    .map_err(|e| tracing::warn!("Skipping unconvertible message: {e}"))
                    .ok()
            })
            .collect();

        let mut human_additional_kwargs = HashMap::new();
        if let Some(ref chips_json) = req.asset_chips_json
            && let Ok(chips_value) = serde_json::from_str::<serde_json::Value>(chips_json)
        {
            human_additional_kwargs.insert("asset_chips".to_string(), chips_value);
        }

        let content_blocks: Vec<ContentBlock> = req
            .content_blocks
            .into_iter()
            .map(ContentBlock::from)
            .collect();

        let human_message = HumanMessage::builder()
            .content(content_blocks)
            .additional_kwargs(human_additional_kwargs)
            .build();

        messages.push(human_message.clone().into());

        let content = serde_json::to_value(&human_message.content).map_err(|e| {
            ThreadServiceError::Internal(format!("Failed to serialize content: {}", e))
        })?;

        let additional_kwargs =
            serde_json::to_value(&human_message.additional_kwargs).map_err(|e| {
                ThreadServiceError::Internal(format!(
                    "Failed to serialize additional_kwargs: {}",
                    e
                ))
            })?;

        let human_db_message = self
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

        let LlmContext {
            messages: llm_messages,
            chat_model: chat_provider,
            tools,
        } = self.prepare_llm_context(messages).await?;

        let db = self.db.clone();
        let human_parent_id = human_db_message
            .parent_message_id
            .map(|id| id.to_string())
            .unwrap_or_default();
        let human_message_id = human_db_message.id;
        let human_proto: BaseMessageWithSibling = {
            let proto_msg = human_db_message.into();
            BaseMessageWithSibling {
                parent_id: human_parent_id,
                message: Some(proto_msg),
                children: vec![],
                sibling_index: 0,
                depth: 0,
            }
        };
        let (tx, rx) = mpsc::channel(32);
        let token = CancellationToken::new();
        let stream = CancellableStream {
            rx,
            token: token.clone(),
        };

        let _ = tx
            .send(Ok(ChatStreamResponse {
                payload: Some(Payload::ConfirmedHumanMessage(human_proto)),
            }))
            .await;

        tokio::spawn(
            run_agent_loop()
                .tx(tx)
                .token(token)
                .db(db)
                .chat_model(chat_provider)
                .tools(tools)
                .messages(llm_messages)
                .thread_id(thread_id)
                .user_id(user_id)
                .human_message_id(human_message_id)
                .max_tool_rounds(MAX_TOOL_ROUNDS)
                .call(),
        );

        Ok(Response::new(Box::pin(stream) as Self::ChatStreamStream))
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

        let messages = if req.all_variants {
            self.fetch_full_tree(thread_id, user_id).await?
        } else {
            self.fetch_branch_with_siblings(
                thread_id,
                user_id,
                PaginationParams::new(req.offset, req.limit, "ASC"),
            )
            .await?
        };

        Ok(Response::new(GetMessagesResponse { messages }))
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

        let recent_messages = self
            .db
            .list_messages()
            .thread_id(thread_id)
            .user_id(user_id)
            .params(PaginationParams::new(0, 5, "DESC"))
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

        messages.extend(recent_messages.into_iter().rev().filter_map(|msg| {
            convert_db_message_to_base_message(msg)
                .map_err(|e| tracing::warn!("Skipping unconvertible message: {e}"))
                .ok()
        }));

        let title_provider = self.get_title_provider();
        let mut title = match title_provider.invoke(messages, None).await {
            Ok(message) => message.content.to_string(),
            Err(_) => "New Chat".to_string(),
        };
        let title_words: Vec<&str> = title.split_whitespace().collect();
        title = title_words[..title_words.len().min(6)].join(" ");
        if title.is_empty() {
            tracing::warn!("Failed to generate title");
            title = "New Chat".to_string();
        }

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
            .fetch_branch_with_siblings(thread_id, user_id, PaginationParams::new(0, 100, "ASC"))
            .await?;

        Ok(Response::new(GetMessagesResponse { messages }))
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
                updated_at: Some(datetime_to_timestamp(r.updated_at)),
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
                created_at: Some(datetime_to_timestamp(r.created_at)),
                snippet: r.snippet,
            })
            .collect();

        Ok(Response::new(SearchMessagesResponse { results }))
    }

    async fn save_preliminary_content_blocks(
        &self,
        request: Request<SavePreliminaryContentBlocksRequest>,
    ) -> Result<Response<SavePreliminaryContentBlocksResponse>, Status> {
        tracing::info!("SavePreliminaryContentBlocks request received");

        let claims = extract_claims(&request)?;
        let user_id = parse_user_id(claims)?;
        let req = request.into_inner();

        let thread_id =
            Uuid::parse_str(&req.thread_id).map_err(|e| ThreadServiceError::InvalidUuid {
                field: "thread_id",
                source: e,
            })?;

        self.db
            .get_thread()
            .id(thread_id)
            .user_id(user_id)
            .call()
            .await
            .map_err(ThreadServiceError::from)?;

        if req.content_blocks.len() > MAX_CONTENT_BLOCKS {
            return Err(ThreadServiceError::invalid_argument(format!(
                "Too many content blocks (max {MAX_CONTENT_BLOCKS})"
            )))?;
        }

        let blocks: Vec<ContentBlock> = req
            .content_blocks
            .into_iter()
            .map(ContentBlock::from)
            .collect();

        let mut result_blocks: Vec<ContentBlock> = Vec::with_capacity(blocks.len());

        for block in blocks {
            match block {
                ContentBlock::PlainText(plain) if plain.text.is_some() => {
                    let text = plain.text.as_ref().unwrap();
                    let content = text.as_bytes().to_vec();
                    let name = plain
                        .title
                        .clone()
                        .unwrap_or_else(|| "content.json".to_string());

                    let (asset_id, storage_uri) = self
                        .upload_block_content(
                            &name,
                            &content,
                            &plain.mime_type,
                            &plain.extras,
                            user_id,
                        )
                        .await?;

                    let mutated = PlainTextContentBlock {
                        text: None,
                        file_id: Some(asset_id),
                        url: Some(storage_uri),
                        ..plain
                    };

                    result_blocks.push(ContentBlock::PlainText(mutated));
                }
                ContentBlock::Image(image) if image.base64.is_some() => {
                    use base64::{Engine as _, engine::general_purpose};

                    let b64 = image.base64.as_ref().unwrap();
                    let content = general_purpose::STANDARD.decode(b64).map_err(|e| {
                        Status::invalid_argument(format!("Invalid base64 in image block: {e}"))
                    })?;
                    let mime = image
                        .mime_type
                        .clone()
                        .unwrap_or_else(|| "image/png".to_string());
                    let name = format!(
                        "image.{}",
                        be_storage::StorageService::extension_from_mime(&mime)
                    );

                    let (asset_id, storage_uri) = self
                        .upload_block_content(&name, &content, &mime, &image.extras, user_id)
                        .await?;

                    let mutated = ImageContentBlock {
                        base64: None,
                        file_id: Some(asset_id),
                        url: Some(storage_uri),
                        ..image
                    };

                    result_blocks.push(ContentBlock::Image(mutated));
                }
                other => {
                    result_blocks.push(other);
                }
            }
        }

        let proto_blocks: Vec<ProtoContentBlock> = result_blocks
            .into_iter()
            .map(ProtoContentBlock::from)
            .collect();

        Ok(Response::new(SavePreliminaryContentBlocksResponse {
            content_blocks: proto_blocks,
        }))
    }
}
