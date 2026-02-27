use crate::crypto::CryptoError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AuthError {
    // 400 - InvalidArgument
    #[error("Missing credentials")]
    MissingCredentials,
    #[error("{0}")]
    InvalidInput(String),

    // 401 - Unauthenticated
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

    // 500 - Internal
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
