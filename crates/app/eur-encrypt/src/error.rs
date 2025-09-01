//! Error types for the activity system

use thiserror::Error;

/// Errors that can occur in the activity system
#[derive(Error, Debug)]
pub enum EncryptError {}

impl EncryptError {}

/// Result type alias for activity operations
pub type EncryptResult<T> = std::result::Result<T, EncryptError>;
