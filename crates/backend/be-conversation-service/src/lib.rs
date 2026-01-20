//! Euro Conversation Service
//!
//! This crate provides a gRPC service for managing user conversations.
//! It serves as a cloud-based replacement for the conversation-related
//! functionality in the local personal database.
//!
//! ## Error Handling
//!
//! The crate uses [`ConversationServiceError`] for all error conditions, which
//! automatically converts to appropriate gRPC [`tonic::Status`] codes.

mod converters;
mod error;
mod server;

pub use error::{ConversationServiceError, ConversationServiceResult};
pub use server::{ConversationService, ProtoConversationService, ProtoConversationServiceServer};
