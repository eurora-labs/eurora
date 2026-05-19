use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use be_auth_core::{InvalidUserId, MissingClaims};
use thiserror::Error;
use thread_core::ThreadErrorResponse;

#[derive(Error, Debug)]
pub enum ThreadServiceError {
    #[error("Authentication failed: {0}")]
    Unauthenticated(String),

    #[error("Invalid argument: {0}")]
    InvalidArgument(String),

    /// The client violated the chat-WebSocket frame protocol — e.g. sent
    /// `Send` before `CapabilityUpdate`, or declared two tools with the
    /// same name. Surfaced to the client as
    /// `ChatServerMessage::Error { kind: "protocol", ... }`.
    #[error("Protocol violation: {0}")]
    ProtocolViolation(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Conflict: {0}")]
    Conflict(String),

    #[error("Database error: {0}")]
    Database(#[source] be_remote_db::DbError),

    #[error("Storage error: {0}")]
    Storage(#[source] be_storage::StorageError),

    #[error("Asset error: {0}")]
    Asset(#[source] be_asset::AssetError),

    #[error("Invalid base64 for field '{field}': {source}")]
    InvalidBase64 {
        field: &'static str,
        #[source]
        source: base64::DecodeError,
    },

    #[error("Internal error: {0}")]
    Internal(String),
}

impl ThreadServiceError {
    pub fn unauthenticated(msg: impl Into<String>) -> Self {
        Self::Unauthenticated(msg.into())
    }

    pub fn invalid_argument(msg: impl Into<String>) -> Self {
        Self::InvalidArgument(msg.into())
    }

    pub fn protocol_violation(msg: impl Into<String>) -> Self {
        Self::ProtocolViolation(msg.into())
    }

    pub fn not_found(msg: impl Into<String>) -> Self {
        Self::NotFound(msg.into())
    }

    pub fn invalid_base64(field: &'static str, source: base64::DecodeError) -> Self {
        Self::InvalidBase64 { field, source }
    }

    /// Stable string identifier surfaced to the client and used for
    /// analytics counters.
    pub fn error_kind(&self) -> &'static str {
        match self {
            Self::Unauthenticated(_) => "unauthenticated",
            Self::InvalidArgument(_) => "invalid_argument",
            Self::ProtocolViolation(_) => "protocol",
            Self::NotFound(_) => "not_found",
            Self::Conflict(_) => "conflict",
            Self::Database(_) => "database_error",
            Self::Storage(_) => "storage_error",
            Self::Asset(_) => "asset_error",
            Self::InvalidBase64 { .. } => "invalid_base64",
            Self::Internal(_) => "internal_error",
        }
    }

    fn status(&self) -> StatusCode {
        match self {
            Self::Unauthenticated(_) => StatusCode::UNAUTHORIZED,
            Self::InvalidArgument(_) | Self::ProtocolViolation(_) | Self::InvalidBase64 { .. } => {
                StatusCode::BAD_REQUEST
            }
            Self::NotFound(_) => StatusCode::NOT_FOUND,
            Self::Conflict(_) => StatusCode::CONFLICT,
            Self::Database(_) | Self::Storage(_) | Self::Asset(_) | Self::Internal(_) => {
                StatusCode::INTERNAL_SERVER_ERROR
            }
        }
    }
}

impl From<be_remote_db::DbError> for ThreadServiceError {
    fn from(err: be_remote_db::DbError) -> Self {
        use be_remote_db::DbError;
        match err {
            DbError::NotFound { entity, id } => Self::NotFound(match id {
                Some(id) => format!("{entity} {id}"),
                None => entity.to_string(),
            }),
            DbError::UniqueViolation { constraint } => {
                Self::Conflict(format!("Unique constraint violated: {constraint}"))
            }
            DbError::ForeignKeyViolation { entity } => {
                Self::InvalidArgument(format!("Referenced {entity} does not exist"))
            }
            DbError::InvalidInput(msg) => Self::InvalidArgument(msg),
            other => Self::Database(other),
        }
    }
}

impl From<be_storage::StorageError> for ThreadServiceError {
    fn from(err: be_storage::StorageError) -> Self {
        Self::Storage(err)
    }
}

impl From<be_asset::AssetError> for ThreadServiceError {
    fn from(err: be_asset::AssetError) -> Self {
        use be_asset::AssetError;
        match err {
            AssetError::EmptyContent
            | AssetError::MissingMimeType
            | AssetError::UnsupportedMimeType(_)
            | AssetError::MimeTypeMismatch => Self::InvalidArgument(err.to_string()),
            other => Self::Asset(other),
        }
    }
}

impl From<MissingClaims> for ThreadServiceError {
    fn from(_: MissingClaims) -> Self {
        Self::unauthenticated("Missing authenticated claims")
    }
}

impl From<InvalidUserId> for ThreadServiceError {
    fn from(err: InvalidUserId) -> Self {
        Self::unauthenticated(err.to_string())
    }
}

impl IntoResponse for ThreadServiceError {
    fn into_response(self) -> Response {
        let status = self.status();
        let kind = self.error_kind();
        let detail = self.to_string();

        match &self {
            Self::Unauthenticated(_) => {
                tracing::warn!(error = %detail, "Thread service authentication error");
            }
            Self::InvalidArgument(_) | Self::InvalidBase64 { .. } => {
                tracing::debug!(error = %detail, "Thread service client error");
            }
            Self::ProtocolViolation(_) => {
                tracing::info!(error = %detail, "Thread service protocol violation");
            }
            Self::NotFound(_) => {
                tracing::debug!(error = %detail, "Thread service resource not found");
            }
            Self::Conflict(_) => {
                tracing::info!(error = %detail, "Thread service conflict");
            }
            Self::Database(_) | Self::Storage(_) | Self::Asset(_) | Self::Internal(_) => {
                tracing::error!(error = %detail, "Thread service internal error");
            }
        }

        let client_message = match self {
            Self::Database(_) => "Database operation failed".to_string(),
            Self::Storage(_) => "Storage operation failed".to_string(),
            Self::Asset(_) => "Asset operation failed".to_string(),
            Self::Internal(_) => "Internal server error".to_string(),
            other => other.to_string(),
        };

        (
            status,
            Json(ThreadErrorResponse {
                error: kind.to_owned(),
                message: client_message,
                details: None,
            }),
        )
            .into_response()
    }
}

pub type ThreadServiceResult<T> = std::result::Result<T, ThreadServiceError>;

#[cfg(test)]
mod tests {
    use super::*;
    use be_remote_db::DbError;

    #[test]
    fn unauthenticated_maps_to_401() {
        let err = ThreadServiceError::unauthenticated("Missing claims");
        assert_eq!(err.status(), StatusCode::UNAUTHORIZED);
    }

    #[test]
    fn not_found_maps_to_404() {
        let err = ThreadServiceError::not_found("Thread 123");
        assert_eq!(err.status(), StatusCode::NOT_FOUND);
    }

    #[test]
    fn invalid_argument_maps_to_400() {
        let err = ThreadServiceError::invalid_argument("title is required");
        assert_eq!(err.status(), StatusCode::BAD_REQUEST);
    }

    #[test]
    fn protocol_violation_maps_to_400_with_protocol_kind() {
        let err = ThreadServiceError::protocol_violation("capability_update must precede send");
        assert_eq!(err.status(), StatusCode::BAD_REQUEST);
        assert_eq!(err.error_kind(), "protocol");
    }

    #[test]
    fn db_not_found_maps_to_404() {
        let err: ThreadServiceError = DbError::not_found_with_id("thread", "abc").into();
        assert_eq!(err.status(), StatusCode::NOT_FOUND);
    }

    #[test]
    fn db_unique_violation_maps_to_409() {
        let err: ThreadServiceError = DbError::unique_violation("threads_pkey").into();
        assert_eq!(err.status(), StatusCode::CONFLICT);
    }

    #[test]
    fn db_foreign_key_maps_to_400() {
        let err: ThreadServiceError = DbError::foreign_key("thread").into();
        assert_eq!(err.status(), StatusCode::BAD_REQUEST);
    }

    #[test]
    fn missing_claims_maps_to_401() {
        let err: ThreadServiceError = MissingClaims.into();
        assert_eq!(err.status(), StatusCode::UNAUTHORIZED);
    }

    #[test]
    fn asset_validation_error_maps_to_400() {
        let err: ThreadServiceError = be_asset::AssetError::EmptyContent.into();
        assert_eq!(err.status(), StatusCode::BAD_REQUEST);
    }
}
