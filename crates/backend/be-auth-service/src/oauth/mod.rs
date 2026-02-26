pub mod github;
pub mod google;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum OAuthError {
    #[error("Missing environment variable: {0}")]
    MissingEnvVar(&'static str),
    #[error("OAuth discovery failed: {0}")]
    Discovery(String),
    #[error("Invalid URL: {0}")]
    InvalidUrl(String),
    #[error("Code exchange failed: {0}")]
    CodeExchange(String),
    #[error("Missing ID token")]
    MissingIdToken,
    #[error("Token verification failed: {0}")]
    TokenVerification(String),
    #[error("Failed to fetch user info: {0}")]
    UserInfoFetch(String),
    #[error("Missing email")]
    MissingEmail,
    #[error("HTTP client error: {0}")]
    HttpClient(String),
}
