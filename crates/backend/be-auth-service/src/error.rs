use auth_core::{AuthErrorResponse, error_kinds};
use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use thiserror::Error;

use crate::crypto::CryptoError;

pub type AuthResult<T> = std::result::Result<T, AuthError>;

#[derive(Debug, Error)]
pub enum AuthError {
    #[error("Missing credentials")]
    MissingCredentials,
    #[error("{0}")]
    InvalidInput(String),

    #[error("Invalid credentials")]
    InvalidCredentials,
    #[error("Missing authorization header")]
    MissingAuthHeader,
    #[error("Invalid authorization header format")]
    InvalidAuthHeader,
    #[error("Invalid or expired token")]
    InvalidToken,
    #[error("Email address is not verified")]
    EmailNotVerified,

    /// The OAuth provider returned an email that is already registered to a
    /// different identity (password credentials or another OAuth provider).
    ///
    /// Auto-linking is intentionally rejected — the user must first sign in
    /// with their original method and explicitly link the new provider from
    /// account settings. The message is deliberately generic to avoid
    /// disclosing which sign-in methods are attached to the account.
    #[error("An account with this email already exists under a different sign-in method")]
    OAuthEmailConflict,

    /// Caller is asking for another verification email before the resend
    /// cooldown has elapsed.
    #[error("Please wait before requesting another verification email")]
    VerificationResendCooldown,

    #[error("Email is already verified")]
    EmailAlreadyVerified,

    #[error("Password hashing failed: {0}")]
    PasswordHash(String),
    #[error("Token generation failed: {0}")]
    TokenGeneration(String),
    #[error("Database error: {0}")]
    Database(#[from] be_remote_db::DbError),
    #[error("OAuth error: {0}")]
    OAuth(#[from] crate::oauth::OAuthError),
    #[error("Crypto error: {0}")]
    Crypto(#[from] CryptoError),
    #[error("Internal error: {0}")]
    Internal(String),
}

impl AuthError {
    fn status(&self) -> StatusCode {
        match self {
            AuthError::MissingCredentials | AuthError::InvalidInput(_) => StatusCode::BAD_REQUEST,
            AuthError::InvalidCredentials
            | AuthError::MissingAuthHeader
            | AuthError::InvalidAuthHeader
            | AuthError::InvalidToken => StatusCode::UNAUTHORIZED,
            AuthError::EmailNotVerified => StatusCode::FORBIDDEN,
            AuthError::EmailAlreadyVerified => StatusCode::CONFLICT,
            AuthError::OAuthEmailConflict => StatusCode::CONFLICT,
            AuthError::VerificationResendCooldown => StatusCode::TOO_MANY_REQUESTS,
            AuthError::PasswordHash(_)
            | AuthError::TokenGeneration(_)
            | AuthError::Database(_)
            | AuthError::OAuth(_)
            | AuthError::Crypto(_)
            | AuthError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    /// Stable, machine-readable identifier for the failure mode. Used by
    /// clients to dispatch on specific errors without parsing free-text
    /// messages. Mirrors the `reason` channel that the gRPC version
    /// carried via `google.rpc.ErrorInfo`.
    pub fn error_kind(&self) -> &'static str {
        match self {
            AuthError::MissingCredentials | AuthError::InvalidInput(_) => {
                error_kinds::INVALID_ARGUMENT
            }
            AuthError::InvalidCredentials
            | AuthError::MissingAuthHeader
            | AuthError::InvalidAuthHeader
            | AuthError::InvalidToken => error_kinds::UNAUTHENTICATED,
            AuthError::EmailNotVerified => error_kinds::EMAIL_NOT_VERIFIED,
            AuthError::EmailAlreadyVerified => "email_already_verified",
            AuthError::OAuthEmailConflict => error_kinds::OAUTH_EMAIL_CONFLICT,
            AuthError::VerificationResendCooldown => error_kinds::RATE_LIMITED,
            AuthError::PasswordHash(_)
            | AuthError::TokenGeneration(_)
            | AuthError::Database(_)
            | AuthError::OAuth(_)
            | AuthError::Crypto(_)
            | AuthError::Internal(_) => error_kinds::INTERNAL_ERROR,
        }
    }

    /// Public-facing message: never leaks internal details.
    fn client_message(&self) -> String {
        match self {
            AuthError::PasswordHash(_) => "Authentication error".to_string(),
            AuthError::TokenGeneration(_) => "Token generation error".to_string(),
            AuthError::Database(_) => "Internal database error".to_string(),
            AuthError::OAuth(_) => "OAuth authentication error".to_string(),
            AuthError::Crypto(_) => "Internal security error".to_string(),
            AuthError::Internal(_) => "Internal error".to_string(),
            other => other.to_string(),
        }
    }
}

impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        let status = self.status();
        let kind = self.error_kind();
        let detail = self.to_string();

        match &self {
            AuthError::PasswordHash(msg) => {
                tracing::error!(error = %msg, "password hashing error");
            }
            AuthError::TokenGeneration(msg) => {
                tracing::error!(error = %msg, "token generation error");
            }
            AuthError::Database(e) => {
                tracing::error!(error = %e, "auth-service database error");
            }
            AuthError::OAuth(e) => {
                tracing::error!(error = %e, "auth-service oauth error");
            }
            AuthError::Crypto(e) => {
                tracing::error!(error = %e, "auth-service crypto error");
            }
            AuthError::Internal(msg) => {
                tracing::error!(error = %msg, "auth-service internal error");
            }
            AuthError::OAuthEmailConflict => {
                tracing::warn!("oauth login rejected: email already registered");
            }
            AuthError::EmailNotVerified
            | AuthError::InvalidCredentials
            | AuthError::InvalidToken
            | AuthError::MissingAuthHeader
            | AuthError::InvalidAuthHeader => {
                tracing::debug!(error = %detail, "auth-service auth error");
            }
            AuthError::MissingCredentials
            | AuthError::InvalidInput(_)
            | AuthError::EmailAlreadyVerified
            | AuthError::VerificationResendCooldown => {
                tracing::debug!(error = %detail, "auth-service client error");
            }
        }

        let body = AuthErrorResponse {
            error: kind.to_owned(),
            message: self.client_message(),
            details: None,
        };

        (status, Json(body)).into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn oauth_email_conflict_uses_stable_kind() {
        let err = AuthError::OAuthEmailConflict;
        assert_eq!(err.error_kind(), "oauth_email_conflict");
        assert_eq!(err.status(), StatusCode::CONFLICT);
    }

    #[test]
    fn invalid_credentials_maps_to_401() {
        let err = AuthError::InvalidCredentials;
        assert_eq!(err.error_kind(), "unauthenticated");
        assert_eq!(err.status(), StatusCode::UNAUTHORIZED);
    }

    #[test]
    fn email_not_verified_maps_to_403_with_kind() {
        let err = AuthError::EmailNotVerified;
        assert_eq!(err.error_kind(), "email_not_verified");
        assert_eq!(err.status(), StatusCode::FORBIDDEN);
    }

    #[test]
    fn internal_errors_dont_leak_detail() {
        let err = AuthError::Database(be_remote_db::DbError::not_found("user"));
        assert_eq!(err.client_message(), "Internal database error");
        assert_eq!(err.error_kind(), "internal_error");
    }

    #[test]
    fn invalid_input_passes_message_through() {
        let err = AuthError::InvalidInput("Email already taken".to_string());
        assert_eq!(err.error_kind(), "invalid_argument");
        assert_eq!(err.client_message(), "Email already taken");
    }
}
