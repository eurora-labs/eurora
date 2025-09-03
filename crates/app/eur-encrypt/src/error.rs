//! Error types for the activity system

// use orion::errors::UnknownCryptoError;
use thiserror::Error;

/// Errors that can occur in the activity system
#[derive(Error, Debug)]
pub enum EncryptError {
    // #[error("cryptography error: {0}")]
    // CryptoError(#[from] UnknownCryptoError),
    #[error("base64 decode error")]
    Base64Decode(#[from] base64::DecodeError),

    #[error("invalid key length")]
    InvalidKeyLength,

    #[error("Key error {0}")]
    Key(#[from] anyhow::Error),

    /// Format error
    #[error("Format error: {0}")]
    Format(String),

    #[error("Encryption error: {0}")]
    Encryption(#[from] chacha20poly1305::Error),
}

/// Result type alias for encryption operations
pub type EncryptResult<T> = std::result::Result<T, EncryptError>;
