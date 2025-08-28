//! Error types for the timeline module

use thiserror::Error;

/// Main error type for timeline operations
#[derive(Debug, Error)]
pub enum TimelineError {
    #[error("Storage error: {0}")]
    Storage(String),

    #[error("Collection error: {0}")]
    Collection(String),

    #[error("Focus tracking error: {0}")]
    FocusTracking(String),

    #[error("Configuration error: {0}")]
    Configuration(String),

    #[error("Activity strategy error: {0}")]
    ActivityStrategy(#[from] crate::ActivityError),

    #[error("Join error: {0}")]
    Join(#[from] tokio::task::JoinError),

    #[error("Channel send error")]
    ChannelSend,

    #[error("Channel receive error")]
    ChannelReceive,

    #[error("Timeline is not running")]
    NotRunning,

    #[error("Timeline is already running")]
    AlreadyRunning,
}

/// Result type alias for timeline operations
pub type Result<T> = std::result::Result<T, TimelineError>;
