use serde::Serialize;
use specta::Type;
use thiserror::Error;

/// Typed error surface for the CRUD/search `thread_*` IPC commands.
/// Externally tagged so the JS side can branch on `error.type` without
/// parsing strings. `NotFound` lifts [`crate::Error::ThreadNotFound`] to
/// a dedicated variant so the UI can render an empty state instead of a
/// generic toast.
#[derive(Debug, Error, Serialize, Type)]
#[serde(tag = "type", content = "data")]
pub enum ThreadError {
    #[error("thread not found")]
    NotFound,
    #[error("backend unreachable: {0}")]
    Backend(String),
    #[error("bad response: {0}")]
    BadResponse(String),
    #[error("state unavailable: {0}")]
    StateUnavailable(&'static str),
    #[error("internal: {0}")]
    Internal(String),
}

impl From<crate::Error> for ThreadError {
    fn from(err: crate::Error) -> Self {
        use crate::Error as E;
        match err {
            E::ThreadNotFound => ThreadError::NotFound,
            E::Transport(ref e) => ThreadError::Backend(e.to_string()),
            E::WebSocket(ref e) => ThreadError::Backend(e.to_string()),
            E::Service { .. } => ThreadError::BadResponse(err.to_string()),
            E::Encode(e) | E::Decode(e) => ThreadError::BadResponse(e.to_string()),
            E::Auth(_) | E::InvalidUrl(_) | E::ChatProtocol(_) | E::Sink(_) | E::Cancelled => {
                ThreadError::Internal(err.to_string())
            }
        }
    }
}

/// Typed error surface for the streaming `chat_*` IPC commands.
/// Externally tagged so the JS side can branch on `error.type`.
/// `Cancelled` is split out from the rest so the UI can suppress its
/// own cancel-induced errors instead of showing a toast for them; an
/// upstream [`crate::Error`] (e.g. a deleted thread mid-stream) is
/// wrapped in `Thread` so the JS side can drill into the same
/// [`ThreadError`] variants it handles for CRUD calls.
#[derive(Debug, Error, Serialize, Type)]
#[serde(tag = "type", content = "data")]
pub enum StreamError {
    #[error("cancelled")]
    Cancelled,
    #[error("stream timed out after {0} seconds")]
    Timeout(u32),
    #[error("channel: {0}")]
    Channel(String),
    #[error("state unavailable: {0}")]
    StateUnavailable(&'static str),
    #[error(transparent)]
    Thread(ThreadError),
}

impl From<crate::Error> for StreamError {
    fn from(err: crate::Error) -> Self {
        match err {
            crate::Error::Cancelled => StreamError::Cancelled,
            crate::Error::Sink(sink_err) => StreamError::Channel(sink_err.to_string()),
            other => StreamError::Thread(ThreadError::from(other)),
        }
    }
}
