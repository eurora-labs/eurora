use crate::crypto::CryptoError;
use thiserror::Error;
use tracing::error;

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

    // 409 - AlreadyExists
    #[error("Username already exists")]
    UsernameExists,
    #[error("Email already exists")]
    EmailExists,

    // 500 - Internal
    #[error("Password hashing failed: {0}")]
    PasswordHash(String),
    #[error("Token generation failed: {0}")]
    TokenGeneration(String),
    #[error("Database error: {0}")]
    Database(#[from] be_remote_db::DbError),
    #[error("OAuth error: {0}")]
    OAuth(#[from] crate::oauth::google::OAuthError),
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

            AuthError::UsernameExists | AuthError::EmailExists => {
                tonic::Status::already_exists(err.to_string())
            }

            AuthError::PasswordHash(ref msg) => {
                error!("Password hashing error: {msg}");
                tonic::Status::internal("Authentication error")
            }
            AuthError::TokenGeneration(ref msg) => {
                error!("Token generation error: {msg}");
                tonic::Status::internal("Token generation error")
            }
            AuthError::Database(ref e) => {
                error!("Database error: {e}");
                tonic::Status::internal("Internal database error")
            }
            AuthError::OAuth(ref e) => {
                error!("OAuth error: {e}");
                tonic::Status::internal("OAuth authentication error")
            }
            AuthError::Crypto(ref e) => {
                error!("Crypto error: {e}");
                tonic::Status::internal("Internal security error")
            }
            AuthError::Internal(ref msg) => {
                error!("Internal error: {msg}");
                tonic::Status::internal("Internal error")
            }
        }
    }
}
