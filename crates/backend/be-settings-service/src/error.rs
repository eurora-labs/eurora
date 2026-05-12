use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use be_auth_core::{InvalidUserId, MissingClaims};
use serde::Serialize;
use thiserror::Error;

/// Wire envelope for error responses emitted by this service.
///
/// Defined locally rather than in `settings-core` because the envelope is a
/// server-policy concern: clients of the settings API only need to know that
/// failures carry a `{ error, message }` JSON body with the matching HTTP
/// status code.
///
/// The `error` discriminator is a `&'static str` — every value originates
/// from [`SettingsServiceError::error_kind`], which returns a literal.
#[derive(Debug, Clone, Serialize)]
pub struct SettingsErrorResponse {
    /// Stable machine identifier (e.g. `not_found`, `conflict`).
    pub error: &'static str,
    /// Human-readable description. Safe to surface in client UIs.
    pub message: String,
    /// Optional structured details. Reserved for future use.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}

#[derive(Error, Debug)]
pub enum SettingsServiceError {
    #[error("Authentication failed: {0}")]
    Unauthenticated(String),

    #[error("Invalid argument: {0}")]
    InvalidArgument(String),

    #[error("Settings not found for user")]
    NotFound,

    #[error("Database error: {0}")]
    Database(#[source] be_remote_db::DbError),

    #[error("Internal error: {0}")]
    Internal(String),
}

impl SettingsServiceError {
    pub fn unauthenticated(msg: impl Into<String>) -> Self {
        Self::Unauthenticated(msg.into())
    }

    pub fn invalid_argument(msg: impl Into<String>) -> Self {
        Self::InvalidArgument(msg.into())
    }

    pub fn internal(msg: impl Into<String>) -> Self {
        Self::Internal(msg.into())
    }

    /// Stable identifier surfaced to clients in the error envelope.
    pub fn error_kind(&self) -> &'static str {
        match self {
            Self::Unauthenticated(_) => "unauthenticated",
            Self::InvalidArgument(_) => "invalid_argument",
            Self::NotFound => "not_found",
            Self::Database(_) => "database_error",
            Self::Internal(_) => "internal_error",
        }
    }

    fn status(&self) -> StatusCode {
        match self {
            Self::Unauthenticated(_) => StatusCode::UNAUTHORIZED,
            Self::InvalidArgument(_) => StatusCode::BAD_REQUEST,
            Self::NotFound => StatusCode::NOT_FOUND,
            Self::Database(_) | Self::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl From<be_remote_db::DbError> for SettingsServiceError {
    fn from(err: be_remote_db::DbError) -> Self {
        use be_remote_db::DbError;
        match err {
            DbError::NotFound { .. } => Self::NotFound,
            DbError::ForeignKeyViolation { entity } => {
                Self::InvalidArgument(format!("Referenced {entity} does not exist"))
            }
            DbError::InvalidInput(msg) => Self::InvalidArgument(msg),
            other => Self::Database(other),
        }
    }
}

impl From<MissingClaims> for SettingsServiceError {
    fn from(_: MissingClaims) -> Self {
        Self::unauthenticated("Missing authenticated claims")
    }
}

impl From<InvalidUserId> for SettingsServiceError {
    fn from(err: InvalidUserId) -> Self {
        Self::unauthenticated(err.to_string())
    }
}

impl IntoResponse for SettingsServiceError {
    fn into_response(self) -> Response {
        let status = self.status();
        let kind = self.error_kind();
        let detail = self.to_string();

        match &self {
            Self::Unauthenticated(_) => {
                tracing::warn!(error = %detail, "Settings service authentication error");
            }
            Self::InvalidArgument(_) => {
                tracing::debug!(error = %detail, "Settings service client error");
            }
            Self::NotFound => {
                tracing::debug!("Settings service resource not found");
            }
            Self::Database(_) | Self::Internal(_) => {
                tracing::error!(error = %detail, "Settings service internal error");
            }
        }

        let client_message = match &self {
            Self::Database(_) => "Database operation failed".to_string(),
            Self::Internal(_) => "Internal server error".to_string(),
            _ => detail,
        };

        (
            status,
            Json(SettingsErrorResponse {
                error: kind,
                message: client_message,
                details: None,
            }),
        )
            .into_response()
    }
}

pub type SettingsResult<T> = std::result::Result<T, SettingsServiceError>;

#[cfg(test)]
mod tests {
    use super::*;
    use be_remote_db::DbError;

    #[test]
    fn unauthenticated_maps_to_401() {
        let err = SettingsServiceError::unauthenticated("Missing claims");
        assert_eq!(err.status(), StatusCode::UNAUTHORIZED);
        assert_eq!(err.error_kind(), "unauthenticated");
    }

    #[test]
    fn invalid_argument_maps_to_400() {
        let err = SettingsServiceError::invalid_argument("bad");
        assert_eq!(err.status(), StatusCode::BAD_REQUEST);
        assert_eq!(err.error_kind(), "invalid_argument");
    }

    #[test]
    fn not_found_maps_to_404() {
        let err = SettingsServiceError::NotFound;
        assert_eq!(err.status(), StatusCode::NOT_FOUND);
        assert_eq!(err.error_kind(), "not_found");
    }

    #[test]
    fn db_not_found_maps_to_404() {
        let err: SettingsServiceError = DbError::not_found_with_id("user_settings", "abc").into();
        assert_eq!(err.status(), StatusCode::NOT_FOUND);
        assert_eq!(err.error_kind(), "not_found");
    }

    #[test]
    fn db_foreign_key_maps_to_400() {
        let err: SettingsServiceError = DbError::foreign_key("user").into();
        assert_eq!(err.status(), StatusCode::BAD_REQUEST);
        assert_eq!(err.error_kind(), "invalid_argument");
    }

    #[test]
    fn db_invalid_input_maps_to_400() {
        let err: SettingsServiceError = DbError::invalid_input("nope").into();
        assert_eq!(err.status(), StatusCode::BAD_REQUEST);
    }

    #[test]
    fn db_pool_error_maps_to_500() {
        let err: SettingsServiceError = DbError::pool("timed out").into();
        assert_eq!(err.status(), StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(err.error_kind(), "database_error");
    }

    #[test]
    fn missing_claims_maps_to_401() {
        let err: SettingsServiceError = MissingClaims.into();
        assert_eq!(err.status(), StatusCode::UNAUTHORIZED);
    }
}
