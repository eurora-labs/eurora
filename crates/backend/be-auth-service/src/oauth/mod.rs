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

pub mod apple;
pub mod github;
pub mod google;
pub mod provider_ext;

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

    #[error("invalid OAuth configuration: {0}")]
    InvalidConfig(&'static str),

    #[error("OAuth code exchange failed: {0}")]
    CodeExchange(String),

    #[error("OAuth token verification failed: {0}")]
    TokenVerification(String),

    #[error("OAuth user-info fetch failed: {0}")]
    UserInfoFetch(String),

    #[error("OAuth response missing required field: {0}")]
    MissingField(&'static str),

    /// Minting the Apple `client_secret` JWT failed. The underlying
    /// `jsonwebtoken` error is preserved as a structured source so the
    /// log boundary can render it without leaking key material into
    /// `Display` output.
    #[error("OAuth client-secret JWT mint failed")]
    ClientSecretMint(#[source] jsonwebtoken::errors::Error),

    #[error("OAuth HTTP request failed")]
    Http(#[from] reqwest::Error),
}

/// OAuth identity tokens, already encrypted at rest under the
/// PKCE-encryption keyring.
///
/// `encrypted_access_token` is `Option` rather than a `Vec<u8>` with
/// a "treat empty as absent" sentinel: Apple's web flow and the
/// native-ID-token flows don't hand the relying party a usable access
/// token at all, and modelling that absence explicitly removes a
/// class of "did the caller forget to populate it?" bugs.
pub struct OAuthTokenBundle {
    pub encrypted_access_token: Option<Vec<u8>>,
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
