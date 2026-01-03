use std::sync::PoisonError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum FocusTrackerError {
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

impl FocusTrackerError {
    pub fn new<S: ToString>(err: S) -> Self {
        FocusTrackerError::Error(err.to_string())
    }
}

pub type FocusTrackerResult<T> = Result<T, FocusTrackerError>;

impl<T> From<PoisonError<T>> for FocusTrackerError {
    fn from(value: PoisonError<T>) -> Self {
        FocusTrackerError::StdSyncPoisonError(value.to_string())
    }
}
