//! Server-side implementation for the Conversation Service.
//!
//! This module contains the gRPC server implementation and is only
//! available when the `server` feature is enabled.

use std::sync::Arc;

use be_auth_grpc::Claims;
use be_remote_db::{
    CreateConversationRequest as DbCreateConversationRequest, DatabaseManager,
    ListConversationsRequest as DbListConversationsRequest,
};
use chrono::{DateTime, Utc};
use prost_types::Timestamp;
use tonic::{Request, Response, Status};
use tracing::info;
use uuid::Uuid;

use crate::error::ConversationServiceError;

use proto_gen::conversation::{
    Conversation, CreateConversationRequest, CreateConversationResponse, ListConversationsRequest,
    ListConversationsResponse,
};

pub use proto_gen::conversation::proto_conversation_service_server::{
    ProtoConversationService, ProtoConversationServiceServer,
};

/// The main conversation service
#[derive(Debug)]
pub struct ConversationService {
    db: Arc<DatabaseManager>,
}

impl ConversationService {
    /// Create a new ConversationService instance
    pub fn new(db: Arc<DatabaseManager>) -> Self {
        info!("Creating new ConversationService instance");
        Self { db }
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

/// Extract and validate claims from a gRPC request.
fn extract_claims<T>(request: &Request<T>) -> Result<&Claims, ConversationServiceError> {
    request
        .extensions()
        .get::<Claims>()
        .ok_or_else(|| ConversationServiceError::unauthenticated("Missing claims"))
}

/// Parse a user ID from claims.
fn parse_user_id(claims: &Claims) -> Result<Uuid, ConversationServiceError> {
    Uuid::parse_str(&claims.sub).map_err(|e| ConversationServiceError::invalid_uuid("user_id", e))
}

#[tonic::async_trait]
impl ProtoConversationService for ConversationService {
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
}
