//! Error types for the Asset Service.
//!
//! This module provides structured error handling using `thiserror` for
//! deriving error implementations and proper conversion to gRPC `Status`.

use thiserror::Error;
use tonic::Status;

/// The main error type for the Asset Service.
///
/// This enum categorizes all possible errors that can occur in the service,
/// enabling type-safe error handling and consistent conversion to gRPC status codes.
#[derive(Debug, Error)]
pub enum AssetServiceError {
    // === Authentication Errors ===
    /// Missing authentication claims in the request.
    #[error("missing authentication claims")]
    MissingClaims,

    #[error("Asset: {0}")]
    Asset(#[source] be_asset::AssetError),

    #[error("Internal error: {0}")]
    Internal(String),
}

impl From<AssetServiceError> for Status {
    fn from(err: AssetServiceError) -> Self {
        use AssetServiceError::*;

        match &err {
            // Authentication errors -> UNAUTHENTICATED
            MissingClaims => Status::unauthenticated(err.to_string()),
            Asset(err) => Status::internal(err.to_string()),
            Internal(err) => Status::internal(err.to_string()),
        }
    }
}

/// A specialized Result type for Asset Service operations.
pub type Result<T> = std::result::Result<T, AssetServiceError>;
