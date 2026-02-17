use std::path::PathBuf;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("failed to read settings from {path}")]
    Read {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("failed to write settings to {path}")]
    Write {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("failed to parse settings")]
    Parse(#[from] serde_json::Error),

    #[error("settings lock poisoned")]
    LockPoisoned,

    #[error("file watcher error")]
    Watcher(#[from] notify::Error),

    #[error("secret storage error: {0}")]
    Secret(String),

    #[error("failed to sync provider settings: {0}")]
    Sync(String),
}
