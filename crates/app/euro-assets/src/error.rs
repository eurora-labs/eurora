//! Error types for the asset module

use thiserror::Error;

/// Main error type for asset operations
#[derive(Debug, Error)]
pub enum AssetError {}

/// Result type alias for asset operations
pub type AssetResult<T> = std::result::Result<T, AssetError>;
