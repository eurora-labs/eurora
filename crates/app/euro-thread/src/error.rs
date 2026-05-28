use thiserror::Error;
use thread_core::ThreadErrorResponse;

use crate::chat_bridge::ChatSinkError;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Failed to acquire access token: {0}")]
    Auth(String),

    #[error("HTTP transport error: {0}")]
    Transport(#[source] reqwest::Error),

    #[error("WebSocket transport error: {0}")]
    WebSocket(#[source] tokio_tungstenite::tungstenite::Error),

    #[error("Server returned {status}: {message} ({error})")]
    Service {
        status: reqwest::StatusCode,
        error: String,
        message: String,
    },

    #[error("Failed to encode request body: {0}")]
    Encode(#[source] serde_json::Error),

    #[error("Failed to decode response body: {0}")]
    Decode(#[source] serde_json::Error),

    #[error("Thread not found")]
    ThreadNotFound,

    #[error("Request cancelled")]
    Cancelled,

    #[error("Invalid endpoint URL: {0}")]
    InvalidUrl(String),

    #[error("Chat protocol error: {0}")]
    ChatProtocol(String),

    #[error("Chat event sink failed: {0}")]
    Sink(#[source] ChatSinkError),
}

impl Error {
    pub(crate) fn from_response(status: reqwest::StatusCode, body: &str) -> Self {
        if status == reqwest::StatusCode::NOT_FOUND {
            return Self::ThreadNotFound;
        }
        match serde_json::from_str::<ThreadErrorResponse>(body) {
            Ok(parsed) => Self::Service {
                status,
                error: parsed.error,
                message: parsed.message,
            },
            Err(_) => Self::Service {
                status,
                error: "unknown".to_string(),
                message: if body.is_empty() {
                    String::from("(empty body)")
                } else {
                    body.to_string()
                },
            },
        }
    }
}

impl From<reqwest::Error> for Error {
    fn from(err: reqwest::Error) -> Self {
        Self::Transport(err)
    }
}

impl From<tokio_tungstenite::tungstenite::Error> for Error {
    fn from(err: tokio_tungstenite::tungstenite::Error) -> Self {
        Self::WebSocket(err)
    }
}

pub type Result<T> = std::result::Result<T, Error>;
