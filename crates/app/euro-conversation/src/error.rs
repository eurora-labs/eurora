use thiserror::Error;
use tokio::sync::broadcast::error::SendError;
use tonic::Status;

use crate::types::ConversationEvent;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Failed to create conversation: {0}")]
    CreateConversation(String),

    #[error("Failed to update conversation: {0}")]
    UpdateConversation(String),

    #[error("Could not send event: {0}")]
    SendEvent(#[source] SendError<ConversationEvent>),

    #[error("Transport error: {0}")]
    Transport(#[source] Status),

    #[error("Could not set conversation id: {0}")]
    SetId(String),

    #[error("Conversation not found")]
    ConversationNotFound,

    #[error("Invalid conversation id")]
    InvalidConversationId,
}

impl From<SendError<ConversationEvent>> for Error {
    fn from(err: SendError<ConversationEvent>) -> Self {
        Error::SendEvent(err)
    }
}

impl From<Status> for Error {
    fn from(err: Status) -> Self {
        Error::Transport(err)
    }
}

pub type Result<T> = std::result::Result<T, Error>;
