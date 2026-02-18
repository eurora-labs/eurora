use thiserror::Error;
use tokio::sync::broadcast::error::SendError;
use tonic::Status;

use crate::types::ThreadEvent;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Failed to create thread: {0}")]
    CreateThread(String),

    #[error("Failed to update thread: {0}")]
    UpdateThread(String),

    #[error("Could not send event: {0}")]
    SendEvent(#[source] SendError<ThreadEvent>),

    #[error("Transport error: {0}")]
    Transport(#[source] Status),

    #[error("Could not set thread id: {0}")]
    SetId(String),

    #[error("Thread not found")]
    ThreadNotFound,

    #[error("Invalid thread id")]
    InvalidThreadId,
}

impl From<SendError<ThreadEvent>> for Error {
    fn from(err: SendError<ThreadEvent>) -> Self {
        Error::SendEvent(err)
    }
}

impl From<Status> for Error {
    fn from(err: Status) -> Self {
        Error::Transport(err)
    }
}

pub type Result<T> = std::result::Result<T, Error>;
