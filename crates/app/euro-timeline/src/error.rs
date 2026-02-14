use thiserror::Error;

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
    Activity(#[from] crate::ActivityError),

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

pub type TimelineResult<T> = std::result::Result<T, TimelineError>;
