use thiserror::Error;
use tokio::sync::broadcast::error::SendError;
use tonic::Status;

use crate::types::ConversationEvent;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Failed to create conversation")]
    CreateConversation,

    #[error("Could not send event: {0}")]
    SendEvent(#[source] SendError<ConversationEvent>),

    #[error("Could not save conversation: {0}")]
    SaveConversation(#[source] Status),
}

impl From<SendError<ConversationEvent>> for Error {
    fn from(err: SendError<ConversationEvent>) -> Self {
        Error::SendEvent(err)
    }
}

impl From<Status> for Error {
    fn from(err: Status) -> Self {
        Error::SaveConversation(err)
    }
}

/// Result type alias for activity service operations.
pub type Result<T> = std::result::Result<T, Error>;
