use thiserror::Error;

#[derive(Error, Debug)]
pub enum EncryptError {
    #[error("base64 decode error")]
    Base64Decode(#[from] base64::DecodeError),

    #[error("invalid key length")]
    InvalidKeyLength,

    #[error("Key error {0}")]
    Key(#[from] anyhow::Error),

    #[error("Format error: {0}")]
    Format(String),

    #[error("Encryption error: {0}")]
    Encryption(#[from] chacha20poly1305::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

pub type EncryptResult<T> = std::result::Result<T, EncryptError>;
