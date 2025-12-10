use std::sync::PoisonError;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum EuroFocusError {
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

impl EuroFocusError {
    pub fn new<S: ToString>(err: S) -> Self {
        EuroFocusError::Error(err.to_string())
    }
}

pub type EuroFocusResult<T> = Result<T, EuroFocusError>;

impl<T> From<PoisonError<T>> for EuroFocusError {
    fn from(value: PoisonError<T>) -> Self {
        EuroFocusError::StdSyncPoisonError(value.to_string())
    }
}
