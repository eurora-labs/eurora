//! Error types for the activity system

use orion::errors::UnknownCryptoError;
use thiserror::Error;

/// Errors that can occur in the activity system
#[derive(Error, Debug)]
pub enum EncryptError {
    #[error("Image processing error: {0}")]
    CryptoError(#[from] UnknownCryptoError),
}

impl EncryptError {}

/// Result type alias for activity operations
pub type EncryptResult<T> = std::result::Result<T, EncryptError>;
