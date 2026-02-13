use std::sync::PoisonError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum FocusTrackerError {
    #[error("unsupported platform or environment")]
    Unsupported,

    #[error("permission denied: {context}")]
    PermissionDenied { context: String },

    #[error("no display available")]
    NoDisplay,

    #[error("not running in interactive session")]
    NotInteractiveSession,

    #[error("channel closed")]
    ChannelClosed,

    #[error("invalid config: {reason}")]
    InvalidConfig { reason: String },

    #[error("{context}")]
    Platform {
        context: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },
}

impl FocusTrackerError {
    pub fn platform(context: impl Into<String>) -> Self {
        Self::Platform {
            context: context.into(),
            source: None,
        }
    }

    pub fn platform_with_source(
        context: impl Into<String>,
        source: impl std::error::Error + Send + Sync + 'static,
    ) -> Self {
        Self::Platform {
            context: context.into(),
            source: Some(Box::new(source)),
        }
    }
}

pub type FocusTrackerResult<T> = Result<T, FocusTrackerError>;

impl<T> From<PoisonError<T>> for FocusTrackerError {
    fn from(value: PoisonError<T>) -> Self {
        FocusTrackerError::platform(format!("mutex poisoned: {value}"))
    }
}
