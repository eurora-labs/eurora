//! Error types for the activity system

use orion::errors::UnknownCryptoError;
use thiserror::Error;

/// Errors that can occur in the activity system
#[derive(Error, Debug)]
pub enum EncryptError {
    #[error("cryptography error: {0}")]
    CryptoError(#[from] UnknownCryptoError),

    #[error("base64 decode error")]
    Base64DecodeError(#[from] base64::DecodeError),

    #[error("invalid key length")]
    InvalidKeyLength,

    #[error("Key error {0}")]
    KeyError(#[from] anyhow::Error),
}

/// Result type alias for encryption operations
pub type EncryptResult<T> = std::result::Result<T, EncryptError>;
