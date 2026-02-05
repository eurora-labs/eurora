//! Server-side implementation for the Conversation Service.

use agent_chain::SystemMessage;
use agent_chain::openai::BuiltinTool;
use agent_chain::{BaseChatModel, BaseMessage, HumanMessage, openai::ChatOpenAI};
use be_auth_grpc::{extract_claims, parse_user_id};
use be_remote_db::{
    DatabaseManager, GetConversation, ListConversations, MessageType, NewConversation,
    PaginationParams,
};
use chrono::{DateTime, Utc};
use prost_types::Timestamp;
use std::{pin::Pin, sync::Arc};
use tokio_stream::{Stream, StreamExt};
use tonic::{Request, Response, Status};
use tracing::{debug, error, info};
use uuid::Uuid;

use crate::error::ConversationServiceError;
use crate::{ConversationServiceResult, converters::convert_db_message_to_base_message};

use proto_gen::conversation::{
    AddHiddenHumanMessageRequest, AddHiddenHumanMessageResponse, AddHumanMessageRequest,
    AddHumanMessageResponse, AddSystemMessageRequest, AddSystemMessageResponse, ChatStreamRequest,
    ChatStreamResponse, Conversation, CreateConversationRequest, CreateConversationResponse,
    GenerateConversationTitleRequest, GenerateConversationTitleResponse, GetConversationResponse,
    GetMessagesRequest, GetMessagesResponse, ListConversationsRequest, ListConversationsResponse,
};

pub use proto_gen::conversation::proto_conversation_service_server::{
    ProtoConversationService, ProtoConversationServiceServer,
};

/// The main conversation service
#[derive(Debug)]
pub struct ConversationService {
    chat_provider: ChatOpenAI,
    title_provider: ChatOpenAI,
    db: Arc<DatabaseManager>,
}

impl ConversationService {
    /// Create a new ConversationService instance
    pub fn from_env(db: Arc<DatabaseManager>) -> ConversationServiceResult<Self> {
        info!("Creating new ConversationService instance");

        let api_key = std::env::var("OPENAI_API_KEY").unwrap_or_else(|_| {
            error!("OPENAI_API_KEY environment variable is not set");
            String::new()
        });
        let model = std::env::var("OPENAI_MODEL").unwrap_or_else(|_| "gpt-5.1".to_string());

        let chat_provider = ChatOpenAI::new(&model)
            .with_builtin_tools(vec![BuiltinTool::WebSearch])
            .api_key(api_key.clone());

        let _title_provider = ChatOpenAI::new("gpt-4.1-mini").api_key(api_key.clone());

        Ok(Self {
            chat_provider,
            title_provider: _title_provider,
            db,
        })
    }

    /// Convert a database Conversation to a proto Conversation
    fn db_conversation_to_proto(conversation: be_remote_db::Conversation) -> Conversation {
        Conversation {
            id: conversation.id.to_string(),
            user_id: conversation.user_id.to_string(),
            title: conversation.title.clone().unwrap_or_default(),
            created_at: Some(datetime_to_timestamp(conversation.created_at)),
            updated_at: Some(datetime_to_timestamp(conversation.updated_at)),
        }
    }
}

/// Convert DateTime<Utc> to prost_types::Timestamp
fn datetime_to_timestamp(dt: DateTime<Utc>) -> Timestamp {
    Timestamp {
        seconds: dt.timestamp(),
        nanos: dt.timestamp_subsec_nanos() as i32,
    }
}

type ChatResult<T> = Result<Response<T>, Status>;
type ChatStreamResult = Pin<Box<dyn Stream<Item = Result<ChatStreamResponse, Status>> + Send>>;

#[tonic::async_trait]
impl ProtoConversationService for ConversationService {
    type ChatStreamStream = ChatStreamResult;

    async fn create_conversation(
        &self,
        request: Request<CreateConversationRequest>,
    ) -> Result<Response<CreateConversationResponse>, Status> {
        info!("CreateConversation request received");

        let claims = extract_claims(&request)?;
        let user_id = parse_user_id(claims)?;

        let req = request.into_inner();

        let title = if req.title.is_empty() {
            "New Chat".to_string()
        } else {
            req.title
        };

        let conversation = self
            .db
            .create_conversation(NewConversation {
                id: None,
                user_id,
                title,
            })
            .await
            .map_err(ConversationServiceError::from)?;

        info!(
            "Created conversation {} for user {}",
            conversation.id, user_id
        );

        Ok(Response::new(CreateConversationResponse {
            conversation: Some(Self::db_conversation_to_proto(conversation)),
        }))
    }

    async fn list_conversations(
        &self,
        request: Request<ListConversationsRequest>,
    ) -> Result<Response<ListConversationsResponse>, Status> {
        info!("ListConversations request received");

        let claims = extract_claims(&request)?;
        let user_id = parse_user_id(claims)?;

        let req = request.into_inner();

        let conversations = self
            .db
            .list_conversations(
                ListConversations { user_id },
                PaginationParams::new(req.offset, req.limit, "DESC".to_string()),
            )
            .await
            .map_err(ConversationServiceError::from)?;

        info!(
            "Listed {} conversations for user {}",
            conversations.len(),
            user_id
        );

        Ok(Response::new(ListConversationsResponse {
            conversations: conversations
                .into_iter()
                .map(Self::db_conversation_to_proto)
                .collect(),
        }))
    }

    async fn add_human_message(
        &self,
        request: Request<AddHumanMessageRequest>,
    ) -> Result<Response<AddHumanMessageResponse>, Status> {
        info!("AddHumanMessage request received");

        let claims = extract_claims(&request)?;
        let user_id = parse_user_id(claims)?;
        let req = request.into_inner();

        let conversation_id = Uuid::parse_str(&req.conversation_id).map_err(|e| {
            ConversationServiceError::InvalidUuid {
                field: "conversation_id",
                source: e,
            }
        })?;

        // Extract the HumanMessage from the proto message
        let proto_message = req
            .message
            .ok_or_else(|| Status::invalid_argument("message field is required"))?;

        // Convert proto message to agent_chain HumanMessage for content serialization
        let human_message: HumanMessage = proto_message.into();

        // Serialize content for database storage
        let content = serde_json::to_value(&human_message.content).map_err(|e| {
            ConversationServiceError::Internal(format!(
                "Failed to serialize message content: {}",
                e
            ))
        })?;

        // Save the human message to the database
        let message = self
            .db
            .create_message()
            .conversation_id(conversation_id)
            .user_id(user_id)
            .message_type(MessageType::Human)
            .content(content)
            .call()
            .await
            .map_err(ConversationServiceError::from)?;

        info!(
            "Added human message to conversation {} for user {}",
            conversation_id, user_id
        );

        Ok(Response::new(AddHumanMessageResponse {
            message: Some(message.into()),
        }))
    }

    async fn add_hidden_human_message(
        &self,
        request: Request<AddHiddenHumanMessageRequest>,
    ) -> Result<Response<AddHiddenHumanMessageResponse>, Status> {
        info!("AddHiddenHumanMessage request received");

        let claims = extract_claims(&request)?;
        let user_id = parse_user_id(claims)?;
        let req = request.into_inner();

        let conversation_id = Uuid::parse_str(&req.conversation_id).map_err(|e| {
            ConversationServiceError::InvalidUuid {
                field: "conversation_id",
                source: e,
            }
        })?;

        // Extract the HumanMessage from the proto message
        let proto_message = req
            .message
            .ok_or_else(|| Status::invalid_argument("message field is required"))?;

        // Convert proto message to agent_chain HumanMessage for content serialization
        let human_message: HumanMessage = proto_message.into();

        // Serialize content for database storage
        let content = serde_json::to_value(&human_message.content).map_err(|e| {
            ConversationServiceError::Internal(format!(
                "Failed to serialize message content: {}",
                e
            ))
        })?;

        // Save the human message to the database with hidden_from_ui set to true
        let message = self
            .db
            .create_message()
            .conversation_id(conversation_id)
            .user_id(user_id)
            .message_type(MessageType::Human)
            .content(content)
            .hidden_from_ui(true)
            .call()
            .await
            .map_err(ConversationServiceError::from)?;

        info!(
            "Added hidden human message to conversation {} for user {}",
            conversation_id, user_id
        );

        Ok(Response::new(AddHiddenHumanMessageResponse {
            message: Some(message.into()),
        }))
    }

    async fn add_system_message(
        &self,
        request: Request<AddSystemMessageRequest>,
    ) -> Result<Response<AddSystemMessageResponse>, Status> {
        info!("AddSystemMessage request received");

        let claims = extract_claims(&request)?;
        let user_id = parse_user_id(claims)?;
        let req = request.into_inner();

        let conversation_id = Uuid::parse_str(&req.conversation_id).map_err(|e| {
            ConversationServiceError::InvalidUuid {
                field: "conversation_id",
                source: e,
            }
        })?;

        // Extract the SystemMessage from the proto message
        let proto_message = req
            .message
            .ok_or_else(|| Status::invalid_argument("message field is required"))?;

        // Convert proto message to agent_chain SystemMessage for content serialization
        let system_message: SystemMessage = proto_message.into();

        // Serialize content for database storage
        let content = serde_json::to_value(&system_message.content).map_err(|e| {
            ConversationServiceError::Internal(format!(
                "Failed to serialize message content: {}",
                e
            ))
        })?;

        // Save the system message to the database
        let message = self
            .db
            .create_message()
            .conversation_id(conversation_id)
            .user_id(user_id)
            .message_type(MessageType::System)
            .content(content)
            .call()
            .await
            .map_err(ConversationServiceError::from)?;

        info!(
            "Added system message to conversation {} for user {}",
            conversation_id, user_id
        );

        Ok(Response::new(AddSystemMessageResponse {
            message: Some(message.into()),
        }))
    }

    async fn chat_stream(
        &self,
        request: Request<ChatStreamRequest>,
    ) -> ChatResult<Self::ChatStreamStream> {
        info!("ChatStream request received");

        let claims = extract_claims(&request)?;
        let user_id = parse_user_id(claims)?;
        let req = request.into_inner();

        let conversation_id = Uuid::parse_str(&req.conversation_id).map_err(|e| {
            ConversationServiceError::InvalidUuid {
                field: "conversation_id",
                source: e,
            }
        })?;

        debug!(
            "ChatStream: user_id = {}, conversation_id = {}",
            user_id, conversation_id
        );

        let db_messages = self
            .db
            .list_messages()
            .conversation_id(conversation_id)
            .user_id(user_id)
            .params(PaginationParams::new(0, 5, "DESC".to_string()))
            .call()
            .await
            .map_err(ConversationServiceError::from)?;

        let mut messages: Vec<BaseMessage> = db_messages
            .into_iter()
            .map(|msg| convert_db_message_to_base_message(msg).unwrap())
            .collect();

        let human_message = HumanMessage::builder().content(req.content.clone()).build();

        messages.push(human_message.clone().into());

        // Serialize content for database storage using the same MessageContent shape
        let content = serde_json::to_value(&human_message.content).map_err(|e| {
            ConversationServiceError::Internal(format!(
                "Failed to serialize message content: {}",
                e
            ))
        })?;

        let _message = self
            .db
            .create_message()
            .conversation_id(conversation_id)
            .user_id(user_id)
            .message_type(MessageType::Human)
            .content(content)
            .call()
            .await
            .map_err(ConversationServiceError::from)?;

        let openai_stream = self
            .chat_provider
            .astream(messages.into(), None)
            .await
            .map_err(|e| {
                debug!("Error in chat_stream: {}", e);
                Status::internal(e.to_string())
            })?;

        let db = self.db.clone();
        let output_stream = async_stream::try_stream! {
            tokio::pin!(openai_stream);
            let mut full_content = String::new();

            while let Some(result) = openai_stream.next().await {
                match result {
                    Ok(chunk) => {
                        // AIMessageChunk has content for getting the text content
                        // We determine finality by empty content or chunk_position
                        let content = chunk.content.to_string();
                        full_content.push_str(&content);
                        // TODO: Don't rely on empty string for finality
                        let is_final = content.is_empty();

                        yield ChatStreamResponse {
                            chunk: content,
                            is_final,
                        };
                    }
                    Err(e) => {
                        Err(Status::internal(e.to_string()))?;
                    }
                }
            }

            // Save the AI message to the database after stream completes
            if !full_content.is_empty() && let Err(e) = db
                    .create_message().conversation_id(conversation_id)
                    .user_id(user_id)
                    .message_type(MessageType::Ai)
                    .content(serde_json::json!(full_content))
                    .call()
                    .await
                {
                    error!("Failed to save AI message to database: {}", e);
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
        info!("Get messages request received");
        let claims = extract_claims(&request)?;
        let user_id = parse_user_id(claims)?;
        let req = request.into_inner();

        let conversation_id = Uuid::parse_str(&req.conversation_id).map_err(|e| {
            ConversationServiceError::InvalidUuid {
                field: "conversation_id",
                source: e,
            }
        })?;

        let messages = self
            .db
            .list_messages()
            .conversation_id(conversation_id)
            .user_id(user_id)
            .params(PaginationParams::new(
                req.offset,
                req.limit,
                "ASC".to_string(),
            ))
            .only_visible(true)
            .call()
            .await
            .map_err(ConversationServiceError::from)?;

        Ok(Response::new(GetMessagesResponse {
            messages: messages.into_iter().map(|m| m.into()).collect(),
        }))
    }

    async fn get_conversation(
        &self,
        request: tonic::Request<proto_gen::conversation::GetConversationRequest>,
    ) -> Result<Response<GetConversationResponse>, Status> {
        info!("Get conversation request received");
        let claims = extract_claims(&request)?;
        let user_id = parse_user_id(claims)?;
        let req = request.into_inner();

        let conversation_id = Uuid::parse_str(&req.conversation_id).map_err(|e| {
            ConversationServiceError::InvalidUuid {
                field: "conversation_id",
                source: e,
            }
        })?;

        let conversation = self
            .db
            .get_conversation(GetConversation {
                id: conversation_id,
                user_id,
            })
            .await
            .map_err(ConversationServiceError::from)?;

        Ok(Response::new(GetConversationResponse {
            conversation: conversation.try_into().ok(),
        }))
    }

    async fn generate_conversation_title(
        &self,
        request: tonic::Request<GenerateConversationTitleRequest>,
    ) -> Result<Response<GenerateConversationTitleResponse>, Status> {
        info!("Generate conversation title request received");
        let claims = extract_claims(&request)?;
        let user_id = parse_user_id(claims)?;
        let req = request.into_inner();

        let conversation_id = Uuid::parse_str(&req.conversation_id).map_err(|e| {
            ConversationServiceError::InvalidUuid {
                field: "conversation_id",
                source: e,
            }
        })?;

        let db_messages = self
            .db
            .list_messages()
            .conversation_id(conversation_id)
            .user_id(user_id)
            .params(PaginationParams::new(0, 5, "ASC".to_string()))
            .call()
            .await
            .map_err(ConversationServiceError::from)?;

        let mut messages: Vec<BaseMessage> = db_messages
            .into_iter()
            .map(|msg| convert_db_message_to_base_message(msg).unwrap())
            .collect();

        messages.push(HumanMessage::builder().content(req.content).build().into());

        messages.push(
            SystemMessage::builder()
                .content(
                    "Generate a title for the past conversation. Your task is:
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

        let title = match self.title_provider.invoke(messages.into()).await {
            Ok(message) => message.content,
            Err(_) => "New Chat".to_string(),
        };
        let title_words: Vec<&str> = title.split_whitespace().collect();
        let title = title_words[..title_words.len().min(6)].join(" ");

        let conversation = self
            .db
            .update_conversation()
            .id(conversation_id)
            .user_id(user_id)
            .title(title.clone())
            .call()
            .await
            .map_err(ConversationServiceError::from)?;

        Ok(Response::new(GenerateConversationTitleResponse {
            conversation: Some(Self::db_conversation_to_proto(conversation)),
        }))
    }
}
