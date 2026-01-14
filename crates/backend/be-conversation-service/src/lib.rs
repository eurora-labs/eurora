//! Euro Conversation Service
//!
//! This crate provides a gRPC service for managing user conversations.
//! It serves as a cloud-based replacement for the conversation-related
//! functionality in the local personal database.
//!
//! ## Features
//!
//! - `server` - Enables server-side functionality including the gRPC service
//!   implementation. This feature adds dependencies on `auth-core` and
//!   `be-remote-db`. Without this feature, only the proto types and
//!   client are available.
//!
//! ## Error Handling
//!
//! The crate uses [`ConversationServiceError`] for all error conditions, which
//! automatically converts to appropriate gRPC [`tonic::Status`] codes.

mod converters;
mod error;
mod server;

pub use error::{ConversationResult, ConversationServiceError};
pub use server::{ConversationService, ProtoConversationService, ProtoConversationServiceServer};
