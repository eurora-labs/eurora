//! Server-side implementation for the Conversation Service.

use agent_chain::{BaseChatModel, BaseMessage, openai::ChatOpenAI};
use be_auth_grpc::{extract_claims, parse_user_id};
use be_remote_db::{
    CreateConversationRequest as DbCreateConversationRequest, DatabaseManager,
    GetLastMessagesRequest, ListConversationsRequest as DbListConversationsRequest,
};
use chrono::{DateTime, Utc};
use prost_types::Timestamp;
use std::{pin::Pin, sync::Arc};
use tokio_stream::{Stream, StreamExt};
use tonic::{Request, Response, Status};
use tracing::{debug, error, info};
use uuid::Uuid;

use crate::error::ConversationServiceError;

use proto_gen::conversation::{
    ChatStreamRequest, ChatStreamResponse, Conversation, CreateConversationRequest,
    CreateConversationResponse, ListConversationsRequest, ListConversationsResponse,
};

pub use proto_gen::conversation::proto_conversation_service_server::{
    ProtoConversationService, ProtoConversationServiceServer,
};

/// The main conversation service
#[derive(Debug)]
pub struct ConversationService {
    provider: ChatOpenAI,
    db: Arc<DatabaseManager>,
}

impl ConversationService {
    /// Create a new ConversationService instance
    pub fn new(db: Arc<DatabaseManager>) -> Self {
        info!("Creating new ConversationService instance");

        let api_key = std::env::var("OPENAI_API_KEY").unwrap_or_else(|_| {
            error!("OPENAI_API_KEY environment variable is not set");
            String::new()
        });
        let model = std::env::var("OPENAI_MODEL").unwrap_or_else(|_| "gpt-4".to_string());

        let provider = ChatOpenAI::new(&model).api_key(api_key);

        Self { provider, db }
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
            None
        } else {
            Some(req.title)
        };

        let conversation = self
            .db
            .create_conversation(DbCreateConversationRequest {
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
            .list_conversations(DbListConversationsRequest {
                user_id,
                limit: req.limit,
                offset: req.offset,
            })
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

        let _db_messages = self
            .db
            .get_last_messages(GetLastMessagesRequest {
                conversation_id,
                user_id,
                limit: 5,
            })
            .await
            .unwrap();

        // let messages: Vec<BaseMessage> = db_messages.into_iter().map(|msg| msg.into()).collect();

        // 1. Get current conversation
        // 2.

        // Convert ProtoBaseMessage to agent_chain_core::BaseMessage
        // let messages: Vec<BaseMessage> = req.messages.into_iter().map(|msg| msg.into()).collect();
        let messages: Vec<BaseMessage> = Vec::new();

        let openai_stream = self
            .provider
            .astream(messages.into(), None)
            .await
            .map_err(|e| {
                debug!("Error in chat_stream: {}", e);
                Status::internal(e.to_string())
            })?;

        let output_stream = openai_stream.map(|result| match result {
            Ok(chunk) => {
                // AIMessageChunk has content() method for getting the text content
                // We determine finality by empty content or chunk_position
                let content = chunk.content().to_string();
                // TODO: Don't rely on empty string for finality
                let is_final = content.is_empty();

                Ok(ChatStreamResponse {
                    chunk: content,
                    is_final,
                })
            }
            Err(e) => Err(Status::internal(e.to_string())),
        });

        Ok(Response::new(
            Box::pin(output_stream) as Self::ChatStreamStream
        ))
    }
}
