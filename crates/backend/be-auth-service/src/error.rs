use crate::crypto::CryptoError;
use thiserror::Error;
use tonic_types::{ErrorDetails, StatusExt};

/// gRPC error domain for auth-service structured error details.
///
/// Clients dispatch on the `reason` field attached via `google.rpc.ErrorInfo`
/// rather than on free-form status messages.
pub const AUTH_ERROR_DOMAIN: &str = "auth.eurora-labs.com";

/// Reason code for [`AuthError::OAuthEmailConflict`]. Stable across releases.
pub const OAUTH_EMAIL_CONFLICT_REASON: &str = "OAUTH_EMAIL_CONFLICT";

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

impl From<AuthError> for tonic::Status {
    fn from(err: AuthError) -> Self {
        match err {
            AuthError::MissingCredentials | AuthError::InvalidInput(_) => {
                tonic::Status::invalid_argument(err.to_string())
            }

            AuthError::InvalidCredentials
            | AuthError::MissingAuthHeader
            | AuthError::InvalidAuthHeader
            | AuthError::InvalidToken
            | AuthError::EmailNotVerified => tonic::Status::unauthenticated(err.to_string()),

            AuthError::OAuthEmailConflict => {
                let details = ErrorDetails::with_error_info(
                    OAUTH_EMAIL_CONFLICT_REASON,
                    AUTH_ERROR_DOMAIN,
                    std::collections::HashMap::<String, String>::new(),
                );
                tonic::Status::with_error_details(
                    tonic::Code::FailedPrecondition,
                    err.to_string(),
                    details,
                )
            }

            AuthError::PasswordHash(ref msg) => {
                tracing::error!("Password hashing error: {msg}");
                tonic::Status::internal("Authentication error")
            }
            AuthError::TokenGeneration(ref msg) => {
                tracing::error!("Token generation error: {msg}");
                tonic::Status::internal("Token generation error")
            }
            AuthError::Database(ref e) => {
                tracing::error!("Database error: {e}");
                tonic::Status::internal("Internal database error")
            }
            AuthError::OAuth(ref e) => {
                tracing::error!("OAuth error: {e}");
                tonic::Status::internal("OAuth authentication error")
            }
            AuthError::Crypto(ref e) => {
                tracing::error!("Crypto error: {e}");
                tonic::Status::internal("Internal security error")
            }
            AuthError::Internal(ref msg) => {
                tracing::error!("Internal error: {msg}");
                tonic::Status::internal("Internal error")
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tonic_types::StatusExt;

    #[test]
    fn oauth_email_conflict_maps_to_failed_precondition_with_error_info() {
        let status: tonic::Status = AuthError::OAuthEmailConflict.into();

        assert_eq!(status.code(), tonic::Code::FailedPrecondition);

        let info = status
            .get_error_details()
            .error_info()
            .cloned()
            .expect("error info attached");
        assert_eq!(info.reason, OAUTH_EMAIL_CONFLICT_REASON);
        assert_eq!(info.domain, AUTH_ERROR_DOMAIN);
        assert!(
            info.metadata.is_empty(),
            "metadata must not leak account composition"
        );
    }
}
