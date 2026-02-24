use agent_chain::SystemMessage;
use agent_chain::openai::BuiltinTool;
use agent_chain::{
    AIMessage, BaseChatModel, BaseMessage, BaseTool, HumanMessage, language_models::ToolLike,
    messages::ToolCall, ollama::ChatOllama, openai::ChatOpenAI,
};
use be_authz::{extract_claims, parse_user_id};
use be_local_settings::{OllamaConfig, OpenAIConfig, ProviderSettings, SettingsReceiver};
use be_remote_db::{DatabaseManager, MessageType, PaginationParams};
use chrono::{DateTime, Utc};
use prost_types::Timestamp;
pub use proto_gen::thread::proto_thread_service_server::{
    ProtoThreadService, ProtoThreadServiceServer,
};
use proto_gen::thread::{
    AddHiddenHumanMessageRequest, AddHiddenHumanMessageResponse, AddHumanMessageRequest,
    AddHumanMessageResponse, AddSystemMessageRequest, AddSystemMessageResponse, ChatStreamRequest,
    ChatStreamResponse, CreateThreadRequest, CreateThreadResponse, GenerateThreadTitleRequest,
    GenerateThreadTitleResponse, GetMessagesRequest, GetMessagesResponse, GetThreadResponse,
    ListThreadsRequest, ListThreadsResponse, Thread,
};
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::{Arc, RwLock};
use tokio_stream::{Stream, StreamExt};
use tonic::{Request, Response, Status};
use uuid::Uuid;

use crate::converters::convert_db_message_to_base_message;
use crate::error::ThreadServiceError;
use crate::tools::firecrawl_search_tool;

const BASE_NEBUL_URL: &str = "https://api.inference.nebul.io/v1";

/// When running inside Docker (`RUNNING_EURORA_FULLY_LOCAL=true`), rewrite
/// `localhost` / `127.0.0.1` to `host.docker.internal` so the container can
/// reach services on the host machine.
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
    Box::new(ChatOllama::new(model).base_url(&base_url))
}

fn build_openai(
    config: &OpenAIConfig,
    model_override: Option<&str>,
    web_search: bool,
) -> Box<dyn BaseChatModel + Send + Sync> {
    let model = model_override.unwrap_or(&config.model);
    let mut provider = ChatOpenAI::new(model)
        .api_key(config.api_key.as_str())
        .api_base(config.base_url.as_str());
    if web_search {
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

fn build_tool_map() -> HashMap<String, Arc<dyn BaseTool>> {
    let tool = firecrawl_search_tool();
    let name = tool.name().to_string();
    HashMap::from([(name, tool)])
}

fn build_env_fallback() -> Option<Providers> {
    let local_mode = std::env::var("RUNNING_EURORA_FULLY_LOCAL")
        .map(|v| v.eq_ignore_ascii_case("true"))
        .unwrap_or(false);

    if local_mode {
        let model = std::env::var("OLLAMA_MODEL").unwrap_or_else(|_| "llama3.2".to_string());
        let host = std::env::var("OLLAMA_HOST")
            .unwrap_or_else(|_| "http://host.docker.internal:11434".to_string());
        let chat: Arc<dyn BaseChatModel + Send + Sync> =
            Arc::new(ChatOllama::new(&model).base_url(&host));
        let title: Arc<dyn BaseChatModel + Send + Sync> =
            Arc::new(ChatOllama::new(&model).base_url(&host));
        Some(Providers {
            chat,
            title,
            tools: HashMap::new(),
        })
    } else {
        let chat_model =
            ChatOpenAI::new(std::env::var("NEBUL_MODEL").expect("Nebul model should be set"))
                .api_base(BASE_NEBUL_URL)
                .api_key(std::env::var("NEBUL_API_KEY").expect("Nebul API key should be set"));
        let bound = chat_model
            .bind_tools(&[ToolLike::Tool(firecrawl_search_tool())], None)
            .expect("Failed to bind firecrawl_search tool");
        let chat: Arc<dyn BaseChatModel + Send + Sync> =
            Arc::from(bound as Box<dyn BaseChatModel + Send + Sync>);

        let title = Arc::new(
            ChatOpenAI::new(
                std::env::var("NEBUL_TITLE_MODEL").expect("Nebul title model should be set"),
            )
            .api_base(BASE_NEBUL_URL)
            .api_key(std::env::var("NEBUL_API_KEY").expect("Nebul API key should be set")),
        );

        Some(Providers {
            chat,
            title,
            tools: build_tool_map(),
        })
    }
}

pub struct ThreadService {
    db: Arc<DatabaseManager>,
    providers: Arc<RwLock<Option<Providers>>>,
}

impl ThreadService {
    pub fn new(db: Arc<DatabaseManager>, mut settings_rx: SettingsReceiver) -> Self {
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
                tools: build_tool_map(),
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
                        tools: build_tool_map(),
                    }
                });
                let mut lock = providers_handle.write().unwrap_or_else(|e| e.into_inner());
                *lock = new_providers;
            }
        });

        Self { db, providers }
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
type ChatStreamResult = Pin<Box<dyn Stream<Item = Result<ChatStreamResponse, Status>> + Send>>;

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

        tracing::info!("Created thread {} for user {}", thread.id, user_id);

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
            .params(PaginationParams::new(
                req.offset,
                req.limit,
                "DESC".to_string(),
            ))
            .call()
            .await
            .map_err(ThreadServiceError::from)?;

        tracing::info!("Listed {} threads for user {}", threads.len(), user_id);

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

        let content = serde_json::to_value(&human_message.content).map_err(|e| {
            ThreadServiceError::Internal(format!("Failed to serialize message content: {}", e))
        })?;

        let message = self
            .db
            .create_message()
            .thread_id(thread_id)
            .user_id(user_id)
            .message_type(MessageType::Human)
            .content(content)
            .call()
            .await
            .map_err(ThreadServiceError::from)?;

        tracing::info!(
            "Added human message to thread {} for user {}",
            thread_id,
            user_id
        );

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

        let content = serde_json::to_value(&human_message.content).map_err(|e| {
            ThreadServiceError::Internal(format!("Failed to serialize message content: {}", e))
        })?;

        let message = self
            .db
            .create_message()
            .thread_id(thread_id)
            .user_id(user_id)
            .message_type(MessageType::Human)
            .content(content)
            .hidden_from_ui(true)
            .call()
            .await
            .map_err(ThreadServiceError::from)?;

        tracing::info!(
            "Added hidden human message to thread {} for user {}",
            thread_id,
            user_id
        );

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

        let content = serde_json::to_value(&system_message.content).map_err(|e| {
            ThreadServiceError::Internal(format!("Failed to serialize message content: {}", e))
        })?;

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

        tracing::info!(
            "Added system message to thread {} for user {}",
            thread_id,
            user_id
        );

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

        tracing::debug!(
            "ChatStream: user_id = {}, thread_id = {}",
            user_id,
            thread_id
        );

        // TODO: this is incorrect. This is essentially
        // a replacement for proper agent-driven rag
        // that should be implemented alongside agent-graph.
        // For now this is fineeee
        let mut hidden_messages = self
            .db
            .list_messages()
            .thread_id(thread_id)
            .user_id(user_id)
            .include_visible(false)
            .params(PaginationParams::new(0, 2, "DESC".to_string()))
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
            .params(PaginationParams::new(0, 3, "DESC".to_string()))
            .call()
            .await
            .map_err(ThreadServiceError::from)?;
        visible_messages.reverse();

        hidden_messages.extend(visible_messages);

        let mut messages: Vec<BaseMessage> = hidden_messages
            .into_iter()
            .map(|msg| convert_db_message_to_base_message(msg).unwrap())
            .collect();

        let human_message = HumanMessage::builder().content(req.content.clone()).build();

        messages.push(human_message.clone().into());

        let content = serde_json::to_value(&human_message.content).map_err(|e| {
            ThreadServiceError::Internal(format!("Failed to serialize message content: {}", e))
        })?;

        let _message = self
            .db
            .create_message()
            .thread_id(thread_id)
            .user_id(user_id)
            .message_type(MessageType::Human)
            .content(content)
            .call()
            .await
            .map_err(ThreadServiceError::from)?;

        let chat_provider = self.get_chat_provider()?;
        let tools = self.get_tools();

        let db = self.db.clone();
        let output_stream = async_stream::try_stream! {
            let mut full_content = String::new();
            let mut total_input_tokens: i64 = 0;
            let mut total_output_tokens: i64 = 0;
            let mut total_reasoning_tokens: i64 = 0;
            let mut total_cache_creation_tokens: i64 = 0;
            let mut total_cache_read_tokens: i64 = 0;
            const MAX_TOOL_ROUNDS: usize = 5;

            for round in 0..=MAX_TOOL_ROUNDS {
                let provider_stream = chat_provider
                    .astream(messages.clone().into(), None, None)
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
                                yield ChatStreamResponse {
                                    chunk: content,
                                    is_final: false,
                                };
                            }
                            if !chunk.tool_calls.is_empty() {
                                tool_calls.extend(chunk.tool_calls);
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

            yield ChatStreamResponse {
                chunk: String::new(),
                is_final: true,
            };

            if !full_content.is_empty() {
                match db
                    .create_message().thread_id(thread_id)
                    .user_id(user_id)
                    .message_type(MessageType::Ai)
                    .content(serde_json::json!(full_content))
                    .call()
                    .await
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
            .params(PaginationParams::new(
                req.offset,
                req.limit,
                "ASC".to_string(),
            ))
            .include_hidden(false)
            .call()
            .await
            .map_err(ThreadServiceError::from)?;

        Ok(Response::new(GetMessagesResponse {
            messages: messages.into_iter().map(|m| m.into()).collect(),
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
            .params(PaginationParams::new(0, 2, "DESC".to_string()))
            .call()
            .await
            .map_err(ThreadServiceError::from)?;

        let mut messages: Vec<BaseMessage> = hidden_messages
            .into_iter()
            .map(|msg| convert_db_message_to_base_message(msg).unwrap())
            .collect();

        messages.push(HumanMessage::builder().content(req.content).build().into());

        messages.push(
            SystemMessage::builder()
                .content(
                    "Generate a title for the past thread. Your task is:
                - Return a concise title, max 6 words.
                - No quotation marks.
                - Use sentence case.
                - Summarize the main topic, not the tone.
                - If the topic is unclear, use a generic title.
                Output only the title text.
                "
                    .to_string(),
                )
                .build()
                .into(),
        );

        let title_provider = self.get_title_provider()?;
        let mut title = match title_provider.invoke(messages.into(), None).await {
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

        // Capitalize the first letter of the title
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
}
