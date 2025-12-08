use std::sync::PoisonError;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum FerrousFocusError {
    #[error("{0}")]
    Error(String),

    #[error("StdSyncPoisonError {0}")]
    StdSyncPoisonError(String),

    #[error("Unsupported")]
    Unsupported,

    #[error("Permission denied")]
    PermissionDenied,

    #[error("No display available")]
    NoDisplay,

    #[error("Not running in interactive session")]
    NotInteractiveSession,

    #[error("Platform error: {0}")]
    Platform(String),
}

impl FerrousFocusError {
    pub fn new<S: ToString>(err: S) -> Self {
        FerrousFocusError::Error(err.to_string())
    }
}

pub type FerrousFocusResult<T> = Result<T, FerrousFocusError>;

impl<T> From<PoisonError<T>> for FerrousFocusError {
    fn from(value: PoisonError<T>) -> Self {
        FerrousFocusError::StdSyncPoisonError(value.to_string())
    }
}
