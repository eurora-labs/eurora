//! Third-party OAuth identity providers.
//!
//! Each provider lives in its own submodule and exposes:
//!
//! - a `*OAuthConfig::from_env` loader,
//! - an opaque client struct holding a shared `reqwest::Client` (so
//!   connection pooling / TLS state survives across requests), and
//! - an `exchange_code` method that turns an authorisation code into a
//!   normalised user-info struct.
//!
//! Cross-provider concerns (linking against an existing user, storing
//! encrypted tokens) live in the `oauth_flow` module on the parent crate.

pub mod github;
pub mod google;

use chrono::{DateTime, Utc};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum OAuthError {
    #[error("missing environment variable: {0}")]
    MissingEnvVar(&'static str),

    #[error("OAuth discovery failed: {0}")]
    Discovery(String),

    #[error("invalid URL: {0}")]
    InvalidUrl(String),

    #[error("OAuth code exchange failed: {0}")]
    CodeExchange(String),

    #[error("OAuth token verification failed: {0}")]
    TokenVerification(String),

    #[error("OAuth user-info fetch failed: {0}")]
    UserInfoFetch(String),

    #[error("OAuth response missing required field: {0}")]
    MissingField(&'static str),

    #[error("OAuth HTTP request failed")]
    Http(#[from] reqwest::Error),
}

/// OAuth identity tokens, already encrypted at rest under the
/// PKCE-encryption keyring.
pub struct OAuthTokenBundle {
    pub encrypted_access_token: Vec<u8>,
    pub encrypted_refresh_token: Option<Vec<u8>>,
    pub access_token_expiry: Option<DateTime<Utc>>,
    pub scope: String,
}

/// A freshly authenticated identity returned by an OAuth provider.
///
/// The auth-service either (a) finds an existing user via
/// `(provider, provider_user_id)` and refreshes their stored tokens, or
/// (b) creates a brand-new user. It never silently links to an existing
/// account on email match — that path is a known account-takeover vector
/// and is rejected with [`crate::AuthError::OAuthEmailConflict`].
pub struct NewOAuthIdentity {
    pub provider: be_remote_db::OAuthProvider,
    pub provider_user_id: String,
    pub email: String,
    pub email_verified: bool,
    pub display_name: Option<String>,
    pub tokens: OAuthTokenBundle,
}
